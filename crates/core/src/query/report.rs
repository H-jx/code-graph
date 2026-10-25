//! 查询已有取数实现、字段血缘和关联口径（计划 §3.7、D11）。
//!
//! report 只解释**已存在**的 SQL/ORM 查询实现：字段来源、关联键、基数、粒度、
//! 聚合与有效状态。未知口径使用 `{status:"unknown",gapIds:[...]}`，不编造 SQL。

use serde_json::{Map, Value};

use crate::contract::error::CoreError;
use crate::contract::query::{QueryResponse, ReportRequest};
use crate::query::{QueryContext, scenario_skeleton};

/// 执行台账查询。
pub fn execute(
    context: &mut QueryContext<'_>,
    request: &ReportRequest,
    request_id: String,
) -> Result<QueryResponse<Value>, CoreError> {
    let mut extra = Map::new();
    extra.insert("query".to_string(), Value::from(request.query.clone()));
    extra.insert("implementations".to_string(), Value::Array(Vec::new()));
    extra.insert("fields".to_string(), Value::Array(Vec::new()));
    extra.insert("joinKeys".to_string(), Value::Array(Vec::new()));
    extra.insert("cardinality".to_string(), Value::Null);
    extra.insert("grain".to_string(), Value::Null);
    extra.insert("aggregates".to_string(), Value::Array(Vec::new()));
    extra.insert("validStates".to_string(), Value::Null);
    extra.insert("unknowns".to_string(), Value::Array(Vec::new()));

    let response = scenario_skeleton(
        context,
        &request.query,
        &["database", "report"],
        extra,
        request_id,
    )?;
    Ok(response)
}
