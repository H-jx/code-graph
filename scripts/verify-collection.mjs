// 真实 CLI 合成闭环证据；固定业务真值验收仍由 T06/T22 承担。
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const outputIndex = process.argv.indexOf('--output');
const output = resolve(
  root,
  outputIndex >= 0 ? process.argv[outputIndex + 1] : 'docs/evidence/T09/cli',
);
mkdirSync(output, { recursive: true });
const work = mkdtempSync(join(output, 'run-'));
const cli = join(
  root,
  'target/debug',
  process.platform === 'win32' ? 'project-knowledge.exe' : 'project-knowledge',
);
const repo = join(work, 'repo');
const plugin = join(work, 'plugin');
mkdirSync(repo);
mkdirSync(plugin);
writeFileSync(join(repo, 'a.ts'), 'export const smoke = 1;\n');
copyFileSync(join(root, 'fixtures/plugins/staging/collect.mjs'), join(plugin, 'collect.mjs'));
const config = join(work, 'project.json');
writeFileSync(
  config,
  JSON.stringify(
    {
      schemaVersion: 1,
      projectId: 'smoke',
      repositories: [{ id: 'r', root: 'repo' }],
      selection: {},
    },
    null,
    2,
  ),
);
const manifestPath = join(plugin, 'manifest.json');
const manifest = {
  protocolVersion: '1.0',
  pluginId: 'smoke.plugin',
  version: '1.0.0',
  runtime: { kind: 'node', versionRange: '>=24 <25' },
  artifacts: ['manifest.json', 'collect.mjs'],
  stages: ['extract', 'link'].map((phase) => ({
    id: phase,
    phase,
    entry: 'collect.mjs',
    args: [],
    replacement: 'project',
    sourceSelectors: [{ id: 'sources', include: ['**/*.ts'] }],
    entitySelectors: [{ id: 'targets', kinds: ['route', 'file'] }],
    dependsOn: phase === 'link' ? ['smoke.plugin:extract'] : [],
  })),
};
writeFileSync(manifestPath, JSON.stringify(manifest, null, 2));
const commands = [];
function run(label, args, expectedExit = 0) {
  const command = spawnSync(cli, args, {
    encoding: 'utf8',
    timeout: 30000,
    maxBuffer: 1024 * 1024,
  });
  assert.equal(command.status, expectedExit, `${label}: ${command.stdout} ${command.stderr}`);
  const payload = JSON.parse(command.stdout);
  writeFileSync(join(work, `${label}.json`), JSON.stringify(payload, null, 2));
  commands.push({
    label,
    executable: cli,
    args,
    exitCode: command.status,
    outputHash: sha(command.stdout),
  });
  return payload;
}
function sha(value) {
  return createHash('sha256').update(value).digest('hex');
}
const base = run('01-base', ['index', '--config', config, '--base']);
const validateOnly = run('02-validate-only', [
  'collect',
  '--config',
  config,
  '--manifest',
  manifestPath,
  '--validate-only',
]);
assert.equal(validateOnly.committed, false);
const submit = run('03-submit', ['plugin', '--config', config, 'submit', '--path', plugin]);
run('04-validate', ['plugin', '--config', config, 'validate', '--candidate', submit.id]);
const activated = run('05-activate', [
  'plugin',
  '--config',
  config,
  'activate',
  '--candidate',
  submit.id,
]);
assert.equal(BigInt(activated.generation), BigInt(base.generation) + 1n);
const collected = run('06-collect', ['collect', '--config', config]);
assert.equal(collected.committed, true);
assert.equal(BigInt(collected.generation), BigInt(activated.generation) + 1n);
manifest.stages[1].args = ['--bad'];
writeFileSync(manifestPath, JSON.stringify(manifest, null, 2));
const bad = run(
  '07-bad-link',
  ['collect', '--config', config, '--manifest', manifestPath, '--validate-only'],
  1,
);
assert.equal(bad.error.code, 'EVIDENCE_INVALID');
const rollback = run('08-rollback', [
  'plugin',
  '--config',
  config,
  'rollback',
  '--plugin',
  'smoke.plugin',
  '--version',
  '1.0.0',
]);
assert.equal(BigInt(rollback.generation), BigInt(collected.generation) + 1n);
const report = {
  date: new Date().toISOString(),
  origin: 'synthetic-smoke',
  independentBusinessTruth: false,
  node: process.version,
  platform: process.platform,
  arch: process.arch,
  commands,
  hashes: Object.fromEntries(
    [config, manifestPath, join(plugin, 'collect.mjs'), join(repo, 'a.ts'), cli].map((file) => [
      file,
      sha(readFileSync(file)),
    ]),
  ),
  assertions: {
    validateOnlyDoesNotCommit: true,
    activationSwitchesOnce: true,
    collectCommits: true,
    badLinkFails: true,
    rollbackCreatesNewGeneration: true,
  },
};
writeFileSync(join(work, 'report.json'), JSON.stringify(report, null, 2));
console.log(
  JSON.stringify({ ok: true, report: join(work, 'report.json'), commands: commands.length }),
);
