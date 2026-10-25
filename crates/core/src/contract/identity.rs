//! 稳定身份与规范 JSON（计划 §3.3）。
//!
//! 身份是 Core 的唯一真相：实体、关系、来源与证据的 ID 都由纯函数从规范身份
//! 计算得到，插件可以自行计算，但 Core 必须重算并拒绝不一致的 ID。
//!
//! 规范身份是**固定顺序的 JSON 数组**：
//!
//! - 数组元素只允许字符串、整数、数组与（递归排序键的）对象；
//! - 不允许浮点数，不允许多余空白；
//! - 必填槽位直接编码其值；可选槽位统一编码为 `[]`（缺省）或 `[value]`（存在），
//!   因此“缺省”与“空字符串”永不混淆；
//! - 字符串原样保留，不做隐式小写或 Unicode 归一化。
//!
//! ID 形式为 `<前缀> + sha256(UTF8(canonicalIdentity))`，SHA-256 使用完整
//! 64 位小写十六进制。

use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::contract::entity::EntityKind;
use crate::contract::relation::RelationKind;

pub const ID_PREFIX_ENTITY: &str = "ent_";
pub const ID_PREFIX_RELATION: &str = "rel_";
pub const ID_PREFIX_EVIDENCE: &str = "ev_";
pub const ID_PREFIX_SOURCE: &str = "src_";

/// 身份值的规范取值。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum IdentityValue {
    Text(String),
    Number(i64),
    List(Vec<IdentityValue>),
}

impl IdentityValue {
    pub fn text(value: impl Into<String>) -> Self {
        IdentityValue::Text(value.into())
    }

    pub fn number(value: i64) -> Self {
        IdentityValue::Number(value)
    }

    pub fn list(values: Vec<IdentityValue>) -> Self {
        IdentityValue::List(values)
    }

    /// 文本形式；非文本值返回 `None`。
    pub fn as_text(&self) -> Option<&str> {
        match self {
            IdentityValue::Text(value) => Some(value.as_str()),
            _ => None,
        }
    }

    fn write_canonical(&self, out: &mut String) {
        match self {
            IdentityValue::Text(value) => write_json_string(value, out),
            IdentityValue::Number(value) => out.push_str(&value.to_string()),
            IdentityValue::List(values) => {
                out.push('[');
                for (index, value) in values.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    value.write_canonical(out);
                }
                out.push(']');
            }
        }
    }
}

/// 一个身份槽位。`value` 为 `None` 表示该槽位缺省（只允许可选槽位）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IdentitySlot {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<IdentityValue>,
}

impl IdentitySlot {
    /// 必填槽位。
    pub fn required(name: &str, value: impl Into<String>) -> Self {
        IdentitySlot {
            name: name.to_string(),
            value: Some(IdentityValue::text(value)),
        }
    }

    /// 可选槽位；`None` 表示缺省。
    pub fn optional(name: &str, value: Option<impl Into<String>>) -> Self {
        IdentitySlot {
            name: name.to_string(),
            value: value.map(IdentityValue::text),
        }
    }

    /// 必填槽位，值必须是文本。
    pub fn required_text(&self) -> Result<&str, IdentityError> {
        match self.value.as_ref() {
            Some(IdentityValue::Text(value)) => Ok(value.as_str()),
            _ => Err(IdentityError::SlotValue {
                name: self.name.clone(),
                reason: "required slot must carry a text value".to_string(),
            }),
        }
    }

    /// 可选槽位的文本值；缺省返回 `Ok(None)`。
    pub fn optional_text(&self) -> Result<Option<&str>, IdentityError> {
        match self.value.as_ref() {
            None => Ok(None),
            Some(IdentityValue::Text(value)) => Ok(Some(value.as_str())),
            Some(_) => Err(IdentityError::SlotValue {
                name: self.name.clone(),
                reason: "optional slot must carry a text value when present".to_string(),
            }),
        }
    }
}

/// 身份计算失败。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityError {
    /// 槽位集合与类型注册表不一致。
    SlotsMismatch {
        kind: String,
        expected: Vec<String>,
        actual: Vec<String>,
    },
    /// 槽位取值不符合要求。
    SlotValue { name: String, reason: String },
    /// 规范 JSON 无法表示（例如出现浮点数）。
    NotCanonical(String),
}

impl fmt::Display for IdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IdentityError::SlotsMismatch {
                kind,
                expected,
                actual,
            } => write!(
                formatter,
                "identity slots for {kind} must be {expected:?}, got {actual:?}"
            ),
            IdentityError::SlotValue { name, reason } => {
                write!(formatter, "identity slot `{name}` invalid: {reason}")
            }
            IdentityError::NotCanonical(reason) => {
                write!(formatter, "value is not canonicalizable: {reason}")
            }
        }
    }
}

impl std::error::Error for IdentityError {}

/// 身份槽位规格：`required = true` 表示必填。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComponentSpec {
    pub name: &'static str,
    pub required: bool,
}

const fn required(name: &'static str) -> ComponentSpec {
    ComponentSpec {
        name,
        required: true,
    }
}

const fn optional(name: &'static str) -> ComponentSpec {
    ComponentSpec {
        name,
        required: false,
    }
}

/// `file` 的身份成分。
static FILE_COMPONENTS: &[ComponentSpec] = &[
    required("repositoryId"),
    required("path"),
    required("scopeId"),
];

/// `symbol` 的身份成分：文件定位 + 词法路径 + 符号种类 + 规范化签名。
static SYMBOL_COMPONENTS: &[ComponentSpec] = &[
    required("repositoryId"),
    required("path"),
    required("scopeId"),
    required("lexicalPath"),
    required("symbolKind"),
    required("normalizedSignature"),
];

/// `component` 的身份成分。
static COMPONENT_COMPONENTS: &[ComponentSpec] = &[
    required("repositoryId"),
    required("path"),
    required("scopeId"),
    required("lexicalPath"),
];

/// `operation` 的身份成分。
static OPERATION_COMPONENTS: &[ComponentSpec] = &[
    required("repositoryId"),
    required("path"),
    required("scopeId"),
    required("lexicalPath"),
    required("normalizedSignature"),
];

/// `route` 的身份成分：应用作用域 + 路由器 + 模式 + 原始声明路径。
static ROUTE_COMPONENTS: &[ComponentSpec] = &[
    required("scopeId"),
    required("routerId"),
    required("mode"),
    required("declaredPath"),
];

/// `httpEndpoint` 的身份成分。
static HTTP_ENDPOINT_COMPONENTS: &[ComponentSpec] = &[
    required("serviceId"),
    required("method"),
    required("registeredPath"),
];

/// `graphqlOperation` 的身份成分。
static GRAPHQL_OPERATION_COMPONENTS: &[ComponentSpec] = &[
    required("serviceId"),
    required("operationType"),
    required("operationName"),
];

/// `graphqlField` 的身份成分。
static GRAPHQL_FIELD_COMPONENTS: &[ComponentSpec] = &[
    required("serviceId"),
    required("schemaType"),
    required("fieldName"),
];

/// `dataTable` 的身份成分。`catalog` 与 `schema` 可缺省（例如 MySQL 没有 catalog）。
static DATA_TABLE_COMPONENTS: &[ComponentSpec] = &[
    required("dataSourceId"),
    optional("catalog"),
    optional("schema"),
    required("table"),
];

/// `dataColumn` 的身份成分。
static DATA_COLUMN_COMPONENTS: &[ComponentSpec] = &[
    required("dataSourceId"),
    optional("catalog"),
    optional("schema"),
    required("table"),
    required("column"),
];

/// `dataKey` 的身份成分：有序 keyColumns。
static DATA_KEY_COMPONENTS: &[ComponentSpec] = &[
    required("dataSourceId"),
    optional("catalog"),
    optional("schema"),
    required("table"),
    required("keyColumns"),
];

/// `module` / `entrypoint` / `documentType` / `queryImplementation` / `reportField` 的身份成分。
static SCOPE_KEY_COMPONENTS: &[ComponentSpec] = &[required("scopeId"), required("semanticKey")];

/// `workflowState` 的身份成分：额外含 workflowId。
static WORKFLOW_STATE_COMPONENTS: &[ComponentSpec] = &[
    required("scopeId"),
    required("workflowId"),
    required("semanticKey"),
];

/// `service` / `dataSource` 的身份成分。
static CONFIGURED_COMPONENTS: &[ComponentSpec] = &[required("scopeId"), required("configuredId")];

/// `event` / `title` / `comment` 的身份成分。
static LOCATED_COMPONENTS: &[ComponentSpec] = &[
    required("repositoryId"),
    required("path"),
    required("scopeId"),
    required("lexicalPath"),
];

/// `request` 的身份成分。
static REQUEST_COMPONENTS: &[ComponentSpec] = &[required("scopeId"), required("requestKey")];

/// 返回指定实体类型的身份槽位规格。
pub fn entity_components(kind: EntityKind) -> &'static [ComponentSpec] {
    match kind {
        EntityKind::File => FILE_COMPONENTS,
        EntityKind::Symbol => SYMBOL_COMPONENTS,
        EntityKind::Component => COMPONENT_COMPONENTS,
        EntityKind::Operation => OPERATION_COMPONENTS,
        EntityKind::Route => ROUTE_COMPONENTS,
        EntityKind::HttpEndpoint => HTTP_ENDPOINT_COMPONENTS,
        EntityKind::GraphqlOperation => GRAPHQL_OPERATION_COMPONENTS,
        EntityKind::GraphqlField => GRAPHQL_FIELD_COMPONENTS,
        EntityKind::DataTable => DATA_TABLE_COMPONENTS,
        EntityKind::DataColumn => DATA_COLUMN_COMPONENTS,
        EntityKind::DataKey => DATA_KEY_COMPONENTS,
        EntityKind::Module
        | EntityKind::Entrypoint
        | EntityKind::DocumentType
        | EntityKind::QueryImplementation
        | EntityKind::ReportField => SCOPE_KEY_COMPONENTS,
        EntityKind::WorkflowState => WORKFLOW_STATE_COMPONENTS,
        EntityKind::Service | EntityKind::DataSource => CONFIGURED_COMPONENTS,
        EntityKind::Event | EntityKind::Title | EntityKind::Comment => LOCATED_COMPONENTS,
        EntityKind::Request => REQUEST_COMPONENTS,
    }
}

/// 返回指定实体类型的全部槽位名（按固定顺序）。
pub fn entity_component_names(kind: EntityKind) -> Vec<&'static str> {
    entity_components(kind)
        .iter()
        .map(|spec| spec.name)
        .collect()
}

/// 把槽位编码进规范身份数组：必填直接取值，可选编码为 `[]` / `[value]`。
fn encode_slots(
    specs: &[ComponentSpec],
    slots: &[IdentitySlot],
) -> Result<Vec<IdentityValue>, IdentityError> {
    let expected: Vec<String> = specs.iter().map(|spec| spec.name.to_string()).collect();
    let actual: Vec<String> = slots.iter().map(|slot| slot.name.clone()).collect();
    if expected != actual {
        return Err(IdentityError::SlotsMismatch {
            kind: expected.join(","),
            expected,
            actual,
        });
    }

    let mut encoded = Vec::with_capacity(specs.len());
    for (spec, slot) in specs.iter().zip(slots.iter()) {
        match (spec.required, slot.value.as_ref()) {
            (true, Some(value)) => encoded.push(value.clone()),
            (true, None) => {
                return Err(IdentityError::SlotValue {
                    name: spec.name.to_string(),
                    reason: "required slot is missing".to_string(),
                });
            }
            (false, None) => encoded.push(IdentityValue::List(Vec::new())),
            (false, Some(value)) => encoded.push(IdentityValue::List(vec![value.clone()])),
        }
    }
    Ok(encoded)
}

/// 规范 JSON 序列化：字符串、整数、数组；对象键递归排序；禁止浮点数。
pub fn canonical_json(value: &serde_json::Value) -> Result<String, IdentityError> {
    let mut out = String::new();
    write_canonical_json(value, &mut out, false)?;
    Ok(out)
}

/// 规范 JSON 序列化（配置哈希用）：额外允许布尔与 null。
///
/// 身份内部禁止布尔与 null，但项目配置里存在布尔开关；两者必须分开，避免
/// 身份规范被配置内容污染。
pub fn canonical_config_json(value: &serde_json::Value) -> Result<String, IdentityError> {
    let mut out = String::new();
    write_canonical_json(value, &mut out, true)?;
    Ok(out)
}

fn write_canonical_json(
    value: &serde_json::Value,
    out: &mut String,
    allow_bool: bool,
) -> Result<(), IdentityError> {
    match value {
        serde_json::Value::Null => {
            if !allow_bool {
                return Err(IdentityError::NotCanonical(
                    "null is not allowed inside an identity".to_string(),
                ));
            }
            out.push_str("null");
        }
        serde_json::Value::Bool(flag) => {
            if !allow_bool {
                return Err(IdentityError::NotCanonical(format!(
                    "boolean {flag} is not allowed inside an identity"
                )));
            }
            out.push_str(if *flag { "true" } else { "false" });
        }
        serde_json::Value::Number(number) => {
            if let Some(value) = number.as_i64() {
                out.push_str(&value.to_string());
            } else if let Some(value) = number.as_u64() {
                out.push_str(&value.to_string());
            } else {
                return Err(IdentityError::NotCanonical(format!(
                    "non-integer number {number} is not allowed inside an identity"
                )));
            }
        }
        serde_json::Value::String(text) => write_json_string(text, out),
        serde_json::Value::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_canonical_json(item, out, allow_bool)?;
            }
            out.push(']');
        }
        serde_json::Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            out.push('{');
            for (index, key) in keys.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_json_string(key, out);
                out.push(':');
                let item = map.get(*key).expect("key comes from the same map");
                write_canonical_json(item, out, allow_bool)?;
            }
            out.push('}');
        }
    }
    Ok(())
}

/// JSON 字符串转义（与 serde_json 默认行为一致：非 ASCII 原样保留）。
fn write_json_string(text: &str, out: &mut String) {
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// 计算稳定 ID：`<前缀> + sha256(canonical)`，64 位小写十六进制。
pub fn compute_id(prefix: &str, canonical: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    let digest = hasher.finalize();
    format!("{prefix}{}", hex::encode(digest))
}

/// 实体 ID。规范身份为 `[projectId, kind, ...components]`。
pub fn compute_entity_id(
    project_id: &str,
    kind: EntityKind,
    slots: &[IdentitySlot],
) -> Result<String, IdentityError> {
    let canonical = canonical_entity_identity(project_id, kind, slots)?;
    Ok(compute_id(ID_PREFIX_ENTITY, &canonical))
}

/// 实体规范身份 JSON：`[projectId, kind, ...components]`。
///
/// 该字符串在库内唯一（`entities.identity_json`），因此不同实体不会共享身份。
pub fn canonical_entity_identity(
    project_id: &str,
    kind: EntityKind,
    slots: &[IdentitySlot],
) -> Result<String, IdentityError> {
    let specs = entity_components(kind);
    let mut parts = vec![
        IdentityValue::text(project_id.to_string()),
        IdentityValue::text(kind.as_str().to_string()),
    ];
    parts.extend(encode_slots(specs, slots)?);
    let mut canonical = String::new();
    canonical.push('[');
    for (index, part) in parts.iter().enumerate() {
        if index > 0 {
            canonical.push(',');
        }
        part.write_canonical(&mut canonical);
    }
    canonical.push(']');
    Ok(canonical)
}

/// 从规范身份 JSON 还原槽位（跳过 projectId 与 kind）。
pub fn identity_slots_from_canonical(
    kind: EntityKind,
    canonical: &str,
) -> Result<Vec<IdentitySlot>, IdentityError> {
    let value: serde_json::Value = serde_json::from_str(canonical).map_err(|error| {
        IdentityError::NotCanonical(format!("canonical identity is not valid JSON: {error}"))
    })?;
    let items = value.as_array().ok_or_else(|| {
        IdentityError::NotCanonical("canonical identity must be a JSON array".to_string())
    })?;
    if items.len() < 2 {
        return Err(IdentityError::NotCanonical(
            "canonical identity must contain projectId and kind".to_string(),
        ));
    }
    let specs = entity_components(kind);
    let mut slots = Vec::with_capacity(specs.len());
    for (spec, item) in specs.iter().zip(items[2..].iter()) {
        let (present, value) = if spec.required {
            (true, Some(item.clone()))
        } else {
            match item.as_array() {
                Some(entries) if entries.is_empty() => (false, None),
                Some(entries) => (true, Some(entries[0].clone())),
                None => (false, None),
            }
        };
        let _ = present;
        let value = match value {
            None => None,
            Some(serde_json::Value::String(text)) => Some(IdentityValue::Text(text)),
            Some(serde_json::Value::Number(number)) => {
                Some(IdentityValue::Number(number.as_i64().ok_or_else(|| {
                    IdentityError::NotCanonical("identity number must be an integer".to_string())
                })?))
            }
            Some(serde_json::Value::Array(entries)) => {
                let mut list = Vec::with_capacity(entries.len());
                for entry in entries {
                    list.push(match entry {
                        serde_json::Value::String(text) => IdentityValue::Text(text.clone()),
                        serde_json::Value::Number(number) => {
                            IdentityValue::Number(number.as_i64().ok_or_else(|| {
                                IdentityError::NotCanonical(
                                    "identity number must be an integer".to_string(),
                                )
                            })?)
                        }
                        _ => {
                            return Err(IdentityError::NotCanonical(
                                "identity list entries must be strings or integers".to_string(),
                            ));
                        }
                    });
                }
                Some(IdentityValue::List(list))
            }
            _ => {
                return Err(IdentityError::NotCanonical(
                    "identity values must be strings, integers or arrays".to_string(),
                ));
            }
        };
        slots.push(IdentitySlot {
            name: spec.name.to_string(),
            value,
        });
    }
    Ok(slots)
}

/// 关系 ID。规范身份为 `[projectId, kind, fromId, toId, ...qualifiers]`。
///
/// `qualifiers` 按键递归排序后规范化；证据位置与生产者不进入关系 ID。
pub fn compute_relation_id(
    project_id: &str,
    kind: RelationKind,
    from_id: &str,
    to_id: &str,
    qualifiers: &serde_json::Map<String, serde_json::Value>,
) -> Result<String, IdentityError> {
    let qualifier_value = serde_json::Value::Object(qualifiers.clone());
    let qualifier_canonical = canonical_json(&qualifier_value)?;
    let mut canonical = String::new();
    canonical.push('[');
    canonical.push_str(&json_literal(project_id));
    canonical.push(',');
    canonical.push_str(&json_literal(kind.as_str()));
    canonical.push(',');
    canonical.push_str(&json_literal(from_id));
    canonical.push(',');
    canonical.push_str(&json_literal(to_id));
    canonical.push(',');
    canonical.push_str(&qualifier_canonical);
    canonical.push(']');
    Ok(compute_id(ID_PREFIX_RELATION, &canonical))
}

/// 来源 ID。规范身份为 `[projectId, repositoryId, path]`。
pub fn compute_source_id(project_id: &str, repository_id: &str, path: &str) -> String {
    let canonical = format!(
        "[{},{},{}]",
        json_literal(project_id),
        json_literal(repository_id),
        json_literal(path)
    );
    compute_id(ID_PREFIX_SOURCE, &canonical)
}

/// 证据 ID。规范身份为 `[sourceId, contentHash, startByte, endByte, role]`。
pub fn compute_evidence_id(
    source_id: &str,
    content_hash: &str,
    start_byte: u64,
    end_byte: u64,
    role: &str,
) -> String {
    let canonical = format!(
        "[{},{},{},{},{}]",
        json_literal(source_id),
        json_literal(content_hash),
        start_byte,
        end_byte,
        json_literal(role)
    );
    compute_id(ID_PREFIX_EVIDENCE, &canonical)
}

fn json_literal(text: &str) -> String {
    let mut out = String::new();
    write_json_string(text, &mut out);
    out
}

/// 计算内容哈希（SHA-256，小写十六进制）。
pub fn content_hash(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}
