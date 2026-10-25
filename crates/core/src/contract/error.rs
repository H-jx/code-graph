//! 统一错误契约（计划 §3.3）。
//!
//! 所有对外失败都使用 `ErrorEnvelope`。可选字段缺省省略，不使用无意义 null。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::contract::evidence::SourceReference;

/// 统一错误信封。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ErrorEnvelope {
    pub error: ErrorBody,
}

/// 错误内容。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    #[serde(default)]
    pub issues: Vec<Issue>,
}

/// 校验问题明细。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Issue {
    pub code: String,
    /// JSON Pointer。
    pub instance_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_reference: Option<SourceReference>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repair_hint: Option<String>,
}

impl Issue {
    pub fn new(code: &str, instance_path: &str) -> Self {
        Issue {
            code: code.to_string(),
            instance_path: instance_path.to_string(),
            source_reference: None,
            expected: None,
            actual: None,
            repair_hint: None,
        }
    }

    pub fn with_expected(mut self, expected: impl Into<String>) -> Self {
        self.expected = Some(expected.into());
        self
    }

    pub fn with_actual(mut self, actual: impl Into<String>) -> Self {
        self.actual = Some(actual.into());
        self
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.repair_hint = Some(hint.into());
        self
    }
}

/// 关键错误码。未知故障统一映射为 `INTERNAL_ERROR`。
pub mod codes {
    pub const CONFIG_INVALID: &str = "CONFIG_INVALID";
    pub const PROTOCOL_UNSUPPORTED: &str = "PROTOCOL_UNSUPPORTED";
    pub const SCHEMA_INVALID: &str = "SCHEMA_INVALID";
    pub const IDENTITY_CONFLICT: &str = "IDENTITY_CONFLICT";
    pub const REFERENCE_MISSING: &str = "REFERENCE_MISSING";
    pub const EVIDENCE_INVALID: &str = "EVIDENCE_INVALID";
    pub const SOURCE_CHANGED: &str = "SOURCE_CHANGED";
    pub const SNAPSHOT_EXPIRED: &str = "SNAPSHOT_EXPIRED";
    pub const DEPENDENCY_CHANGED: &str = "DEPENDENCY_CHANGED";
    pub const CANDIDATE_SET_INCOMPLETE: &str = "CANDIDATE_SET_INCOMPLETE";
    pub const RUNTIME_MISSING: &str = "RUNTIME_MISSING";
    pub const DEPENDENCY_PREPARE_FAILED: &str = "DEPENDENCY_PREPARE_FAILED";
    pub const PLUGIN_EXIT: &str = "PLUGIN_EXIT";
    pub const PLUGIN_TIMEOUT: &str = "PLUGIN_TIMEOUT";
    pub const OUTPUT_LIMIT: &str = "OUTPUT_LIMIT";
    pub const ARTIFACT_CHANGED: &str = "ARTIFACT_CHANGED";
    pub const ACTIVATION_CONFLICT: &str = "ACTIVATION_CONFLICT";
    pub const INDEX_MISSING: &str = "INDEX_MISSING";
    pub const INDEX_STALE: &str = "INDEX_STALE";
    pub const CANCELLED: &str = "CANCELLED";
    pub const BUDGET_EXHAUSTED: &str = "BUDGET_EXHAUSTED";
    pub const MODEL_CONFIG_INVALID: &str = "MODEL_CONFIG_INVALID";
    pub const BUSY: &str = "BUSY";
    pub const CLOSED: &str = "CLOSED";
    pub const INTERNAL_ERROR: &str = "INTERNAL_ERROR";
    pub const NOT_IMPLEMENTED: &str = "NOT_IMPLEMENTED";
    pub const INVALID_REQUEST: &str = "INVALID_REQUEST";
}

/// 缺口（gap）代码登记表。
pub mod gap_codes {
    /// 候选集存在歧义。
    pub const AMBIGUOUS_BINDING: &str = "AMBIGUOUS_BINDING";
    /// 目标是动态构造的，无法静态确定。
    pub const DYNAMIC_TARGET: &str = "DYNAMIC_TARGET";
    /// 能力未配置。
    pub const NOT_CONFIGURED: &str = "NOT_CONFIGURED";
    /// 缺少流程依据。
    pub const WORKFLOW_EVIDENCE_MISSING: &str = "WORKFLOW_EVIDENCE_MISSING";
    /// 缺少业务口径。
    pub const DEFINITION_MISSING: &str = "DEFINITION_MISSING";
    /// 证据互相冲突。
    pub const CONFLICTING_EVIDENCE: &str = "CONFLICTING_EVIDENCE";
    /// 复合键映射不完整。
    pub const KEY_MAPPING_INCOMPLETE: &str = "KEY_MAPPING_INCOMPLETE";
    /// 源文件被跳过。
    pub const SOURCE_SKIPPED: &str = "SOURCE_SKIPPED";
    /// 语法解析失败。
    pub const PARSE_FAILED: &str = "PARSE_FAILED";
    /// 路径大小写冲突。
    pub const PATH_CASE_CONFLICT: &str = "PATH_CASE_CONFLICT";
    /// 符号链接被跳过。
    pub const SYMLINK_SKIPPED: &str = "SYMLINK_SKIPPED";
    /// 文件超出大小限制。
    pub const FILE_TOO_LARGE: &str = "FILE_TOO_LARGE";
    /// 文件不是合法 UTF-8。
    pub const NOT_UTF8: &str = "NOT_UTF8";
    /// 文件不可读。
    pub const UNREADABLE: &str = "UNREADABLE";
    /// 数据源未确认。
    pub const DATA_SOURCE_UNRESOLVED: &str = "DATA_SOURCE_UNRESOLVED";
    /// 命名空间解析失败。
    pub const NAMESPACE_UNRESOLVED: &str = "NAMESPACE_UNRESOLVED";
}

/// Core 错误类型。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CoreError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    #[serde(default)]
    pub issues: Vec<Issue>,
}

impl CoreError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        CoreError {
            code: code.to_string(),
            message: message.into(),
            retryable: false,
            run_id: None,
            phase: None,
            issues: Vec::new(),
        }
    }

    pub fn retryable(code: &str, message: impl Into<String>) -> Self {
        CoreError {
            code: code.to_string(),
            message: message.into(),
            retryable: true,
            run_id: None,
            phase: None,
            issues: Vec::new(),
        }
    }

    pub fn with_issues(mut self, issues: Vec<Issue>) -> Self {
        self.issues = issues;
        self
    }

    pub fn with_phase(mut self, phase: &str) -> Self {
        self.phase = Some(phase.to_string());
        self
    }

    pub fn with_run_id(mut self, run_id: &str) -> Self {
        self.run_id = Some(run_id.to_string());
        self
    }

    pub fn to_envelope(&self) -> ErrorEnvelope {
        ErrorEnvelope {
            error: ErrorBody {
                code: self.code.clone(),
                message: self.message.clone(),
                retryable: self.retryable,
                run_id: self.run_id.clone(),
                phase: self.phase.clone(),
                issues: self.issues.clone(),
            },
        }
    }
}

impl std::fmt::Display for CoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for CoreError {}
