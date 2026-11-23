// Resolve the host explicitly so napi metadata does not fetch unrelated platform crates.
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { execFileSync, spawnSync } from 'node:child_process';

const require = createRequire(import.meta.url);
const packagePath = require.resolve('@napi-rs/cli/package.json');
const pkg = JSON.parse(readFileSync(packagePath, 'utf8'));
const host = execFileSync('rustc', ['-vV'], { encoding: 'utf8' }).match(/^host: (.+)$/m)?.[1];
if (!host) throw new Error('Cannot determine Rust host target');
const options = process.argv.slice(2);
const result = spawnSync(
  process.execPath,
  [
    resolve(dirname(packagePath), pkg.bin.napi),
    'build',
    '--platform',
    '--target',
    process.env.CARGO_BUILD_TARGET ?? host,
    ...options,
    '--',
    '--locked',
  ],
  { stdio: 'inherit' },
);
if (result.error) throw result.error;
process.exit(result.status ?? 1);
