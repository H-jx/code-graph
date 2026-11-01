//! Explicit maintenance DTOs shared by native, SDK and MCP.
use super::knowledge::{ConventionsImport, TopicArtifact, TopicContextRequest};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "operation",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum MaintenanceRequest {
    Index {
        mode: IndexMode,
    },
    DiscoverPlugins {},
    SubmitPlugin {
        path: String,
    },
    PreparePlugin {
        candidate_id: String,
    },
    ValidatePlugin {
        candidate_ids: Vec<String>,
    },
    ActivatePlugin {
        candidate_id: String,
    },
    RollbackPlugin {
        plugin_id: String,
        version: String,
    },
    Collect {},
    ExportContext {
        request: Box<TopicContextRequest>,
    },
    ImportTopic {
        artifact: Box<TopicArtifact>,
    },
    ImportConventions {
        input: Box<ConventionsImport>,
    },
    GenerateSkill {
        name: String,
        output: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum IndexMode {
    Base,
    Incremental,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Capabilities {
    pub protocol_version: String,
    pub schema_hash: String,
    pub schema: serde_json::Value,
    pub entity_kinds: Vec<String>,
    pub relation_kinds: Vec<String>,
    pub operations: Vec<String>,
    pub maintenance_operations: Vec<String>,
    pub languages: Vec<String>,
    pub providers: Vec<String>,
    pub plugin_runtimes: Vec<String>,
    pub runtime_checked: bool,
    pub limits: std::collections::BTreeMap<String, u64>,
}
