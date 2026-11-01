import assert from 'node:assert/strict';
import { spawnSync, execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { createRequire } from 'node:module';
import {
  mkdtempSync,
  mkdirSync,
  readFileSync,
  writeFileSync,
  copyFileSync,
  existsSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve, dirname, join, delimiter, basename } from 'node:path';
import { fileURLToPath } from 'node:url';
import { installedPackageMetadata } from './package-metadata.mjs';
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const args = process.argv.slice(2).filter((arg) => arg !== '--');
const index = args.indexOf('--artifacts');
assert.ok(index >= 0 && args[index + 1], '--artifacts <candidate directory> required');
const artifacts = resolve(args[index + 1]);
const manifest = JSON.parse(readFileSync(join(artifacts, 'release-manifest.json')));
assert.equal(manifest.platform, process.platform);
assert.equal(manifest.architecture, process.arch);
const sha = (file) => createHash('sha256').update(readFileSync(file)).digest('hex');
for (const [file, digest] of Object.entries(manifest.sha256)) {
  assert.equal(basename(file), file, 'Manifest artifact must be a filename');
  assert.equal(sha(join(artifacts, file)), digest, file);
}
const directory = mkdtempSync(join(tmpdir(), 'pk-installed-中文 空格-'));
const commands = [];
const npm = join(dirname(process.execPath), 'node_modules/npm/bin/npm-cli.js');
const sanitized = Object.fromEntries(
  Object.entries(process.env).filter(
    ([key]) =>
      !/(?:^path$|api.*key|token|password|secret|node_path|node_options|napi_rs_native_library_path)/i.test(
        key,
      ),
  ),
);
const system = process.platform === 'win32' ? join(process.env.SystemRoot, 'System32') : '';
const runtime = join(directory, 'runtime');
mkdirSync(runtime);
const node = join(runtime, basename(process.execPath));
copyFileSync(process.execPath, node);
const runtimeEnv = { ...sanitized, PATH: runtime + delimiter + system };
const cliEnv = { ...sanitized, PATH: system };
function run(label, command, parameters, env = runtimeEnv, allowed = [0]) {
  const result = spawnSync(command, parameters, {
    cwd: directory,
    env,
    encoding: 'utf8',
    timeout: 180000,
    maxBuffer: 32 * 1024 * 1024,
  });
  const log = join(artifacts, `install-${label}.log`);
  writeFileSync(log, (result.stdout ?? '') + (result.stderr ?? ''));
  commands.push({
    label,
    command,
    parameters,
    exitCode: result.status,
    log: basename(log),
    sha256: sha(log),
  });
  writeFileSync(join(artifacts, 'install-commands.json'), JSON.stringify(commands, null, 2) + '\n');
  assert.ok(!result.error && allowed.includes(result.status), `${label} failed: ${result.stderr}`);
  return result.stdout;
}
const tar =
  process.platform === 'win32'
    ? join(system, 'tar.exe')
    : execFileSync('/usr/bin/which', ['tar'], { encoding: 'utf8' }).trim();
run('unpack-cli', tar, ['-xzf', join(artifacts, manifest.cli)]);
const cli = join(
  directory,
  process.platform === 'win32' ? 'project-knowledge.exe' : 'project-knowledge',
);
const core = join(
  directory,
  process.platform === 'win32' ? 'project-knowledge-core.exe' : 'project-knowledge-core',
);
const project = join(directory, 'project');
mkdirSync(project);
writeFileSync(join(project, 'purchase.ts'), '// 采购\nexport function purchase() { return 1; }\n');
run('cli-init', cli, ['init', '--root', project], cliEnv);
const config = join(project, '.project-knowledge/project.json');
run('cli-base', core, ['index', '--config', config, '--base'], cliEnv);
const status = JSON.parse(
  run('cli-status', cli, ['status', '--config', config, '--check-freshness'], cliEnv, [0, 2]),
);
assert.ok(status.snapshot);
const search = JSON.parse(
  run(
    'cli-query',
    core,
    ['query', '--config', config, '--operation', 'search', '--text', 'purchase'],
    cliEnv,
    [0, 2],
  ),
);
assert.ok(search.data.results.length);
const request = join(directory, 'source-request.json');
writeFileSync(
  request,
  JSON.stringify({ operation: 'readSource', references: search.sourceReferences.slice(0, 1) }),
);
assert.equal(
  JSON.parse(
    run('cli-source', core, ['query', '--config', config, '--request', request], cliEnv, [0, 2]),
  ).data.reads[0].status,
  'current',
);
run('agent-help', cli, ['agent', '--help'], cliEnv);
const missing = JSON.parse(
  run('core-agent-disabled', core, ['agent', 'configure', '--config', config], cliEnv, [1]),
);
assert.ok(missing.error);
// Verify the binaries use embedded resources while all build tools are absent from PATH.
run('runtime-exclusions', node, [
  '--input-type=module',
  '-e',
  `import {spawnSync} from 'node:child_process';for(const command of ['rustc','cargo','cl','gcc','python','pnpm']) {const r=spawnSync(command,['--version']);if(!r.error) throw Error(command+' unexpectedly in runtime PATH');}`,
]);
writeFileSync(
  join(directory, 'package.json'),
  JSON.stringify({ name: 'clean-consumer', private: true, type: 'module' }),
);
const require = createRequire(join(root, 'packages/mcp/package.json'));
const clientVersion = installedPackageMetadata(require, '@modelcontextprotocol/client').version;
run(
  'npm-install',
  process.execPath,
  [
    npm,
    'install',
    '--ignore-scripts',
    '--no-audit',
    '--no-fund',
    ...manifest.packages.map((pkg) => join(artifacts, pkg.path)),
    `@modelcontextprotocol/client@${clientVersion}`,
  ],
  sanitized,
);
// Repeat entirely from the generated lock using npm's cache, with registry networking disabled.
run(
  'npm-ci-offline',
  process.execPath,
  [npm, 'ci', '--offline', '--ignore-scripts', '--no-audit', '--no-fund'],
  sanitized,
);
copyFileSync(join(root, 'scripts/package-consumer.mjs'), join(directory, 'verify.mjs'));
run('sdk-mcp-consumer', node, ['verify.mjs', config]);
const consumer = JSON.parse(readFileSync(join(directory, 'consumer-report.json')));
assert.equal(consumer.schemaHash, manifest.schemaHash);
const report = {
  passed: true,
  artifacts,
  directory,
  manifestSha256: sha(join(artifacts, 'release-manifest.json')),
  consumer,
  commands,
  platform: process.platform,
  architecture: process.arch,
  buildProfile: manifest.profile,
  externalModelRequests: 0,
  localProviderRequests: consumer.localProviderRequests,
  businessDatabaseConnections: 0,
  publication: 'not-published',
  limitations: [
    'Only the current host is validated',
    'No new live-model business run; prior T19 evidence remains separate',
    'Node/Python plugins retain their explicit runtime requirements',
  ],
};
writeFileSync(join(artifacts, 'installation-report.json'), JSON.stringify(report, null, 2) + '\n');
console.log(
  JSON.stringify({ passed: true, directory, report: join(artifacts, 'installation-report.json') }),
);
