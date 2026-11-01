//! 查询已有取数实现、字段血缘和关联口径（计划 §3.7、D11）。
//!
//! report 只解释**已存在**的 SQL/ORM 查询实现：字段来源、关联键、基数、粒度、
//! 聚合与有效状态。未知口径使用 `{status:"unknown",gapIds:[...]}`，不编造 SQL。

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
    request: &ReportRequest,
    id: String,
    rt: &mut Runtime,
) -> Result<QueryResponse<Value>, CoreError> {
    let filters = scenario::roots_filters(
        request.filters.as_ref(),
        request.scope_ids.as_ref(),
        Some(vec![EntityKind::QueryImplementation]),
    );
    let (roots, truncated) =
        scenario::select(context, &request.query, request.roots.as_ref(), filters, rt)?;
    let trace = TraceRequest {
        roots: roots.clone(),
        direction: Direction::Out,
        relation_kinds: Some(vec![RelationKind::References]),
        max_depth: None,
        filters: request.filters.clone(),
        budget: request.budget.clone(),
    };
    let mut walk = crate::graph::paths::walk(context, &trace, rt)?;
    walk.truncated |= truncated;
    let mut implementations = Vec::new();
    let mut unknowns = Vec::new();
    let mut gaps = Vec::new();
    for entity in walk
        .entities
        .iter()
        .filter(|e| e["kind"] == "queryImplementation")
    {
        let mut policies = Vec::new();
        let mut grain_proven = false;
        for assertion in entity["assertions"].as_array().unwrap() {
            let attrs = &assertion["attributes"];
            let mut policy = json!({"assertionId":assertion["assertionId"],"basis":assertion["basis"],"origin":assertion["origin"],"evidenceRefs":assertion["evidenceRefs"]});
            for field in [
                "joins",
                "grainColumnIds",
                "aggregates",
                "validStates",
                "timeFields",
                "nullHandling",
                "units",
            ] {
                let value = if let Some(value) = attrs.get(field) {
                    value.clone()
                } else {
                    let gap = scenario::gap(
                        gap_codes::DEFINITION_MISSING,
                        "report",
                        &format!("implementation assertion does not provide {field}"),
                        vec![entity["entityRef"]["entityId"].as_str().unwrap().into()],
                    );
                    let value = json!({"status":"unknown","gapIds":[gap.id]});
                    gaps.push(gap);
                    value
                };
                if value["status"] == "unknown" {
                    unknowns.push(json!({"entityRef":entity["entityRef"],"assertionId":assertion["assertionId"],"field":field,"value":value}));
                }
                policy[field] = value;
            }
            grain_proven |= attrs["grainColumnIds"]
                .as_array()
                .is_some_and(|a| !a.is_empty())
                && attrs["validStates"]
                    .as_array()
                    .is_some_and(|a| !a.is_empty())
                && attrs["joins"].is_array();
            policies.push(policy);
        }
        implementations.push(json!({"entityRef":entity["entityRef"],"name":entity["name"],"assertions":entity["assertions"],"policies":policies,"grainProven":grain_proven}));
    }
    if roots.len() > 1 {
        gaps.push(scenario::gap(gap_codes::AMBIGUOUS_BINDING,"report","multiple existing implementations match; refine roots, scope and required fields instead of choosing a shortest path",roots.iter().map(|r|r.entity_id.clone()).collect()));
    }
    let fields = scenario::ids(&walk.entities, &["reportField"]);
    if fields.is_empty() {
        gaps.push(scenario::gap(
            gap_codes::DEFINITION_MISSING,
            "report",
            "no proven output field lineage is available for the selected implementation",
            Vec::new(),
        ));
    }
    let data = json!({"query":request.query,"implementations":implementations,"fields":fields,"unknowns":unknowns,"lineage":walk.hops.iter().map(|h|h["relationRef"].clone()).collect::<Vec<_>>()});
    scenario::finish(
        context,
        id,
        roots,
        walk,
        data,
        &["database", "report"],
        request.filters.as_ref(),
        gaps,
        rt,
    )
}
