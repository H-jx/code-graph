// Runs inside the Rust fixture lifetime, against the exact database used by Core/CLI.
import assert from 'node:assert/strict';
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { createRequire } from 'node:module';
import { createHash } from 'node:crypto';
import { ProjectKnowledge } from '../packages/sdk/dist/index.js';
import { queryTools } from '../packages/mcp/dist/tools.js';
const require = createRequire(new URL('../packages/mcp/package.json', import.meta.url));
const { Client } = require('@modelcontextprotocol/client');
const { StdioClientTransport } = require('@modelcontextprotocol/client/stdio');
const [config, requestPath, expectedPath, directory] = process.argv.slice(2);
const request = JSON.parse(readFileSync(requestPath, 'utf8'));
const expected = JSON.parse(readFileSync(expectedPath, 'utf8'));
function clean(v) {
  return Array.isArray(v)
    ? v.map(clean)
    : v && typeof v === 'object'
      ? Object.fromEntries(
          Object.entries(v)
            .filter(([k]) => !['requestId', 'checkedAt', 'elapsedMs', 'responseChars'].includes(k))
            .map(([k, c]) => [k, clean(c)]),
        )
      : v;
}
const results = { cli: expected };
const sdk = await ProjectKnowledge.open({ configPath: config });
try {
  results.sdk = await sdk.execute(request);
  assert.deepEqual(clean(results.sdk), clean(expected));
} finally {
  await sdk.close();
}
const transport = new StdioClientTransport({
  command: process.execPath,
  args: [resolve('packages/mcp/dist/index.js'), '--config', config],
  stderr: 'pipe',
});
let stderr = '';
transport.stderr?.on('data', (chunk) => {
  stderr += chunk;
});
const client = new Client({ name: 'truth-acceptance', version: '0.1.0' });
try {
  await client.connect(transport);
  const { operation, ...args } = request;
  const name = Object.entries(queryTools).find(([, op]) => op === operation)?.[0];
  assert.ok(name);
  const response = await client.callTool({ name, arguments: args });
  assert.notEqual(response.isError, true);
  results.mcp = response.structuredContent;
  assert.deepEqual(clean(results.mcp), clean(expected));
} finally {
  await client.close();
  await transport.close();
}
assert.equal(stderr, '');
mkdirSync(directory, { recursive: true });
const id = createHash('sha256')
  .update(JSON.stringify({ request, snapshot: expected.snapshot }))
  .digest('hex');
writeFileSync(
  join(directory, `${id}.json`),
  JSON.stringify(
    {
      passed: true,
      request,
      ...results,
      normalization: ['requestId', 'checkedAt', 'elapsedMs', 'responseChars'],
    },
    null,
    2,
  ) + '\n',
);
