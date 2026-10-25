/**
 * 规范身份与稳定 ID 的独立实现（Node 侧）。
 *
 * 这份实现只依据计划 §3.3 的规则编写，不复用 Rust 代码，用来交叉验证
 * `fixtures/protocol/identity-vectors.json` 中的测试向量：
 * - 规范身份是固定顺序的 JSON 数组；
 * - 必填槽位直接编码取值，可选槽位编码为 `[]`（缺省）或 `[value]`（存在）；
 * - 字符串原样保留，不做隐式小写或 Unicode 归一化；
 * - ID = `<前缀> + sha256(UTF8(canonicalIdentity))`，64 位小写十六进制。
 */

import { createHash } from "node:crypto";

export const ID_PREFIX = {
  entity: "ent_",
  relation: "rel_",
  evidence: "ev_",
  source: "src_",
};

/**
 * 实体类型的身份槽位规格（顺序即规范顺序）。
 * required = true 的槽位直接取值；required = false 的槽位用 [] / [value] 编码。
 */
export const ENTITY_COMPONENTS = {
  file: [
    ["repositoryId", true],
    ["path", true],
    ["scopeId", true],
  ],
  symbol: [
    ["repositoryId", true],
    ["path", true],
    ["scopeId", true],
    ["lexicalPath", true],
    ["symbolKind", true],
    ["normalizedSignature", true],
  ],
  component: [
    ["repositoryId", true],
    ["path", true],
    ["scopeId", true],
    ["lexicalPath", true],
  ],
  operation: [
    ["repositoryId", true],
    ["path", true],
    ["scopeId", true],
    ["lexicalPath", true],
    ["normalizedSignature", true],
  ],
  route: [
    ["scopeId", true],
    ["routerId", true],
    ["mode", true],
    ["declaredPath", true],
  ],
  httpEndpoint: [
    ["serviceId", true],
    ["method", true],
    ["registeredPath", true],
  ],
  graphqlOperation: [
    ["serviceId", true],
    ["operationType", true],
    ["operationName", true],
  ],
  graphqlField: [
    ["serviceId", true],
    ["schemaType", true],
    ["fieldName", true],
  ],
  dataTable: [
    ["dataSourceId", true],
    ["catalog", false],
    ["schema", false],
    ["table", true],
  ],
  dataColumn: [
    ["dataSourceId", true],
    ["catalog", false],
    ["schema", false],
    ["table", true],
    ["column", true],
  ],
  dataKey: [
    ["dataSourceId", true],
    ["catalog", false],
    ["schema", false],
    ["table", true],
    ["keyColumns", true],
  ],
  module: [
    ["scopeId", true],
    ["semanticKey", true],
  ],
  entrypoint: [
    ["scopeId", true],
    ["semanticKey", true],
  ],
  workflowState: [
    ["scopeId", true],
    ["workflowId", true],
    ["semanticKey", true],
  ],
  documentType: [
    ["scopeId", true],
    ["semanticKey", true],
  ],
  queryImplementation: [
    ["scopeId", true],
    ["semanticKey", true],
  ],
  reportField: [
    ["scopeId", true],
    ["semanticKey", true],
  ],
  service: [
    ["scopeId", true],
    ["configuredId", true],
  ],
  dataSource: [
    ["scopeId", true],
    ["configuredId", true],
  ],
  event: [
    ["repositoryId", true],
    ["path", true],
    ["scopeId", true],
    ["lexicalPath", true],
  ],
  title: [
    ["repositoryId", true],
    ["path", true],
    ["scopeId", true],
    ["lexicalPath", true],
  ],
  comment: [
    ["repositoryId", true],
    ["path", true],
    ["scopeId", true],
    ["lexicalPath", true],
  ],
  request: [
    ["scopeId", true],
    ["requestKey", true],
  ],
};

/** 规范 JSON 序列化：对象键递归排序，禁止浮点数。 */
export function canonicalJson(value) {
  if (value === null) {
    throw new Error("null is not allowed inside an identity");
  }
  if (typeof value === "boolean") {
    throw new Error("boolean is not allowed inside an identity");
  }
  if (typeof value === "number") {
    if (!Number.isInteger(value)) {
      throw new Error(`non-integer number ${value} is not allowed inside an identity`);
    }
    return JSON.stringify(value);
  }
  if (typeof value === "string") {
    return JSON.stringify(value);
  }
  if (Array.isArray(value)) {
    return `[${value.map((item) => canonicalJson(item)).join(",")}]`;
  }
  if (typeof value === "object") {
    const keys = Object.keys(value).sort();
    return `{${keys
      .map((key) => `${JSON.stringify(key)}:${canonicalJson(value[key])}`)
      .join(",")}}`;
  }
  throw new Error(`unsupported identity value: ${typeof value}`);
}

function sha256Hex(text) {
  return createHash("sha256").update(Buffer.from(text, "utf8")).digest("hex");
}

/**
 * 实体规范身份：`[projectId, kind, ...slots]`。
 * slots 是数组，元素为 `{name, value}`；value 缺失表示可选槽位缺省。
 */
export function canonicalEntityIdentity(projectId, kind, slots) {
  const spec = ENTITY_COMPONENTS[kind];
  if (!spec) {
    throw new Error(`unknown entity kind: ${kind}`);
  }
  if (spec.length !== slots.length) {
    throw new Error(
      `identity slots for ${kind} must have ${spec.length} entries, got ${slots.length}`,
    );
  }
  const parts = [projectId, kind];
  spec.forEach(([name, required], index) => {
    const slot = slots[index];
    if (slot.name !== name) {
      throw new Error(
        `identity slot ${index} must be "${name}", got "${slot.name}"`,
      );
    }
    const present = Object.prototype.hasOwnProperty.call(slot, "value");
    if (required) {
      if (!present) {
        throw new Error(`required identity slot "${name}" is missing`);
      }
      parts.push(slot.value);
    } else if (present) {
      parts.push([slot.value]);
    } else {
      parts.push([]);
    }
  });
  return canonicalJson(parts);
}

/** 实体 ID。 */
export function computeEntityId(projectId, kind, slots) {
  return ID_PREFIX.entity + sha256Hex(canonicalEntityIdentity(projectId, kind, slots));
}

/** 关系 ID：`[projectId, kind, fromId, toId, ...qualifiers]`。 */
export function computeRelationId(projectId, kind, fromId, toId, qualifiers = {}) {
  const canonical = canonicalJson([
    projectId,
    kind,
    fromId,
    toId,
    qualifiers,
  ]);
  return ID_PREFIX.relation + sha256Hex(canonical);
}

/** 来源 ID：`[projectId, repositoryId, path]`。 */
export function computeSourceId(projectId, repositoryId, path) {
  return (
    ID_PREFIX.source +
    sha256Hex(canonicalJson([projectId, repositoryId, path]))
  );
}

/** 证据 ID：`[sourceId, contentHash, startByte, endByte, role]`。 */
export function computeEvidenceId(sourceId, contentHash, startByte, endByte, role) {
  return (
    ID_PREFIX.evidence +
    sha256Hex(canonicalJson([sourceId, contentHash, startByte, endByte, role]))
  );
}
