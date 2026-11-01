import { readFileSync } from "node:fs";
import { computeEntityId, computeRelationId, computeEvidenceId } from "../identity.mjs";

const WORKFLOW_ID = "audited-acceptance-model";
const TRANSITION_RE = /^([A-Z][A-Z0-9_]*)\s*->\s*([A-Z][A-Z0-9_]*)\s*:\s*(.+)$/;
const DOCUMENTS_RE = /^Documents\s+(.+?)\s+use ordered keys\s+(\S[^.]*)\.\s*$/;

function parseInput() {
  const raw = readFileSync(0, "utf8");
  if (!raw.trim()) throw new Error("stdin is empty; expected CollectorInput JSON");
  return JSON.parse(raw);
}

function lastLineOf(content, byteOffset) {
  let line = 1;
  for (let i = 0; i < byteOffset && i < content.length; i += 1) {
    if (content.charCodeAt(i) === 10) line += 1;
  }
  return line;
}

function evidenceFor(source) {
  const endByte = Buffer.byteLength(source.content, "utf8");
  let trimmed = source.content;
  while (trimmed.length > 0 && /\s/.test(trimmed.slice(-1))) trimmed = trimmed.slice(0, -1);
  const endLine = trimmed.length === 0 ? 1 : lastLineOf(source.content, Buffer.byteLength(trimmed, "utf8"));
  return {
    id: computeEvidenceId(source.id, source.contentHash, 0, endByte, "declared-document"),
    sourceId: source.id,
    contentHash: source.contentHash,
    startByte: 0,
    endByte,
    startLine: 1,
    endLine,
    role: "declared-document",
  };
}

function makeAssertion(producerKey, ev, name, attributes) {
  return { producerKey, name: name ?? null, basis: "declared", origin: "static", attributes: attributes ?? {}, evidenceRefs: [ev.id] };
}

function main() {
  const input = parseInput();
  const out = {
    protocolVersion: input.protocolVersion,
    runId: input.runId,
    stageId: input.stageId,
    inputSnapshotId: input.inputSnapshotId,
    entities: [],
    relations: [],
    evidence: [],
    dependencies: [],
    coverage: [],
    gaps: [],
  };
  const config = input.configSubset ?? {};
  const projectId = config.projectId ?? input.pluginId;
  const scopeId = `repo.${(config.repositories ?? [])[0]?.id ?? "business"}`;
  const producerKey = `${input.pluginId}@${input.version}:${input.stageId}`;
  const seenEntities = new Map();
  const seenRelations = new Set();
  const addEntity = (kind, slots, ev, name, attributes) => {
    const entityId = computeEntityId(projectId, kind, slots);
    if (!seenEntities.has(entityId)) {
      seenEntities.set(entityId, true);
      out.entities.push({ entityId, kind, scopeId, identity: slots, assertion: makeAssertion(producerKey, ev, name, attributes) });
    }
    return entityId;
  };
  const addRelation = (kind, fromId, toId, qualifiers, ev) => {
    const relationId = computeRelationId(projectId, kind, fromId, toId, qualifiers);
    if (!seenRelations.has(relationId)) {
      seenRelations.add(relationId);
      out.relations.push({ relationId, kind, fromId, toId, qualifiers, resolution: "confirmed", assertion: makeAssertion(producerKey, ev) });
    }
  };
  for (const source of input.sources) {
    if (!/rules\.md$/.test(source.path)) continue;
    const ev = evidenceFor(source);
    out.evidence.push(ev);
    for (const line of source.content.split(/\r?\n/)) {
      const match = TRANSITION_RE.exec(line);
      if (match) {
        const [, fromKey, toKey, rest] = match;
        const comma = rest.indexOf(",");
        const trigger = (comma < 0 ? rest : rest.slice(0, comma)).trim();
        const guard = comma < 0 ? "" : rest.slice(comma + 1).trim();
        const stateSlot = (key) => [
          { name: "scopeId", value: scopeId },
          { name: "workflowId", value: WORKFLOW_ID },
          { name: "semanticKey", value: key },
        ];
        const fromId = addEntity("workflowState", stateSlot(fromKey), ev, fromKey, { workflowId: WORKFLOW_ID });
        const toId = addEntity("workflowState", stateSlot(toKey), ev, toKey, { workflowId: WORKFLOW_ID });
        addRelation("transitions_to", fromId, toId, { trigger, guard }, ev);
        continue;
      }
      const docMatch = DOCUMENTS_RE.exec(line);
      if (docMatch) {
        const chain = docMatch[1].split("->").map((token) => token.trim()).filter(Boolean);
        const orderedKeys = docMatch[2].split(",").map((token) => token.trim()).filter(Boolean);
        const docIds = chain.map((name) => addEntity("documentType", [
          { name: "scopeId", value: scopeId },
          { name: "semanticKey", value: name },
        ], ev, name));
        for (let i = 0; i + 1 < docIds.length; i += 1) {
          addRelation("relates_document", docIds[i], docIds[i + 1], { documentTypeId: docIds[i], orderedKeys }, ev);
        }
      }
    }
  }
  for (const set of input.sourceSets ?? []) {
    out.coverage.push({ capability: "workflow", scopeId, sourceSetId: set.selectorId, status: "complete", counts: { entities: out.entities.length, relations: out.relations.length }, gapIds: [] });
  }
  process.stdout.write(`${JSON.stringify(out)}\n`);
}

main();
