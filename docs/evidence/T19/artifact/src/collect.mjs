/**
 * Routes extractor for the project's routes.json format.
 * stdin: CollectorInput JSON; stdout: CollectorOutput JSON.
 * Every fact is recomputed from the current source content of each selected
 * source; nothing about individual routes is fixed in this script.
 */
import { readFileSync } from "node:fs";
import { computeEntityId, computeEvidenceId } from "../identity.mjs";

const input = JSON.parse(readFileSync(0, "utf8"));
const projectId = input.configSubset.projectId;
const out = {
  protocolVersion: input.protocolVersion,
  runId: input.runId,
  stageId: input.stageId,
  inputSnapshotId: input.inputSnapshotId,
  entities: [], relations: [], evidence: [], dependencies: [], coverage: [], gaps: [],
};
let sets = [];
try {
  sets = input.sourceSets.filter((s) => (s.selectorId ?? s.id) === "routes-config");
} catch { /* missing sourceSets handled below */ }
if (sets.length === 0) sets = [{ sourceIds: input.sources.map((s) => s.id), digest: "" }];
const byId = new Map(input.sources.map((s) => [s.id, s]));
const seenEv = new Map();
const routeSources = [];
for (const src of sets.flatMap((set) => set.sourceIds.map((id) => byId.get(id)).filter(Boolean))) {
  const content = src.content ?? "";
  const bytes = Buffer.byteLength(content);
  const endLine = Math.max(1, content.split("\n").length - (content.endsWith("\n") ? 1 : 0));
  const evId = computeEvidenceId(src.id, src.contentHash, 0, bytes, "route-source");
  if (!seenEv.has(evId)) {
    seenEv.set(evId, { id: evId, sourceId: src.id, contentHash: src.contentHash,
      startByte: 0, endByte: bytes, startLine: 1, endLine, role: "route-source" });
  }
  let doc;
  try {
    doc = JSON.parse(content);
  } catch {
    out.gaps.push({ id: `unparsed-${src.id}`, code: "ROUTES_SOURCE_UNPARSEABLE", capability: "routes",
      scopeId: `repo.${src.repositoryId}`, sourceRefs: [evId], reason: "routes JSON could not be parsed" });
    continue;
  }
  const routerId = String(doc.routerId ?? "");
  const mode = String(doc.mode ?? "");
  const scopeId = `repo.${src.repositoryId}`;
  let declared = 0;
  for (const item of Array.isArray(doc.routes) ? doc.routes : []) {
    if (!item || typeof item.path !== "string" || !routerId || !mode) {
      out.gaps.push({ id: `skipped-${src.id}-${declared}`, code: "ROUTE_DECLARATION_INCOMPLETE",
        capability: "routes", scopeId, sourceRefs: [evId], reason: "route lacks path or router context" });
      continue;
    }
    declared += 1;
    const slots = [
      { name: "scopeId", value: scopeId },
      { name: "routerId", value: routerId },
      { name: "mode", value: mode },
      { name: "declaredPath", value: item.path },
    ];
    out.entities.push({
      entityId: computeEntityId(projectId, "route", slots),
      kind: "route", scopeId, identity: slots,
      assertion: { producerKey: "routes-json", name: item.name ?? null,
        attributes: { routerId, mode, declaredPath: item.path },
        basis: "observed", origin: "static", evidenceRefs: [evId] },
    });
  }
  routeSources.push({ setId: sets.indexOf(sets.find((x) => x.sourceIds.includes(src.id))), count: declared });
}
out.evidence = [...seenEv.values()];
for (const set of sets) {
  const count = set.sourceIds.reduce((n, id) => n + (byId.get(id) ? 1 : 0), 0);
  out.coverage.push({ capability: "routes", scopeId: projectId, sourceSetId: set.digest || "routes-config",
    status: count > 0 ? "complete" : "notConfigured", counts: { sources: count, routes: out.entities.length },
    gapIds: out.gaps.map((g) => g.id) });
}
process.stderr.write(`routes extract: ${out.entities.length} routes, ${out.gaps.length} gaps\n`);
process.stdout.write(`${JSON.stringify(out)}\n`);
