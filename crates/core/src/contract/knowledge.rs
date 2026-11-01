//! 主题上下文、外部解释和派生约定的公共契约；生成材料不作为事实来源。
use super::{
    entity::{Basis, Origin},
    evidence::{SnapshotRef, SourceReference},
    query::{Gap, QueryRequest, QueryResponse},
};
use crate::project::config::IdentityDelta;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TopicContextRequest {
    pub topic_id: String,
    pub query: QueryRequest,
    #[serde(default = "default_chars")]
    pub max_chars: u64,
}
fn default_chars() -> u64 {
    80_000
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TopicContext {
    pub protocol_version: String,
    pub topic_id: String,
    pub snapshot: SnapshotRef,
    pub input_digest: String,
    pub material_digest: String,
    pub request: TopicContextRequest,
    pub facts: QueryResponse<Value>,
    pub code_map: QueryResponse<Value>,
    pub source_reads: Value,
    pub coverage: Value,
    pub source_references: Vec<SourceReference>,
    pub conventions: Vec<Value>,
    pub gaps: Vec<Gap>,
    pub prompt_hash: String,
    pub template_hash: String,
    pub dependencies: Vec<KnowledgeDependency>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct KnowledgeDependency {
    pub kind: String,
    pub key: String,
    pub digest: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TopicClaim {
    pub id: String,
    pub text: String,
    pub basis: Basis,
    pub origin: Origin,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub gap_ids: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TopicModelInfo {
    pub provider: String,
    pub model: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TopicArtifact {
    pub protocol_version: String,
    pub topic_id: String,
    pub title: String,
    pub snapshot: SnapshotRef,
    pub input_digest: String,
    pub markdown: String,
    pub claims: Vec<TopicClaim>,
    pub source_references: Vec<SourceReference>,
    pub gaps: Vec<Gap>,
    pub prompt_hash: String,
    pub template_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_info: Option<TopicModelInfo>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ConventionsImport {
    pub expected_config_hash: String,
    pub notes: String,
    pub source_references: Vec<SourceReference>,
    #[serde(default)]
    pub identity_delta: IdentityDelta,
}
