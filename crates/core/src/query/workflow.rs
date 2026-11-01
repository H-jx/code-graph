//! 查询已有流程、状态、操作及上下游（计划 §3.7、D11）。
//!
//! `calls` 不产生 `transitions_to`；流程需要状态、条件或原始流程资料。没有流程
//! 依据时返回 `WORKFLOW_EVIDENCE_MISSING` 缺口，不宣称无后续业务流程。

use crate::contract::{
    entity::EntityKind,
    error::{CoreError, gap_codes},
    query::*,
    relation::RelationKind,
};
use crate::query::{QueryContext, runtime::Runtime, scenario};
use serde_json::{Value, json};

pub fn execute(
    context: &mut QueryContext<'_>,
    request: &WorkflowRequest,
    id: String,
    rt: &mut Runtime,
) -> Result<QueryResponse<Value>, CoreError> {
    let filters = scenario::roots_filters(
        request.filters.as_ref(),
        request.scope_ids.as_ref(),
        Some(vec![
            EntityKind::WorkflowState,
            EntityKind::Operation,
            EntityKind::DocumentType,
        ]),
    );
    let (roots, truncated) =
        scenario::select(context, &request.query, request.roots.as_ref(), filters, rt)?;
    let trace = TraceRequest {
        roots: roots.clone(),
        direction: Direction::Both,
        relation_kinds: Some(vec![
            RelationKind::TransitionsTo,
            RelationKind::Precedes,
            RelationKind::RelatesDocument,
            RelationKind::Triggers,
            RelationKind::Calls,
        ]),
        max_depth: None,
        filters: request.filters.clone(),
        budget: request.budget.clone(),
    };
    let mut walk = crate::graph::paths::walk(context, &trace, rt)?;
    walk.truncated |= truncated;
    let transitions = walk
        .hops
        .iter()
        .filter(|h| h["kind"] == "transitions_to")
        .cloned()
        .collect::<Vec<_>>();
    let mut gaps = Vec::new();
    if transitions.is_empty() {
        gaps.push(scenario::gap(
            gap_codes::WORKFLOW_EVIDENCE_MISSING,
            "workflow",
            "no original business transition was found; calls do not establish workflow successors",
            Vec::new(),
        ));
    }
    let data = json!({"query":request.query,"operations":scenario::ids(&walk.entities,&["operation"]),"states":scenario::ids(&walk.entities,&["workflowState"]),"transitions":transitions,"documents":scenario::ids(&walk.entities,&["documentType"]),"previousPages":walk.hops.iter().filter(|h|h["kind"]=="precedes").map(|h|json!({"pageId":h["fromId"],"relationRef":h["relationRef"]})).collect::<Vec<_>>(),"nextPages":walk.hops.iter().filter(|h|h["kind"]=="precedes").map(|h|json!({"pageId":h["toId"],"relationRef":h["relationRef"]})).collect::<Vec<_>>(),"documentLinks":walk.hops.iter().filter(|h|h["kind"]=="relates_document").cloned().collect::<Vec<_>>(),"codeCalls":walk.hops.iter().filter(|h|h["kind"]=="calls").cloned().collect::<Vec<_>>()});
    scenario::finish(
        context,
        id,
        roots,
        walk,
        data,
        &["workflow"],
        request.filters.as_ref(),
        gaps,
        rt,
    )
}
