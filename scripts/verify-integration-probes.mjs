// T00 的可复现入口；探针使用独立锁文件，napi/Schema 仍直接调用产品 Core。
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const evidence = resolve(root, 'docs/evidence/T00');
mkdirSync(evidence, { recursive: true });
const commands = [];
function run(command, args, log, env = process.env) {
  const result = spawnSync(command, args, {
    cwd: root,
    env,
    encoding: 'utf8',
    timeout: 300000,
    maxBuffer: 8 * 1024 * 1024,
  });
  const output = (result.stdout || '') + (result.stderr || '');
  writeFileSync(resolve(evidence, log), output);
  commands.push({ command, args, exitCode: result.status, log });
  assert.equal(
    result.status,
    0,
    `${command} failed; see ${log}: ${result.error?.message || output.slice(-2000)}`,
  );
  return output;
}
let python = process.env.PROJECT_KNOWLEDGE_PYTHON || 'python';
let version = spawnSync(python, ['--version'], { encoding: 'utf8', timeout: 5000 });
if (!/^Python 3\.12\./.test(version.stdout || '')) {
  const found = spawnSync('uv', ['python', 'find', '3.12'], { encoding: 'utf8', timeout: 10000 });
  if (found.status === 0) python = found.stdout.trim();
  version = spawnSync(python, ['--version'], { encoding: 'utf8', timeout: 5000 });
}
assert.match(
  version.stdout || '',
  /^Python 3\.12\./,
  'install Python 3.12 or set PROJECT_KNOWLEDGE_PYTHON',
);
const manifest = 'docs/evidence/T00/probe/Cargo.toml';
run(
  'cargo',
  [
    '+1.97.1',
    'build',
    '--manifest-path',
    manifest,
    '--locked',
    '--target-dir',
    'target',
    '-p',
    't00-native-probe',
  ],
  'native-build.log',
);
const rust = run(
  'cargo',
  [
    '+1.97.1',
    'test',
    '--manifest-path',
    manifest,
    '--locked',
    '--target-dir',
    'target',
    '-p',
    't00-rig-probe',
  ],
  'rig-tests.log',
);
assert.match(rust, /4 passed; 0 failed/);
const node = run(
  process.execPath,
  [
    '--test',
    'docs/evidence/T00/probe/node/integration.test.mjs',
    'docs/evidence/T00/probe/node/python.test.mjs',
  ],
  'node-tests.log',
  { ...process.env, PROJECT_KNOWLEDGE_PYTHON: python },
);
assert.match(node, /(?:#|ℹ) pass 5/);
const vectors = run(python, ['-X', 'utf8', 'scripts/verify-vectors.py'], 'python-vectors.json');
const vectorReport = JSON.parse(vectors);
assert.ok(vectorReport.total >= 18);
assert.equal(vectorReport.passed, vectorReport.total);
const files = [
  'Cargo.lock',
  'pnpm-lock.yaml',
  manifest,
  'docs/evidence/T00/probe/Cargo.lock',
  'docs/evidence/T00/probe/node/package-lock.json',
  'docs/evidence/T00/probe/native/Cargo.toml',
  'docs/evidence/T00/probe/native/src/lib.rs',
  'docs/evidence/T00/probe/rig/Cargo.toml',
  'docs/evidence/T00/probe/rig/tests/integration.rs',
  'docs/evidence/T00/probe/node/shared.mjs',
  'docs/evidence/T00/probe/node/server.mjs',
  'docs/evidence/T00/probe/node/integration.test.mjs',
  'docs/evidence/T00/probe/node/python.test.mjs',
  'scripts/verify-vectors.py',
  'docs/NEW-PROJECT-DESIGN.md',
  'docs/protocol.md',
];
const sha256 = (file) =>
  createHash('sha256')
    .update(readFileSync(resolve(root, file)))
    .digest('hex');
const report = {
  date: '2026-10-26',
  node: process.version,
  python: version.stdout.trim(),
  pythonExecutable: python,
  rustTests: 4,
  nodeTests: 5,
  pythonVectors: vectorReport.passed,
  commands,
  hashes: Object.fromEntries(files.map((file) => [file, sha256(file)])),
  protocols: ['2025-11-25', '2026-07-28'],
  providerNetworkCalls: 0,
  independentBusinessTruth: false,
};
writeFileSync(resolve(evidence, 'probe-report.json'), JSON.stringify(report, null, 2) + '\n');
console.log(
  JSON.stringify({
    ok: true,
    rustTests: 4,
    nodeTests: 5,
    pythonVectors: vectorReport.passed,
    report: resolve(evidence, 'probe-report.json'),
  }),
);
