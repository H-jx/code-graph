//! 查询契约（计划 §3.7）。
//!
//! Core 暴露统一的 `QueryRequest` tagged union；SDK 与 MCP 使用同一 DTO，
//! 只做字段映射，不各自实现定位算法。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::contract::entity::{Attributes, Basis, EntityKind, Origin};
use crate::contract::evidence::{SnapshotRef, SourceReference};
use crate::contract::identity::IdentitySlot;
use crate::contract::relation::{RelationKind, Resolution};

/// 查询请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "operation", rename_all = "camelCase", deny_unknown_fields)]
pub enum QueryRequest {
    Status(StatusRequest),
    Search(SearchRequest),
    LocateFeature(LocateFeatureRequest),
    Trace(TraceRequest),
    CodeMap(CodeMapRequest),
    ReadSource(ReadSourceRequest),
    Page(PageRequest),
    Workflow(WorkflowRequest),
    Report(ReportRequest),
}

/// 公共过滤条件。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommonFilters {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_ids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository_ids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kinds: Option<Vec<EntityKind>>,
    /// 依据过滤，默认 `observed` + `declared`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub basis_filter: Option<Vec<Basis>>,
    /// 是否包含候选关系，默认 `false`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include_candidates: Option<bool>,
    /// 新鲜度策略，默认 `verify`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub freshness_policy: Option<FreshnessPolicy>,
}

/// 新鲜度策略。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum FreshnessPolicy {
    /// 在预算内核对相关来源与集合；确认不了返回 unknown。
    #[default]
    Verify,
    /// 只说明库内状态，不核验磁盘。
    Indexed,
    /// 明确返回历史断言并附警示。
    AllowStale,
}

/// 根选择过滤：只限制起点，不限制后续遍历类型。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RootFilter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kinds: Option<Vec<EntityKind>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_ids: Option<Vec<String>>,
}

/// 遍历过滤。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TraversalFilter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kinds: Option<Vec<EntityKind>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relation_kinds: Option<Vec<RelationKind>>,
}

/// 遍历参数。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Traversal {
    pub direction: Direction,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_depth: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<TraversalFilter>,
}

/// 遍历方向。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Direction {
    In,
    Out,
    Both,
}

/// 查询预算。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Budget {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_entities: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_relations: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_visited: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_depth: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_response_chars: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

/// 状态查询。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StatusRequest {
    #[serde(default = "default_true")]
    pub check_freshness: bool,
}

fn default_true() -> bool {
    true
}

/// 搜索请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SearchRequest {
    pub query: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<SearchField>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filters: Option<CommonFilters>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_size: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continuation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget: Option<Budget>,
}

/// 搜索字段。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SearchField {
    BusinessTerm,
    Route,
    Title,
    Comment,
    Module,
    Symbol,
    Path,
    Source,
}

/// 需求定位请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct LocateFeatureRequest {
    pub query: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route_hint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_filter: Option<RootFilter>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub traversal: Option<Traversal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filters: Option<CommonFilters>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget: Option<Budget>,
}

/// 链路追踪请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TraceRequest {
    pub roots: Vec<EntityRef>,
    pub direction: Direction,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relation_kinds: Option<Vec<RelationKind>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_depth: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filters: Option<CommonFilters>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget: Option<Budget>,
}

/// 实体引用。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct EntityRef {
    pub snapshot: SnapshotRef,
    pub entity_id: String,
}

/// 关系引用。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RelationRef {
    pub snapshot: SnapshotRef,
    pub relation_id: String,
}

/// 代码地图请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CodeMapRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roots: Option<Vec<EntityRef>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_files: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_symbols: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filters: Option<CommonFilters>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget: Option<Budget>,
}

/// 读取来源请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ReadSourceRequest {
    pub references: Vec<SourceReference>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_response_chars: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filters: Option<CommonFilters>,
}

/// 页面查询请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PageRequest {
    pub route: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_ids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filters: Option<CommonFilters>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget: Option<Budget>,
}

/// 流程查询请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct WorkflowRequest {
    pub query: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roots: Option<Vec<EntityRef>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_ids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filters: Option<CommonFilters>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget: Option<Budget>,
}

/// 台账查询请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ReportRequest {
    pub query: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roots: Option<Vec<EntityRef>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_ids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filters: Option<CommonFilters>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget: Option<Budget>,
}

/// 查询响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct QueryResponse<T = Value> {
    pub protocol_version: String,
    pub request_id: String,
    pub snapshot: SnapshotRef,
    pub status: ResponseStatus,
    pub data: T,
    #[serde(default)]
    pub source_references: Vec<SourceReference>,
    #[serde(default)]
    pub gaps: Vec<Gap>,
    pub completeness: Completeness,
    pub budget_used: BudgetUsed,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continuation: Option<String>,
    #[serde(default)]
    pub next_queries: Vec<NextQuery>,
}

/// 响应状态。`complete` 只代表请求范围和预算内结果完整。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ResponseStatus {
    Complete,
    Partial,
}

/// 完整性信息。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Completeness {
    pub freshness: Freshness,
    pub disk_checked: bool,
    pub checked_at: String,
    pub coverage: CoverageLevel,
    pub candidate_sets_complete: bool,
    pub truncated: bool,
    #[serde(default)]
    pub reasons: Vec<String>,
}

/// 新鲜度。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Freshness {
    Fresh,
    Stale,
    Unknown,
}

/// 覆盖级别。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum CoverageLevel {
    Complete,
    Partial,
    NotConfigured,
    Failed,
}

/// 预算使用情况。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BudgetUsed {
    pub entities: u64,
    pub relations: u64,
    pub visited: u64,
    pub elapsed_ms: u64,
    pub response_chars: u64,
}

/// 建议的下一条查询。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct NextQuery {
    pub operation: String,
    #[serde(default)]
    pub arguments: Map<String, Value>,
    pub reason: String,
}

/// 缺口（查询响应内使用）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Gap {
    pub id: String,
    pub code: String,
    pub capability: String,
    pub scope_id: String,
    #[serde(default)]
    pub source_refs: Vec<String>,
    #[serde(default)]
    pub candidate_ids: Vec<String>,
    pub reason: String,
    #[serde(default)]
    pub required_evidence: Vec<String>,
}

/// 实体投影。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct EntityProjection {
    pub entity_ref: EntityRef,
    pub kind: EntityKind,
    pub scope_id: String,
    pub identity: Vec<IdentitySlot>,
    #[serde(default)]
    pub assertions: Vec<EntityAssertionProjection>,
}

/// 实体断言投影。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct EntityAssertionProjection {
    pub assertion_id: String,
    pub producer_key: String,
    pub batch_id: String,
    pub basis: Basis,
    pub origin: Origin,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default)]
    pub attributes: Attributes,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

/// 关系投影。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RelationProjection {
    pub relation_ref: RelationRef,
    pub kind: RelationKind,
    pub from_id: String,
    pub to_id: String,
    #[serde(default)]
    pub qualifiers: Map<String, Value>,
    #[serde(default)]
    pub assertions: Vec<RelationAssertionProjection>,
}

/// 关系断言投影。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RelationAssertionProjection {
    pub assertion_id: String,
    pub producer_key: String,
    pub batch_id: String,
    pub basis: Basis,
    pub origin: Origin,
    pub resolution: Resolution,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default)]
    pub qualifiers: Map<String, Value>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

/// 默认预算（计划 §3.7）。
pub mod default_budget {
    pub const MAX_ENTITIES: u64 = 60;
    pub const MAX_RELATIONS: u64 = 120;
    pub const MAX_VISITED: u64 = 2_000;
    pub const MAX_DEPTH: u32 = 8;
    pub const MAX_RESPONSE_CHARS: u64 = 24_000;
    pub const TIMEOUT_MS: u64 = 2_000;
    pub const HARD_MAX_ENTITIES: u64 = 1_000;
    pub const HARD_MAX_RELATIONS: u64 = 3_000;
    pub const HARD_MAX_VISITED: u64 = 20_000;
    pub const HARD_MAX_DEPTH: u32 = 16;
    pub const HARD_MAX_RESPONSE_CHARS: u64 = 200_000;
    pub const HARD_TIMEOUT_MS: u64 = 30_000;
    pub const PAGE_SIZE_DEFAULT: u32 = 20;
    pub const PAGE_SIZE_MAX: u32 = 100;
}
