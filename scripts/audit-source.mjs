// Inventory unfinished source without treating .gitkeep files or module exports as implementations.
import { spawnSync } from 'node:child_process';
import { readFileSync, writeFileSync, statSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const root = fileURLToPath(new URL('../', import.meta.url));
const result = spawnSync(
  'rg',
  ['--files', 'crates', 'bindings', 'packages', 'collectors', 'scripts', 'xtask', 'skills'],
  { cwd: root, encoding: 'utf8' },
);
if (result.error || result.status !== 0) throw result.error ?? Error(result.stderr);
const files = result.stdout
  .trim()
  .split(/\r?\n/)
  .map((name) => name.replaceAll('\\', '/'))
  .filter(
    (name) =>
      /\.(?:rs|ts|tsx|js|mjs|py|go)$/.test(name) &&
      !/(?:^|\/)(?:dist|node_modules|deps)(?:\/|$)/.test(name),
  )
  .sort();
const task = (name) => {
  if (name.startsWith('bindings/') || name.startsWith('packages/sdk/')) return 'T20';
  if (name.startsWith('packages/mcp/')) return 'T21';
  if (name.includes('/knowledge/') || name.endsWith('/generate.rs')) return 'T17';
  if (/\/(?:graph|codemap)\//.test(name)) return 'T15';
  if (name.startsWith('crates/core/src/query/') && !name.endsWith('/mod.rs')) return 'T16';
  if (name.includes('verify-scenarios')) return 'T16/T22';
  if (name.includes('verify-packages') || name.startsWith('xtask/')) return 'T23';
  if (name.includes('/syntax/scip.rs')) return 'optional-out-of-scope';
  return null;
};
const rows = files.map((name) => {
  const bytes = readFileSync(path.join(root, name));
  const source = bytes.toString('utf8');
  const withoutComments = source
    .replace(/\/\*[\s\S]*?\*\//g, '')
    .replace(/^\s*\/\/.*$/gm, '')
    .replace(/^\s*#.*$/gm, '')
    .trim();
  const markers = source
    .split(/\r?\n/)
    .flatMap((line, index) =>
      /TODO\(pk\)|todo!\s*\(|unimplemented!\s*\(|尚未实现|未实现|scenario_skeleton\s*\(/.test(line)
        ? [{ line: index + 1, text: line.trim().slice(0, 220) }]
        : [],
    );
  return {
    path: name,
    bytes: statSync(path.join(root, name)).size,
    sha256: createHash('sha256').update(bytes).digest('hex'),
    zeroBytes: bytes.length === 0,
    commentOnly: bytes.length > 0 && withoutComments.length === 0,
    emptyModule: withoutComments === 'export {};',
    task: task(name),
    markers,
  };
});
const report = {
  scope:
    'Product, example plugin and verification source; excludes dependency/build output and .gitkeep.',
  method:
    'Zero byte check; comment-only lexical check; explicit placeholder/skeleton markers. Semantic completeness still requires source review and task acceptance.',
  filesScanned: rows.length,
  zeroByteSources: rows.filter((row) => row.zeroBytes),
  commentOnlySources: rows.filter((row) => row.commentOnly),
  emptyModuleSources: rows.filter((row) => row.emptyModule),
  explicitPlaceholders: rows.filter((row) => row.markers.length),
  fileHashes: Object.fromEntries(rows.map((row) => [row.path, row.sha256])),
};
const outputIndex = process.argv.indexOf('--output');
if (outputIndex >= 0)
  writeFileSync(
    path.resolve(root, process.argv[outputIndex + 1]),
    JSON.stringify(report, null, 2) + '\n',
  );
console.log(
  JSON.stringify(
    {
      filesScanned: rows.length,
      zeroByteSources: report.zeroByteSources.length,
      commentOnlySources: report.commentOnlySources.map((row) => ({
        path: row.path,
        task: row.task,
      })),
      emptyModuleSources: report.emptyModuleSources.map((row) => ({
        path: row.path,
        task: row.task,
      })),
      explicitPlaceholderFiles: report.explicitPlaceholders.length,
    },
    null,
    2,
  ),
);
