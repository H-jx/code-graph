// 显式联网 prepare；后续验证/激活/采集必须只复用缓存，合成协议 smoke。
import assert from 'node:assert/strict';
import {
  readdirSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  writeFileSync,
  existsSync,
  statSync,
} from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const outputFlag = process.argv.indexOf('--output');
const output = resolve(
  root,
  outputFlag >= 0 ? process.argv[outputFlag + 1] : 'docs/evidence/T07/dependencies',
);
mkdirSync(output, { recursive: true });
const work = mkdtempSync(join(output, '运行 空格-'));
let python = process.env.PROJECT_KNOWLEDGE_PYTHON;
if (!python) {
  for (const candidate of ['python', 'python3']) {
    const result = spawnSync(candidate, ['--version'], { encoding: 'utf8' });
    if (result.status === 0 && result.stdout.startsWith('Python 3.12.')) {
      python = candidate;
      break;
    }
  }
}
if (!python) {
  const result = spawnSync('uv', ['python', 'find', '3.12'], { encoding: 'utf8' });
  assert.equal(result.status, 0, result.stderr);
  python = result.stdout.trim();
}
if (!python.includes('/') && !python.includes('\\')) {
  const result = spawnSync(python, ['-c', 'import sys; print(sys.executable)'], {
    encoding: 'utf8',
  });
  assert.equal(result.status, 0);
  python = result.stdout.trim();
}
const cli =
  process.env.PROJECT_KNOWLEDGE_CLI ??
  join(
    root,
    'target/debug',
    process.platform === 'win32' ? 'project-knowledge.exe' : 'project-knowledge',
  );
const config = join(work, 'project.json');
mkdirSync(join(work, 'repo'));
writeFileSync(join(work, 'repo/a.ts'), 'export const smoke=1;\n');
writeFileSync(
  config,
  JSON.stringify({
    schemaVersion: 1,
    projectId: 'dependency-smoke',
    repositories: [{ id: 'r', root: 'repo' }],
    selection: {},
    runtimes: [{ id: 'python312', kind: 'python', executable: python }],
  }),
);
const commands = [];
function copyTree(source, destination) {
  mkdirSync(destination, { recursive: true });
  for (const entry of readdirSync(source, { withFileTypes: true })) {
    const from = join(source, entry.name),
      to = join(destination, entry.name);
    if (entry.isDirectory()) copyTree(from, to);
    else writeFileSync(to, readFileSync(from));
  }
}
const sha = (bytes) => createHash('sha256').update(bytes).digest('hex');
function run(label, args, exitCode = 0) {
  const result = spawnSync(cli, args, {
    encoding: 'utf8',
    timeout: 180000,
    maxBuffer: 4 * 1024 * 1024,
    env: {
      ...process.env,
      OPENAI_API_KEY: 'probe-value-must-not-leak',
      ANTHROPIC_API_KEY: 'probe-value-must-not-leak',
    },
  });
  writeFileSync(join(work, label + '.stdout.json'), result.stdout ?? '');
  writeFileSync(join(work, label + '.stderr.log'), result.stderr ?? '');
  commands.push({ label, args, exitCode: result.status, stdoutHash: sha(result.stdout ?? '') });
  assert.equal(
    result.status,
    exitCode,
    `${label}: ${result.stdout} ${result.stderr} ${result.error ?? ''}`,
  );
  return JSON.parse(result.stdout);
}
run('base', ['index', '--config', config, '--base']);
const caches = [];
for (const kind of ['node', 'python']) {
  const directory = join(work, kind);
  copyTree(join(root, 'fixtures/plugins/dependencies', kind), directory);
  const manifestPath = join(directory, 'manifest.json');
  const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
  manifest.version = `1.0.${Date.now()}`;
  writeFileSync(manifestPath, JSON.stringify(manifest));
  const candidate = run(`${kind}-submit`, [
    'plugin',
    '--config',
    config,
    'submit',
    '--path',
    directory,
  ]);
  const before = run(
    `${kind}-missing-cache`,
    ['plugin', '--config', config, 'validate', '--candidate', candidate.id],
    1,
  );
  assert.equal(before.error.code, 'DEPENDENCY_PREPARE_FAILED');
  const prepared = run(`${kind}-prepare`, [
    'plugin',
    '--config',
    config,
    'prepare',
    '--candidate',
    candidate.id,
  ]);
  assert.equal(prepared.status, 'dependenciesPrepared');
  assert.equal(existsSync(join(prepared.cache.directory, 'install-script-ran')), false);
  const log = join(prepared.cache.directory, 'prepare.log');
  const logHash = sha(readFileSync(log));
  const modified = statSync(log).mtimeMs;
  writeFileSync(join(work, `${kind}-install.log`), readFileSync(log));
  run(`${kind}-validate`, ['plugin', '--config', config, 'validate', '--candidate', candidate.id]);
  run(`${kind}-activate`, ['plugin', '--config', config, 'activate', '--candidate', candidate.id]);
  run(`${kind}-collect`, ['collect', '--config', config, '--plugin', manifest.pluginId]);
  const reused = run(`${kind}-reuse-prepare`, [
    'plugin',
    '--config',
    config,
    'prepare',
    '--candidate',
    candidate.id,
  ]);
  assert.equal(reused.cache.integrity, prepared.cache.integrity);
  assert.equal(sha(readFileSync(log)), logHash);
  assert.equal(statSync(log).mtimeMs, modified);
  // 在已验证与 activate 之间篡改真实依赖文件；不能继续提交旧验证。
  const again = run(`${kind}-resubmit`, [
    'plugin',
    '--config',
    config,
    'submit',
    '--path',
    directory,
  ]);
  run(`${kind}-revalidate`, ['plugin', '--config', config, 'validate', '--candidate', again.id]);
  const file =
    kind === 'node'
      ? join(prepared.cache.directory, 'node_modules/is-number/index.js')
      : join(
          prepared.cache.directory,
          'venv',
          process.platform === 'win32'
            ? 'Lib/site-packages/packaging/__init__.py'
            : 'lib/python3.12/site-packages/packaging/__init__.py',
        );
  const bytes = readFileSync(file);
  try {
    writeFileSync(file, Buffer.concat([bytes, Buffer.from('\n# tamper\n')]));
    const rejected = run(
      `${kind}-tampered-activate`,
      ['plugin', '--config', config, 'activate', '--candidate', again.id],
      1,
    );
    assert.equal(rejected.error.code, 'DEPENDENCY_CHANGED');
    const collect = run(
      `${kind}-tampered-collect`,
      ['collect', '--config', config, '--manifest', manifestPath, '--validate-only'],
      1,
    );
    assert.equal(collect.error.code, 'DEPENDENCY_CHANGED');
  } finally {
    writeFileSync(file, bytes);
  }
  caches.push({
    kind,
    directory: prepared.cache.directory,
    digest: prepared.cache.digest,
    integrity: prepared.cache.integrity,
    installLogHash: logHash,
    reusedWithoutInstaller: true,
  });
  // 错误锁：npm manifest/lock 不匹配；pip 哈希不匹配。失败后插件不能执行。
  const bad = join(work, 'bad-' + kind);
  copyTree(directory, bad);
  const badManifest = JSON.parse(readFileSync(join(bad, 'manifest.json'), 'utf8'));
  badManifest.pluginId += '-bad';
  writeFileSync(join(bad, 'manifest.json'), JSON.stringify(badManifest));
  if (kind === 'node') {
    const pkg = JSON.parse(readFileSync(join(bad, 'deps/package.json'), 'utf8'));
    pkg.dependencies['is-number'] = '6.0.0';
    writeFileSync(join(bad, 'deps/package.json'), JSON.stringify(pkg));
  } else {
    writeFileSync(
      join(bad, 'deps/requirements.lock'),
      'packaging==25.0 --hash=sha256:' + '0'.repeat(64) + '\n',
    );
  }
  const badCandidate = run(`${kind}-bad-submit`, [
    'plugin',
    '--config',
    config,
    'submit',
    '--path',
    bad,
  ]);
  const failure = run(
    `${kind}-bad-lock`,
    ['plugin', '--config', config, 'prepare', '--candidate', badCandidate.id],
    1,
  );
  assert.equal(failure.error.code, 'DEPENDENCY_PREPARE_FAILED');
}
const latest = join(output, 'latest');
mkdirSync(latest, { recursive: true });
const outputs = [];
for (const entry of readdirSync(work, { withFileTypes: true })) {
  if (entry.isFile() && (entry.name.endsWith('.json') || entry.name.endsWith('.log'))) {
    const bytes = readFileSync(join(work, entry.name));
    writeFileSync(join(latest, entry.name), bytes);
    outputs.push({ file: entry.name, hash: sha(bytes) });
  }
}
const report = {
  date: new Date().toISOString(),
  work,
  commands,
  caches,
  outputs,
  providerCalls: 0,
  independentBusinessTruth: false,
  networkBoundary:
    'Only explicit prepare installs. Later commands leave install logs and mtimes unchanged; no OS network namespace was used.',
};
writeFileSync(join(output, 'report.json'), JSON.stringify(report, null, 2) + '\n');
console.log(
  JSON.stringify({
    ok: true,
    commands: commands.length,
    caches: caches.length,
    report: join(output, 'report.json'),
  }),
);
