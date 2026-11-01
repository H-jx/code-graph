import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
import {
  computeEntityId,
  computeRelationId,
  computeEvidenceId,
  ENTITY_COMPONENTS,
} from './identity.mjs';

const root = fileURLToPath(new URL('../', import.meta.url));
const read = (name) => readFileSync(path.join(root, name), 'utf8');
const json = (value) => JSON.stringify(value, null, 2) + '\n';
const hash = (value) => createHash('sha256').update(value).digest('hex');
const check = process.argv.includes('--check');
const reviewText = read('fixtures/truth-supplements/T13-link.json');
const review = JSON.parse(reviewText);
const seal = JSON.parse(read('fixtures/truth/manifest.json'));
if (review.baselineTruthHash !== seal.truthHash) throw Error('Link review baseline mismatch');
function put(name, content) {
  if (check) {
    if (read(name) !== content) throw Error('Outdated link artifact: ' + name);
  } else {
    mkdirSync(path.dirname(path.join(root, name)), { recursive: true });
    writeFileSync(path.join(root, name), content);
  }
}
for (const layout of review.layouts) {
  const key = layout.layout,
    base = `collectors/examples/frontend-backend-${key}/link`;
  const config = JSON.parse(read(`fixtures/page-location/${key}/project.json`));
  const frontend = JSON.parse(
    read(`collectors/examples/frontend-backend-${key}/frontend/cases/input.json`),
  );
  const backend = JSON.parse(
    read(`collectors/examples/frontend-backend-${key}/api/cases/input.json`),
  );
  const sources = [...frontend.sources, ...backend.sources];
  for (const source of sources) {
    const repository = config.repositories.find((item) => item.id === source.repositoryId);
    const name = `fixtures/page-location/${key}/${repository.root}/${source.path}`;
    if (hash(readFileSync(path.join(root, name))) !== seal.files[name])
      throw Error('Frozen link input changed: ' + name);
  }
  const evidence = [];
  const cite = (repositoryId, name) => {
    const source = sources.find(
      (source) => source.repositoryId === repositoryId && source.path === name,
    );
    const endByte = Buffer.byteLength(source.content);
    const id = computeEvidenceId(source.id, source.contentHash, 0, endByte, 'binding');
    if (!evidence.some((item) => item.id === id))
      evidence.push({
        id,
        sourceId: source.id,
        contentHash: source.contentHash,
        startByte: 0,
        endByte,
        startLine: 1,
        endLine: source.content.slice(0, -1).split('\n').length,
        role: 'binding',
      });
    return id;
  };
  const requestRef = cite('front', layout.requestFile),
    clientRef = cite('front', layout.clientFile),
    registerRef = cite('back', layout.registrationFile),
    handlerRef = cite('back', layout.handlerFile);
  const entities = [],
    relations = [];
  function entity(kind, fields, attrs, refs) {
    const identity = ENTITY_COMPONENTS[kind].map(([name]) => ({
      name,
      ...(name in fields ? { value: fields[name] } : {}),
    }));
    const entityId = computeEntityId(config.projectId, kind, identity);
    entities.push({
      entityId,
      kind,
      scopeId: fields.scopeId ?? 'svc.erp',
      identity,
      assertions: [
        {
          assertionId: 'case:' + entityId,
          producerKey: 'source-reviewed.case',
          batchId: 'case',
          basis: 'observed',
          origin: 'static',
          attributes: attrs,
          evidenceRefs: refs,
        },
      ],
    });
    return entityId;
  }
  function relation(kind, fromId, toId, refs) {
    const relationId = computeRelationId(config.projectId, kind, fromId, toId, {});
    relations.push({
      relationId,
      kind,
      fromId,
      toId,
      qualifiers: {},
      assertions: [
        {
          assertionId: 'case:' + relationId,
          producerKey: 'source-reviewed.case',
          batchId: 'case',
          basis: 'observed',
          origin: 'static',
          resolution: 'confirmed',
          evidenceRefs: refs,
        },
      ],
    });
  }
  const owner = (name, repositoryId, file, scopeId) =>
    entity(
      'operation',
      { repositoryId, path: file, scopeId, lexicalPath: name, normalizedSignature: '()' },
      {},
      [repositoryId === 'front' ? requestRef : handlerRef],
    );
  const httpCaller = owner(layout.requestHandler, 'front', layout.requestFile, 'app.web');
  const graphqlCaller = owner(layout.graphqlHandler, 'front', layout.requestFile, 'app.web');
  const request = entity(
    'request',
    { scopeId: 'app.web', requestKey: layout.requestFile + '#' + layout.requestHandler },
    {
      clientName: 'http',
      serviceId: 'erp',
      baseURL: '/erp',
      bindingComplete: true,
      method: 'GET',
      path: '/purchase/ledger',
    },
    [requestRef],
  );
  const graphqlRequest = entity(
    'request',
    { scopeId: 'app.web', requestKey: layout.requestFile + '#' + layout.graphqlHandler },
    { clientName: 'http', serviceId: 'erp', baseURL: '/erp', bindingComplete: true },
    [requestRef],
  );
  const endpoint = entity('httpEndpoint', { serviceId: 'erp', ...review.http }, {}, [registerRef]);
  const graphql = entity(
    'graphqlOperation',
    { serviceId: 'erp', operationType: 'query', operationName: 'PurchaseSummary' },
    {},
    [requestRef],
  );
  const field = entity(
    'graphqlField',
    { serviceId: 'erp', schemaType: 'Query', fieldName: 'purchaseSummary' },
    {},
    [registerRef],
  );
  const handler = owner(layout.handler, 'back', layout.handlerFile, 'svc.erp');
  relation('requests', httpCaller, request, [requestRef, clientRef]);
  relation('requests', graphqlCaller, graphqlRequest, [requestRef, clientRef]);
  relation('handles', endpoint, handler, [registerRef, handlerRef]);
  relation('references', graphqlRequest, graphql, [requestRef, clientRef]);
  relation('references', graphql, field, [requestRef]);
  relation('resolves', field, handler, [registerRef, handlerRef]);
  const pluginId = `example.${key}.link`;
  const input = {
    ...frontend,
    pluginId,
    stageId: 'link',
    runId: 'frozen-link-case',
    sources,
    sourceSets: [
      {
        selectorId: 'bindings',
        digest: seal.truthHash,
        sourceIds: sources.map((s) => s.id),
        complete: true,
      },
    ],
    entities,
    relations,
    evidence,
    candidateSets: [
      {
        selectorId: 'targets',
        digest: seal.truthHash,
        entityIds: entities.map((e) => e.entityId),
        complete: true,
      },
    ],
  };
  put(base + '/cases/input.json', json(input));
  put(base + '/cases/truth.json', reviewText);
  put(
    base + '/cases/link.json',
    json({
      id: 'link.' + key,
      inputFixture: 'cases/input.json',
      truthRef: 'cases/truth.json',
      truthHash: hash(reviewText),
      expected: {
        relations: [
          computeRelationId(config.projectId, 'references', request, endpoint, {}),
          computeRelationId(config.projectId, 'calls', graphql, handler, {}),
        ],
      },
    }),
  );
  put(base + '/identity.mjs', read('scripts/identity.mjs'));
  put(base + '/link-facts.mjs', read('collectors/templates/node/lib/link-facts.mjs'));
  put(
    base + '/collect.mjs',
    "import { linkFacts } from './link-facts.mjs';\nlet text=''; for await (const chunk of process.stdin) text+=chunk;\nprocess.stdout.write(JSON.stringify(linkFacts(JSON.parse(text))));\n",
  );
  put(
    base + '/manifest.json',
    json({
      protocolVersion: '1.0',
      pluginId,
      version: '1.0.0',
      capabilities: ['link'],
      runtime: { kind: 'node', versionRange: '>=24 <25' },
      artifacts: [
        'manifest.json',
        'collect.mjs',
        'link-facts.mjs',
        'identity.mjs',
        'cases/input.json',
        'cases/link.json',
        'cases/truth.json',
      ],
      stages: [
        {
          id: 'link',
          phase: 'link',
          entry: 'collect.mjs',
          replacement: 'project',
          dependsOn: ['frontend', 'api', 'database'].map(
            (kind) => `example.${key}.${kind}:extract`,
          ),
          sourceSelectors: [
            {
              id: 'bindings',
              repositoryIds: ['front', 'back'],
              include: ['**/*.ts', '**/*.tsx', '**/*.js', '**/*.py', '**/*.sql'],
            },
          ],
          entitySelectors: [
            {
              id: 'targets',
              kinds: [
                'request',
                'httpEndpoint',
                'graphqlOperation',
                'graphqlField',
                'operation',
                'queryImplementation',
                'dataSource',
                'dataTable',
                'dataColumn',
                'dataKey',
              ],
              scopeIds: [],
            },
          ],
        },
      ],
      acceptanceCases: ['cases/link.json'],
    }),
  );
}
console.log(
  (check ? 'Verified' : 'Prepared') + ' link artifacts with independent reviewed bindings.',
);
