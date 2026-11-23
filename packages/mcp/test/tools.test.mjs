import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  DEFAULT_QUERY_TIMEOUT_MS,
  withDefaultQueryTimeout,
  degradedHint,
  toolDescription,
  result,
} from '../dist/tools.js';

// The core contract freezes the default query deadline at 2s, so a caller that omits
// `budget` silently gets truncated/partial results. The MCP layer compensates by injecting
// a generous timeout for read tools — these tests pin that behaviour, not the core value.

test('injects the MCP default timeout for read tools that omit budget', () => {
  const request = { operation: 'search', query: 'purchase' };
  const bounded = withDefaultQueryTimeout(request, false);
  assert.equal(DEFAULT_QUERY_TIMEOUT_MS, 30_000);
  assert.deepEqual(bounded.budget, { timeoutMs: DEFAULT_QUERY_TIMEOUT_MS });
  // Input must not be mutated: SDK/MCP share the parsed arguments object.
  assert.equal('budget' in request, false);
});

test('preserves other budget fields and only fills a missing timeoutMs', () => {
  const bounded = withDefaultQueryTimeout(
    { operation: 'trace', budget: { maxEntities: 10, timeoutMs: null } },
    false,
  );
  assert.deepEqual(bounded.budget, { maxEntities: 10, timeoutMs: DEFAULT_QUERY_TIMEOUT_MS });
});

test('an explicit caller timeout always wins, including 0', () => {
  for (const timeoutMs of [5_000, 1, 0]) {
    const request = { operation: 'page', budget: { timeoutMs } };
    assert.equal(withDefaultQueryTimeout(request, false), request);
    assert.deepEqual(withDefaultQueryTimeout(request, false).budget, { timeoutMs });
  }
});

test('status and readSource are exempt: they carry no budget and skip the core deadline', () => {
  for (const operation of ['status', 'readSource']) {
    const request = { operation };
    assert.equal(withDefaultQueryTimeout(request, false), request);
    assert.equal('budget' in request, false);
  }
});

test('maintenance tools are left untouched', () => {
  const request = { operation: 'index', mode: 'base' };
  assert.equal(withDefaultQueryTimeout(request, true), request);
  assert.equal('budget' in request, false);
});

test('degradedHint flags a tripped query deadline (emptied or serialized-overrun)', () => {
  assert.match(
    String(
      degradedHint(
        { completeness: { reasons: ['query deadline reached'] }, status: 'partial' },
        'search',
      ),
    ),
    /truncated/,
  );
  assert.match(
    String(
      degradedHint(
        { completeness: { reasons: ['query deadline reached during response serialization'] } },
        'trace',
      ),
    ),
    /timeoutMs/,
  );
});

test('degradedHint flags an emptied stale result only for bounded reads', () => {
  const stale = { completeness: { freshness: 'stale', reasons: [] }, status: 'partial' };
  assert.match(String(degradedHint(stale, 'page')), /stale/);
  // status/readSource never get emptied, so a stale index there must not read as "no data".
  assert.equal(degradedHint(stale, 'status'), undefined);
  assert.equal(degradedHint(stale, 'readSource'), undefined);
});

test('degradedHint stays silent on ordinary pagination and complete results', () => {
  const paginated = {
    status: 'partial',
    continuation: 'x',
    completeness: {
      freshness: 'fresh',
      truncated: true,
      reasons: ['query budget or page boundary reached'],
    },
  };
  assert.equal(degradedHint(paginated, 'search'), undefined);
  assert.equal(
    degradedHint({ status: 'ok', completeness: { freshness: 'fresh', reasons: [] } }, 'search'),
    undefined,
  );
  assert.equal(degradedHint({ error: { code: 'X' } }, 'search'), undefined);
  assert.equal(degradedHint(null, 'search'), undefined);
});

test('result() prepends the degraded header to text but leaves structuredContent untouched', () => {
  const value = { status: 'partial', completeness: { freshness: 'stale', reasons: [] } };
  const envelope = result(value, false, degradedHint(value, 'trace'));
  assert.equal(envelope.structuredContent, value);
  const text = envelope.content[0].text;
  assert.ok(text.startsWith('⚠️ degraded: results were emptied'));
  assert.deepEqual(JSON.parse(text.slice(text.indexOf('\n') + 1)), envelope.structuredContent);
  // No hint => plain JSON, byte-identical to before.
  assert.equal(result(value).content[0].text, JSON.stringify(value));
});

test('toolDescription advertises budget defaults only for bounded read tools', () => {
  const search = toolDescription('search', false);
  assert.match(search, /timeout 30s \(not the core 2s\)/);
  assert.match(search, /maxEntities 60/);
  assert.match(search, /pageSize 20/);
  const status = toolDescription('status', false);
  assert.ok(!/maxEntities/.test(status));
  assert.equal(
    toolDescription('index', true),
    'Core index. Explicit maintenance; may update project state.',
  );
});
