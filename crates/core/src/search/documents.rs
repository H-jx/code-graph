//! 标题、路由、别名与源码的搜索投影（计划 §3.7、D10）。
//!
//! 投影分两类：
//! - `entity`：实体名称、种类、路径、签名与业务属性；
//! - `source`：被选中的原始文本（SQL、注释、Markdown、无 parser 语言同样可检索）。
//!
//! 字段源保留断言与 evidence 引用：投影行绑定 `assertion_id`，失效批次不参与
//! 当前结果。

use crate::search::tokenize::token_string;

/// 投影种类。
pub const PROJECTION_ENTITY: &str = "entity";
pub const PROJECTION_SOURCE: &str = "source";

/// 一行搜索投影。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SearchDocument {
    pub assertion_id: String,
    pub entity_id: String,
    pub scope_id: String,
    pub projection_kind: String,
    pub route_exact: Option<String>,
    pub text: String,
    pub tokens: String,
}

impl SearchDocument {
    fn new(
        assertion_id: String,
        entity_id: String,
        scope_id: String,
        projection_kind: &str,
        route_exact: Option<String>,
        text: String,
    ) -> Self {
        let tokens = token_string(&text);
        SearchDocument {
            assertion_id,
            entity_id,
            scope_id,
            projection_kind: projection_kind.to_string(),
            route_exact,
            text,
            tokens,
        }
    }
}

/// 实体投影。
#[allow(clippy::too_many_arguments)]
pub fn entity_projection(
    assertion_id: &str,
    entity_id: &str,
    scope_id: &str,
    name: &str,
    kind: &str,
    path: &str,
    signature: &str,
    attributes: &serde_json::Value,
    route_exact: Option<String>,
) -> SearchDocument {
    let mut text = String::new();
    text.push_str(name);
    text.push('\n');
    text.push_str(kind);
    if !path.is_empty() {
        text.push('\n');
        text.push_str(path);
    }
    if !signature.is_empty() {
        text.push('\n');
        text.push_str(signature);
    }
    append_attribute_text(attributes, &mut text);
    SearchDocument::new(
        assertion_id.to_string(),
        entity_id.to_string(),
        scope_id.to_string(),
        PROJECTION_ENTITY,
        route_exact,
        text,
    )
}

// Semantic statements, i18n metadata and lineage live in nested attributes too.
// Index their original string values; do not invent a flattened business assertion.
fn append_attribute_text(value: &serde_json::Value, text: &mut String) {
    match value {
        serde_json::Value::String(value) => {
            text.push('\n');
            text.push_str(value);
        }
        serde_json::Value::Array(values) => {
            for value in values {
                append_attribute_text(value, text);
            }
        }
        serde_json::Value::Object(values) => {
            for (key, value) in values {
                text.push('\n');
                text.push_str(key);
                append_attribute_text(value, text);
            }
        }
        _ => (),
    }
}

/// 来源投影：整份被选中的原始文本。
pub fn source_projection(
    assertion_id: &str,
    entity_id: &str,
    scope_id: &str,
    path: &str,
    content: &str,
) -> SearchDocument {
    let mut text = String::new();
    text.push_str(path);
    text.push('\n');
    text.push_str(content);
    SearchDocument::new(
        assertion_id.to_string(),
        entity_id.to_string(),
        scope_id.to_string(),
        PROJECTION_SOURCE,
        None,
        text,
    )
}

/// 关系投影：关系两端与限定条件的可检索文本。
pub fn relation_projection(
    assertion_id: &str,
    entity_id: &str,
    scope_id: &str,
    kind: &str,
    from_id: &str,
    to_id: &str,
    qualifiers: &serde_json::Map<String, serde_json::Value>,
) -> SearchDocument {
    let mut text = String::new();
    text.push_str(kind);
    text.push('\n');
    text.push_str(from_id);
    text.push('\n');
    text.push_str(to_id);
    for (key, value) in qualifiers.iter() {
        if let Some(text_value) = value.as_str() {
            text.push('\n');
            text.push_str(key);
            text.push('=');
            text.push_str(text_value);
        }
    }
    SearchDocument::new(
        assertion_id.to_string(),
        entity_id.to_string(),
        scope_id.to_string(),
        PROJECTION_ENTITY,
        None,
        text,
    )
}
