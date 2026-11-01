//! 使用 Rust DTO 同源的本地 Schema；不接受插件提供的 Schema 或远程引用。
use super::error::{CoreError, Issue, codes};
use schemars::JsonSchema;
use serde_json::Value;

pub fn validate<T: JsonSchema>(value: &Value, prefix: &str) -> Result<(), CoreError> {
    let schema = serde_json::to_value(schemars::schema_for!(T)).expect("generated schema");
    let validator = jsonschema::validator_for(&schema).map_err(|_| {
        CoreError::new(
            codes::INTERNAL_ERROR,
            "Core generated an invalid local schema",
        )
    })?;
    let issues = validator
        .iter_errors(value)
        .map(|error| {
            let mut pointer = format!("{prefix}{}", error.instance_path());
            if let jsonschema::error::ValidationErrorKind::Required { property } = error.kind()
                && let Some(property) = property.as_str()
            {
                pointer.push('/');
                pointer.push_str(&property.replace('~', "~0").replace('/', "~1"));
            }
            // Validator Display includes the entire invalid value. Returning it can
            // leak source/credentials and turn one malformed tool call into huge context.
            let expected = match error.kind() {
                jsonschema::error::ValidationErrorKind::Type { kind } => format!(
                    "JSON type {kind:?}; send the value directly, not a JSON-encoded string"
                ),
                kind => format!("must satisfy schema keyword `{}`", kind.keyword()),
            };
            Issue::new(codes::SCHEMA_INVALID, &pointer)
                .with_expected(expected)
                .with_hint("按 Core 导出的 JSON Schema 修复该字段")
        })
        .collect::<Vec<_>>();
    if issues.is_empty() {
        Ok(())
    } else {
        Err(
            CoreError::new(codes::SCHEMA_INVALID, "JSON does not match the Core schema")
                .with_issues(issues),
        )
    }
}

pub fn document() -> Value {
    use super::*;
    use serde_json::json;
    let mut generator = schemars::SchemaGenerator::default();
    // 先登记所有公共类型，使 `$defs` 覆盖完整契约面。
    for schema in [
        generator.subschema_for::<Entity>(),
        generator.subschema_for::<Relation>(),
        generator.subschema_for::<Evidence>(),
        generator.subschema_for::<Source>(),
        generator.subschema_for::<PluginManifest>(),
        generator.subschema_for::<CollectorInput>(),
        generator.subschema_for::<CollectorOutput>(),
        generator.subschema_for::<QueryRequest>(),
        generator.subschema_for::<QueryResponse<Value>>(),
        generator.subschema_for::<ErrorEnvelope>(),
        generator.subschema_for::<crate::contract::client::MaintenanceRequest>(),
        generator.subschema_for::<crate::contract::client::Capabilities>(),
        generator.subschema_for::<Gap>(),
        generator.subschema_for::<crate::contract::RelationRef>(),
        generator.subschema_for::<crate::contract::QueryImplementationAttributes>(),
        generator.subschema_for::<crate::contract::ReportFieldAttributes>(),
        generator.subschema_for::<crate::contract::JoinQualifiers>(),
        generator.subschema_for::<crate::contract::TransitionQualifiers>(),
        generator.subschema_for::<crate::contract::AcceptanceCase>(),
        generator.subschema_for::<crate::contract::knowledge::TopicContext>(),
        generator.subschema_for::<crate::contract::knowledge::TopicArtifact>(),
        generator.subschema_for::<crate::contract::knowledge::ConventionsImport>(),
    ] {
        let _ = schema;
    }
    let root = generator.into_root_schema_for::<QueryRequest>();
    let mut document = serde_json::to_value(root).expect("Core schema");
    if let Some(object) = document.as_object_mut() {
        object.insert(
            "$schema".to_string(),
            json!("https://json-schema.org/draft/2020-12/schema"),
        );
        object.insert(
            "$id".to_string(),
            json!("https://project-knowledge.invalid/schema/contract.json"),
        );
        object.insert("title".to_string(), json!("Project Knowledge Contract"));
        object.insert(
            "description".to_string(),
            json!("Core、CLI、SDK、MCP 与插件共同遵守的协议；类型由 Rust 契约导出。"),
        );
        object.insert(
            "x-protocolVersion".to_string(),
            json!(crate::contract::PROTOCOL_VERSION),
        );
        object.insert(
            "x-schemaVersion".to_string(),
            json!(crate::contract::CONFIG_SCHEMA_VERSION),
        );
    }
    document
}
