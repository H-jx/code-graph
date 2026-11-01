//! 只验证 napi ABI 与 Core 同源契约，不提供产品的平行实现。
use napi_derive::napi;
use project_knowledge_core::contract::{CollectorOutput, codes};

#[napi]
pub fn collector_schema() -> String {
    serde_json::to_string(&schemars::schema_for!(CollectorOutput)).expect("Core schema")
}

#[napi]
pub fn validate_collector(raw: String) -> String {
    let result = serde_json::from_str::<serde_json::Value>(&raw)
        .map_err(|_| {
            project_knowledge_core::contract::CoreError::new(codes::SCHEMA_INVALID, "invalid JSON")
        })
        .and_then(|value| {
            project_knowledge_core::contract::schema::validate::<CollectorOutput>(&value, "")
        });
    serde_json::to_string(&match result {
        Ok(()) => serde_json::json!({"valid":true}),
        Err(error) => serde_json::json!({"error":error}),
    })
    .expect("validation report")
}
