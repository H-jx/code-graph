//! 插件 manifest、输入输出与运行记录契约（计划 §3.5）。
//!
//! 插件通过独立 JSON 协议接入：stdin 一个 UTF-8 JSON 对象，stdout 只能一个
//! JSON 对象，调试日志写 stderr。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::contract::entity::{Attributes, Basis, EntityKind, Origin};
use crate::contract::error::Issue;
use crate::contract::evidence::{Evidence, SnapshotRef, Source};
use crate::contract::identity::IdentitySlot;
use crate::contract::relation::{RelationKind, Resolution};

/// 插件 manifest。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PluginManifest {
    pub protocol_version: String,
    pub plugin_id: String,
    pub version: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
    pub runtime: RuntimeSpec,
    #[serde(default)]
    pub artifacts: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<DependencySpec>,
    #[serde(default)]
    pub stages: Vec<StageSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limits: Option<Limits>,
    #[serde(default)]
    pub acceptance_cases: Vec<String>,
    #[serde(default)]
    pub extensions: Map<String, Value>,
}

/// 运行时声明。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RuntimeSpec {
    pub kind: RuntimeKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_range: Option<String>,
    /// `executable` 运行时只能引用项目显式配置的 runtime ID。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executable_id: Option<String>,
}

/// 运行时种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeKind {
    Node,
    Python,
    Executable,
}

/// 依赖声明。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DependencySpec {
    pub kind: DependencyManagerKind,
    pub manifest: String,
    pub lock_file: String,
}

/// 依赖管理方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum DependencyManagerKind {
    Npm,
    Pip,
}

/// 阶段声明。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StageSpec {
    pub id: String,
    pub phase: StagePhase,
    pub entry: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub source_selectors: Vec<SourceSelector>,
    #[serde(default)]
    pub entity_selectors: Vec<EntitySelector>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    pub replacement: ReplacementScope,
}

/// 阶段所属阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum StagePhase {
    Extract,
    Link,
}

/// 替换范围。v1 只支持整项目替换。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ReplacementScope {
    Project,
}

/// 来源选择器。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SourceSelector {
    pub id: String,
    #[serde(default)]
    pub repository_ids: Vec<String>,
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

/// 实体选择器。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct EntitySelector {
    pub id: String,
    #[serde(default)]
    pub kinds: Vec<EntityKind>,
    #[serde(default)]
    pub scope_ids: Vec<String>,
}

/// 运行限额。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Limits {
    pub timeout_ms: u64,
    pub max_input_bytes: u64,
    pub max_output_bytes: u64,
    pub max_stderr_bytes: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            timeout_ms: 60_000,
            max_input_bytes: 33_554_432,
            max_output_bytes: 33_554_432,
            max_stderr_bytes: 1_048_576,
        }
    }
}

/// 插件验收用例。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AcceptanceCase {
    pub id: String,
    pub input_fixture: String,
    pub expected: CaseExpectation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub truth_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub truth_hash: Option<String>,
}

/// 用例预期。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CaseExpectation {
    #[serde(default)]
    pub entities: Vec<String>,
    #[serde(default)]
    pub relations: Vec<String>,
    #[serde(default)]
    pub forbidden_relations: Vec<String>,
    #[serde(default)]
    pub gap_codes: Vec<String>,
}

/// 插件输入。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CollectorInput {
    pub protocol_version: String,
    pub run_id: String,
    pub plugin_id: String,
    pub version: String,
    pub stage_id: String,
    pub snapshot: SnapshotRef,
    pub input_snapshot_id: String,
    #[serde(default)]
    pub config_subset: Value,
    pub sources: Vec<SourcePayload>,
    #[serde(default)]
    pub source_sets: Vec<SourceSet>,
    #[serde(default)]
    pub entities: Vec<EntityInput>,
    #[serde(default)]
    pub relations: Vec<RelationInput>,
    #[serde(default)]
    pub candidate_sets: Vec<CandidateSet>,
    #[serde(default)]
    pub allowed_extensions: Vec<String>,
    pub limits: Limits,
}

/// 带内容的来源。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SourcePayload {
    #[serde(flatten)]
    pub source: Source,
    /// 原始字节内容（UTF-8 文本）。
    pub content: String,
}

/// 来源集合摘要。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SourceSet {
    pub selector_id: String,
    pub digest: String,
    #[serde(default)]
    pub source_ids: Vec<String>,
    /// 集合是否完整导出；`false` 表示被限额截断。
    pub complete: bool,
}

/// 候选集合摘要。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CandidateSet {
    pub selector_id: String,
    pub digest: String,
    #[serde(default)]
    pub entity_ids: Vec<String>,
    pub complete: bool,
}

/// 插件输入中的实体（只读）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct EntityInput {
    pub entity_id: String,
    pub kind: EntityKind,
    pub scope_id: String,
    pub identity: Vec<IdentitySlot>,
    #[serde(default)]
    pub assertions: Vec<AssertionInput>,
}

/// 插件输入中的关系（只读）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RelationInput {
    pub relation_id: String,
    pub kind: RelationKind,
    pub from_id: String,
    pub to_id: String,
    #[serde(default)]
    pub qualifiers: Map<String, Value>,
    #[serde(default)]
    pub assertions: Vec<AssertionInput>,
}

/// 已存在的断言。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AssertionInput {
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution: Option<Resolution>,
}

/// 插件输出。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CollectorOutput {
    pub protocol_version: String,
    pub run_id: String,
    pub stage_id: String,
    pub input_snapshot_id: String,
    #[serde(default)]
    pub entities: Vec<EntityOutput>,
    #[serde(default)]
    pub relations: Vec<RelationOutput>,
    #[serde(default)]
    pub evidence: Vec<Evidence>,
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
    #[serde(default)]
    pub coverage: Vec<Coverage>,
    #[serde(default)]
    pub gaps: Vec<Gap>,
}

/// 插件产出的实体。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct EntityOutput {
    pub entity_id: String,
    pub kind: EntityKind,
    pub scope_id: String,
    pub identity: Vec<IdentitySlot>,
    pub assertion: AssertionOutput,
}

/// 插件产出的关系。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RelationOutput {
    pub relation_id: String,
    pub kind: RelationKind,
    pub from_id: String,
    pub to_id: String,
    #[serde(default)]
    pub qualifiers: Map<String, Value>,
    pub resolution: Resolution,
    pub assertion: AssertionOutput,
}

/// 插件产出的断言。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AssertionOutput {
    pub producer_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default)]
    pub attributes: Attributes,
    pub basis: Basis,
    pub origin: Origin,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

/// 依赖记录。`digest` 由 Core 计算或复核，插件不能自报任意新鲜值。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Dependency {
    pub kind: DependencyKind,
    pub key: String,
    pub digest: String,
}

/// 依赖种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum DependencyKind {
    Source,
    SourceSet,
    Config,
    Artifact,
    Runtime,
    CandidateSet,
    UpstreamBatch,
    Prompt,
    TopicInput,
}

impl DependencyKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            DependencyKind::Source => "source",
            DependencyKind::SourceSet => "sourceSet",
            DependencyKind::Config => "config",
            DependencyKind::Artifact => "artifact",
            DependencyKind::Runtime => "runtime",
            DependencyKind::CandidateSet => "candidateSet",
            DependencyKind::UpstreamBatch => "upstreamBatch",
            DependencyKind::Prompt => "prompt",
            DependencyKind::TopicInput => "topicInput",
        }
    }
}

/// 覆盖声明。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Coverage {
    pub capability: String,
    pub scope_id: String,
    pub source_set_id: String,
    pub status: CoverageStatus,
    #[serde(default)]
    pub counts: Map<String, Value>,
    #[serde(default)]
    pub gap_ids: Vec<String>,
}

/// 覆盖状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum CoverageStatus {
    Complete,
    Partial,
    NotConfigured,
    Failed,
}

impl CoverageStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            CoverageStatus::Complete => "complete",
            CoverageStatus::Partial => "partial",
            CoverageStatus::NotConfigured => "not_configured",
            CoverageStatus::Failed => "failed",
        }
    }
}

/// 缺口。
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

/// 校验结果报告。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ValidationReport {
    pub status: ValidationStatus,
    #[serde(default)]
    pub issues: Vec<Issue>,
    #[serde(default)]
    pub staged_result_hash: Option<String>,
}

/// 校验状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ValidationStatus {
    Passed,
    Failed,
}
