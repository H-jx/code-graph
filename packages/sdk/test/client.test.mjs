import assert from 'node:assert/strict';
import { test } from 'node:test';
import { createRequire } from 'node:module';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import {
  mkdtempSync,
  mkdirSync,
  writeFileSync,
  readFileSync,
  existsSync,
  renameSync,
  rmSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, relative, resolve } from 'node:path';
import { readFile } from 'node:fs/promises';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { setTimeout as delay } from 'node:timers/promises';
import { ProjectKnowledge, ProjectKnowledgeError } from '../dist/index.js';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../../..');
const cli =
  process.env.PK_TEST_CLI ??
  join(
    root,
    'target/debug',
    process.platform === 'win32' ? 'project-knowledge.exe' : 'project-knowledge',
  );
const require = createRequire(import.meta.url);
const native = require('@project-knowledge/native');
const evidence = process.env.PK_SDK_EVIDENCE_DIR;
function save(name, value) {
  if (evidence) {
    mkdirSync(evidence, { recursive: true });
    writeFileSync(join(evidence, name), JSON.stringify(value, null, 2) + '\n');
  }
}
const hash = (data) => createHash('sha256').update(data).digest('hex');
const errorCode = (code) => (error) =>
  error instanceof ProjectKnowledgeError && error.code === code;
function normalized(value) {
  if (Array.isArray(value)) return value.map(normalized);
  if (value && typeof value === 'object')
    return Object.fromEntries(
      Object.entries(value)
        .filter(([key]) => !['requestId', 'checkedAt', 'elapsedMs', 'responseChars'].includes(key))
        .map(([key, child]) => [key, normalized(child)]),
    );
  return value;
}
async function fixture(t, files = 4) {
  const directory = mkdtempSync(join(tmpdir(), 'pk-sdk-中文 空格-'));
  mkdirSync(join(directory, 'repo'));
  mkdirSync(join(directory, '.project-knowledge'));
  for (let i = 0; i < files; i++)
    writeFileSync(
      join(directory, 'repo', `source-${i}.ts`),
      `// 采购 😀 ${i}\nexport function purchase${i}() { return ${i}; }\n`,
    );
  const configPath = join(directory, '.project-knowledge/project.json');
  writeFileSync(
    configPath,
    JSON.stringify({
      schemaVersion: 1,
      projectId: 'sdk-test',
      repositories: [{ id: 'r', root: '../repo' }],
      selection: {},
      plugins: { directories: [], autoLoad: false, autoActivate: false },
    }),
  );
  const handles = [];
  t.after(async () => {
    await Promise.allSettled(handles.map((h) => h.close()));
    rmSync(directory, { recursive: true, force: true });
  });
  const f = { directory, configPath, database: join(dirname(configPath), 'index.sqlite'), handles };
  const maintenance = await ProjectKnowledge.openMaintenance({ configPath });
  handles.push(maintenance);
  await maintenance.index({ mode: 'base' });
  await maintenance.close();
  f.open = async (write = false) => {
    const handle = await (write
      ? ProjectKnowledge.openMaintenance({ configPath })
      : ProjectKnowledge.open({ configPath }));
    handles.push(handle);
    return handle;
  };
  return f;
}

test('capabilities uses the exported Core schema without config/index or runtime discovery', async () => {
  const cap = await ProjectKnowledge.getCapabilities();
  assert.equal(cap.protocolVersion, '1.0');
  assert.equal(cap.runtimeChecked, false);
  assert.deepEqual(cap.providers, []);
  assert.equal(cap.operations.length, 10);
  assert.deepEqual(
    cap.schema,
    JSON.parse(readFileSync(join(root, 'docs/schema/contract.schema.json'), 'utf8')),
  );
  function ordered(v) {
    return Array.isArray(v)
      ? v.map(ordered)
      : v && typeof v === 'object'
        ? Object.fromEntries(
            Object.keys(v)
              .sort()
              .map((k) => [k, ordered(v[k])]),
          )
        : v;
  }
  assert.equal(cap.schemaHash, hash(JSON.stringify(ordered(cap.schema))));
  save('capabilities.json', cap);
});

test('read open never creates/migrates a missing or incompatible index', async (t) => {
  const directory = mkdtempSync(join(tmpdir(), 'pk-sdk-missing-'));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  mkdirSync(join(directory, 'repo'));
  const configPath = join(directory, 'project.json');
  writeFileSync(
    configPath,
    JSON.stringify({
      schemaVersion: 1,
      projectId: 'missing',
      repositories: [{ id: 'r', root: 'repo' }],
      selection: {},
    }),
  );
  await assert.rejects(ProjectKnowledge.open({ configPath }), errorCode('INDEX_MISSING'));
  assert.equal(existsSync(join(directory, 'index.sqlite')), false);
  assert.equal(existsSync(join(directory, '.project-knowledge')), false);
  const py = spawnSync('python', [
    '-c',
    "import sqlite3,sys; c=sqlite3.connect(sys.argv[1]); c.execute('CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY)'); c.execute('INSERT INTO schema_migrations VALUES(999)'); c.commit(); c.close()",
    join(directory, 'index.sqlite'),
  ]);
  assert.equal(py.status, 0, py.stderr.toString());
  const before = hash(readFileSync(join(directory, 'index.sqlite')));
  await assert.rejects(ProjectKnowledge.open({ configPath }), errorCode('DB_VERSION_UNSUPPORTED'));
  assert.equal(hash(readFileSync(join(directory, 'index.sqlite'))), before);
});

test('all ten query methods match CLI and keep database bytes/tables unchanged', async (t) => {
  const f = await fixture(t);
  const client = await f.open();
  const found = await client.search({ query: 'purchase0' });
  assert.ok(found.sourceReferences.length);
  const rootRef = found.data.results[0].entityRef;
  const snapshot = found.snapshot;
  const requests = [
    ['status', {}],
    ['search', { query: '采购', pageSize: 1 }],
    ['locateFeature', { query: 'purchase0' }],
    ['trace', { roots: [rootRef], direction: 'both' }],
    ['neighbors', { root: rootRef, direction: 'both', pageSize: 1 }],
    ['getCodeMap', { roots: [rootRef] }],
    ['readSource', { references: found.sourceReferences }],
    ['locatePage', { route: '/missing' }],
    ['explainWorkflow', { query: '采购' }],
    ['explainReport', { query: '采购' }],
  ];
  const operations = {
    getCodeMap: 'codeMap',
    locatePage: 'page',
    explainWorkflow: 'workflow',
    explainReport: 'report',
  };
  const before = hash(readFileSync(f.database));
  const goldens = [];
  for (const [method, input] of requests) {
    const request = { ...input, operation: operations[method] ?? method };
    const path = join(f.directory, 'request.json');
    writeFileSync(path, JSON.stringify(request));
    const actual = await client[method](input);
    const run = spawnSync(cli, ['query', '--config', f.configPath, '--request', path], {
      encoding: 'utf8',
      timeout: 30000,
    });
    assert.ok([0, 2].includes(run.status), run.stderr);
    const expected = JSON.parse(run.stdout);
    assert.deepEqual(normalized(actual), normalized(expected), method);
    assert.deepEqual(actual.snapshot, snapshot);
    goldens.push({ method, request, cli: expected, sdk: actual });
  }
  assert.equal(hash(readFileSync(f.database)), before);
  assert.equal(typeof client.maintain, 'undefined');
  await assert.rejects(
    client.execute({ operation: 'index', mode: 'base' }),
    errorCode('SCHEMA_INVALID'),
  );
  const raw = await native.NativeClient.open(f.configPath, false);
  try {
    const encoded = await raw.execute(
      'r'.repeat(128),
      JSON.stringify({
        operation: 'readSource',
        references: found.sourceReferences,
        maxResponseChars: 2400,
      }),
      false,
    );
    assert.equal(JSON.parse(encoded).requestId, 'r'.repeat(128));
    assert.equal(JSON.parse(encoded).budgetUsed.responseChars, [...encoded].length);
    assert.ok([...encoded].length <= 2400);
    assert.throws(
      () => raw.execute('forbidden', '{"operation":"index","mode":"base"}', true),
      /maintenance requires/,
    );
  } finally {
    await raw.close();
  }
  assert.equal(hash(readFileSync(f.database)), before);
  save('cli-sdk-golden.json', {
    passed: true,
    databaseHashUnchanged: before,
    comparisons: goldens,
  });
});

test('Core validation issues, pagination and snapshot/source references survive the transport', async (t) => {
  const f = await fixture(t, 12);
  const client = await f.open();
  await assert.rejects(
    client.search({ query: 'private-value', unexpected: 'private-value' }),
    (error) => {
      assert.ok(errorCode('SCHEMA_INVALID')(error));
      assert.ok(error.issues.length);
      assert.equal(JSON.stringify(error).includes('private-value'), false);
      return true;
    },
  );
  await assert.rejects(client.execute({ operation: 'search' }), errorCode('SCHEMA_INVALID'));
  const page = await client.search({ query: '采购', pageSize: 1 });
  assert.ok(page.continuation);
  const second = await client.search({
    query: '采购',
    pageSize: 1,
    continuation: page.continuation,
  });
  assert.notDeepEqual(second.data, page.data);
  await assert.rejects(
    client.search({ query: 'changed', continuation: page.continuation }),
    errorCode('INVALID_REQUEST'),
  );
  const maintenance = await f.open(true);
  writeFileSync(join(f.directory, 'repo/new.ts'), 'export const added = 1;\n');
  await maintenance.index({ mode: 'incremental' });
  await assert.rejects(
    client.search({ query: '采购', pageSize: 1, continuation: page.continuation }),
    errorCode('SNAPSHOT_EXPIRED'),
  );
  await assert.rejects(
    client.readSource({ references: page.sourceReferences }),
    errorCode('SNAPSHOT_EXPIRED'),
  );
});

test('query load leaves the event loop responsive; pre-abort, request IDs and close are deterministic', async (t) => {
  const f = await fixture(t, 50);
  const client = await f.open();
  const raw = await native.NativeClient.open(relative(process.cwd(), f.configPath), false);
  const cwd = process.cwd();
  try {
    process.chdir(f.directory);
    assert.equal(
      JSON.parse(
        await raw.execute('relative-path', '{"operation":"status","checkFreshness":false}', false),
      ).snapshot.indexId,
      (await client.status({ checkFreshness: false })).snapshot.indexId,
    );
  } finally {
    process.chdir(cwd);
    await raw.close();
  }
  assert.equal(
    (await client.status({ checkFreshness: false }, { requestId: 'caller-id' })).requestId,
    'caller-id',
  );
  const signal = AbortSignal.abort();
  await assert.rejects(client.search({ query: '采购' }, { signal }), errorCode('CANCELLED'));
  await assert.rejects(
    client.execute({ operation: 'search', query: 1n }),
    errorCode('INVALID_REQUEST'),
  );
  let ticks = 0;
  let largestGap = 0;
  let last = performance.now();
  const timer = setInterval(() => {
    const now = performance.now();
    largestGap = Math.max(largestGap, now - last);
    last = now;
    ticks++;
  }, 2);
  const started = performance.now();
  try {
    await Promise.all(
      Array.from({ length: 40 }, () =>
        client.search({ query: '采购', filters: { freshnessPolicy: 'indexed' } }),
      ),
    );
  } finally {
    clearInterval(timer);
  }
  assert.ok(ticks > 5, `event loop ticks: ${ticks}`);
  const controller = new AbortController();
  const running = client.search(
    { query: '采购', budget: { timeoutMs: 30000 } },
    { signal: controller.signal, requestId: 'cancel-query' },
  );
  controller.abort();
  await assert.rejects(running, errorCode('CANCELLED'));
  assert.equal(client.cancel('unknown'), false);
  const closing = client.close();
  assert.equal(client.close(), closing);
  await assert.rejects(client.status(), errorCode('CLOSED'));
  await closing;
  renameSync(f.database, f.database + '.released');
  renameSync(f.database + '.released', f.database);
  save('event-loop.json', {
    passed: true,
    queries: 40,
    ticks,
    largestGapMs: largestGap,
    elapsedMs: performance.now() - started,
    fileLockReleased: true,
  });
});

async function heldPlugin(f, name, hold) {
  const directory = join(f.directory, name);
  mkdirSync(directory);
  const marker = join(f.directory, `${name}.started.json`);
  writeFileSync(
    join(directory, 'collect.mjs'),
    readFileSync(join(root, 'fixtures/plugins/staging/collect.mjs')),
  );
  const manifest = {
    protocolVersion: '1.0',
    pluginId: name,
    version: '1.0.0',
    runtime: { kind: 'node', versionRange: '>=24 <25' },
    artifacts: ['manifest.json', 'collect.mjs'],
    stages: [
      {
        id: 'extract',
        phase: 'extract',
        entry: 'collect.mjs',
        args: hold ? ['--hold', marker] : ['--empty'],
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
  writeFileSync(join(directory, 'manifest.json'), JSON.stringify(manifest));
  return { path: join(directory, 'manifest.json'), marker };
}
async function waitUntil(check, timeout = 10000) {
  const started = Date.now();
  while (!check()) {
    assert.ok(Date.now() - started < timeout, 'condition timed out');
    await delay(10);
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

test('maintenance uses Core candidate lifecycle; running abort and close stop owned plugin trees', async (t) => {
  const f = await fixture(t);
  const maintenance = await f.open(true);
  const empty = await heldPlugin(f, 'sdk.empty', false);
  const submitted = await maintenance.submitPlugin({ path: empty.path });
  assert.equal(
    (await maintenance.preparePlugin({ candidateId: submitted.id })).status,
    'noDependencies',
  );
  await maintenance.validatePlugin({ candidateIds: [submitted.id] });
  const activated = await maintenance.activatePlugin({ candidateId: submitted.id });
  assert.equal(activated.status, 'active');
  assert.equal(
    (await maintenance.activatePlugin({ candidateId: submitted.id })).alreadyCommitted,
    true,
  );
  assert.equal((await maintenance.collect()).status, 'committed');
  const record = [];
  for (const stop of ['abort', 'close']) {
    const handle = await f.open(true);
    const plugin = await heldPlugin(f, `sdk.${stop}`, true);
    const candidate = await handle.submitPlugin({ path: plugin.path });
    const controller = new AbortController();
    const work = handle.validatePlugin(
      { candidateIds: [candidate.id] },
      { signal: controller.signal, requestId: `held-${stop}` },
    );
    const rejected = assert.rejects(work, errorCode('CANCELLED'));
    await waitUntil(() => existsSync(plugin.marker));
    const pids = JSON.parse(readFileSync(plugin.marker, 'utf8'));
    assert.ok(alive(pids.parent) && alive(pids.descendant));
    const started = performance.now();
    let queued;
    if (stop === 'close') {
      const first = handle.status({ checkFreshness: false }, { requestId: 'queued-id' });
      const firstSettled = Promise.allSettled([first]);
      await assert.rejects(
        handle.status({}, { requestId: 'queued-id' }),
        errorCode('INVALID_REQUEST'),
      );
      assert.equal(handle.cancel('queued-id'), true);
      queued = Promise.allSettled(
        Array.from({ length: 70 }, () => handle.status({ checkFreshness: false })),
      );
      assert.ok(
        (
          await readFile(f.configPath, { encoding: 'utf8', signal: AbortSignal.timeout(2000) })
        ).includes('sdk-test'),
      );
      await handle.close();
      const results = [...(await firstSettled), ...(await queued)];
      assert.ok(
        results.every(
          (r) => r.status === 'rejected' && ['CANCELLED', 'BUSY'].includes(r.reason.code),
        ),
      );
      assert.ok(results.some((r) => r.reason.code === 'BUSY'));
    }
    if (stop === 'abort') controller.abort();
    else await handle.close();
    await rejected;
    await waitUntil(() => !alive(pids.parent) && !alive(pids.descendant));
    assert.ok(performance.now() - started < 10000);
    if (stop === 'abort') await handle.status({ checkFreshness: false });
    await handle.close();
    record.push({ stop, pids, elapsedMs: performance.now() - started, treeStopped: true });
  }
  save('plugin-lifecycle.json', {
    passed: true,
    modelRequests: 0,
    activation: activated,
    cancellation: record,
  });
});

test('out-of-repo loading works and missing native reports an installation error', () => {
  const directory = mkdtempSync(join(tmpdir(), 'pk-sdk-loader-'));
  try {
    const result = spawnSync(
      process.execPath,
      [
        '--input-type=module',
        '-e',
        `import {ProjectKnowledge} from ${JSON.stringify(new URL('../dist/index.js', import.meta.url).href)}; console.log((await ProjectKnowledge.getCapabilities()).protocolVersion)`,
      ],
      { cwd: directory, encoding: 'utf8' },
    );
    assert.equal(result.status, 0, result.stderr);
    assert.equal(result.stdout.trim(), '1.0');
    const isolated = join(directory, 'isolated-client.mjs');
    writeFileSync(isolated, readFileSync(new URL('../dist/client.js', import.meta.url)));
    const missing = spawnSync(
      process.execPath,
      [
        '--input-type=module',
        '-e',
        `import {ProjectKnowledge} from ${JSON.stringify(pathToFileURL(isolated).href)}; try { await ProjectKnowledge.getCapabilities(); process.exitCode=1; } catch(e) { console.log(JSON.stringify({code:e.code,message:e.message})); }`,
      ],
      { cwd: directory, encoding: 'utf8' },
    );
    assert.equal(missing.status, 0, missing.stderr);
    const error = JSON.parse(missing.stdout);
    assert.equal(error.code, 'RUNTIME_MISSING');
    assert.match(error.message, /matching platform package/);
    // Published package installation across four platforms remains T23.
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});
