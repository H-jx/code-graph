//! 从需求定位入口、业务方法和关联代码（计划 §3.7）。
//!
//! 事实组合由 Core 完成；自然语言解释由调用方 LLM 负责。查询不生成新的事实。

use serde_json::{Map, Value};

use crate::contract::query::{LocateFeatureRequest, QueryResponse, ResponseStatus};
use crate::query::{QueryContext, scenario_skeleton};

/// 执行需求定位。
pub fn execute(
    context: &mut QueryContext<'_>,
    request: &LocateFeatureRequest,
    request_id: String,
) -> Result<QueryResponse<Value>, crate::contract::error::CoreError> {
    let mut extra = Map::new();
    extra.insert("query".to_string(), Value::from(request.query.clone()));
    if let Some(route_hint) = &request.route_hint {
        extra.insert("routeHint".to_string(), Value::from(route_hint.clone()));
    }
    if let Some(root_filter) = &request.root_filter {
        extra.insert(
            "rootFilter".to_string(),
            serde_json::to_value(root_filter).unwrap_or(Value::Null),
        );
    }
    if let Some(traversal) = &request.traversal {
        extra.insert(
            "traversal".to_string(),
            serde_json::to_value(traversal).unwrap_or(Value::Null),
        );
    }
    extra.insert("roots".to_string(), Value::Array(Vec::new()));
    extra.insert("paths".to_string(), Value::Array(Vec::new()));
    extra.insert("relatedCode".to_string(), Value::Array(Vec::new()));
    extra.insert("data".to_string(), Value::Array(Vec::new()));

    let response = scenario_skeleton(
        context,
        &request.query,
        &["routes", "frontend", "api", "database"],
        extra,
        request_id,
    )?;
    Ok(response)
}

/// 便捷函数：只返回状态。
pub fn is_partial(response: &QueryResponse<Value>) -> bool {
    response.status == ResponseStatus::Partial
}
