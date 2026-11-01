//! 实体契约（计划 §3.3）。
//!
//! 实体身份与生产者断言分离：同一实体的多个生产者断言分别存储，属性冲突时
//! 分别返回断言，不以最后写入者覆盖。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::contract::identity::IdentitySlot;

/// 内置实体类型。扩展类型形如 `x.<pluginId>.<localName>`，另行注册。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum EntityKind {
    File,
    Symbol,
    Component,
    Operation,
    Route,
    HttpEndpoint,
    GraphqlOperation,
    GraphqlField,
    DataTable,
    DataColumn,
    DataKey,
    Module,
    Entrypoint,
    WorkflowState,
    DocumentType,
    QueryImplementation,
    ReportField,
    Event,
    Request,
    Title,
    Comment,
    DataSource,
    Service,
    #[serde(untagged)]
    Extension(crate::contract::extensions::ExtensionName),
}

impl EntityKind {
    pub fn as_str(&self) -> &str {
        match self {
            EntityKind::File => "file",
            EntityKind::Symbol => "symbol",
            EntityKind::Component => "component",
            EntityKind::Operation => "operation",
            EntityKind::Route => "route",
            EntityKind::HttpEndpoint => "httpEndpoint",
            EntityKind::GraphqlOperation => "graphqlOperation",
            EntityKind::GraphqlField => "graphqlField",
            EntityKind::DataTable => "dataTable",
            EntityKind::DataColumn => "dataColumn",
            EntityKind::DataKey => "dataKey",
            EntityKind::Module => "module",
            EntityKind::Entrypoint => "entrypoint",
            EntityKind::WorkflowState => "workflowState",
            EntityKind::DocumentType => "documentType",
            EntityKind::QueryImplementation => "queryImplementation",
            EntityKind::ReportField => "reportField",
            EntityKind::Event => "event",
            EntityKind::Request => "request",
            EntityKind::Title => "title",
            EntityKind::Comment => "comment",
            EntityKind::DataSource => "dataSource",
            EntityKind::Service => "service",
            EntityKind::Extension(name) => name.as_str(),
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        ALL_ENTITY_KINDS
            .iter()
            .find(|kind| kind.as_str() == value)
            .cloned()
            .or_else(|| {
                crate::contract::extensions::ExtensionName::new(value)
                    .ok()
                    .map(Self::Extension)
            })
    }
}

/// 全部内置实体类型（固定顺序，供注册表与 Schema 导出使用）。
pub const ALL_ENTITY_KINDS: &[EntityKind] = &[
    EntityKind::File,
    EntityKind::Symbol,
    EntityKind::Component,
    EntityKind::Operation,
    EntityKind::Route,
    EntityKind::HttpEndpoint,
    EntityKind::GraphqlOperation,
    EntityKind::GraphqlField,
    EntityKind::DataTable,
    EntityKind::DataColumn,
    EntityKind::DataKey,
    EntityKind::Module,
    EntityKind::Entrypoint,
    EntityKind::WorkflowState,
    EntityKind::DocumentType,
    EntityKind::QueryImplementation,
    EntityKind::ReportField,
    EntityKind::Event,
    EntityKind::Request,
    EntityKind::Title,
    EntityKind::Comment,
    EntityKind::DataSource,
    EntityKind::Service,
];

/// 事实依据。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Basis {
    /// 原始代码或配置中可观察的结构。
    Observed,
    /// 原始规范或资料中的声明。
    Declared,
    /// 推断。
    Inferred,
}

impl Basis {
    pub fn as_str(&self) -> &'static str {
        match self {
            Basis::Observed => "observed",
            Basis::Declared => "declared",
            Basis::Inferred => "inferred",
        }
    }

    pub fn parse(value: &str) -> Basis {
        match value {
            "declared" => Basis::Declared,
            "inferred" => Basis::Inferred,
            _ => Basis::Observed,
        }
    }
}

/// 断言来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Origin {
    /// 静态规则或插件脚本产生。
    Static,
    /// 人工登记。
    Human,
    /// 模型判断（必须是 `inferred` 依据）。
    Ai,
}

impl Origin {
    pub fn as_str(&self) -> &'static str {
        match self {
            Origin::Static => "static",
            Origin::Human => "human",
            Origin::Ai => "ai",
        }
    }

    pub fn parse(value: &str) -> Origin {
        match value {
            "human" => Origin::Human,
            "ai" => Origin::Ai,
            _ => Origin::Static,
        }
    }
}

/// 业务属性容器（JSON 对象）。Schema 只约束为对象，不预设业务字段。
pub type Attributes = Map<String, Value>;

/// 实体。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Entity {
    pub id: String,
    pub kind: EntityKind,
    pub scope_id: String,
    pub identity: Vec<IdentitySlot>,
    #[serde(default)]
    pub attributes: Attributes,
    pub basis: Basis,
    pub origin: Origin,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub extensions: Map<String, Value>,
}

/// 未知项：未确定的业务口径必须显式标注，不得用空数组冒充“已确认无”。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UnknownValue {
    pub status: UnknownStatus,
    #[serde(default)]
    pub gap_ids: Vec<String>,
}

/// 未知项状态标记。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum UnknownStatus {
    Unknown,
}

impl UnknownValue {
    pub fn unknown(gap_ids: Vec<String>) -> Self {
        UnknownValue {
            status: UnknownStatus::Unknown,
            gap_ids,
        }
    }
}

/// 已知值或未知项。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum MaybeKnown<T> {
    Known(T),
    Unknown(UnknownValue),
}

impl<T> MaybeKnown<T> {
    pub fn known(value: T) -> Self {
        MaybeKnown::Known(value)
    }

    pub fn unknown(gap_ids: Vec<String>) -> Self {
        MaybeKnown::Unknown(UnknownValue::unknown(gap_ids))
    }

    pub fn is_unknown(&self) -> bool {
        matches!(self, MaybeKnown::Unknown(_))
    }

    pub fn as_known(&self) -> Option<&T> {
        match self {
            MaybeKnown::Known(value) => Some(value),
            MaybeKnown::Unknown(_) => None,
        }
    }
}
