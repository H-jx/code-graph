//! 关系契约与业务载荷（计划 §3.3）。
//!
//! 关系的解析状态区分 `confirmed` 与 `candidate`：候选关系指向已经存在的候选
//! 实体；目标完全不确定时只输出 gap，不产生悬空关系。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::contract::entity::{Attributes, Basis, MaybeKnown, Origin};

/// 内置关系类型。扩展类型形如 `x.<pluginId>.<localName>`，另行注册。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum RelationKind {
    Contains,
    Defines,
    Imports,
    References,
    Calls,
    Renders,
    Handles,
    Triggers,
    Wraps,
    Requests,
    Serves,
    Resolves,
    Reads,
    Writes,
    #[serde(rename = "maps_to")]
    MapsTo,
    Joins,
    #[serde(rename = "transitions_to")]
    TransitionsTo,
    Precedes,
    #[serde(rename = "relates_document")]
    RelatesDocument,
    Describes,
    #[serde(untagged)]
    Extension(crate::contract::extensions::ExtensionName),
}

impl RelationKind {
    pub fn as_str(&self) -> &str {
        match self {
            RelationKind::Contains => "contains",
            RelationKind::Defines => "defines",
            RelationKind::Imports => "imports",
            RelationKind::References => "references",
            RelationKind::Calls => "calls",
            RelationKind::Renders => "renders",
            RelationKind::Handles => "handles",
            RelationKind::Triggers => "triggers",
            RelationKind::Wraps => "wraps",
            RelationKind::Requests => "requests",
            RelationKind::Serves => "serves",
            RelationKind::Resolves => "resolves",
            RelationKind::Reads => "reads",
            RelationKind::Writes => "writes",
            RelationKind::MapsTo => "maps_to",
            RelationKind::Joins => "joins",
            RelationKind::TransitionsTo => "transitions_to",
            RelationKind::Precedes => "precedes",
            RelationKind::RelatesDocument => "relates_document",
            RelationKind::Describes => "describes",
            RelationKind::Extension(name) => name.as_str(),
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        ALL_RELATION_KINDS
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

/// 全部内置关系类型（固定顺序）。
pub const ALL_RELATION_KINDS: &[RelationKind] = &[
    RelationKind::Contains,
    RelationKind::Defines,
    RelationKind::Imports,
    RelationKind::References,
    RelationKind::Calls,
    RelationKind::Renders,
    RelationKind::Handles,
    RelationKind::Triggers,
    RelationKind::Wraps,
    RelationKind::Requests,
    RelationKind::Serves,
    RelationKind::Resolves,
    RelationKind::Reads,
    RelationKind::Writes,
    RelationKind::MapsTo,
    RelationKind::Joins,
    RelationKind::TransitionsTo,
    RelationKind::Precedes,
    RelationKind::RelatesDocument,
    RelationKind::Describes,
];

/// 关系解析状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Resolution {
    Confirmed,
    Candidate,
}

impl Resolution {
    pub fn as_str(&self) -> &'static str {
        match self {
            Resolution::Confirmed => "confirmed",
            Resolution::Candidate => "candidate",
        }
    }

    pub fn parse(value: &str) -> Resolution {
        match value {
            "candidate" => Resolution::Candidate,
            _ => Resolution::Confirmed,
        }
    }
}

/// 关系。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Relation {
    pub id: String,
    pub kind: RelationKind,
    pub from_id: String,
    pub to_id: String,
    #[serde(default)]
    pub qualifiers: Map<String, Value>,
    pub basis: Basis,
    pub origin: Origin,
    pub resolution: Resolution,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub extensions: Map<String, Value>,
}

/// join 类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
    Cross,
}

/// 连接算子（初版仅支持等值）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum JoinOperator {
    Equals,
}

/// 关联键对。`keyPairs` 按来源键顺序保存，顺序参与语义。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct KeyPair {
    pub left_column_id: String,
    pub right_column_id: String,
    pub operator: JoinOperator,
}

/// 关联基数。未声明或未验证时必须是 `unknown`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Cardinality {
    OneToOne,
    OneToMany,
    ManyToOne,
    ManyToMany,
    Unknown,
}

/// `joins` 关系的业务限定。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct JoinQualifiers {
    pub join_type: JoinType,
    pub key_pairs: Vec<KeyPair>,
    pub cardinality: Cardinality,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
}

/// `transitions_to` 关系的业务限定。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TransitionQualifiers {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guard: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_type_id: Option<String>,
    #[serde(default)]
    pub key_pairs: Vec<KeyPair>,
}

/// 聚合表达式。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AggregateSpec {
    pub function: String,
    pub expression: String,
    #[serde(default)]
    pub source_column_ids: Vec<String>,
}

/// 时间字段口径。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TimeFieldSpec {
    pub column_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

/// 单位口径。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UnitSpec {
    pub field: String,
    pub unit: String,
}

/// 原始语句范围（字节区间 + 原始文本）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StatementRange {
    pub start_byte: u64,
    pub end_byte: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// `queryImplementation` 实体属性。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct QueryImplementationAttributes {
    pub dialect: String,
    pub statement: StatementRange,
    pub grain_column_ids: MaybeKnown<Vec<String>>,
    pub joins: MaybeKnown<Vec<JoinQualifiers>>,
    pub aggregates: MaybeKnown<Vec<AggregateSpec>>,
    pub valid_states: MaybeKnown<Vec<String>>,
    pub time_fields: MaybeKnown<Vec<TimeFieldSpec>>,
    pub null_handling: MaybeKnown<String>,
    pub units: MaybeKnown<Vec<UnitSpec>>,
}

/// `reportField` 实体属性。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ReportFieldAttributes {
    pub query_id: String,
    pub output_name: String,
    pub source_column_ids: MaybeKnown<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expression: Option<String>,
    #[serde(default)]
    pub transforms: Vec<String>,
}

/// 关系端点合法性登记表（计划 §3.3：不允许任意连线）。
///
/// 每种关系类型声明允许的起点类型与终点类型；扩展类型另行注册。
pub fn endpoint_allowed(kind: RelationKind, from: &str, to: &str) -> bool {
    if matches!(&kind, RelationKind::Extension(_)) {
        return false;
    }
    let allowed: (&[&str], &[&str]) = match kind {
        RelationKind::Contains => (
            &["file", "module", "component"],
            &[
                "file",
                "symbol",
                "component",
                "module",
                "operation",
                "route",
                "entrypoint",
                "workflowState",
                "documentType",
                "queryImplementation",
                "reportField",
                "title",
                "comment",
                "event",
                "request",
                "dataTable",
                "dataColumn",
                "dataKey",
                "service",
                "dataSource",
            ],
        ),
        RelationKind::Defines => (
            &["file", "module", "component", "service", "dataSource"],
            &[
                "symbol",
                "component",
                "operation",
                "route",
                "httpEndpoint",
                "graphqlOperation",
                "graphqlField",
                "dataTable",
                "dataColumn",
                "dataKey",
                "event",
                "entrypoint",
                "workflowState",
                "documentType",
                "queryImplementation",
                "reportField",
                "title",
            ],
        ),
        RelationKind::Imports => (
            &["file", "symbol", "component"],
            &["file", "symbol", "module"],
        ),
        RelationKind::References => (
            &[
                "symbol",
                "component",
                "operation",
                "route",
                "httpEndpoint",
                "graphqlOperation",
                "graphqlField",
                "queryImplementation",
                "reportField",
                "comment",
                "title",
                "event",
                "request",
                "documentType",
                "workflowState",
            ],
            &[
                "symbol",
                "file",
                "component",
                "route",
                "httpEndpoint",
                "graphqlOperation",
                "graphqlField",
                "dataTable",
                "dataColumn",
                "dataKey",
                "documentType",
                "workflowState",
                "queryImplementation",
                "reportField",
                "service",
                "dataSource",
            ],
        ),
        RelationKind::Calls => (
            &[
                "symbol",
                "operation",
                "component",
                "httpEndpoint",
                "graphqlOperation",
                "entrypoint",
                "event",
                "request",
            ],
            &[
                "symbol",
                "operation",
                "httpEndpoint",
                "graphqlOperation",
                "service",
                "queryImplementation",
            ],
        ),
        RelationKind::Renders => (
            &["route", "component", "file"],
            &["title", "component", "comment", "documentType"],
        ),
        RelationKind::Handles => (
            &[
                "httpEndpoint",
                "graphqlOperation",
                "event",
                "entrypoint",
                "route",
            ],
            &["symbol", "operation", "component"],
        ),
        RelationKind::Triggers => (
            &["event", "operation", "request"],
            &[
                "symbol",
                "operation",
                "httpEndpoint",
                "graphqlOperation",
                "workflowState",
            ],
        ),
        RelationKind::Wraps => (
            &["symbol", "component", "request"],
            &[
                "symbol",
                "component",
                "operation",
                "httpEndpoint",
                "graphqlOperation",
            ],
        ),
        RelationKind::Requests => (
            &["component", "symbol", "operation", "event"],
            &["httpEndpoint", "graphqlOperation", "request", "service"],
        ),
        RelationKind::Serves => (&["service"], &["httpEndpoint", "graphqlOperation"]),
        RelationKind::Resolves => (
            &["graphqlOperation", "graphqlField"],
            &["symbol", "operation"],
        ),
        RelationKind::Reads => (
            &["symbol", "operation", "queryImplementation"],
            &["dataTable", "dataColumn", "dataKey", "dataSource"],
        ),
        RelationKind::Writes => (
            &["symbol", "operation", "queryImplementation"],
            &["dataTable", "dataColumn", "dataKey", "dataSource"],
        ),
        RelationKind::MapsTo => (
            &["dataColumn", "dataKey", "dataTable", "symbol", "operation"],
            &["dataColumn", "dataKey", "dataTable", "symbol", "operation"],
        ),
        RelationKind::Joins => (
            &["queryImplementation", "operation", "symbol"],
            &["dataTable", "dataColumn", "dataKey"],
        ),
        RelationKind::TransitionsTo => (&["workflowState", "operation"], &["workflowState"]),
        RelationKind::Precedes => (
            &["operation", "workflowState", "documentType"],
            &["operation", "workflowState", "documentType"],
        ),
        RelationKind::RelatesDocument => (
            &[
                "operation",
                "workflowState",
                "documentType",
                "queryImplementation",
            ],
            &["documentType"],
        ),
        RelationKind::Describes => (
            &["comment", "title", "documentType"],
            &[
                "symbol",
                "component",
                "operation",
                "route",
                "httpEndpoint",
                "dataTable",
                "workflowState",
                "documentType",
            ],
        ),
        RelationKind::Extension(_) => (&[], &[]),
    };
    allowed.0.contains(&from) && allowed.1.contains(&to)
}

/// 关系类型登记表：返回该关系允许的端点类型列表（用于 Schema 与文档）。
pub fn endpoint_kinds(kind: RelationKind) -> (&'static [&'static str], &'static [&'static str]) {
    match kind {
        RelationKind::Extension(_) => (&[], &[]),
        RelationKind::Contains => (
            &["file", "module", "component"],
            &[
                "file",
                "symbol",
                "component",
                "module",
                "operation",
                "route",
                "entrypoint",
                "workflowState",
                "documentType",
                "queryImplementation",
                "reportField",
                "title",
                "comment",
                "event",
                "request",
                "dataTable",
                "dataColumn",
                "dataKey",
                "service",
                "dataSource",
            ],
        ),
        RelationKind::Defines => (
            &["file", "module", "component", "service", "dataSource"],
            &[
                "symbol",
                "component",
                "operation",
                "route",
                "httpEndpoint",
                "graphqlOperation",
                "graphqlField",
                "dataTable",
                "dataColumn",
                "dataKey",
                "event",
                "entrypoint",
                "workflowState",
                "documentType",
                "queryImplementation",
                "reportField",
                "title",
            ],
        ),
        RelationKind::Imports => (
            &["file", "symbol", "component"],
            &["file", "symbol", "module"],
        ),
        RelationKind::References => (
            &[
                "symbol",
                "component",
                "operation",
                "route",
                "httpEndpoint",
                "graphqlOperation",
                "graphqlField",
                "queryImplementation",
                "reportField",
                "comment",
                "title",
                "event",
                "request",
                "documentType",
                "workflowState",
            ],
            &[
                "symbol",
                "file",
                "component",
                "route",
                "httpEndpoint",
                "graphqlOperation",
                "graphqlField",
                "dataTable",
                "dataColumn",
                "dataKey",
                "documentType",
                "workflowState",
                "queryImplementation",
                "reportField",
                "service",
                "dataSource",
            ],
        ),
        RelationKind::Calls => (
            &[
                "symbol",
                "operation",
                "component",
                "httpEndpoint",
                "graphqlOperation",
                "entrypoint",
                "event",
                "request",
            ],
            &[
                "symbol",
                "operation",
                "httpEndpoint",
                "graphqlOperation",
                "service",
                "queryImplementation",
            ],
        ),
        RelationKind::Renders => (
            &["route", "component", "file"],
            &["title", "component", "comment", "documentType"],
        ),
        RelationKind::Handles => (
            &[
                "httpEndpoint",
                "graphqlOperation",
                "event",
                "entrypoint",
                "route",
            ],
            &["symbol", "operation", "component"],
        ),
        RelationKind::Triggers => (
            &["event", "operation", "request"],
            &[
                "symbol",
                "operation",
                "httpEndpoint",
                "graphqlOperation",
                "workflowState",
            ],
        ),
        RelationKind::Wraps => (
            &["symbol", "component", "request"],
            &[
                "symbol",
                "component",
                "operation",
                "httpEndpoint",
                "graphqlOperation",
            ],
        ),
        RelationKind::Requests => (
            &["component", "symbol", "operation", "event"],
            &["httpEndpoint", "graphqlOperation", "request", "service"],
        ),
        RelationKind::Serves => (&["service"], &["httpEndpoint", "graphqlOperation"]),
        RelationKind::Resolves => (
            &["graphqlOperation", "graphqlField"],
            &["symbol", "operation"],
        ),
        RelationKind::Reads => (
            &["symbol", "operation", "queryImplementation"],
            &["dataTable", "dataColumn", "dataKey", "dataSource"],
        ),
        RelationKind::Writes => (
            &["symbol", "operation", "queryImplementation"],
            &["dataTable", "dataColumn", "dataKey", "dataSource"],
        ),
        RelationKind::MapsTo => (
            &["dataColumn", "dataKey", "dataTable", "symbol", "operation"],
            &["dataColumn", "dataKey", "dataTable", "symbol", "operation"],
        ),
        RelationKind::Joins => (
            &["queryImplementation", "operation", "symbol"],
            &["dataTable", "dataColumn", "dataKey"],
        ),
        RelationKind::TransitionsTo => (&["workflowState", "operation"], &["workflowState"]),
        RelationKind::Precedes => (
            &["operation", "workflowState", "documentType"],
            &["operation", "workflowState", "documentType"],
        ),
        RelationKind::RelatesDocument => (
            &[
                "operation",
                "workflowState",
                "documentType",
                "queryImplementation",
            ],
            &["documentType"],
        ),
        RelationKind::Describes => (
            &["comment", "title", "documentType"],
            &[
                "symbol",
                "component",
                "operation",
                "route",
                "httpEndpoint",
                "dataTable",
                "workflowState",
                "documentType",
            ],
        ),
    }
}

/// 便于把 `Attributes` 与强类型业务载荷互转。
pub fn attributes_from<T: Serialize>(value: &T) -> Result<Attributes, serde_json::Error> {
    serde_json::to_value(value).map(|value| match value {
        Value::Object(map) => map,
        _ => Map::new(),
    })
}

/// 便于把 `Attributes` 转回强类型业务载荷。
pub fn attributes_into<T: for<'de> Deserialize<'de>>(
    attributes: &Attributes,
) -> Result<T, serde_json::Error> {
    serde_json::from_value(Value::Object(attributes.clone()))
}
