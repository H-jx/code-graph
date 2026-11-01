import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
import {
  computeEntityId,
  computeRelationId,
  computeSourceId,
  computeEvidenceId,
  ENTITY_COMPONENTS,
} from './identity.mjs';

const root = fileURLToPath(new URL('../', import.meta.url));
const read = (name) => readFileSync(path.join(root, name), 'utf8');
const json = (value) => JSON.stringify(value, null, 2) + '\n';
const hash = (value) => createHash('sha256').update(value).digest('hex');
const check = process.argv.includes('--check');
const reviewText = read('fixtures/truth-supplements/T14-semantics.json'),
  review = JSON.parse(reviewText);
const ddl = JSON.parse(read('fixtures/truth-supplements/T12-database/review.json'));
if (review.baselineTruthHash !== JSON.parse(read('fixtures/truth/manifest.json')).truthHash)
  throw Error('Semantic review baseline mismatch');
for (const [file, expected] of Object.entries(review.sourceHashes))
  if (hash(readFileSync(path.join(root, file))) !== expected)
    throw Error('Semantic input changed: ' + file);
function put(name, content) {
  if (check) {
    if (read(name) !== content) throw Error('Outdated semantic artifact: ' + name);
  } else {
    mkdirSync(path.dirname(path.join(root, name)), { recursive: true });
    writeFileSync(path.join(root, name), content);
  }
}
for (const layout of ['a', 'b']) {
  const base = `collectors/examples/frontend-backend-${layout}/semantics`,
    pluginId = `example.${layout}.semantics`;
  const database = `collectors/examples/frontend-backend-${layout}/database`;
  const input = JSON.parse(read(database + '/cases/input.json'));
  for (const [name, file] of Object.entries({
    'flow.ts': 'fixtures/purchase-workflow/flow.ts',
    'workflow.md': 'fixtures/purchase-workflow/rules.md',
    'policy.md': 'fixtures/purchase-ledger/policy.md',
  })) {
    const content = read(file);
    input.sources.push({
      id: computeSourceId(input.configSubset.projectId, 'back', name),
      repositoryId: 'back',
      path: name,
      sourceClass: name.endsWith('.md') ? 'documentation' : 'code',
      contentHash: hash(content),
      byteLength: Buffer.byteLength(content),
      content,
    });
  }
  const project = input.configSubset.projectId;
  const scopeId = 'svc.erp';
  const rules = {
    workflowId: 'purchase',
    stateParameter: 'order',
    stateProperty: 'state',
    workflowCode: 'flow.ts',
    workflowDocument: 'workflow.md',
    reportPolicy: 'policy.md',
    dataSourceId: 'ds.erp',
    reportBaseTable: 'purchase_detail',
    documentTables: { purchase: 'purchase_detail', receipt: 'receipt', return: 'return_line' },
  };
  const entities = [],
    evidence = [];
  function cite(name) {
    const source = input.sources.find((s) => s.path === name),
      endByte = Buffer.byteLength(source.content);
    const id = computeEvidenceId(source.id, source.contentHash, 0, endByte, 'semantic-case');
    if (!evidence.some((e) => e.id === id))
      evidence.push({
        id,
        sourceId: source.id,
        contentHash: source.contentHash,
        startByte: 0,
        endByte,
        startLine: 1,
        endLine: source.content.slice(0, -1).split('\n').length,
        role: 'semantic-case',
      });
    return id;
  }
  function identity(kind, fields) {
    return ENTITY_COMPONENTS[kind].map(([name]) => ({
      name,
      ...(name in fields ? { value: fields[name] } : {}),
    }));
  }
  function id(kind, fields) {
    return computeEntityId(project, kind, identity(kind, fields));
  }
  function entity(kind, fields, attributes, source) {
    const entityId = id(kind, fields);
    entities.push({
      entityId,
      kind,
      scopeId,
      identity: identity(kind, fields),
      assertions: [
        {
          assertionId: 'case:' + entityId,
          producerKey: 'independent.source-review',
          batchId: 'case',
          basis: 'declared',
          origin: 'static',
          attributes,
          evidenceRefs: [cite(source)],
        },
      ],
    });
    return entityId;
  }
  for (const ds of ddl.dataSourceIds) {
    entity(
      'dataSource',
      { scopeId, configuredId: ds },
      { dialect: 'sqlite' },
      ds === 'ds.erp' ? 'schema.sql' : 'analytics.sql',
    );
    for (const [table, columns] of Object.entries(ddl.tables)) {
      const source = ds === 'ds.erp' ? 'schema.sql' : 'analytics.sql';
      entity('dataTable', { dataSourceId: ds, table }, {}, source);
      const ids = columns.map((column) =>
        entity('dataColumn', { dataSourceId: ds, table, column }, {}, source),
      );
      const keyColumns = ddl.primaryKeys[table];
      entity(
        'dataKey',
        { dataSourceId: ds, table, keyColumns },
        { keyType: 'primary', columnIds: keyColumns.map((column) => ids[columns.indexOf(column)]) },
        source,
      );
    }
  }
  const unknown = { status: 'unknown', gapIds: [] };
  const sqlSource = input.sources.find((s) => s.path === 'correct.sql');
  const joins = ddl.correctQueryJoins.map((join) => ({
    joinType: 'left',
    cardinality: 'unknown',
    keyPairs: join.columns.map((column) => ({
      leftColumnId: id('dataColumn', { dataSourceId: 'ds.erp', table: join.leftTable, column }),
      rightColumnId: id('dataColumn', { dataSourceId: 'ds.erp', table: join.rightTable, column }),
      operator: 'equals',
    })),
  }));
  const query = entity(
    'queryImplementation',
    { scopeId, semanticKey: 'back:correct.sql:0' },
    {
      dialect: 'sqlite',
      statement: { startByte: 0, endByte: sqlSource.byteLength, text: sqlSource.content },
      grainColumnIds: unknown,
      joins,
      aggregates: unknown,
      validStates: unknown,
      timeFields: unknown,
      nullHandling: unknown,
      units: unknown,
    },
    'correct.sql',
  );
  const relations = Object.keys(ddl.tables).map((table) => {
    const toId = id('dataTable', { dataSourceId: 'ds.erp', table }),
      relationId = computeRelationId(project, 'reads', query, toId, {});
    return {
      relationId,
      kind: 'reads',
      fromId: query,
      toId,
      qualifiers: {},
      assertions: [
        {
          assertionId: 'case:' + relationId,
          producerKey: 'independent.source-review',
          batchId: 'case',
          basis: 'observed',
          origin: 'static',
          resolution: 'confirmed',
          evidenceRefs: [cite('correct.sql')],
        },
      ],
    };
  });
  const artifacts = [
    'manifest.json',
    'conventions.json',
    'collect.py',
    'semantic_facts.py',
    'workflow_ast.py',
    'report_ast.py',
    'source_ast.py',
    'identity.py',
    'identity-spec.json',
    'deps/requirements.in',
    'deps/requirements.lock',
    'cases/workflow-input.json',
    'cases/report-input.json',
    'cases/workflow.json',
    'cases/report.json',
    'cases/truth.json',
  ];
  for (const name of ['semantic_facts.py', 'workflow_ast.py', 'report_ast.py', 'source_ast.py'])
    put(base + '/' + name, read('collectors/templates/python/lib/' + name));
  for (const name of [
    'identity.py',
    'identity-spec.json',
    'deps/requirements.in',
    'deps/requirements.lock',
  ])
    put(base + '/' + name, read(database + '/' + name));
  put(
    base + '/collect.py',
    "import json\nimport pathlib\nimport sys\nfrom workflow_ast import collect_workflow\nfrom report_ast import collect_report\nrules=json.loads(pathlib.Path(__file__).with_name('conventions.json').read_text(encoding='utf-8'))\npayload=json.load(sys.stdin)\nresult=(collect_workflow if payload['stageId']=='workflow' else collect_report)(payload,rules)\nsys.stdout.write(json.dumps(result,ensure_ascii=False))\n",
  );
  put(base + '/conventions.json', json(rules));
  put(base + '/cases/truth.json', reviewText);
  const kinds = ['dataSource', 'dataTable', 'dataColumn', 'dataKey', 'queryImplementation'];
  for (const stage of ['workflow', 'report']) {
    const selected =
      stage === 'workflow' ? entities.filter((e) => e.kind !== 'queryImplementation') : entities;
    const payload = {
      ...input,
      pluginId,
      stageId: stage,
      runId: 'frozen-semantic-case',
      entities: selected,
      relations: stage === 'workflow' ? [] : relations,
      evidence,
      sourceSets: [
        {
          selectorId: 'semantics',
          digest: review.baselineTruthHash,
          sourceIds: input.sources.map((s) => s.id),
          complete: true,
        },
      ],
      candidateSets: [
        {
          selectorId: 'targets',
          digest: review.baselineTruthHash,
          entityIds: selected.map((e) => e.entityId),
          complete: true,
        },
      ],
    };
    put(base + `/cases/${stage}-input.json`, json(payload));
    const expected =
      stage === 'workflow'
        ? [
            ...review.states.map((name) =>
              id('workflowState', { scopeId, workflowId: 'purchase', semanticKey: name }),
            ),
            ...Object.entries(review.operations).map(([lexicalPath, normalizedSignature]) =>
              id('operation', {
                repositoryId: 'back',
                path: 'flow.ts',
                scopeId,
                lexicalPath,
                normalizedSignature,
              }),
            ),
          ]
        : Object.keys(review.fields).map((name) =>
            id('reportField', { scopeId, semanticKey: 'back:correct.sql:0:field:' + name }),
          );
    put(
      base + `/cases/${stage}.json`,
      json({
        id: stage + '.' + layout,
        inputFixture: `cases/${stage}-input.json`,
        truthRef: 'cases/truth.json',
        truthHash: hash(reviewText),
        expected: {
          entities: expected,
          gapCodes:
            stage === 'workflow'
              ? ['CONFLICTING_EVIDENCE', 'WORKFLOW_EVIDENCE_MISSING']
              : ['REPORT_TIME_FIELD_UNKNOWN', 'REPORT_CARDINALITY_UNKNOWN'],
        },
      }),
    );
  }
  put(
    base + '/manifest.json',
    json({
      protocolVersion: '1.0',
      pluginId,
      version: '1.0.0',
      runtime: { kind: 'python', versionRange: '>=3.12 <3.13' },
      capabilities: ['workflow', 'report'],
      artifacts,
      dependencies: {
        kind: 'pip',
        manifest: 'deps/requirements.in',
        lockFile: 'deps/requirements.lock',
      },
      stages: ['workflow', 'report'].map((stage) => ({
        id: stage,
        phase: stage === 'workflow' ? 'extract' : 'link',
        entry: 'collect.py',
        replacement: 'project',
        sourceSelectors: [
          {
            id: 'semantics',
            repositoryIds: ['back'],
            include: ['**/*.ts', '**/*.py', '**/*.sql', '**/*.md'],
          },
        ],
        entitySelectors: [
          {
            id: 'targets',
            kinds: stage === 'workflow' ? kinds.filter((k) => k !== 'queryImplementation') : kinds,
            scopeIds: [],
          },
        ],
        dependsOn:
          stage === 'workflow'
            ? [`example.${layout}.database:extract`]
            : [`${pluginId}:workflow`, `example.${layout}.link:link`],
      })),
      acceptanceCases: ['cases/workflow.json', 'cases/report.json'],
    }),
  );
}
console.log((check ? 'Verified' : 'Prepared') + ' semantic artifacts; review ' + hash(reviewText));
