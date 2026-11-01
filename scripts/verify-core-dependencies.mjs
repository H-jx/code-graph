import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';

const result = spawnSync(
  'cargo',
  ['tree', '-p', 'project-knowledge-core', '--edges', 'normal', '--locked', '--prefix', 'none'],
  { encoding: 'utf8', timeout: 60000, maxBuffer: 4 * 1024 * 1024 },
);
if (result.error || result.status !== 0) {
  console.error(result.error?.message || result.stderr);
  process.exit(1);
}
const packages = result.stdout
  .split(/\r?\n/)
  .map((line) => line.match(/^([\w-]+) v/)?.[1])
  .filter(Boolean);
assert.ok(packages.length > 0, 'cargo tree must contain real package nodes');
const forbidden = [
  ...new Set(
    packages.filter((name) => /^rig(?:-|$)/.test(name) || name === 'project-knowledge-agent'),
  ),
];
assert.deepEqual(forbidden, [], 'Core must not depend on Agent or Rig');
console.log(JSON.stringify({ nodes: packages.length, forbidden }));
