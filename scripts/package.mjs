// Build on the intended host. Cross-target compilation is not installation evidence.
import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { createRequire } from 'node:module';
import {
  readFileSync,
  writeFileSync,
  mkdirSync,
  cpSync,
  copyFileSync,
  existsSync,
  mkdtempSync,
} from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { release } from 'node:os';
import { installedPackageMetadata } from './package-metadata.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const require = createRequire(import.meta.url);
const args = process.argv.slice(2);
const option = (name, fallback) => (args.includes(name) ? args[args.indexOf(name) + 1] : fallback);
const rust = execFileSync('rustc', ['-vV'], { encoding: 'utf8' });
const host = rust.match(/^host: (.+)$/m)?.[1];
const target = option('--target', host);
assert.equal(target, host, 'Build and verify each candidate on its native host');
const profile = option('--profile', 'release');
assert.ok(['dev', 'release', 'dist'].includes(profile));
const platforms = {
  'x86_64-pc-windows-msvc': { suffix: 'win32-x64-msvc', os: 'win32', cpu: 'x64' },
  'x86_64-unknown-linux-gnu': { suffix: 'linux-x64-gnu', os: 'linux', cpu: 'x64', libc: 'glibc' },
  'x86_64-apple-darwin': { suffix: 'darwin-x64', os: 'darwin', cpu: 'x64' },
  'aarch64-apple-darwin': { suffix: 'darwin-arm64', os: 'darwin', cpu: 'arm64' },
};
const platform = platforms[target];
assert.ok(platform, 'Unsupported candidate target');
if (process.platform === 'linux')
  assert.ok(process.report.getReport().header.glibcVersionRuntime, 'glibc required');
const version = JSON.parse(readFileSync(join(root, 'package.json'))).version;
const parent = join(root, 'target/release-candidate', version, target);
mkdirSync(parent, { recursive: true });
const out = mkdtempSync(join(parent, 'candidate-'));
const stage = join(out, 'staging');
mkdirSync(stage);
const npm = join(dirname(process.execPath), 'node_modules/npm/bin/npm-cli.js');
assert.ok(existsSync(npm), 'npm required to build tarballs');
const commands = [];
const sha = (path) => createHash('sha256').update(readFileSync(path)).digest('hex');
const save = (path, value) => writeFileSync(path, JSON.stringify(value, null, 2) + '\n');
function run(label, command, parameters, cwd = root, env = process.env) {
  const log = join(out, `${label}.log`);
  try {
    const result = spawnSync(command, parameters, {
      cwd,
      env,
      encoding: 'utf8',
      maxBuffer: 64 * 1024 * 1024,
      timeout: 2400000,
    });
    writeFileSync(log, (result.stdout ?? '') + (result.stderr ?? ''));
    if (result.error || result.status !== 0)
      throw Object.assign(new Error('command failed'), {
        stdout: result.stdout,
        stderr: result.stderr,
        status: result.status,
      });
    commands.push({
      label,
      command,
      parameters,
      exitCode: 0,
      log: `${label}.log`,
      sha256: sha(log),
    });
    save(join(out, 'commands.json'), commands);
    return result.stdout;
  } catch (error) {
    writeFileSync(log, (error.stdout ?? '') + (error.stderr ?? ''));
    commands.push({
      label,
      command,
      parameters,
      exitCode: error.status ?? -1,
      log: `${label}.log`,
      sha256: sha(log),
    });
    save(join(out, 'commands.json'), commands);
    throw Error(`${label} failed; inspect ${log}`);
  }
}
console.log(`Building ${profile} candidate: ${out}`);
const sourcePaths = [
  ...execFileSync(
    'rg',
    [
      '--files',
      'crates',
      'bindings/node',
      'packages/sdk/src',
      'packages/mcp/src',
      'prompts',
      'xtask/src',
    ],
    { cwd: root, encoding: 'utf8' },
  )
    .trim()
    .split(/\r?\n/)
    .map((p) => p.replaceAll('\\', '/'))
    .filter(
      (p) =>
        !p
          .split('/')
          .some((part) => ['node_modules', 'dist', 'deps', 'npm', 'artifacts'].includes(part)),
    ),
  'Cargo.toml',
  'Cargo.lock',
  'package.json',
  'pnpm-lock.yaml',
  'packages/sdk/package.json',
  'packages/mcp/package.json',
  'docs/schema/contract.schema.json',
  'scripts/package.mjs',
  'scripts/package-consumer.mjs',
  'scripts/verify-packages.mjs',
];
const sourceHashes = () =>
  Object.fromEntries([...new Set(sourcePaths)].sort().map((p) => [p, sha(join(root, p))]));
const sourceManifest = sourceHashes();
save(join(out, 'source-manifest.json'), sourceManifest);
const cliName = process.platform === 'win32' ? 'project-knowledge.exe' : 'project-knowledge';
const buildDirectory = join(root, 'target', target, profile === 'dev' ? 'debug' : profile);
const cliOut = join(out, 'cli');
mkdirSync(cliOut);
for (const coreOnly of [true, false]) {
  run(coreOnly ? 'build-core-only' : 'build-cli', 'cargo', [
    'build',
    '-p',
    'project-knowledge-cli',
    '--locked',
    '--target',
    target,
    '--profile',
    profile,
    '-j',
    '1',
    ...(coreOnly ? ['--no-default-features'] : []),
  ]);
  copyFileSync(
    join(buildDirectory, cliName),
    join(
      cliOut,
      coreOnly ? cliName.replace('project-knowledge', 'project-knowledge-core') : cliName,
    ),
  );
}
const binding = join(root, 'bindings/node');
const nativeBuild = join(out, 'native-build');
run(
  'build-native',
  process.execPath,
  [join(binding, 'build.mjs'), '--profile', profile, '--output-dir', nativeBuild],
  binding,
  {
    ...process.env,
    CARGO_BUILD_TARGET: target,
  },
);
const tsc = require.resolve('typescript/bin/tsc');
for (const pkg of ['sdk', 'mcp'])
  run(`build-${pkg}`, process.execPath, [
    tsc,
    '-p',
    join(root, `packages/${pkg}/tsconfig.json`),
    '--outDir',
    join(out, `${pkg}-dist`),
  ]);
const licenses = ['LICENSE-MIT', 'LICENSE-APACHE'];
for (const name of licenses) {
  assert.ok(existsSync(join(root, name)), `Missing ${name}`);
  copyFileSync(join(root, name), join(cliOut, name));
}
copyFileSync(join(root, 'README.md'), join(cliOut, 'README.md'));
const archive = `project-knowledge-${version}-${platform.suffix}.tar.gz`;
run('archive-cli', 'tar', ['-czf', join(out, archive), '-C', cliOut, '.']);
const basePackage = (name, files) => ({
  name,
  version,
  license: 'MIT OR Apache-2.0',
  engines: { node: '>=24.0.0' },
  files: [...files, ...licenses],
});
const platformName = `@project-knowledge/native-${platform.suffix}`;
const specs = [
  {
    dir: 'platform',
    manifest: {
      ...basePackage(platformName, [`project-knowledge-native.${platform.suffix}.node`]),
      main: `project-knowledge-native.${platform.suffix}.node`,
      os: [platform.os],
      cpu: [platform.cpu],
      ...(platform.libc ? { libc: [platform.libc] } : {}),
    },
  },
  {
    dir: 'native',
    manifest: {
      ...basePackage('@project-knowledge/native', ['index.js', 'index.d.ts']),
      main: 'index.js',
      types: 'index.d.ts',
      optionalDependencies: { [platformName]: version },
    },
  },
  ...['sdk', 'mcp'].map((pkg) => {
    const original = JSON.parse(readFileSync(join(root, `packages/${pkg}/package.json`)));
    const localRequire = createRequire(join(root, `packages/${pkg}/package.json`));
    const dependencies = Object.fromEntries(
      Object.keys(original.dependencies).map((name) => [
        name,
        name.startsWith('@project-knowledge/')
          ? version
          : installedPackageMetadata(localRequire, name).version,
      ]),
    );
    return {
      dir: pkg,
      manifest: {
        ...basePackage(original.name, ['dist']),
        type: 'module',
        main: original.main,
        types: original.types,
        exports: original.exports,
        ...(original.bin ? { bin: original.bin } : {}),
        dependencies,
      },
    };
  }),
];
const tarballs = [];
for (const { dir, manifest } of specs) {
  const directory = join(stage, dir);
  mkdirSync(directory);
  save(join(directory, 'package.json'), manifest);
  for (const name of licenses) copyFileSync(join(root, name), join(directory, name));
  if (dir === 'platform')
    copyFileSync(
      join(nativeBuild, `project-knowledge-native.${platform.suffix}.node`),
      join(directory, manifest.main),
    );
  else if (dir === 'native')
    for (const name of ['index.js', 'index.d.ts'])
      copyFileSync(join(nativeBuild, name), join(directory, name));
  else cpSync(join(out, `${dir}-dist`), join(directory, 'dist'), { recursive: true });
  const dry = JSON.parse(
    run(
      `dry-run-${dir}`,
      process.execPath,
      [npm, 'pack', '--dry-run', '--json', '--ignore-scripts'],
      directory,
    ),
  );
  for (const file of dry[0].files)
    assert.ok(
      !/(?:\.env|\.sqlite|node_modules|work\/candidates)/.test(file.path),
      `Forbidden package file ${file.path}`,
    );
  const packed = JSON.parse(
    run(
      `pack-${dir}`,
      process.execPath,
      [npm, 'pack', '--json', '--ignore-scripts', '--pack-destination', out],
      directory,
    ),
  );
  tarballs.push({
    name: manifest.name,
    path: packed[0].filename,
    sha256: sha(join(out, packed[0].filename)),
    files: dry[0].files.map((f) => f.path),
  });
}
const metadata = JSON.parse(
  run('cargo-metadata', 'cargo', [
    'metadata',
    '--format-version',
    '1',
    '--locked',
    '--filter-platform',
    target,
  ]),
);
const sqlitePackage = metadata.packages.find((p) => p.name === 'libsqlite3-sys');
assert.ok(sqlitePackage);
const sqliteVersion = readFileSync(
  join(dirname(sqlitePackage.manifest_path), 'sqlite3/sqlite3.h'),
  'utf8',
).match(/^#define SQLITE_VERSION\s+"([^"]+)"/m)?.[1];
assert.ok(sqliteVersion);
const databaseSchemaVersion = Number(
  readFileSync(join(root, 'crates/core/src/storage/database.rs'), 'utf8').match(
    /pub const SCHEMA_VERSION: i64 = (\d+);/,
  )?.[1],
);
assert.ok(databaseSchemaVersion);
save(join(out, 'sbom.json'), {
  format: 'project-knowledge-dependency-inventory-v1',
  description: 'Locked inventory; not a vulnerability or license-compliance certification',
  rust: metadata.packages.map((p) => ({
    name: p.name,
    version: p.version,
    license: p.license,
    source: p.source,
  })),
  npmDirect: specs.map((s) => ({
    name: s.manifest.name,
    version,
    dependencies: s.manifest.dependencies ?? {},
    optionalDependencies: s.manifest.optionalDependencies ?? {},
  })),
});
const capabilities = JSON.parse(
  run('capabilities', process.execPath, [
    '--input-type=module',
    '-e',
    `import {createRequire} from 'node:module';const addon=createRequire(import.meta.url)(${JSON.stringify(join(nativeBuild, `project-knowledge-native.${platform.suffix}.node`))});console.log(await addon.getCapabilities());`,
  ]),
);
assert.deepEqual(
  sourceHashes(),
  sourceManifest,
  'Source changed during candidate build; use a new candidate',
);
const outputs = [archive, 'sbom.json', 'source-manifest.json', ...tarballs.map((t) => t.path)];
for (const name of ['Cargo.lock', 'pnpm-lock.yaml']) {
  copyFileSync(join(root, name), join(out, name));
  outputs.push(name);
}
save(join(out, 'release-manifest.json'), {
  version,
  target,
  profile,
  platform: process.platform,
  architecture: process.arch,
  osRelease: release(),
  glibc: process.report.getReport().header.glibcVersionRuntime ?? null,
  protocolVersion: capabilities.protocolVersion,
  schemaHash: capabilities.schemaHash,
  databaseSchemaVersion,
  configSchemaVersion: capabilities.schema['x-schemaVersion'],
  components: {
    cli: version,
    core: version,
    agent: version,
    native: version,
    sdk: version,
    mcp: version,
    napiRs: metadata.packages.find((p) => p.name === 'napi')?.version,
    sqlite: sqliteVersion,
    libsqlite3Sys: sqlitePackage.version,
    sqliteBundled: true,
  },
  nativeBinarySha256: sha(join(nativeBuild, `project-knowledge-native.${platform.suffix}.node`)),
  node: process.version,
  rust: rust.trim(),
  lockHashes: Object.fromEntries(
    ['Cargo.lock', 'pnpm-lock.yaml'].map((p) => [p, sha(join(root, p))]),
  ),
  cli: archive,
  cliBinaryHashes: Object.fromEntries(
    [cliName, cliName.replace('project-knowledge', 'project-knowledge-core')].map((name) => [
      name,
      sha(join(cliOut, name)),
    ]),
  ),
  packages: tarballs,
  sha256: Object.fromEntries(outputs.map((p) => [p, sha(join(out, p))])),
  publication: 'not-published',
  installation: 'not-yet-verified',
  otherPlatforms: 'not-validated-by-this-run',
});
writeFileSync(
  join(out, 'SHA256SUMS'),
  outputs.map((p) => `${sha(join(out, p))}  ${p}`).join('\n') + '\n',
);
console.log(
  JSON.stringify({
    candidate: out,
    manifest: join(out, 'release-manifest.json'),
    publiclyPublished: false,
  }),
);
