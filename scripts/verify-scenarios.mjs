import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readFileSync, writeFileSync, mkdirSync, copyFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { acceptanceCases } from './acceptance-cases.mjs';

const root = fileURLToPath(new URL('../', import.meta.url));
const args = process.argv.slice(2).filter((arg) => arg !== '--');
const option = (key, fallback) => (args.includes(key) ? args[args.indexOf(key) + 1] : fallback);
const scenario = option('--scenario', 'all');
const transport = option('--transport', 'cli');
const selectedCase = option('--case', null);
if (!['1', '2', '3', 'all'].includes(scenario)) throw Error('scenario must be 1, 2, 3 or all');
if (!['cli', 'sdk', 'mcp', 'all'].includes(transport))
  throw Error('transport must be cli, sdk, mcp or all');
if (selectedCase && !acceptanceCases[selectedCase]) throw Error('Unknown acceptance case');
if (selectedCase && scenario !== 'all' && !selectedCase.startsWith(`S${scenario}-`))
  throw Error('case does not belong to the selected scenario');
const evidence = path.resolve(
  root,
  option('--evidence', transport === 'cli' ? 'docs/evidence/T16' : 'docs/evidence/T22/scenarios'),
);
mkdirSync(evidence, { recursive: true });
const builtExecutable = path.resolve(
  root,
  process.env.CARGO_TARGET_DIR ?? 'target',
  'debug',
  process.platform === 'win32' ? 'project-knowledge.exe' : 'project-knowledge',
);
let executable = builtExecutable;
const hash = (bytes) => createHash('sha256').update(bytes).digest('hex');
const commands = [];
function run(name, command) {
  const result = spawnSync(command[0], command.slice(1), {
    cwd: root,
    env: {
      ...process.env,
      PYTHONIOENCODING: 'utf-8',
      PK_TEST_CLI: executable,
      PK_SCENARIO_EVIDENCE_DIR: evidence,
      ...(transport !== 'cli'
        ? { PK_SCENARIO_TRANSPORTS_DIR: path.join(evidence, 'transports') }
        : {}),
    },
    encoding: 'utf8',
    timeout: 1200000,
    maxBuffer: 32 * 1024 * 1024,
  });
  const log = path.join(evidence, name + '.log');
  writeFileSync(log, (result.stdout ?? '') + (result.stderr ?? ''));
  commands.push({
    command,
    exitCode: result.status,
    log: path.relative(root, log).replaceAll('\\', '/'),
    logSha256: hash(readFileSync(log)),
  });
  console.log(JSON.stringify(commands.at(-1)));
  if (result.error || result.status !== 0)
    throw result.error ?? Error(`${name} failed; inspect ${log}`);
  return result.stdout;
}
run('cli-build', ['cargo', 'build', '-p', 'project-knowledge-cli', '--locked', '-j', '1']);
const cliHash = hash(readFileSync(builtExecutable));
const artifactDirectory = path.join(root, 'target', 'verified-cli', cliHash);
mkdirSync(artifactDirectory, { recursive: true });
executable = path.join(artifactDirectory, path.basename(builtExecutable));
copyFileSync(builtExecutable, executable);
if (selectedCase) {
  for (const [i, check] of acceptanceCases[selectedCase].entries()) {
    const output = run(`${selectedCase}-${i}`, [
      'cargo',
      'test',
      '-p',
      check.package,
      '--test',
      check.suite,
      check.name,
      '--locked',
      '-j',
      '1',
      '--',
      '--test-threads=1',
    ]);
    assert.match(output, /1 passed; 0 failed; 0 ignored/);
  }
  writeFileSync(
    path.join(evidence, `${selectedCase}-report.json`),
    JSON.stringify(
      {
        passed: true,
        selectedCase,
        transport,
        commands,
        scope:
          'Mapped host tests; scenario test also runs CLI/SDK/MCP when a non-cli transport is selected',
      },
      null,
      2,
    ) + '\n',
  );
  process.exit(0);
}
const queries = run('cli-scenarios', [
  'cargo',
  'test',
  '-p',
  'project-knowledge-core',
  '--test',
  'business_scenarios',
  'scenario_',
  '--locked',
  '-j',
  '1',
  '--',
  '--test-threads=1',
]);
assert.match(queries, /2 passed; 0 failed; 0 ignored/);
const numeric = run('numeric-sql', [
  'cargo',
  'test',
  '-p',
  'project-knowledge-core',
  '--test',
  'business_scenarios',
  'truth_numeric_sql_',
  '--locked',
  '-j',
  '1',
  '--',
  '--test-threads=1',
]);
assert.match(numeric, /1 passed; 0 failed; 0 ignored/);
const oracle = JSON.parse(run('integer-oracle', ['python', 'scripts/verify-truth.py']));
const truth = JSON.parse(readFileSync(path.join(root, 'fixtures/truth/manifest.json'), 'utf8'));
assert.equal(oracle.truthHash, truth.truthHash);
const results = [];
for (const layout of ['a', 'b'])
  for (const [number, name] of [
    [1, 'page'],
    [2, 'workflow'],
    [3, 'report'],
  ]) {
    const file = path.join(evidence, `${layout}-${name}.json`);
    const value = JSON.parse(readFileSync(file, 'utf8'));
    assert.ok(value.cli, `${layout}/${name}: actual CLI response is required`);
    assert.ok(value.sourceReferencesRead > 0);
    results.push({
      layout,
      scenario: number,
      file: path.relative(root, file).replaceAll('\\', '/'),
      sha256: hash(readFileSync(file)),
      sourceReferencesRead: value.sourceReferencesRead,
      entities: value.core.data.entities.length,
      relations: value.core.data.hops.length,
      roots: value.core.data.roots.length,
      status: value.core.status,
    });
  }
const requested = scenario === 'all' ? [1, 2, 3] : [Number(scenario)];
const report = {
  task: transport === 'cli' ? 'T16' : 'T22',
  transport,
  scenario,
  selectedCase,
  truthHash: truth.truthHash,
  commands,
  results: results.filter((r) => requested.includes(r.scenario)),
  cliArtifactPath: path.relative(root, executable).replaceAll('\\', '/'),
  cliSha256: hash(readFileSync(executable)),
  oracle,
  externalModelRequests: 0,
  businessDatabaseConnections: 0,
  productionAccuracyClaimed: false,
  scope:
    'A/B reviewed fixtures; missing-import negatives plus separately reviewed explicit bindings; numeric SQL runs only in the local test oracle',
};
writeFileSync(path.join(evidence, 'cli-report.json'), JSON.stringify(report, null, 2) + '\n');
console.log(
  JSON.stringify({
    ok: true,
    scenario,
    transport,
    layouts: 2,
    cliResponses: results.length,
    truthHash: truth.truthHash,
  }),
);
