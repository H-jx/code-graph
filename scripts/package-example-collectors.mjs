// Prepare standalone artifacts from shared AST/identity sources and frozen
// acceptance inputs. Neither fixture paths nor expected facts enter the SDK/Core.
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
const check = process.argv.includes('--check');
const read = (p) => readFileSync(path.join(root, p), 'utf8');
const truth = JSON.parse(read('fixtures/truth/frontend.json'));
const seal = JSON.parse(read('fixtures/truth/manifest.json'));
let count = 0;
function put(relative, content) {
  const target = path.join(root, relative);
  if (check) {
    if (readFileSync(target, 'utf8') !== content)
      throw Error(`outdated example artifact: ${relative}`);
  } else {
    mkdirSync(path.dirname(target), { recursive: true });
    writeFileSync(target, content);
  }
  count++;
}
for (const layout of truth.layouts) {
  const base = `collectors/examples/frontend-backend-${layout.layout}/frontend`;
  const config = JSON.parse(read(`fixtures/page-location/${layout.layout}/project.json`));
  const repository = config.repositories.find((r) => r.id === 'front');
  const sourceRoot = `fixtures/page-location/${layout.layout}/${repository.root}`;
  const sources = [];
  function scan(folder) {
    for (const entry of readdirSync(path.join(root, folder), { withFileTypes: true })) {
      const name = folder + '/' + entry.name;
      if (entry.isDirectory()) scan(name);
      else {
        const file = name.slice(sourceRoot.length + 1),
          content = read(name);
        sources.push({
          id: computeSourceId(config.projectId, 'front', file),
          repositoryId: 'front',
          path: file,
          sourceClass: 'code',
          contentHash: createHash('sha256').update(content).digest('hex'),
          byteLength: Buffer.byteLength(content),
          content,
        });
      }
    }
  }
  scan(sourceRoot);
  sources.sort((a, b) => a.path.localeCompare(b.path, 'en'));
  const rules = {
    routesExport: layout.layout === 'a' ? 'routes' : 'screens',
    routerExport: 'router',
    pathKey: layout.layout === 'a' ? 'path' : 'url',
    componentKey: layout.layout === 'a' ? 'component' : 'load',
    titleKey: layout.layout === 'a' ? 'titleKey' : 'labelKey',
    titleLiteralKey: 'title',
    messagesExport: 'messages',
    moduleExport: 'moduleInfo',
    clientFactory: 'createClient',
    httpMethods: ['get', 'post', 'put', 'patch', 'delete'],
    entrypoints: [
      { call: 'scheduler.register', kind: 'schedule' },
      { call: 'messages.subscribe', kind: 'message' },
      { call: 'commands.register', kind: 'command' },
    ],
  };
  const json = (value) => JSON.stringify(value, null, 2) + '\n';
  const pluginId = `example.${layout.layout}.frontend`;
  const manifest = {
    protocolVersion: '1.0',
    pluginId,
    version: '1.0.0',
    capabilities: ['routes', 'frontend', 'comments', 'modules', 'entrypoints'],
    runtime: { kind: 'node', versionRange: '>=24 <25' },
    dependencies: {
      kind: 'npm',
      manifest: 'deps/package.json',
      lockFile: 'deps/package-lock.json',
    },
    artifacts: [
      'manifest.json',
      'collect.mjs',
      'frontend-ast.mjs',
      'identity.mjs',
      'conventions.json',
      'deps/package.json',
      'deps/package-lock.json',
      'cases/frontend.json',
      'cases/input.json',
      'cases/truth.json',
    ],
    stages: [
      {
        id: 'extract',
        phase: 'extract',
        entry: 'collect.mjs',
        replacement: 'project',
        sourceSelectors: [
          {
            id: 'frontend',
            repositoryIds: ['front'],
            include: ['**/*.ts', '**/*.tsx', '**/*.js', '**/*.jsx'],
          },
        ],
      },
    ],
    acceptanceCases: ['cases/frontend.json'],
  };
  put(base + '/manifest.json', json(manifest));
  put(base + '/conventions.json', json(rules));
  put(base + '/frontend-ast.mjs', read('collectors/templates/node/lib/frontend-ast.mjs'));
  put(base + '/identity.mjs', read('scripts/identity.mjs'));
  put(
    base + '/collect.mjs',
    `import { readFileSync } from 'node:fs';\nimport { collect } from './frontend-ast.mjs';\nconst rules=JSON.parse(readFileSync(new URL('./conventions.json',import.meta.url),'utf8'));\nlet text='';\nfor await(const chunk of process.stdin) text+=chunk;\nprocess.stdout.write(JSON.stringify(collect(JSON.parse(text),rules)));\n`,
  );
  const pkg = {
    name: 'project-knowledge-frontend-example',
    version: '1.0.0',
    private: true,
    type: 'module',
    dependencies: { typescript: '6.0.3', graphql: '16.11.0' },
  };
  put(base + '/deps/package.json', json(pkg));
  const ids = Object.fromEntries(
    layout.expectedEntities.map((e) => [
      e.key,
      computeEntityId(
        config.projectId,
        e.kind,
        ENTITY_COMPONENTS[e.kind].map(([name]) => ({ name, value: e.identity[name] })),
      ),
    ]),
  );
  const relationId = (r) =>
    computeRelationId(config.projectId, r.kind, ids[r.from], ids[r.to], r.qualifiers);
  const input = {
    protocolVersion: '1.0',
    runId: 'frozen-case',
    pluginId,
    version: '1.0.0',
    stageId: 'extract',
    snapshot: {
      indexId: 'frozen-case',
      generation: '0',
      sourceSnapshotId: 'fixture.' + layout.layout,
    },
    inputSnapshotId: 'fixture.' + layout.layout,
    configSubset: config,
    sources,
    sourceSets: [
      {
        selectorId: 'frontend',
        digest: seal.truthHash,
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
  };
  put(base + '/cases/input.json', json(input));
  put(base + '/cases/truth.json', read('fixtures/truth/frontend.json'));
  put(
    base + '/cases/frontend.json',
    json({
      id: 'frontend.' + layout.layout,
      inputFixture: 'cases/input.json',
      truthRef: 'cases/truth.json',
      truthHash: seal.files['fixtures/truth/frontend.json'],
      expected: {
        entities: Object.values(ids),
        relations: layout.expectedRelations.map(relationId),
        forbiddenRelations: layout.forbiddenRelations.map((r) =>
          computeRelationId(
            config.projectId,
            r.kind,
            ids[r.from],
            computeEntityId(
              config.projectId,
              r.toKind,
              ENTITY_COMPONENTS[r.toKind].map(([name]) => ({ name, value: r.toIdentity[name] })),
            ),
            r.qualifiers,
          ),
        ),
        gapCodes: ['DYNAMIC_TARGET'],
      },
    }),
  );
}
console.log(
  `${check ? 'Verified' : 'Prepared'} ${count} example artifact files; truthHash ${seal.truthHash}`,
);
