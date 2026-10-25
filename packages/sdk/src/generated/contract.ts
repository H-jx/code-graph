// 由 scripts/generate-types.mjs 从 docs/schema/contract.schema.json 生成。
// 请勿手工编辑：改动 Rust 契约后重新运行 `pnpm run build:contracts`。
/**
 * Core、CLI、SDK、MCP 与插件共同遵守的协议；类型由 Rust 契约导出。
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
 */
export type SearchField =
  "businessTerm" | "route" | "title" | "comment" | "module" | "symbol" | "path" | "source";
/**
 * 事实依据。
 */
export type Basis = "observed" | "declared" | "inferred";
/**
 * 新鲜度策略。
 */
export type FreshnessPolicy = "verify" | "indexed" | "allowStale";
/**
 * 内置实体类型。扩展类型形如 `x.<pluginId>.<localName>`，另行注册。
 */
export type EntityKind =
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
  | "service";
/**
 * 遍历方向。
 */
export type Direction = "in" | "out" | "both";
/**
 * 内置关系类型。扩展类型形如 `x.<pluginId>.<localName>`，另行注册。
 */
export type RelationKind =
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
  | "mapsTo"
  | "joins"
  | "transitionsTo"
  | "precedes"
  | "relatesDocument"
  | "describes";

/**
 * 查询预算。
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
 * 公共过滤条件。
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
 * 根选择过滤：只限制起点，不限制后续遍历类型。
 */
export interface RootFilter {
  kinds?: EntityKind[] | null;
  scopeIds?: string[] | null;
}
/**
 * 遍历参数。
 */
export interface Traversal {
  direction: Direction;
  filter?: TraversalFilter | null;
  maxDepth?: number | null;
}
/**
 * 遍历过滤。
 */
export interface TraversalFilter {
  kinds?: EntityKind[] | null;
  relationKinds?: RelationKind[] | null;
}
/**
 * 实体引用。
 */
export interface EntityRef {
  entityId: string;
  snapshot: SnapshotRef;
}
/**
 * 索引快照标识。
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
 * 查询响应中回传的来源引用；客户端必须原样传回，不得自行拼接 ID。
 */
export interface SourceReference {
  contentHash: string;
  endByte: number;
  evidenceId: string;
  snapshot: SnapshotRef;
  sourceId: string;
  startByte: number;
}
