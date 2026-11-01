//! 场景只组合已有断言和覆盖；入口筛选不限制后继类型，不把图距离转成业务结论。
use super::{
    Filters, QueryContext,
    runtime::{self, Runtime},
};
use crate::contract::{
    error::{CoreError, gap_codes},
    evidence::SourceReference,
    query::*,
};
use crate::graph::paths::Walk;
use crate::storage;
use serde_json::{Value, json};
use std::collections::BTreeSet;

pub fn gap(code: &str, capability: &str, reason: &str, candidates: Vec<String>) -> Gap {
    Gap {
        id: format!(
            "gap_{}",
            crate::contract::identity::content_hash(
                format!("{code}:{capability}:{reason}:{candidates:?}").as_bytes()
            )
        ),
        code: code.into(),
        capability: capability.into(),
        scope_id: String::new(),
        source_refs: Vec::new(),
        candidate_ids: candidates,
        reason: reason.into(),
        required_evidence: vec![
            "collect the missing original evidence in the requested scope".into(),
        ],
    }
}
pub fn value_slot<'a>(entity: &'a Value, name: &str) -> Option<&'a str> {
    entity["identity"]
        .as_array()?
        .iter()
        .find(|s| s["name"] == name)?["value"]
        .as_str()
}
pub fn attributes(entity: &Value) -> impl Iterator<Item = &Value> {
    entity["assertions"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|a| &a["attributes"])
}
pub fn ids(entities: &[Value], kinds: &[&str]) -> Vec<Value> {
    entities
        .iter()
        .filter(|e| e["kind"].as_str().is_some_and(|k| kinds.contains(&k)))
        .map(|e| e["entityRef"].clone())
        .collect()
}
pub fn roots_filters(
    filters: Option<&CommonFilters>,
    scopes: Option<&Vec<String>>,
    kinds: Option<Vec<crate::contract::entity::EntityKind>>,
) -> CommonFilters {
    let mut filters = filters.cloned().unwrap_or_default();
    if let Some(scopes) = scopes {
        filters.scope_ids = Some(match filters.scope_ids {
            Some(existing) => existing
                .into_iter()
                .filter(|id| scopes.contains(id))
                .collect(),
            None => scopes.clone(),
        });
    }
    if let Some(kinds) = kinds {
        filters.kinds = Some(match filters.kinds {
            Some(existing) => existing
                .into_iter()
                .filter(|kind| kinds.contains(kind))
                .collect(),
            None => kinds,
        });
    }
    filters
}
pub fn select(
    context: &QueryContext<'_>,
    query: &str,
    explicit: Option<&Vec<EntityRef>>,
    filters: CommonFilters,
    rt: &mut Runtime,
) -> Result<(Vec<EntityRef>, bool), CoreError> {
    let snapshot = super::snapshot_ref(context.database)?;
    let allowed = Filters::from_request(Some(&filters));
    let mut roots = Vec::new();
    let mut truncated = false;
    if let Some(explicit) = explicit {
        for root in explicit {
            runtime::validate_entity_reference(context.database.connection(), root, &snapshot)?;
            if !rt.visit() {
                truncated = true;
                break;
            }
            if runtime::entity_allowed(context.database.connection(), &root.entity_id, &allowed)? {
                roots.push(root.clone());
            }
        }
    } else if !query.trim().is_empty() {
        let response = super::generic::search(
            context,
            &SearchRequest {
                query: query.into(),
                fields: None,
                filters: Some(filters),
                page_size: Some(100),
                continuation: None,
                budget: None,
            },
            format!("req_{}", uuid::Uuid::new_v4()),
            rt,
        )?;
        truncated = response.completeness.truncated;
        for value in response.data["results"].as_array().unwrap() {
            roots.push(
                serde_json::from_value(value["entityRef"].clone()).map_err(|_| {
                    CoreError::new(
                        crate::contract::error::codes::INTERNAL_ERROR,
                        "invalid stored entity reference",
                    )
                })?,
            );
        }
    }
    roots.sort_by(|a, b| a.entity_id.cmp(&b.entity_id));
    roots.dedup_by(|a, b| a.entity_id == b.entity_id);
    rt.pagination = None;
    Ok((roots, truncated))
}

#[allow(clippy::too_many_arguments)]
pub fn finish(
    context: &QueryContext<'_>,
    id: String,
    roots: Vec<EntityRef>,
    mut walk: Walk,
    mut data: Value,
    capabilities: &[&str],
    filters: Option<&CommonFilters>,
    mut gaps: Vec<Gap>,
    rt: &mut Runtime,
) -> Result<QueryResponse<Value>, CoreError> {
    let connection = context.database.connection();
    let scope_ids = walk
        .entities
        .iter()
        .filter_map(|e| e["scopeId"].as_str())
        .collect::<BTreeSet<_>>();
    let entity_ids = walk
        .entities
        .iter()
        .filter_map(|e| e["entityRef"]["entityId"].as_str())
        .collect::<BTreeSet<_>>();
    let sources = walk
        .references
        .iter()
        .flat_map(|r| [r.source_id.as_str(), r.evidence_id.as_str()])
        .collect::<BTreeSet<_>>();
    let mut coverage = Vec::new();
    for capability in capabilities {
        let mut found = false;
        for (_, batch) in storage::current_batches(connection)? {
            if storage::batch_state(connection, &batch)? != Some(storage::BatchState::Fresh) {
                continue;
            }
            for (cap, scope, source_set, status) in storage::load_coverage(connection, &batch)? {
                if !rt.visit() {
                    walk.truncated = true;
                    break;
                }
                if cap != *capability
                    || (!scope_ids.is_empty() && !scope_ids.contains(scope.as_str()))
                {
                    continue;
                }
                found = true;
                coverage.push(json!({"capability":cap,"scopeId":scope,"sourceSetId":source_set,"status":status,"batchId":batch}));
            }
        }
        if !found {
            gaps.push(gap(
                gap_codes::NOT_CONFIGURED,
                capability,
                "required capability has no current coverage in the selected scope",
                Vec::new(),
            ));
        }
    }
    for (_, batch) in storage::current_batches(connection)? {
        if storage::batch_state(connection, &batch)? != Some(storage::BatchState::Fresh) {
            continue;
        }
        for (_, code, payload) in storage::load_gaps(connection, &batch)? {
            if !rt.visit() {
                walk.truncated = true;
                break;
            }
            let cap = payload["capability"].as_str().unwrap_or_default();
            let scope = payload["scopeId"].as_str().unwrap_or_default();
            if !capabilities.contains(&cap) || (!scope_ids.is_empty() && !scope_ids.contains(scope))
            {
                continue;
            }
            let relevant = payload["sourceRefs"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .any(|s| sources.contains(s))
                || payload["candidateIds"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .any(|e| entity_ids.contains(e));
            if relevant {
                let mut gap = super::gap_from_payload(&code, &payload);
                gap.source_refs = payload["sourceRefs"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect();
                gap.candidate_ids = payload["candidateIds"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect();
                gap.required_evidence = payload["requiredEvidence"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect();
                gaps.push(gap);
            }
        }
    }
    if roots.is_empty() && gaps.is_empty() {
        gaps.push(gap(
            gap_codes::DEFINITION_MISSING,
            "scenario",
            "no supported entry was found for the requested input",
            Vec::new(),
        ));
    }
    if walk.hops.iter().any(|h| h["resolution"] == "candidate") {
        gaps.push(gap(
            gap_codes::AMBIGUOUS_BINDING,
            "scenario",
            "candidate relations are shown explicitly; they are not confirmed business links",
            Vec::new(),
        ));
    }
    gaps.sort_by(|a, b| a.id.cmp(&b.id));
    gaps.dedup_by(|a, b| a.id == b.id);
    let mut completeness =
        rt.completeness(context, &Filters::from_request(filters), walk.truncated)?;
    if coverage.iter().any(|c| c["status"] != "complete") || !gaps.is_empty() {
        completeness.coverage = CoverageLevel::Partial;
    }
    if roots.is_empty() {
        completeness.candidate_sets_complete = !walk.truncated;
    }
    let snapshot = super::snapshot_ref(context.database)?;
    let frontier = walk
        .frontier
        .iter()
        .map(|entity| json!({"snapshot":snapshot,"entityId":entity}))
        .collect::<Vec<_>>();
    let next = if walk.truncated {
        vec![NextQuery {
            operation: "trace".into(),
            arguments: json!({"roots":frontier,"direction":"both","maxDepth":1})
                .as_object()
                .unwrap()
                .clone(),
            reason: "continue with a narrower query from the bounded frontier".into(),
        }]
    } else {
        Vec::new()
    };
    data["roots"] = json!(roots);
    data["entities"] = json!(walk.entities);
    data["hops"] = json!(walk.hops);
    data["frontier"] = json!(frontier);
    data["coverage"] = json!(coverage);
    let mut references: Vec<SourceReference> = walk.references;
    super::generic::dedup_refs(&mut references);
    rt.pagination = None;
    super::envelope(
        context,
        id,
        ResponseStatus::Complete,
        data,
        gaps,
        references,
        completeness,
        super::generic::used(walk.entities.len(), walk.hops.len()),
        next,
    )
}
