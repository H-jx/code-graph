// Zero-product-dependency repro of the Node 24.14.0 / libuv Windows fs.cpSync
// non-ASCII path defect that caused the T23 consumer 0xC0000409 crash.
// Each case runs in its own child process so a crash does not stop the report.
// Run: node cpsync-repro.mjs   (Windows + Node 24.14.0)
import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync, mkdtempSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';

const base = mkdtempSync(join(tmpdir(), 'cpsync-repro-'));
const asciiSrc = join(base, 'ascii-src');
const zhSrc = join(base, '中文源');
for (const src of [asciiSrc, zhSrc]) {
  mkdirSync(join(src, 'sub'), { recursive: true });
  writeFileSync(join(src, 'a.js'), 'export const a = 1;\n');
  writeFileSync(join(src, 'sub', 'b.js'), 'export const b = 2;\n');
}
const child = `
import { cpSync, existsSync, readdirSync } from 'node:fs';
const [src, dst] = process.argv.slice(2);
cpSync(src, dst, { recursive: true });
console.log('SURVIVED files=' + (existsSync(dst) ? readdirSync(dst).length : 'MISSING'));
`;
const childPath = join(base, 'child.mjs');
writeFileSync(childPath, child);
const cases = [
  ['ascii-src -> ascii-dst', asciiSrc, join(base, 'out-aa')],
  ['zh-src    -> ascii-dst', zhSrc, join(base, 'out-za')],
  ['ascii-src -> zh-dst   ', asciiSrc, join(base, 'out-az-目标')],
  ['zh-src    -> zh-dst   ', zhSrc, join(base, 'out-zz-目标')],
];
const results = [];
for (const [label, src, dst] of cases) {
  const r = spawnSync(process.execPath, [childPath, src, dst], { encoding: 'utf8', timeout: 30000 });
  const crashed = r.status === -1073740791 || r.status === 3221226505;
  results.push({
    case: label,
    exitCode: r.status,
    crashed0xC0000409: crashed,
    stdout: (r.stdout ?? '').trim(),
  });
  console.log(
    `${label}  exit=${r.status}${crashed ? '  (0xC0000409 CRASH)' : ''}  ${(r.stdout ?? '').trim()}`,
  );
}
console.log('\nnode', process.version, 'platform', process.platform);
console.log(
  'Expected on affected Node: zh-src cases crash 0xC0000409; ascii-src->zh-dst silently MISSING.',
);
rmSync(base, { recursive: true, force: true });