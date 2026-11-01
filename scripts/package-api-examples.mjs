import { readFileSync, writeFileSync, mkdirSync, readdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { createHash } from 'node:crypto';
import {
  computeEntityId,
  computeRelationId,
  computeSourceId,
  ENTITY_COMPONENTS,
} from './identity.mjs';
const root = fileURLToPath(new URL('../', import.meta.url));
const read = (p) => readFileSync(path.join(root, p), 'utf8');
const check = process.argv.includes('--check');
const json = (value) => JSON.stringify(value, null, 2) + '\n';
const hash = (content) => createHash('sha256').update(content).digest('hex');
const original = JSON.parse(read('fixtures/truth/manifest.json'));
const truthText = read('fixtures/truth-supplements/T12-api.json');
const truth = JSON.parse(truthText);
if (truth.baselineTruthHash !== original.truthHash) throw Error('API review baseline mismatch');
function put(relative, content) {
  const target = path.join(root, relative);
  if (check) {
    if (readFileSync(target, 'utf8') !== content) throw Error('outdated API artifact: ' + relative);
  } else {
    mkdirSync(path.dirname(target), { recursive: true });
    writeFileSync(target, content);
  }
}
for (const layout of truth.layouts) {
  for (const cite of layout.evidence) {
    const content = read(cite.path);
    if (
      hash(content) !== original.files[cite.path] ||
      !content.split(/\r?\n/)[cite.line - 1].includes(cite.text)
    )
      throw Error('API truth citation mismatch');
  }
  const key = layout.layout,
    base = `collectors/examples/frontend-backend-${key}/api`;
  const config = JSON.parse(read(`fixtures/page-location/${key}/project.json`));
  const repo = config.repositories.find((r) => r.id === 'back');
  const sourceRoot = `fixtures/page-location/${key}/${repo.root}`;
  const sources = [];
  function scan(folder) {
    for (const entry of readdirSync(path.join(root, folder), { withFileTypes: true })) {
      const name = folder + '/' + entry.name;
      if (entry.isDirectory()) scan(name);
      else {
        const relative = name.slice(sourceRoot.length + 1),
          content = read(name);
        sources.push({
          id: computeSourceId(config.projectId, 'back', relative),
          repositoryId: 'back',
          path: relative,
          sourceClass: 'code',
          contentHash: hash(content),
          byteLength: Buffer.byteLength(content),
          content,
        });
      }
    }
  }
  scan(sourceRoot);
  const rules = {
    httpObject: key === 'a' ? 'http' : 'app',
    httpMethods: ['get', 'post', 'put', 'patch', 'delete'],
    graphqlObject: 'graphql',
    graphqlRegister: 'register',
  };
  const pluginId = `example.${key}.api`;
  const artifacts = [
    'manifest.json',
    'conventions.json',
    'cases/api.json',
    'cases/input.json',
    'cases/truth.json',
  ];
  if (key === 'a') {
    for (const [target, source] of Object.entries({
      'frontend-ast.mjs': 'collectors/templates/node/lib/frontend-ast.mjs',
      'api-ast.mjs': 'collectors/templates/node/lib/api-ast.mjs',
      'identity.mjs': 'scripts/identity.mjs',
      'deps/package.json': 'collectors/examples/frontend-backend-a/frontend/deps/package.json',
      'deps/package-lock.json':
        'collectors/examples/frontend-backend-a/frontend/deps/package-lock.json',
    })) {
      put(base + '/' + target, read(source));
      artifacts.push(target);
    }
    put(
      base + '/collect.mjs',
      "import { readFileSync } from 'node:fs';\nimport { collectApi } from './api-ast.mjs';\nconst rules=JSON.parse(readFileSync(new URL('./conventions.json',import.meta.url),'utf8'));\nlet text='';\nfor await(const chunk of process.stdin) text+=chunk;\nprocess.stdout.write(JSON.stringify(collectApi(JSON.parse(text),rules)));\n",
    );
    artifacts.push('collect.mjs');
  } else {
    for (const [target, source] of Object.entries({
      'api_ast.py': 'collectors/templates/python/lib/api_ast.py',
      'identity.py': 'scripts/verify-vectors.py',
      'identity-spec.json': 'docs/schema/entity-kinds.json',
    })) {
      put(base + '/' + target, read(source));
      artifacts.push(target);
    }
    put(
      base + '/collect.py',
      "import json\nimport pathlib\nimport sys\nfrom api_ast import collect_api\nrules=json.loads(pathlib.Path(__file__).with_name('conventions.json').read_text(encoding='utf-8'))\nsys.stdout.write(json.dumps(collect_api(json.load(sys.stdin),rules),ensure_ascii=False))\n",
    );
    artifacts.push('collect.py');
  }
  put(base + '/conventions.json', json(rules));
  put(base + '/cases/truth.json', truthText);
  put(
    base + '/manifest.json',
    json({
      protocolVersion: '1.0',
      pluginId,
      version: '1.0.0',
      capabilities: ['api'],
      runtime: {
        kind: key === 'a' ? 'node' : 'python',
        versionRange: key === 'a' ? '>=24 <25' : '>=3.12 <3.13',
      },
      artifacts,
      ...(key === 'a'
        ? {
            dependencies: {
              kind: 'npm',
              manifest: 'deps/package.json',
              lockFile: 'deps/package-lock.json',
            },
          }
        : {}),
      stages: [
        {
          id: 'extract',
          phase: 'extract',
          entry: key === 'a' ? 'collect.mjs' : 'collect.py',
          replacement: 'project',
          sourceSelectors: [
            { id: 'backend', repositoryIds: ['back'], include: ['**/*.ts', '**/*.js', '**/*.py'] },
          ],
        },
      ],
      acceptanceCases: ['cases/api.json'],
    }),
  );
  const entity = (kind, fields) =>
    computeEntityId(
      config.projectId,
      kind,
      ENTITY_COMPONENTS[kind].map(([name]) => ({ name, value: fields[name] })),
    );
  const service = entity('service', { scopeId: truth.scopeId, configuredId: truth.serviceId });
  const endpoint = entity('httpEndpoint', { serviceId: truth.serviceId, ...truth.http });
  const field = entity('graphqlField', { serviceId: truth.serviceId, ...truth.graphql });
  const operation = entity('operation', {
    repositoryId: 'back',
    path: layout.handlerFile,
    scopeId: truth.scopeId,
    lexicalPath: layout.handler,
    normalizedSignature: '()',
  });
  const decoy = entity('operation', {
    repositoryId: 'back',
    path: 'decoy/' + layout.handlerFile,
    scopeId: truth.scopeId,
    lexicalPath: layout.handler,
    normalizedSignature: '()',
  });
  const relation = (kind, from, to) => computeRelationId(config.projectId, kind, from, to, {});
  put(
    base + '/cases/api.json',
    json({
      id: 'api.' + key,
      inputFixture: 'cases/input.json',
      truthRef: 'cases/truth.json',
      truthHash: hash(truthText),
      expected: {
        entities: [service, endpoint, field, operation],
        relations: [
          relation('serves', service, endpoint),
          relation('handles', endpoint, operation),
          relation('resolves', field, operation),
        ],
        forbiddenRelations: [relation('handles', endpoint, decoy)],
      },
    }),
  );
  put(
    base + '/cases/input.json',
    json({
      protocolVersion: '1.0',
      runId: 'frozen-api-case',
      pluginId,
      version: '1.0.0',
      stageId: 'extract',
      snapshot: { indexId: 'api-case', generation: '0' },
      inputSnapshotId: 'api.' + key,
      configSubset: config,
      sources,
      sourceSets: [
        {
          selectorId: 'backend',
          digest: original.truthHash,
          sourceIds: sources.map((s) => s.id),
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
}
console.log(
  (check ? 'Verified' : 'Prepared') + ' both API artifacts; supplementary truth ' + hash(truthText),
);
