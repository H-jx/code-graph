//! 查询已有流程、状态、操作及上下游（计划 §3.7、D11）。
//!
//! `calls` 不产生 `transitions_to`；流程需要状态、条件或原始流程资料。没有流程
//! 依据时返回 `WORKFLOW_EVIDENCE_MISSING` 缺口，不宣称无后续业务流程。

use serde_json::{Map, Value};

use crate::contract::error::CoreError;
use crate::contract::query::{QueryResponse, WorkflowRequest};
use crate::query::{QueryContext, scenario_skeleton};

/// 执行流程查询。
pub fn execute(
    context: &mut QueryContext<'_>,
    request: &WorkflowRequest,
    request_id: String,
) -> Result<QueryResponse<Value>, CoreError> {
    let mut extra = Map::new();
    extra.insert("query".to_string(), Value::from(request.query.clone()));
    extra.insert("operations".to_string(), Value::Array(Vec::new()));
    extra.insert("states".to_string(), Value::Array(Vec::new()));
    extra.insert("transitions".to_string(), Value::Array(Vec::new()));
    extra.insert("documents".to_string(), Value::Array(Vec::new()));
    extra.insert("previousPages".to_string(), Value::Array(Vec::new()));
    extra.insert("nextPages".to_string(), Value::Array(Vec::new()));

    let response = scenario_skeleton(
        context,
        &request.query,
        &["workflow", "frontend", "api"],
        extra,
        request_id,
    )?;
    Ok(response)
}
