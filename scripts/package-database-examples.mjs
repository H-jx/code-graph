import { readFileSync, writeFileSync, mkdirSync, readdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { computeSourceId, computeEntityId, ENTITY_COMPONENTS } from './identity.mjs';

const root = fileURLToPath(new URL('../', import.meta.url));
const read = (name) => readFileSync(path.join(root, name), 'utf8');
const json = (value) => JSON.stringify(value, null, 2) + '\n';
const hash = (content) => createHash('sha256').update(content).digest('hex');
const check = process.argv.includes('--check');
const metadata = JSON.parse(
  read('docs/evidence/T12/database/dependency-metadata.json').replace(/^\uFEFF/, ''),
);
const baseline = JSON.parse(read('fixtures/truth/manifest.json'));
const reviewText = read('fixtures/truth-supplements/T12-database/review.json');
const review = JSON.parse(reviewText);
if (review.baselineTruthHash !== baseline.truthHash)
  throw Error('Database truth baseline mismatch');
for (const [name, expected] of Object.entries(review.fileHashes)) {
  if (hash(readFileSync(path.join(root, name))) !== expected)
    throw Error('Database review hash mismatch: ' + name);
}
function put(name, content) {
  if (check) {
    if (read(name) !== content) throw Error('Outdated database artifact: ' + name);
  } else {
    mkdirSync(path.dirname(path.join(root, name)), { recursive: true });
    writeFileSync(path.join(root, name), content);
  }
}
for (const key of ['a', 'b']) {
  const base = `collectors/examples/frontend-backend-${key}/database`;
  const config = JSON.parse(read(`fixtures/page-location/${key}/project.json`));
  const repo = config.repositories.find((repo) => repo.id === 'back');
  const sources = new Map();
  function source(name, content) {
    sources.set(name, {
      id: computeSourceId(config.projectId, 'back', name),
      repositoryId: 'back',
      path: name,
      sourceClass: 'code',
      contentHash: hash(content),
      byteLength: Buffer.byteLength(content),
      content,
    });
  }
  function scan(folder, relative = '') {
    for (const entry of readdirSync(path.join(root, folder), { withFileTypes: true })) {
      const name = relative + entry.name;
      if (entry.isDirectory()) scan(folder + '/' + entry.name, name + '/');
      else source(name, read(folder + '/' + entry.name));
    }
  }
  scan(`fixtures/page-location/${key}/${repo.root}`);
  for (const [name, content] of Object.entries({
    'schema.sql': read('fixtures/purchase-ledger/schema.sql'),
    'analytics.sql': read('fixtures/purchase-ledger/schema.sql'),
    'correct.sql': read('fixtures/purchase-ledger/correct.sql'),
    'fanout.sql': read('fixtures/purchase-ledger/fanout.sql'),
    'where-child.sql': read('fixtures/purchase-ledger/where-child.sql'),
    [`database.${key === 'a' ? 'ts' : 'py'}`]: read(
      `fixtures/truth-supplements/T12-database/database.${key === 'a' ? 'ts' : 'py'}`,
    ),
  }))
    source(name, content);
  const rules = {
    dialects: ['sqlite'],
    ormObject: 'orm',
    connectMethod: 'connect',
    modelMethod: 'model',
    queryMethod: 'query',
    sourceBindings: Object.fromEntries(
      ['schema.sql', 'analytics.sql', 'correct.sql', 'fanout.sql', 'where-child.sql'].map(
        (name) => [name, name === 'analytics.sql' ? 'ds.analytics' : 'ds.erp'],
      ),
    ),
  };
  const pluginId = `example.${key}.database`;
  const artifacts = [
    'manifest.json',
    'conventions.json',
    'collect.py',
    'database_ast.py',
    'source_ast.py',
    'identity.py',
    'identity-spec.json',
    'deps/requirements.in',
    'deps/requirements.lock',
    'cases/input.json',
    'cases/database.json',
    'cases/truth.json',
  ];
  for (const [target, from] of Object.entries({
    'database_ast.py': 'collectors/templates/python/lib/database_ast.py',
    'source_ast.py': 'collectors/templates/python/lib/source_ast.py',
    'identity.py': 'scripts/verify-vectors.py',
    'identity-spec.json': 'docs/schema/entity-kinds.json',
  }))
    put(base + '/' + target, read(from));
  put(
    base + '/collect.py',
    "import json\nimport pathlib\nimport sys\nfrom database_ast import collect_database\nrules=json.loads(pathlib.Path(__file__).with_name('conventions.json').read_text(encoding='utf-8'))\nsys.stdout.write(json.dumps(collect_database(json.load(sys.stdin),rules),ensure_ascii=False))\n",
  );
  put(base + '/conventions.json', json(rules));
  put(
    base + '/deps/requirements.in',
    metadata.map((p) => `${p.name}==${p.version}`).join('\n') + '\n',
  );
  put(
    base + '/deps/requirements.lock',
    metadata
      .map(
        (p) =>
          `${p.name}==${p.version} ${p.files.map((f) => '--hash=sha256:' + f.sha256).join(' ')}`,
      )
      .join('\n') + '\n',
  );
  put(base + '/cases/truth.json', reviewText);
  const entity = (kind, fields) =>
    computeEntityId(
      config.projectId,
      kind,
      ENTITY_COMPONENTS[kind].map(([name]) => ({
        name,
        ...(name in fields ? { value: fields[name] } : {}),
      })),
    );
  const expectedEntities = [];
  for (const dataSourceId of review.dataSourceIds) {
    expectedEntities.push(
      entity('dataSource', { scopeId: review.scopeId, configuredId: dataSourceId }),
    );
    for (const [table, columns] of Object.entries(review.tables)) {
      expectedEntities.push(entity('dataTable', { dataSourceId, table }));
      for (const column of columns)
        expectedEntities.push(entity('dataColumn', { dataSourceId, table, column }));
      expectedEntities.push(
        entity('dataKey', { dataSourceId, table, keyColumns: review.primaryKeys[table] }),
      );
      if (review.foreignKeys[table])
        expectedEntities.push(
          entity('dataKey', { dataSourceId, table, keyColumns: review.foreignKeys[table] }),
        );
    }
  }
  put(
    base + '/cases/database.json',
    json({
      id: 'database.' + key,
      inputFixture: 'cases/input.json',
      truthRef: 'cases/truth.json',
      truthHash: hash(reviewText),
      expected: { entities: expectedEntities, gapCodes: review.requiredGaps },
    }),
  );
  put(
    base + '/cases/input.json',
    json({
      protocolVersion: '1.0',
      runId: 'frozen-database-case',
      pluginId,
      version: '1.0.0',
      stageId: 'extract',
      snapshot: { indexId: 'database-case', generation: '0' },
      inputSnapshotId: 'database.' + key,
      configSubset: config,
      sources: [...sources.values()],
      sourceSets: [
        {
          selectorId: 'backend',
          digest: baseline.truthHash,
          sourceIds: [...sources.values()].map((s) => s.id),
          complete: true,
        },
      ],
      entities: [],
      relations: [],
      candidateSets: [],
      allowedExtensions: [],
      limits: {
        timeoutMs: 60000,
        maxInputBytes: 33554432,
        maxOutputBytes: 33554432,
        maxStderrBytes: 1048576,
      },
    }),
  );
  put(
    base + '/manifest.json',
    json({
      protocolVersion: '1.0',
      pluginId,
      version: '1.0.0',
      capabilities: ['database'],
      runtime: { kind: 'python', versionRange: '>=3.12 <3.13' },
      dependencies: {
        kind: 'pip',
        manifest: 'deps/requirements.in',
        lockFile: 'deps/requirements.lock',
      },
      artifacts,
      stages: [
        {
          id: 'extract',
          phase: 'extract',
          entry: 'collect.py',
          replacement: 'project',
          sourceSelectors: [
            { id: 'backend', repositoryIds: ['back'], include: ['**/*.ts', '**/*.py', '**/*.sql'] },
          ],
        },
      ],
      acceptanceCases: ['cases/database.json'],
    }),
  );
}
console.log(
  (check ? 'Verified' : 'Prepared') + ' both database artifacts; review ' + hash(reviewText),
);
