// 合成协议 smoke，不作为独立业务真值。
import { createHash } from 'node:crypto';
const hash = (value) => createHash('sha256').update(JSON.stringify(value)).digest('hex');
let text = '';
for await (const chunk of process.stdin) text += chunk;
const input = JSON.parse(text);
const kind = 'x.smoke.plugin.invoice';
const relationKind = 'x.smoke.plugin.binds';
if (![kind, relationKind].every((k) => input.allowedExtensions.includes(k))) process.exit(31);
const source = input.sources[0];
const identity = [
  { name: 'scopeId', value: 'repo.r' },
  { name: 'semanticKey', value: 'invoice😀' },
  { name: 'catalog' },
];
const entityId =
  'ent_' +
  hash([
    input.configSubset.projectId,
    kind,
    identity.map((s) => [s.name, 'value' in s ? [s.value] : []]),
  ]);
const evidenceId = 'ev_' + hash([source.id, source.contentHash, 0, 1, 'binding']);
const assertion = {
  producerKey: 'fixture',
  name: 'Invoice smoke',
  attributes: {},
  basis: 'observed',
  origin: 'static',
  evidenceRefs: [evidenceId],
};
const output = {
  protocolVersion: '1.0',
  runId: input.runId,
  stageId: input.stageId,
  inputSnapshotId: input.inputSnapshotId,
  entities: [],
  relations: [],
  evidence: [
    {
      id: evidenceId,
      sourceId: source.id,
      contentHash: source.contentHash,
      startByte: 0,
      endByte: 1,
      startLine: 1,
      endLine: 1,
      role: 'binding',
    },
  ],
  dependencies: [],
  coverage: [],
  gaps: [],
};
if (input.stageId === 'extract') {
  output.entities.push({
    entityId,
    kind,
    scopeId: 'repo.r',
    identity,
    assertion: { ...assertion, attributes: { total: '100.00' } },
  });
  if (process.argv.includes('--invalid-attributes'))
    output.entities[0].assertion.attributes.total = 100;
} else {
  if (
    !input.entities.some((e) => e.entityId === entityId) ||
    !input.candidateSets.some((s) => s.complete && s.entityIds.includes(entityId))
  )
    process.exit(32);
  const fromId = process.argv.includes('--bad-endpoint')
    ? entityId
    : input.entities.find((e) => e.kind === 'file').entityId;
  const qualifiers = {
    role: process.argv.includes('--invalid-qualifiers') ? 'invented' : 'original',
  };
  output.relations.push({
    relationId:
      'rel_' + hash([input.configSubset.projectId, relationKind, fromId, entityId, qualifiers]),
    kind: relationKind,
    fromId,
    toId: entityId,
    qualifiers,
    resolution: 'confirmed',
    assertion,
  });
}
if (process.argv.includes('--no-evidence')) {
  for (const e of [...output.entities, ...output.relations]) e.assertion.evidenceRefs = [];
}
process.stdout.write(JSON.stringify(output));
