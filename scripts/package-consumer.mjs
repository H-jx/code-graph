// Copied to a fresh installation; imports resolve solely from that consumer.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import {
  readFileSync,
  writeFileSync,
  renameSync,
  existsSync,
  readdirSync,
  mkdirSync,
  mkdtempSync,
  copyFileSync,
} from 'node:fs';
import { join, resolve, dirname } from 'node:path';
import { tmpdir } from 'node:os';
import { spawnSync, spawn } from 'node:child_process';
import { createServer } from 'node:http';
import { createRequire } from 'node:module';
import { ProjectKnowledge } from '@project-knowledge/sdk';
import { Client } from '@modelcontextprotocol/client';
import { StdioClientTransport } from '@modelcontextprotocol/client/stdio';
const require = createRequire(import.meta.url);
// Recursive copy via mkdirSync/copyFileSync. Node 24.14.0 fs.cpSync on Windows
// corrupts the stack (0xC0000409) or silently no-ops when a path contains
// non-ASCII characters; this consumer intentionally runs under a Chinese+space
// directory, so the harness must not rely on cpSync. The product itself handles
// such paths correctly (Core/native/SDK/MCP/Agent all pass under them).
function copyTree(source, destination) {
  mkdirSync(destination, { recursive: true });
  for (const entry of readdirSync(source, { withFileTypes: true })) {
    const from = join(source, entry.name);
    const to = join(destination, entry.name);
    if (entry.isDirectory()) copyTree(from, to);
    else copyFileSync(from, to);
  }
}
const config = process.argv[2];
const db = join(resolve(config, '..'), 'index.sqlite');
const sha = () => createHash('sha256').update(readFileSync(db)).digest('hex');
const before = sha();
const sdk = await ProjectKnowledge.open({ configPath: config });
const cap = await sdk.getCapabilities();
const result = await sdk.search({ query: 'purchase' });
assert.ok(result.data.results.length);
const source = await sdk.readSource({ references: result.sourceReferences.slice(0, 1) });
assert.equal(source.data.reads[0].status, 'current');
await sdk.close();
const transport = new StdioClientTransport({
  command: process.execPath,
  args: [resolve('node_modules/@project-knowledge/mcp/dist/index.js'), '--config', config],
  stderr: 'pipe',
});
let stderr = '';
transport.stderr?.on('data', (chunk) => {
  stderr += chunk;
});
const client = new Client({ name: 'fresh-consumer', version: '0.1.0' });
try {
  await client.connect(transport);
  assert.equal((await client.listTools()).tools.length, 10);
  const response = await client.callTool({ name: 'search', arguments: { query: 'purchase' } });
  assert.notEqual(response.isError, true);
  assert.deepEqual(response.structuredContent.data, result.data);
  const schema = JSON.parse(
    (await client.readResource({ uri: 'project-knowledge://schema' })).contents[0].text,
  );
  assert.equal(schema.schemaHash, cap.schemaHash);
} finally {
  await client.close();
  await transport.close();
}
assert.equal(stderr, '');
assert.equal(sha(), before);
renameSync(db, db + '.closed');
renameSync(db + '.closed', db);
// Exercise an installed runtime plugin with no access to repository files or build tools.
const plugin = resolve('installed-plugin');
mkdirSync(plugin);
writeFileSync(
  join(plugin, 'collect.mjs'),
  `let raw='';for await(const chunk of process.stdin)raw+=chunk;const input=JSON.parse(raw);process.stdout.write(JSON.stringify({protocolVersion:'1.0',runId:input.runId,stageId:input.stageId,inputSnapshotId:input.inputSnapshotId,entities:[],relations:[],evidence:[],dependencies:[],coverage:[],gaps:[]}));`,
);
const manifestPath = join(plugin, 'manifest.json');
writeFileSync(
  manifestPath,
  JSON.stringify({
    protocolVersion: '1.0',
    pluginId: 'installed.empty',
    version: '1.0.0',
    runtime: { kind: 'node', versionRange: '>=24 <25' },
    artifacts: ['manifest.json', 'collect.mjs'],
    stages: [
      {
        id: 'extract',
        phase: 'extract',
        entry: 'collect.mjs',
        args: [],
        sourceSelectors: [{ id: 'sources', include: ['**/*.ts'] }],
        entitySelectors: [],
        dependsOn: [],
        replacement: 'project',
      },
    ],
  }),
);
const maintenance = await ProjectKnowledge.openMaintenance({ configPath: config });
let activation;
try {
  const candidate = await maintenance.submitPlugin({ path: manifestPath });
  await maintenance.preparePlugin({ candidateId: candidate.id });
  await maintenance.validatePlugin({ candidateIds: [candidate.id] });
  activation = await maintenance.activatePlugin({ candidateId: candidate.id });
  assert.equal(activation.status, 'active');
  assert.equal((await maintenance.collect()).status, 'committed');
} finally {
  await maintenance.close();
}

// Verify embedded Agent prompts/tools from the installed CLI with a local mock provider.
let providerRequests = 0;
const server = createServer(async (request, response) => {
  let raw = '';
  for await (const chunk of request) {
    raw += chunk;
    if (raw.length > 200000) {
      request.destroy();
      return;
    }
  }
  const body = JSON.parse(raw);
  assert.ok(!raw.includes('local-install-test-value'));
  const first = providerRequests++ === 0;
  if (first) assert.ok(body.tools.some((tool) => tool.function.name === 'status'));
  response.setHeader('content-type', 'application/json');
  response.end(
    JSON.stringify({
      id: 'install-mock',
      object: 'chat.completion',
      created: 1,
      model: 'install-check',
      choices: [
        {
          index: 0,
          message: first
            ? {
                role: 'assistant',
                content: null,
                tool_calls: [
                  {
                    id: 'call-status',
                    type: 'function',
                    function: { name: 'status', arguments: '{}' },
                  },
                ],
              }
            : { role: 'assistant', content: 'OK' },
          finish_reason: first ? 'tool_calls' : 'stop',
        },
      ],
      usage: { prompt_tokens: 10, completion_tokens: 10, total_tokens: 20 },
    }),
  );
});
await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
const installedCli = resolve(
  process.platform === 'win32' ? 'project-knowledge.exe' : 'project-knowledge',
);
const invoke = (args, env) =>
  new Promise((resolve, reject) => {
    const child = spawn(installedCli, args, { env, stdio: ['ignore', 'pipe', 'pipe'] });
    let stdout = '',
      stderr = '';
    const timeout = setTimeout(() => child.kill(), 30000);
    timeout.unref();
    child.stdout.on('data', (chunk) => {
      stdout += chunk;
    });
    child.stderr.on('data', (chunk) => {
      stderr += chunk;
    });
    child.once('error', reject);
    child.once('close', (code) => {
      clearTimeout(timeout);
      resolve({ code, stdout, stderr });
    });
  });
let agent;
try {
  const response = await invoke(
    [
      'agent',
      'configure',
      '--config',
      config,
      '--provider',
      'openai-compatible',
      '--model',
      'install-check',
      '--base-url',
      `http://127.0.0.1:${server.address().port}/v1`,
      '--api-key-env',
      'PK_INSTALL_TEST_KEY',
      '--max-turns',
      '3',
      '--check',
    ],
    { ...process.env, PK_INSTALL_TEST_KEY: 'local-install-test-value' },
  );
  assert.equal(response.code, 0, response.stderr + response.stdout);
  agent = JSON.parse(response.stdout);
  assert.equal(agent.status, 'complete');
  assert.equal(agent.modelTurns, 2);
  assert.equal(agent.toolCalls, 1);
  assert.equal(providerRequests, 2);
  const missingEnv = { ...process.env };
  delete missingEnv['PK_INSTALL_TEST_KEY'];
  const missing = await invoke(['agent', 'configure', '--config', config, '--check'], missingEnv);
  assert.equal(missing.code, 1);
  assert.ok(JSON.parse(missing.stdout).error.code);
} finally {
  await new Promise((resolve) => server.close(resolve));
}
const platform = Object.keys(
  require('@project-knowledge/native/package.json').optionalDependencies,
)[0];
const platformDirectory = resolve('node_modules', platform);
assert.ok(existsSync(platformDirectory));
// Keep the loaded Windows DLL untouched; test actual package loading in a separate consumer.
const binary = readdirSync(platformDirectory).find((name) => name.endsWith('.node'));
assert.ok(binary);
const missingDirectory = mkdtempSync(join(tmpdir(), 'pk-missing-native-'));
for (const name of ['sdk', 'native']) {
  const installed = dirname(require.resolve(`@project-knowledge/${name}/package.json`));
  const destination = join(missingDirectory, 'node_modules/@project-knowledge', name);
  mkdirSync(destination, { recursive: true });
  copyFileSync(join(installed, 'package.json'), join(destination, 'package.json'));
  if (name === 'sdk')
    copyTree(join(installed, 'dist'), join(destination, 'dist'));
  else
    for (const file of ['index.js', 'index.d.ts'])
      copyFileSync(join(installed, file), join(destination, file));
}
const absent = spawnSync(
  process.execPath,
  [
    '--input-type=module',
    '-e',
    `import {ProjectKnowledge} from '@project-knowledge/sdk';try{await ProjectKnowledge.getCapabilities();process.exitCode=1;}catch(e){console.log(JSON.stringify({code:e.code,message:e.message}));}`,
  ],
  { cwd: missingDirectory, encoding: 'utf8', env: process.env },
);
assert.equal(absent.status, 0, absent.stderr);
assert.equal(JSON.parse(absent.stdout).code, 'RUNTIME_MISSING');
writeFileSync(
  'consumer-report.json',
  JSON.stringify(
    {
      passed: true,
      protocolVersion: cap.protocolVersion,
      schemaHash: cap.schemaHash,
      platformPackage: platform,
      databaseHashUnchanged: before,
      nativeMissing: 'RUNTIME_MISSING',
      node: process.version,
      externalModelRequests: 0,
      installedPluginActivation: activation,
      localProviderRequests: providerRequests,
      installedAgent: {
        status: agent.status,
        modelTurns: agent.modelTurns,
        toolCalls: agent.toolCalls,
        missingKeyRejected: true,
      },
    },
    null,
    2,
  ) + '\n',
);
