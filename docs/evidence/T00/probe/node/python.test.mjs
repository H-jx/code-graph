import assert from 'node:assert/strict';
import test from 'node:test';
import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';
import { native, root, valid } from './shared.mjs';

test('Python 3.12 parses UTF-8 AST byte positions and runs the protocol template', () => {
  const python = process.env.PROJECT_KNOWLEDGE_PYTHON || 'python';
  const args = ['-X', 'utf8'];
  const ast = spawnSync(
    python,
    [
      ...args,
      '-c',
      `import ast,json,sys
assert sys.version_info[:2]==(3,12)
source="变量 = '😀'"
node=ast.parse(source).body[0].value
assert node.col_offset==len("变量 = ".encode('utf-8'))
assert node.col_offset!=len("变量 = ".encode('utf-16-le'))//2
assert ast.get_source_segment(source,node)=="'😀'"
print(json.dumps({'python':sys.version.split()[0],'startByte':node.col_offset,'endByte':node.end_col_offset}))`,
    ],
    { encoding: 'utf8', timeout: 5000 },
  );
  assert.equal(ast.status, 0, ast.stderr);
  console.log(ast.stdout.trim());
  const outcome = spawnSync(
    python,
    [...args, resolve(root, 'collectors/templates/python/src/collect.py')],
    {
      input: JSON.stringify({
        ...valid,
        pluginId: 'template.python',
        version: '0.1.0',
        sources: [],
        sourceSets: [],
        candidateSets: [],
        configSubset: { title: '中文😀' },
      }),
      encoding: 'utf8',
      timeout: 5000,
    },
  );
  assert.equal(outcome.status, 0, outcome.stderr);
  const payload = JSON.parse(outcome.stdout);
  assert.equal(payload.inputSnapshotId, valid.inputSnapshotId);
  assert.deepEqual(JSON.parse(native().validateCollector(JSON.stringify(payload))), {
    valid: true,
  });
});
