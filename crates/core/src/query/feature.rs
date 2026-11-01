//! 从需求定位入口、业务方法和关联代码（计划 §3.7）。
//!
//! 事实组合由 Core 完成；自然语言解释由调用方 LLM 负责。查询不生成新的事实。

use crate::contract::{error::CoreError, query::*};
use crate::query::{QueryContext, runtime::Runtime, scenario};
use serde_json::{Value, json};

pub fn execute(
    context: &mut QueryContext<'_>,
    request: &LocateFeatureRequest,
    id: String,
    rt: &mut Runtime,
) -> Result<QueryResponse<Value>, CoreError> {
    let root_filter = request.root_filter.as_ref();
    let filters = scenario::roots_filters(
        request.filters.as_ref(),
        root_filter.and_then(|r| r.scope_ids.as_ref()),
        root_filter.and_then(|r| r.kinds.clone()),
    );
    let (mut roots, mut truncated, mut extra_gaps) = if let Some(route) = &request.route_hint {
        let (roots, gaps, truncated) = super::page::select_routes(context, route, &filters, rt)?;
        (roots, truncated, gaps)
    } else {
        let (roots, truncated) =
            scenario::select(context, &request.query, None, filters.clone(), rt)?;
        (roots, truncated, Vec::new())
    };
    let mut matched_terms = Vec::new();
    if roots.is_empty() && request.route_hint.is_none() && !truncated {
        for term in crate::search::tokenize::tokenize(&request.query)
            .into_iter()
            .filter(|term| term.chars().count() >= 2)
            .take(8)
        {
            if rt.stopped() {
                truncated = true;
                break;
            }
            let (found, incomplete) = scenario::select(context, &term, None, filters.clone(), rt)?;
            truncated |= incomplete;
            if !found.is_empty() {
                matched_terms.push(term);
                roots.extend(found);
            }
        }
        roots.sort_by(|a, b| a.entity_id.cmp(&b.entity_id));
        roots.dedup_by(|a, b| a.entity_id == b.entity_id);
        if !roots.is_empty() {
            extra_gaps.push(scenario::gap("PARTIAL_QUERY_MATCH","search","entries matched explicit lexical terms; the full request was not interpreted as a new business fact",roots.iter().map(|r|r.entity_id.clone()).collect()));
        }
    }
    let mut traversal_filters = request.filters.clone().unwrap_or_default();
    if let Some(kinds) = request
        .traversal
        .as_ref()
        .and_then(|t| t.filter.as_ref())
        .and_then(|f| f.kinds.clone())
    {
        traversal_filters.kinds = Some(kinds);
    }
    let trace = TraceRequest {
        roots: roots.clone(),
        direction: request
            .traversal
            .as_ref()
            .map_or(Direction::Both, |t| t.direction),
        relation_kinds: request
            .traversal
            .as_ref()
            .and_then(|t| t.filter.as_ref())
            .and_then(|f| f.relation_kinds.clone()),
        max_depth: request.traversal.as_ref().and_then(|t| t.max_depth),
        filters: Some(traversal_filters),
        budget: request.budget.clone(),
    };
    let mut walk = crate::graph::paths::walk_with_root_filters(
        context,
        &trace,
        rt,
        &super::Filters::from_request(request.filters.as_ref()),
    )?;
    walk.truncated |= truncated;
    let data = json!({"query":request.query,"matchedTerms":matched_terms,"view":request.view,"paths":walk.hops.iter().map(|h|h["relationRef"].clone()).collect::<Vec<_>>(),"relatedCode":scenario::ids(&walk.entities,&["symbol","operation","component","module","entrypoint"]),"data":scenario::ids(&walk.entities,&["queryImplementation","dataSource","dataTable","dataColumn","reportField"])});
    scenario::finish(
        context,
        id,
        roots,
        walk,
        data,
        &["frontend", "api", "database", "link"],
        request.filters.as_ref(),
        extra_gaps,
        rt,
    )
}

/// 便捷函数：只返回状态。
pub fn is_partial(response: &QueryResponse<Value>) -> bool {
    response.status == ResponseStatus::Partial
}
