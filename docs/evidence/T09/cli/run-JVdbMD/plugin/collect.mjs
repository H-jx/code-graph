import { createHash } from "node:crypto";

const hash = (value) => createHash("sha256").update(JSON.stringify(value)).digest("hex");
let text = "";
for await (const chunk of process.stdin) text += chunk;
const input = JSON.parse(text);
const output = { protocolVersion: "1.0", runId: input.runId, stageId: input.stageId,
  inputSnapshotId: input.inputSnapshotId, entities: [], relations: [], evidence: [], dependencies: [], coverage: [], gaps: [] };
if (process.argv.includes("--empty")) {
  process.stdout.write(JSON.stringify(output));
  process.exit(0);
}
const source = input.sources[0];
const project = input.configSubset.projectId;
const scope = "repo.r";
const entityId = `ent_${hash([project, "route", scope, "router", "history", "/smoke"])}`;
const evidenceId = `ev_${hash([source.id, source.contentHash, 0, 1, "binding"])}`;
output.evidence.push({ id: evidenceId, sourceId: source.id, contentHash: source.contentHash,
  startByte: 0, endByte: 1, startLine: 1, endLine: 1, role: "binding" });
const assertion = { producerKey: "untrusted-output-owner", name: "Smoke route", attributes: {}, basis: "observed", origin: "static", evidenceRefs: [evidenceId] };
if (input.stageId === "extract") {
  // 旧的同 owner 批次不能泄漏进候选输入。
  if (input.entities.some((e) => e.assertions.some((a) => a.producerKey.startsWith(input.pluginId + "@")))) process.exit(21);
  output.entities.push({ entityId, kind: "route", scopeId: scope,
    identity: [["scopeId", scope], ["routerId", "router"], ["mode", "history"], ["declaredPath", "/smoke"]]
      .map(([name, value]) => ({ name, value })), assertion });
} else {
  if (!input.entities.some((e) => e.entityId === entityId && e.assertions.some((a) => a.producerKey.includes(":extract")))) process.exit(22);
  if (!input.candidateSets.some((set) => set.complete && set.entityIds.includes(entityId))) process.exit(23);
  const file = input.entities.find((e) => e.kind === "file");
  const relationId = `rel_${hash([project, "contains", file.entityId, entityId, {}])}`;
  output.relations.push({ relationId, kind: "contains", fromId: file.entityId, toId: entityId,
    qualifiers: {}, resolution: "confirmed", assertion });
}
if (process.argv.includes("--bad")) output.evidence[0].endByte = 999999;
if (process.argv.includes("--forge-dep")) output.dependencies.push({ kind: "config", key: project, digest: "forged" });
process.stdout.write(JSON.stringify(output));
