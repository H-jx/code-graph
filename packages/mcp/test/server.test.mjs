import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  mkdtempSync,
  mkdirSync,
  writeFileSync,
  readFileSync,
  existsSync,
  rmSync,
  renameSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
import { setTimeout as delay } from 'node:timers/promises';
import { Client } from '@modelcontextprotocol/client';
import { StdioClientTransport } from '@modelcontextprotocol/client/stdio';
import { ProjectKnowledge } from '@project-knowledge/sdk';
import { queryTools, maintenanceTools, withDefaultQueryTimeout } from '../dist/tools.js';
import { coreSchema } from '../dist/schema-adapter.js';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../../..');
const entry = join(root, 'packages/mcp/dist/index.js');
const sha = (value) => createHash('sha256').update(value).digest('hex');
function save(name, value) {
  if (!process.env.PK_MCP_EVIDENCE_DIR) return;
  mkdirSync(process.env.PK_MCP_EVIDENCE_DIR, { recursive: true });
  writeFileSync(join(process.env.PK_MCP_EVIDENCE_DIR, name), JSON.stringify(value, null, 2) + '\n');
}
function clean(value) {
  if (Array.isArray(value)) return value.map(clean);
  if (value && typeof value === 'object')
    return Object.fromEntries(
      Object.entries(value)
        .filter(([key]) => !['requestId', 'checkedAt', 'elapsedMs', 'responseChars'].includes(key))
        .map(([key, v]) => [key, clean(v)]),
    );
  return value;
}
async function fixture(t) {
  const dir = mkdtempSync(join(tmpdir(), 'pk-mcp-中文 空格-'));
  mkdirSync(join(dir, 'repo'));
  for (let i = 0; i < 4; i++)
    writeFileSync(
      join(dir, 'repo', `source-${i}.ts`),
      `// 采购 😀\nexport function purchase${i}() {return ${i};}\n`,
    );
  const config = join(dir, 'project.json');
  writeFileSync(
    config,
    JSON.stringify({
      schemaVersion: 1,
      projectId: 'mcp',
      repositories: [{ id: 'r', root: 'repo' }],
      selection: {},
      plugins: { directories: [], autoLoad: false, autoActivate: false },
    }),
  );
  const m = await ProjectKnowledge.openMaintenance({ configPath: config });
  await m.index({ mode: 'base' });
  await m.close();
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  return { dir, config, db: join(dir, 'index.sqlite') };
}
async function connect(t, config, mode = 'query', negotiation = 'legacy') {
  const transport = new StdioClientTransport({
    command: process.execPath,
    args: [entry, '--config', config, '--mode', mode],
    stderr: 'pipe',
  });
  let stderr = '';
  transport.stderr?.on('data', (chunk) => {
    stderr += chunk;
  });
  const client = new Client(
    { name: 'acceptance', version: '0.1.0' },
    { versionNegotiation: { mode: negotiation } },
  );
  await client.connect(transport);
  t.after(async () => {
    await client.close();
    await transport.close();
    assert.equal(stderr, '');
  });
  return { client, transport };
}
async function call(client, name, args = {}) {
  const response = await client.callTool({ name, arguments: args });
  assert.ok(response.structuredContent);
  // The text may lead with a "⚠️ degraded:" header (see result/degradedHint); strip it
  // before asserting the payload still round-trips to structuredContent byte-for-byte.
  const text = String(response.content[0].text);
  const json = text.startsWith('⚠️ degraded:') ? text.slice(text.indexOf('\n') + 1) : text;
  assert.deepEqual(JSON.parse(json), response.structuredContent);
  return response;
}
async function until(check) {
  const end = Date.now() + 15000;
  while (!check()) {
    assert.ok(Date.now() < end, 'condition timed out');
    await delay(15);
  }
}
const alive = (pid) => {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
};

test(
  'official stdio clients negotiate both eras; schemas, all queries, resources, pagination and readonly isolation',
  { timeout: 90000 },
  async (t) => {
    const f = await fixture(t);
    const initial = sha(readFileSync(f.db));
    const sdk = await ProjectKnowledge.open({ configPath: f.config });
    t.after(() => sdk.close());
    const cap = await sdk.getCapabilities();
    const records = [];
    for (const era of ['legacy', { pin: '2026-07-28' }]) {
      const { client } = await connect(t, f.config, 'query', era);
      if (era !== 'legacy') assert.equal(client.getNegotiatedProtocolVersion(), '2026-07-28');
      const { tools } = await client.listTools();
      assert.equal(tools.length, 10);
      for (const tool of tools) {
        assert.equal(tool.annotations.readOnlyHint, true);
        assert.deepEqual(
          tool.inputSchema,
          coreSchema(cap.schema, queryTools[tool.name], false)['~standard'].jsonSchema.input({
            target: 'draft-2020-12',
          }),
        );
      }
      const search = await sdk.search({ query: 'purchase', pageSize: 1 });
      const entity = search.data.results[0].entityRef;
      const requests = {
        index_status: {},
        search: { query: 'purchase', pageSize: 1 },
        locate_feature: { query: 'purchase' },
        trace_links: { roots: [entity], direction: 'both' },
        neighbors: { root: entity, direction: 'both' },
        get_code_map: { query: 'purchase' },
        read_source: { references: search.sourceReferences.slice(0, 1) },
        locate_page: { route: '/missing' },
        explain_workflow: { query: 'purchase' },
        explain_report: { query: 'purchase' },
      };
      for (const [name, args] of Object.entries(requests)) {
        const response = await call(client, name, args);
        assert.notEqual(response.isError, true);
        assert.deepEqual(
          clean(response.structuredContent),
          // The MCP layer injects a default budget.timeoutMs for bounded read tools; apply
          // the same default here so this asserts MCP == SDK plus that documented injection.
          clean(
            await sdk.execute(
              withDefaultQueryTimeout({ ...args, operation: queryTools[name] }, false),
            ),
          ),
        );
        records.push({ era, tool: name, arguments: args, response });
      }
      const first = (await call(client, 'search', requests.search)).structuredContent;
      assert.ok(first.continuation);
      const second = await call(client, 'search', {
        ...requests.search,
        continuation: first.continuation,
      });
      assert.notDeepEqual(first.data.results, second.structuredContent.data.results);
      const invalid = await client
        .callTool({
          name: 'search',
          arguments: { query: 'purchase', mode: 'maintenance', secret: 'do-not-echo' },
        })
        .catch((e) => ({ isError: true, message: e.message }));
      assert.equal(invalid.isError, true);
      assert.ok(!JSON.stringify(invalid).includes('do-not-echo'));
      const forbidden = await client
        .callTool({ name: 'update_index', arguments: { mode: 'base' } })
        .catch((e) => ({ isError: true }));
      assert.equal(forbidden.isError, true);
      const bad = await call(client, 'search', { query: 'purchase', continuation: 'invalid' });
      assert.equal(bad.isError, true);
      assert.ok(bad.structuredContent.error.code);
      assert.equal((await client.listResources()).resources.length, 3);
      assert.equal((await client.listResourceTemplates()).resourceTemplates.length, 2);
      const schema = await client.readResource({ uri: 'project-knowledge://schema' });
      assert.deepEqual(JSON.parse(schema.contents[0].text), cap);
      for (const uri of [
        'project-knowledge://index',
        'project-knowledge://skill-manifest',
        `project-knowledge://facts/${Buffer.from(JSON.stringify(entity)).toString('base64url')}`,
        `project-knowledge://evidence/${Buffer.from(JSON.stringify(search.sourceReferences[0])).toString('base64url')}`,
      ]) {
        const resource = JSON.parse((await client.readResource({ uri })).contents[0].text);
        assert.ok(resource.snapshot);
        assert.ok(resource.completeness);
      }
      await client.close();
    }
    await sdk.close();
    assert.equal(sha(readFileSync(f.db)), initial);
    renameSync(f.db, f.db + '.closed');
    renameSync(f.db + '.closed', f.db);
    save('query-resources.json', {
      passed: true,
      schemaHash: cap.schemaHash,
      records,
      databaseHashUnchanged: initial,
      modelRequests: 0,
    });
  },
);

test('schema works without an index; missing index calls never create a database', async (t) => {
  const f = await fixture(t);
  rmSync(f.db);
  const { client } = await connect(t, f.config);
  assert.equal((await client.listTools()).tools.length, 10);
  await client.readResource({ uri: 'project-knowledge://schema' });
  const response = await call(client, 'index_status');
  assert.equal(response.isError, true);
  assert.equal(existsSync(f.db), false);
});

test(
  'explicit maintenance lifecycle, cancel and disconnect stop real plugin descendants',
  { timeout: 90000 },
  async (t) => {
    const f = await fixture(t);
    const records = [];
    const success = await connect(t, f.config, 'maintenance');
    const plugin = join(f.dir, 'empty');
    mkdirSync(plugin);
    writeFileSync(
      join(plugin, 'collect.mjs'),
      readFileSync(join(root, 'fixtures/plugins/staging/collect.mjs')),
    );
    const path = join(plugin, 'manifest.json');
    writeFileSync(
      path,
      JSON.stringify({
        protocolVersion: '1.0',
        pluginId: 'mcp.empty',
        version: '1.0.0',
        runtime: { kind: 'node', versionRange: '>=24 <25' },
        artifacts: ['manifest.json', 'collect.mjs'],
        stages: [
          {
            id: 'extract',
            phase: 'extract',
            entry: 'collect.mjs',
            args: ['--empty'],
            sourceSelectors: [{ id: 'sources', include: ['**/*.ts'] }],
            entitySelectors: [],
            dependsOn: [],
            replacement: 'project',
          },
        ],
      }),
    );
    const submitted = await call(success.client, 'write_candidate', { path });
    assert.notEqual(submitted.isError, true);
    const candidateId = submitted.structuredContent.id;
    for (const [name, args] of [
      ['prepare_plugin', { candidateId }],
      ['validate_plugin', { candidateIds: [candidateId] }],
      ['run_plugin_cases', { candidateIds: [candidateId] }],
      ['activate_plugin', { candidateId }],
      ['activate_plugin', { candidateId }],
      ['collect', {}],
      ['update_index', { mode: 'incremental' }],
      ['rollback_plugin', { pluginId: 'mcp.empty', version: '1.0.0' }],
    ]) {
      const response = await call(success.client, name, args);
      assert.notEqual(response.isError, true, JSON.stringify(response));
      records.push({ tool: name, arguments: args, response });
    }
    const contextRequest = {
      request: {
        topicId: 'purchase',
        query: { operation: 'search', query: 'purchase' },
        maxChars: 80000,
      },
    };
    for (const name of ['export_snapshot', 'export_topic']) {
      const response = await call(success.client, name, contextRequest);
      assert.notEqual(response.isError, true);
      assert.ok(response.structuredContent.inputDigest);
      records.push({ tool: name, response });
    }
    const context = (await call(success.client, 'export_topic', contextRequest)).structuredContent;
    const artifact = {
      protocolVersion: '1.0',
      topicId: 'purchase',
      title: 'Purchase symbols',
      snapshot: context.snapshot,
      inputDigest: context.inputDigest,
      markdown: 'Original code contains purchase symbols. [[claim:symbol]]',
      claims: [
        {
          id: 'symbol',
          text: 'Original code contains purchase symbols.',
          basis: 'observed',
          origin: 'human',
          evidenceRefs: [context.sourceReferences[0].evidenceId],
          gapIds: [],
        },
      ],
      sourceReferences: context.sourceReferences,
      gaps: context.gaps,
      promptHash: context.promptHash,
      templateHash: context.templateHash,
    };
    const imported = await call(success.client, 'import_topic', { artifact });
    assert.notEqual(imported.isError, true, JSON.stringify(imported));
    const generated = await call(success.client, 'generate_skill', {
      name: 'purchase',
      output: join(f.dir, '.agents/skills/purchase'),
    });
    assert.notEqual(generated.isError, true, JSON.stringify(generated));
    const conventions = await call(success.client, 'import_conventions', {
      input: { expectedConfigHash: 'invalid', notes: 'Unverified', sourceReferences: [] },
    });
    assert.equal(conventions.isError, true);
    records.push(
      { tool: 'import_topic', response: imported },
      { tool: 'generate_skill', response: generated },
      { tool: 'import_conventions', response: conventions },
    );
    await success.client.close();
    for (const stop of ['cancel', 'disconnect']) {
      const { client } = await connect(t, f.config, 'maintenance');
      const tools = (await client.listTools()).tools;
      assert.equal(tools.length, 10 + Object.keys(maintenanceTools).length);
      assert.equal(
        tools.find((tool) => tool.name === 'activate_plugin').annotations.readOnlyHint,
        false,
      );
      assert.notEqual((await call(client, 'update_index', { mode: 'base' })).isError, true);
      assert.notEqual((await call(client, 'discover_plugins')).isError, true);
      const plugin = join(f.dir, stop);
      mkdirSync(plugin);
      const marker = join(plugin, 'pids.json');
      writeFileSync(
        join(plugin, 'collect.mjs'),
        readFileSync(join(root, 'fixtures/plugins/staging/collect.mjs')),
      );
      const manifest = {
        protocolVersion: '1.0',
        pluginId: `mcp.${stop}`,
        version: '1.0.0',
        runtime: { kind: 'node', versionRange: '>=24 <25' },
        artifacts: ['manifest.json', 'collect.mjs'],
        stages: [
          {
            id: 'extract',
            phase: 'extract',
            entry: 'collect.mjs',
            args: ['--hold', marker],
            sourceSelectors: [{ id: 'sources', include: ['**/*.ts'] }],
            entitySelectors: [],
            dependsOn: [],
            replacement: 'project',
          },
        ],
        limits: {
          timeoutMs: 30000,
          maxInputBytes: 33554432,
          maxOutputBytes: 33554432,
          maxStderrBytes: 1048576,
        },
      };
      const path = join(plugin, 'manifest.json');
      writeFileSync(path, JSON.stringify(manifest));
      const submitted = await call(client, 'write_candidate', { path });
      assert.notEqual(submitted.isError, true);
      const candidateId = submitted.structuredContent.id;
      assert.ok(candidateId);
      assert.notEqual((await call(client, 'prepare_plugin', { candidateId })).isError, true);
      const controller = new AbortController();
      const work = client.callTool(
        { name: 'validate_plugin', arguments: { candidateIds: [candidateId] } },
        { signal: controller.signal },
      );
      const outcome = work.then(
        (value) => ({ value }),
        (error) => ({ error: String(error) }),
      );
      await until(() => existsSync(marker));
      const pids = JSON.parse(readFileSync(marker, 'utf8'));
      assert.ok(alive(pids.parent) && alive(pids.descendant));
      if (stop === 'cancel') controller.abort();
      else await client.close();
      await outcome;
      await until(() => !alive(pids.parent) && !alive(pids.descendant));
      if (stop === 'cancel') assert.notEqual((await call(client, 'index_status')).isError, true);
      await client.close();
      records.push({ stop, pids, treeStopped: true });
    }
    renameSync(f.db, f.db + '.closed');
    renameSync(f.db + '.closed', f.db);
    save('maintenance-cancellation.json', { passed: true, records, modelRequests: 0 });
  },
);
