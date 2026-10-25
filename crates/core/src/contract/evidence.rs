//! 来源、证据与快照契约（计划 §3.3）。
//!
//! 证据区间是原文件的 UTF-8 原始字节 `[startByte, endByte)`，行号从 1 开始，
//! `endLine` 为最后覆盖的行。行列不参与身份计算。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// 来源类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SourceClass {
    Code,
    Config,
    Schema,
    Documentation,
    /// 已调查、派生自原始资料的约定。
    DerivedConvention,
    /// 模型生成的材料；不得作为事实采集来源。
    GeneratedKnowledge,
}

impl SourceClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            SourceClass::Code => "code",
            SourceClass::Config => "config",
            SourceClass::Schema => "schema",
            SourceClass::Documentation => "documentation",
            SourceClass::DerivedConvention => "derivedConvention",
            SourceClass::GeneratedKnowledge => "generatedKnowledge",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "code" => Some(SourceClass::Code),
            "config" => Some(SourceClass::Config),
            "schema" => Some(SourceClass::Schema),
            "documentation" => Some(SourceClass::Documentation),
            "derivedConvention" => Some(SourceClass::DerivedConvention),
            "generatedKnowledge" => Some(SourceClass::GeneratedKnowledge),
            _ => None,
        }
    }

    /// 是否可以作为原始证据来源。生成知识不可以。
    pub fn is_original(&self) -> bool {
        !matches!(self, SourceClass::GeneratedKnowledge)
    }
}

/// 来源身份：按仓库与相对路径确定。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Source {
    pub id: String,
    pub repository_id: String,
    pub path: String,
    pub source_class: SourceClass,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    pub content_hash: String,
    pub byte_length: u64,
}

/// 证据：指向来源版本中的字节区间。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Evidence {
    pub id: String,
    pub source_id: String,
    pub content_hash: String,
    pub start_byte: u64,
    pub end_byte: u64,
    pub start_line: u64,
    pub end_line: u64,
    pub role: String,
}

/// 查询响应中回传的来源引用；客户端必须原样传回，不得自行拼接 ID。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SourceReference {
    pub snapshot: SnapshotRef,
    pub evidence_id: String,
    pub source_id: String,
    pub content_hash: String,
    pub start_byte: u64,
    pub end_byte: u64,
}

/// 索引快照标识。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SnapshotRef {
    pub index_id: String,
    /// 无符号十进制字符串。
    pub generation: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_snapshot_id: Option<String>,
}

/// 读取来源片段的结果状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SourceReadStatus {
    /// 与磁盘当前内容一致。
    Current,
    /// 来源版本已过期（磁盘内容已变化或已删除）。
    Expired,
    /// 来源已不存在。
    Missing,
}
