// 由 scripts/generate-types.mjs 从 docs/schema/contract.schema.json 生成。
// 请勿手工编辑：改动 Rust 契约后重新运行 `pnpm run build:contracts`。
/**
 * 事实依据。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "Basis".
 */
export type Basis = "observed" | "declared" | "inferred";
/**
 * 断言来源。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "Origin".
 */
export type Origin = "static" | "human" | "ai";
/**
 * 关系解析状态。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "Resolution".
 */
export type Resolution = "confirmed" | "candidate";
/**
 * 绑定种类。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "BindingKind".
 */
export type BindingKind = "clientToService" | "proxyToService" | "ormInstanceToDataSource";
/**
 * 关联基数。未声明或未验证时必须是 `unknown`。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "Cardinality".
 */
export type Cardinality = "oneToOne" | "oneToMany" | "manyToOne" | "manyToMany" | "unknown";
/**
 * 身份值的规范取值。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "IdentityValue".
 */
export type IdentityValue = string | number | IdentityValue[];
/**
 * 内置实体类型。扩展类型形如 `x.<pluginId>.<localName>`，另行注册。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "EntityKind".
 */
export type EntityKind =
  | (
      | "file"
      | "symbol"
      | "component"
      | "operation"
      | "route"
      | "httpEndpoint"
      | "graphqlOperation"
      | "graphqlField"
      | "dataTable"
      | "dataColumn"
      | "dataKey"
      | "module"
      | "entrypoint"
      | "workflowState"
      | "documentType"
      | "queryImplementation"
      | "reportField"
      | "event"
      | "request"
      | "title"
      | "comment"
      | "dataSource"
      | "service"
    )
  | ExtensionName;
/**
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "ExtensionName".
 */
export type ExtensionName = string;
/**
 * 内置关系类型。扩展类型形如 `x.<pluginId>.<localName>`，另行注册。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "RelationKind".
 */
export type RelationKind =
  | (
      | "contains"
      | "defines"
      | "imports"
      | "references"
      | "calls"
      | "renders"
      | "handles"
      | "triggers"
      | "wraps"
      | "requests"
      | "serves"
      | "resolves"
      | "reads"
      | "writes"
      | "maps_to"
      | "joins"
      | "transitions_to"
      | "precedes"
      | "relates_document"
      | "describes"
    )
  | ExtensionName;
/**
 * 来源类别。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "SourceClass".
 */
export type SourceClass =
  ("code" | "config" | "schema" | "documentation") | "derivedConvention" | "generatedKnowledge";
/**
 * 覆盖状态。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "CoverageStatus".
 */
export type CoverageStatus = "complete" | "partial" | "notConfigured" | "failed";
/**
 * 依赖种类。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "DependencyKind".
 */
export type DependencyKind =
  | "source"
  | "sourceSet"
  | "config"
  | "artifact"
  | "runtime"
  | "candidateSet"
  | "upstreamBatch"
  | "prompt"
  | "topicInput";
/**
 * 新鲜度策略。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "FreshnessPolicy".
 */
export type FreshnessPolicy = "verify" | "indexed" | "allowStale";
/**
 * 覆盖级别。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "CoverageLevel".
 */
export type CoverageLevel = "complete" | "partial" | "notConfigured" | "failed";
/**
 * 新鲜度。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "Freshness".
 */
export type Freshness = "fresh" | "stale" | "unknown";
/**
 * 作用域种类。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "ScopeKind".
 */
export type ScopeKind = "application" | "module" | "service";
/**
 * 依赖管理方式。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "DependencyManagerKind".
 */
export type DependencyManagerKind = "npm" | "pip";
/**
 * 遍历方向。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "Direction".
 */
export type Direction = "in" | "out" | "both";
/**
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "IndexMode".
 */
export type IndexMode = "base" | "incremental";
/**
 * 连接算子（初版仅支持等值）。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "JoinOperator".
 */
export type JoinOperator = "equals";
/**
 * join 类型。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "JoinType".
 */
export type JoinType = "inner" | "left" | "right" | "full" | "cross";
/**
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "MaintenanceRequest".
 */
export type MaintenanceRequest =
  | {
      mode: IndexMode;
      operation: "index";
    }
  | {
      operation: "discoverPlugins";
    }
  | {
      operation: "submitPlugin";
      path: string;
    }
  | {
      candidateId: string;
      operation: "preparePlugin";
    }
  | {
      candidateIds: string[];
      operation: "validatePlugin";
    }
  | {
      candidateId: string;
      operation: "activatePlugin";
    }
  | {
      operation: "rollbackPlugin";
      pluginId: string;
      version: string;
    }
  | {
      operation: "collect";
    }
  | {
      operation: "exportContext";
      request: TopicContextRequest;
    }
  | {
      artifact: TopicArtifact;
      operation: "importTopic";
    }
  | {
      input: ConventionsImport;
      operation: "importConventions";
    }
  | {
      name: string;
      operation: "generateSkill";
      output?: string | null;
    };
/**
 * 查询请求。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "QueryRequest".
 */
export type QueryRequest =
  | {
      checkFreshness?: boolean;
      operation: "status";
    }
  | {
      budget?: Budget | null;
      continuation?: string | null;
      fields?: SearchField[] | null;
      filters?: CommonFilters | null;
      operation: "search";
      pageSize?: number | null;
      query: string;
    }
  | {
      budget?: Budget | null;
      filters?: CommonFilters | null;
      operation: "locateFeature";
      query: string;
      rootFilter?: RootFilter | null;
      routeHint?: string | null;
      traversal?: Traversal | null;
      view?: string | null;
    }
  | {
      budget?: Budget | null;
      direction: Direction;
      filters?: CommonFilters | null;
      maxDepth?: number | null;
      operation: "trace";
      relationKinds?: RelationKind[] | null;
      roots: EntityRef[];
    }
  | {
      budget?: Budget | null;
      continuation?: string | null;
      direction: Direction;
      filters?: CommonFilters | null;
      operation: "neighbors";
      pageSize?: number | null;
      relationKinds?: RelationKind[] | null;
      root: EntityRef;
    }
  | {
      budget?: Budget | null;
      filters?: CommonFilters | null;
      maxFiles?: number | null;
      maxSymbols?: number | null;
      operation: "codeMap";
      query?: string | null;
      roots?: EntityRef[] | null;
    }
  | {
      filters?: CommonFilters | null;
      maxResponseChars?: number | null;
      operation: "readSource";
      references: SourceReference[];
    }
  | {
      budget?: Budget | null;
      filters?: CommonFilters | null;
      operation: "page";
      route: string;
      scopeIds?: string[] | null;
    }
  | {
      budget?: Budget | null;
      filters?: CommonFilters | null;
      operation: "workflow";
      query: string;
      roots?: EntityRef[] | null;
      scopeIds?: string[] | null;
    }
  | {
      budget?: Budget | null;
      filters?: CommonFilters | null;
      operation: "report";
      query: string;
      roots?: EntityRef[] | null;
      scopeIds?: string[] | null;
    };
/**
 * 搜索字段。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "SearchField".
 */
export type SearchField =
  "businessTerm" | "route" | "title" | "comment" | "module" | "symbol" | "path" | "source";
/**
 * 已知值或未知项。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "MaybeKnown".
 */
export type MaybeKnown = string[] | UnknownValue;
/**
 * 未知项状态标记。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "UnknownStatus".
 */
export type UnknownStatus = "unknown";
/**
 * 已知值或未知项。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "MaybeKnown2".
 */
export type MaybeKnown2 = JoinQualifiers[] | UnknownValue;
/**
 * 已知值或未知项。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "MaybeKnown3".
 */
export type MaybeKnown3 = AggregateSpec[] | UnknownValue;
/**
 * 已知值或未知项。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "MaybeKnown4".
 */
export type MaybeKnown4 = TimeFieldSpec[] | UnknownValue;
/**
 * 已知值或未知项。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "MaybeKnown5".
 */
export type MaybeKnown5 = string | UnknownValue;
/**
 * 已知值或未知项。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "MaybeKnown6".
 */
export type MaybeKnown6 = UnitSpec[] | UnknownValue;
/**
 * 运行时种类。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "RuntimeKind".
 */
export type RuntimeKind = "node" | "python" | "executable";
/**
 * 阶段所属阶段。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "StagePhase".
 */
export type StagePhase = "extract" | "link";
/**
 * 替换范围。v1 只支持整项目替换。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "ReplacementScope".
 */
export type ReplacementScope = "project";
/**
 * 响应状态。`complete` 只代表请求范围和预算内结果完整。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "ResponseStatus".
 */
export type ResponseStatus = "complete" | "partial";
/**
 * Core、CLI、SDK、MCP 与插件共同遵守的协议；类型由 Rust 契约导出。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "ProjectKnowledgeContract".
 */
export type ProjectKnowledgeContract =
  | {
      checkFreshness?: boolean;
      operation: "status";
    }
  | {
      budget?: Budget | null;
      continuation?: string | null;
      fields?: SearchField[] | null;
      filters?: CommonFilters | null;
      operation: "search";
      pageSize?: number | null;
      query: string;
    }
  | {
      budget?: Budget | null;
      filters?: CommonFilters | null;
      operation: "locateFeature";
      query: string;
      rootFilter?: RootFilter | null;
      routeHint?: string | null;
      traversal?: Traversal | null;
      view?: string | null;
    }
  | {
      budget?: Budget | null;
      direction: Direction;
      filters?: CommonFilters | null;
      maxDepth?: number | null;
      operation: "trace";
      relationKinds?: RelationKind[] | null;
      roots: EntityRef[];
    }
  | {
      budget?: Budget | null;
      continuation?: string | null;
      direction: Direction;
      filters?: CommonFilters | null;
      operation: "neighbors";
      pageSize?: number | null;
      relationKinds?: RelationKind[] | null;
      root: EntityRef;
    }
  | {
      budget?: Budget | null;
      filters?: CommonFilters | null;
      maxFiles?: number | null;
      maxSymbols?: number | null;
      operation: "codeMap";
      query?: string | null;
      roots?: EntityRef[] | null;
    }
  | {
      filters?: CommonFilters | null;
      maxResponseChars?: number | null;
      operation: "readSource";
      references: SourceReference[];
    }
  | {
      budget?: Budget | null;
      filters?: CommonFilters | null;
      operation: "page";
      route: string;
      scopeIds?: string[] | null;
    }
  | {
      budget?: Budget | null;
      filters?: CommonFilters | null;
      operation: "workflow";
      query: string;
      roots?: EntityRef[] | null;
      scopeIds?: string[] | null;
    }
  | {
      budget?: Budget | null;
      filters?: CommonFilters | null;
      operation: "report";
      query: string;
      roots?: EntityRef[] | null;
      scopeIds?: string[] | null;
    };

/**
 * 仅供完整导出类型的生成锚点；运行时协议仍为 Core 的 contract.schema.json。
 */
export interface ContractDeclarations {}
/**
 * 插件验收用例。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "AcceptanceCase".
 */
export interface AcceptanceCase {
  expected: CaseExpectation;
  id: string;
  inputFixture: string;
  truthHash?: string | null;
  truthRef?: string | null;
}
/**
 * 用例预期。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "CaseExpectation".
 */
export interface CaseExpectation {
  entities?: string[];
  forbiddenRelations?: string[];
  gapCodes?: string[];
  relations?: string[];
}
/**
 * 聚合表达式。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "AggregateSpec".
 */
export interface AggregateSpec {
  expression: string;
  function: string;
  sourceColumnIds?: string[];
}
/**
 * 已存在的断言。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "AssertionInput".
 */
export interface AssertionInput {
  assertionId: string;
  attributes?: {
    [k: string]: unknown;
  };
  basis: Basis;
  batchId: string;
  evidenceRefs?: string[];
  name?: string | null;
  origin: Origin;
  producerKey: string;
  resolution?: Resolution | null;
}
/**
 * 插件产出的断言。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "AssertionOutput".
 */
export interface AssertionOutput {
  attributes?: {
    [k: string]: unknown;
  };
  basis: Basis;
  evidenceRefs?: string[];
  name?: string | null;
  origin: Origin;
  producerKey: string;
}
/**
 * 绑定登记：客户端或代理 → service，或 ORM instance → dataSource。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "BindingConfig".
 */
export interface BindingConfig {
  evidenceRefs?: string[];
  from: string;
  id: string;
  kind: BindingKind;
  scopeId: string;
  to: string;
}
/**
 * 查询预算。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "Budget".
 */
export interface Budget {
  maxDepth?: number | null;
  maxEntities?: number | null;
  maxRelations?: number | null;
  maxResponseChars?: number | null;
  maxVisited?: number | null;
  timeoutMs?: number | null;
}
/**
 * 预算使用情况。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "BudgetUsed".
 */
export interface BudgetUsed {
  elapsedMs: number;
  entities: number;
  relations: number;
  responseChars: number;
  visited: number;
}
/**
 * 候选集合摘要。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "CandidateSet".
 */
export interface CandidateSet {
  complete: boolean;
  digest: string;
  entityIds?: string[];
  selectorId: string;
}
/**
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "Capabilities".
 */
export interface Capabilities {
  entityKinds: string[];
  languages: string[];
  limits: {
    [k: string]: number;
  };
  maintenanceOperations: string[];
  operations: string[];
  pluginRuntimes: string[];
  protocolVersion: string;
  providers: string[];
  relationKinds: string[];
  runtimeChecked: boolean;
  schema: unknown;
  schemaHash: string;
}
/**
 * 插件输入。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "CollectorInput".
 */
export interface CollectorInput {
  allowedExtensions?: string[];
  candidateSets?: CandidateSet[];
  configSubset?: {
    [k: string]: unknown;
  };
  entities?: EntityInput[];
  /**
   * Original evidence for exported assertions whose source versions are selected.
   * Link stages reuse ranges instead of inventing offsets or reparsing producer logic.
   */
  evidence?: Evidence[];
  inputSnapshotId: string;
  limits: Limits;
  pluginId: string;
  protocolVersion: string;
  relations?: RelationInput[];
  runId: string;
  snapshot: SnapshotRef;
  sourceSets?: SourceSet[];
  sources: SourcePayload[];
  stageId: string;
  version: string;
}
/**
 * 插件输入中的实体（只读）。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "EntityInput".
 */
export interface EntityInput {
  assertions?: AssertionInput[];
  entityId: string;
  identity: IdentitySlot[];
  kind: EntityKind;
  scopeId: string;
}
/**
 * 一个身份槽位。`value` 为 `None` 表示该槽位缺省（只允许可选槽位）。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "IdentitySlot".
 */
export interface IdentitySlot {
  name: string;
  value?: IdentityValue;
}
/**
 * 证据：指向来源版本中的字节区间。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "Evidence".
 */
export interface Evidence {
  contentHash: string;
  endByte: number;
  endLine: number;
  id: string;
  role: string;
  sourceId: string;
  startByte: number;
  startLine: number;
}
/**
 * 运行限额。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "Limits".
 */
export interface Limits {
  maxInputBytes: number;
  maxOutputBytes: number;
  maxStderrBytes: number;
  timeoutMs: number;
}
/**
 * 插件输入中的关系（只读）。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "RelationInput".
 */
export interface RelationInput {
  assertions?: AssertionInput[];
  fromId: string;
  kind: RelationKind;
  qualifiers?: {
    [k: string]: unknown;
  };
  relationId: string;
  toId: string;
}
/**
 * 索引快照标识。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "SnapshotRef".
 */
export interface SnapshotRef {
  /**
   * 无符号十进制字符串。
   */
  generation: string;
  indexId: string;
  sourceSnapshotId?: string | null;
}
/**
 * 来源集合摘要。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "SourceSet".
 */
export interface SourceSet {
  /**
   * 集合是否完整导出；`false` 表示被限额截断。
   */
  complete: boolean;
  digest: string;
  selectorId: string;
  sourceIds?: string[];
}
/**
 * 带内容的来源。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "SourcePayload".
 */
export interface SourcePayload {
  byteLength: number;
  /**
   * 原始字节内容（UTF-8 文本）。
   */
  content: string;
  contentHash: string;
  id: string;
  language?: string | null;
  path: string;
  repositoryId: string;
  sourceClass: SourceClass;
}
/**
 * 插件输出。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "CollectorOutput".
 */
export interface CollectorOutput {
  coverage: Coverage[];
  dependencies: Dependency[];
  entities: EntityOutput[];
  evidence: Evidence[];
  gaps: Gap[];
  inputSnapshotId: string;
  protocolVersion: string;
  relations: RelationOutput[];
  runId: string;
  stageId: string;
}
/**
 * 覆盖声明。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "Coverage".
 */
export interface Coverage {
  capability: string;
  counts?: {
    [k: string]: unknown;
  };
  gapIds?: string[];
  scopeId: string;
  sourceSetId: string;
  status: CoverageStatus;
}
/**
 * 依赖记录。`digest` 由 Core 计算或复核，插件不能自报任意新鲜值。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "Dependency".
 */
export interface Dependency {
  digest: string;
  key: string;
  kind: DependencyKind;
}
/**
 * 插件产出的实体。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "EntityOutput".
 */
export interface EntityOutput {
  assertion: AssertionOutput;
  entityId: string;
  identity: IdentitySlot[];
  kind: EntityKind;
  scopeId: string;
}
/**
 * 缺口。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "Gap".
 */
export interface Gap {
  candidateIds?: string[];
  capability: string;
  code: string;
  id: string;
  reason: string;
  requiredEvidence?: string[];
  scopeId: string;
  sourceRefs?: string[];
}
/**
 * 插件产出的关系。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "RelationOutput".
 */
export interface RelationOutput {
  assertion: AssertionOutput;
  fromId: string;
  kind: RelationKind;
  qualifiers?: {
    [k: string]: unknown;
  };
  relationId: string;
  resolution: Resolution;
  toId: string;
}
/**
 * 公共过滤条件。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "CommonFilters".
 */
export interface CommonFilters {
  /**
   * 依据过滤，默认 `observed` + `declared`。
   */
  basisFilter?: Basis[] | null;
  /**
   * 新鲜度策略，默认 `verify`。
   */
  freshnessPolicy?: FreshnessPolicy | null;
  /**
   * 是否包含候选关系，默认 `false`。
   */
  includeCandidates?: boolean | null;
  kinds?: EntityKind[] | null;
  repositoryIds?: string[] | null;
  scopeIds?: string[] | null;
}
/**
 * 完整性信息。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "Completeness".
 */
export interface Completeness {
  candidateSetsComplete: boolean;
  checkedAt: string;
  coverage: CoverageLevel;
  diskChecked: boolean;
  freshness: Freshness;
  reasons?: string[];
  truncated: boolean;
}
/**
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "ConventionsImport".
 */
export interface ConventionsImport {
  expectedConfigHash: string;
  identityDelta?: IdentityDelta;
  notes: string;
  sourceReferences: SourceReference[];
}
/**
 * 身份增量：只允许新增或显式修改，不允许按名字覆盖既有 ID。
 */
export interface IdentityDelta {
  bindings?: BindingConfig[];
  dataSources?: DataSourceConfig[];
  scopes?: ScopeConfig[];
  services?: ServiceConfig[];
}
/**
 * 数据源登记。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "DataSourceConfig".
 */
export interface DataSourceConfig {
  catalog?: string | null;
  dialect: string;
  id: string;
  schema?: string | null;
  scopeId: string;
}
/**
 * 作用域。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "ScopeConfig".
 */
export interface ScopeConfig {
  id: string;
  kind: ScopeKind;
  repositoryIds?: string[];
  roots?: ScopeRoot[];
}
/**
 * 作用域根。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "ScopeRoot".
 */
export interface ScopeRoot {
  path: string;
  repositoryId: string;
}
/**
 * 服务登记。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "ServiceConfig".
 */
export interface ServiceConfig {
  baseUrls?: string[];
  id: string;
  protocol: string;
  scopeId: string;
}
/**
 * 查询响应中回传的来源引用；客户端必须原样传回，不得自行拼接 ID。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "SourceReference".
 */
export interface SourceReference {
  contentHash: string;
  endByte: number;
  evidenceId: string;
  snapshot: SnapshotRef;
  sourceId: string;
  startByte: number;
}
/**
 * 依赖声明。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "DependencySpec".
 */
export interface DependencySpec {
  kind: DependencyManagerKind;
  lockFile: string;
  manifest: string;
}
/**
 * 实体。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "Entity".
 */
export interface Entity {
  attributes?: {
    [k: string]: unknown;
  };
  basis: Basis;
  evidenceRefs?: string[];
  extensions?: {
    [k: string]: unknown;
  };
  id: string;
  identity: IdentitySlot[];
  kind: EntityKind;
  origin: Origin;
  scopeId: string;
}
/**
 * 实体引用。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "EntityRef".
 */
export interface EntityRef {
  entityId: string;
  snapshot: SnapshotRef;
}
/**
 * 实体选择器。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "EntitySelector".
 */
export interface EntitySelector {
  id: string;
  kinds?: EntityKind[];
  scopeIds?: string[];
}
/**
 * 错误内容。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "ErrorBody".
 */
export interface ErrorBody {
  code: string;
  issues?: Issue[];
  message: string;
  phase?: string | null;
  retryable: boolean;
  runId?: string | null;
}
/**
 * 校验问题明细。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "Issue".
 */
export interface Issue {
  actual?: string | null;
  code: string;
  expected?: string | null;
  /**
   * JSON Pointer。
   */
  instancePath: string;
  repairHint?: string | null;
  sourceReference?: SourceReference | null;
}
/**
 * 统一错误信封。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "ErrorEnvelope".
 */
export interface ErrorEnvelope {
  error: ErrorBody;
}
/**
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "ExtensionEntity".
 */
export interface ExtensionEntity {
  attributesSchema: unknown;
  identityComponents: ExtensionIdentityComponent[];
  name: ExtensionName;
}
/**
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "ExtensionIdentityComponent".
 */
export interface ExtensionIdentityComponent {
  name: string;
  required: boolean;
}
/**
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "ExtensionRelation".
 */
export interface ExtensionRelation {
  allowedFrom: EntityKind[];
  allowedTo: EntityKind[];
  name: ExtensionName;
  qualifiersSchema: unknown;
}
/**
 * 缺口（查询响应内使用）。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "Gap2".
 */
export interface Gap2 {
  candidateIds?: string[];
  capability: string;
  code: string;
  id: string;
  reason: string;
  requiredEvidence?: string[];
  scopeId: string;
  sourceRefs?: string[];
}
/**
 * 身份增量：只允许新增或显式修改，不允许按名字覆盖既有 ID。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "IdentityDelta".
 */
export interface IdentityDelta1 {
  bindings?: BindingConfig[];
  dataSources?: DataSourceConfig[];
  scopes?: ScopeConfig[];
  services?: ServiceConfig[];
}
/**
 * `joins` 关系的业务限定。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "JoinQualifiers".
 */
export interface JoinQualifiers {
  cardinality: Cardinality;
  condition?: string | null;
  joinType: JoinType;
  keyPairs: KeyPair[];
}
/**
 * 关联键对。`keyPairs` 按来源键顺序保存，顺序参与语义。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "KeyPair".
 */
export interface KeyPair {
  leftColumnId: string;
  operator: JoinOperator;
  rightColumnId: string;
}
/**
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "KnowledgeDependency".
 */
export interface KnowledgeDependency {
  digest: string;
  key: string;
  kind: string;
}
/**
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "TopicContextRequest".
 */
export interface TopicContextRequest {
  maxChars?: number;
  query: QueryRequest;
  topicId: string;
}
/**
 * 根选择过滤：只限制起点，不限制后续遍历类型。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "RootFilter".
 */
export interface RootFilter {
  kinds?: EntityKind[] | null;
  scopeIds?: string[] | null;
}
/**
 * 遍历参数。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "Traversal".
 */
export interface Traversal {
  direction: Direction;
  filter?: TraversalFilter | null;
  maxDepth?: number | null;
}
/**
 * 遍历过滤。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "TraversalFilter".
 */
export interface TraversalFilter {
  kinds?: EntityKind[] | null;
  relationKinds?: RelationKind[] | null;
}
/**
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "TopicArtifact".
 */
export interface TopicArtifact {
  claims: TopicClaim[];
  gaps: Gap2[];
  inputDigest: string;
  markdown: string;
  modelInfo?: TopicModelInfo | null;
  promptHash: string;
  protocolVersion: string;
  snapshot: SnapshotRef;
  sourceReferences: SourceReference[];
  templateHash: string;
  title: string;
  topicId: string;
}
/**
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "TopicClaim".
 */
export interface TopicClaim {
  basis: Basis;
  evidenceRefs?: string[];
  gapIds?: string[];
  id: string;
  origin: Origin;
  text: string;
}
/**
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "TopicModelInfo".
 */
export interface TopicModelInfo {
  model: string;
  provider: string;
}
/**
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "ManifestExtensions".
 */
export interface ManifestExtensions {
  entities?: ExtensionEntity[];
  relations?: ExtensionRelation[];
}
/**
 * 未知项：未确定的业务口径必须显式标注，不得用空数组冒充“已确认无”。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "UnknownValue".
 */
export interface UnknownValue {
  gapIds?: string[];
  status: UnknownStatus;
}
/**
 * 时间字段口径。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "TimeFieldSpec".
 */
export interface TimeFieldSpec {
  columnId: string;
  role?: string | null;
}
/**
 * 单位口径。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "UnitSpec".
 */
export interface UnitSpec {
  field: string;
  unit: string;
}
/**
 * 建议的下一条查询。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "NextQuery".
 */
export interface NextQuery {
  arguments?: {
    [k: string]: unknown;
  };
  operation: string;
  reason: string;
}
/**
 * 插件 manifest。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "PluginManifest".
 */
export interface PluginManifest {
  acceptanceCases?: string[];
  artifacts?: string[];
  capabilities?: string[];
  dependencies?: DependencySpec | null;
  extensions?: ManifestExtensions1;
  limits?: Limits | null;
  pluginId: string;
  protocolVersion: string;
  runtime: RuntimeSpec;
  stages?: StageSpec[];
  version: string;
}
export interface ManifestExtensions1 {
  entities?: ExtensionEntity[];
  relations?: ExtensionRelation[];
}
/**
 * 运行时声明。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "RuntimeSpec".
 */
export interface RuntimeSpec {
  /**
   * `executable` 运行时只能引用项目显式配置的 runtime ID。
   */
  executableId?: string | null;
  kind: RuntimeKind;
  versionRange?: string | null;
}
/**
 * 阶段声明。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "StageSpec".
 */
export interface StageSpec {
  args?: string[];
  dependsOn?: string[];
  entitySelectors?: EntitySelector[];
  entry: string;
  id: string;
  phase: StagePhase;
  replacement: ReplacementScope;
  sourceSelectors?: SourceSelector[];
}
/**
 * 来源选择器。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "SourceSelector".
 */
export interface SourceSelector {
  exclude?: string[];
  id: string;
  include?: string[];
  repositoryIds?: string[];
}
/**
 * `queryImplementation` 实体属性。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "QueryImplementationAttributes".
 */
export interface QueryImplementationAttributes {
  aggregates: MaybeKnown3;
  dialect: string;
  grainColumnIds: MaybeKnown;
  joins: MaybeKnown2;
  nullHandling: MaybeKnown5;
  statement: StatementRange;
  timeFields: MaybeKnown4;
  units: MaybeKnown6;
  validStates: MaybeKnown;
}
/**
 * 原始语句范围（字节区间 + 原始文本）。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "StatementRange".
 */
export interface StatementRange {
  endByte: number;
  startByte: number;
  text?: string | null;
}
/**
 * 查询响应。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "QueryResponse".
 */
export interface QueryResponse {
  budgetUsed: BudgetUsed;
  completeness: Completeness;
  continuation?: string | null;
  data: unknown;
  gaps?: Gap2[];
  nextQueries?: NextQuery[];
  protocolVersion: string;
  requestId: string;
  snapshot: SnapshotRef;
  sourceReferences?: SourceReference[];
  status: ResponseStatus;
}
/**
 * 关系。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "Relation".
 */
export interface Relation {
  basis: Basis;
  evidenceRefs?: string[];
  extensions?: {
    [k: string]: unknown;
  };
  fromId: string;
  id: string;
  kind: RelationKind;
  origin: Origin;
  qualifiers?: {
    [k: string]: unknown;
  };
  resolution: Resolution;
  toId: string;
}
/**
 * 关系引用。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "RelationRef".
 */
export interface RelationRef {
  relationId: string;
  snapshot: SnapshotRef;
}
/**
 * `reportField` 实体属性。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "ReportFieldAttributes".
 */
export interface ReportFieldAttributes {
  expression?: string | null;
  outputName: string;
  queryId: string;
  sourceColumnIds: MaybeKnown;
  transforms?: string[];
}
/**
 * 来源身份：按仓库与相对路径确定。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "Source".
 */
export interface Source {
  byteLength: number;
  contentHash: string;
  id: string;
  language?: string | null;
  path: string;
  repositoryId: string;
  sourceClass: SourceClass;
}
/**
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "TopicContext".
 */
export interface TopicContext {
  codeMap: QueryResponse;
  conventions: unknown[];
  coverage: unknown;
  dependencies: KnowledgeDependency[];
  facts: QueryResponse;
  gaps: Gap2[];
  inputDigest: string;
  materialDigest: string;
  promptHash: string;
  protocolVersion: string;
  request: TopicContextRequest;
  snapshot: SnapshotRef;
  sourceReads: unknown;
  sourceReferences: SourceReference[];
  templateHash: string;
  topicId: string;
}
/**
 * `transitions_to` 关系的业务限定。
 *
 * This interface was referenced by `ContractDeclarations`'s JSON-Schema
 * via the `definition` "TransitionQualifiers".
 */
export interface TransitionQualifiers {
  documentTypeId?: string | null;
  guard?: string | null;
  keyPairs?: KeyPair[];
  operationId?: string | null;
  trigger?: string | null;
}
