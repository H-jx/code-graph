//! 确定性 BFS；每个关系断言只返回一次，每个实体只排队一次，截断不伪造可恢复游标。
use super::neighbors;
use crate::contract::error::CoreError;
use crate::contract::evidence::SourceReference;
use crate::contract::query::TraceRequest;
use crate::query::{
    Filters, QueryContext,
    runtime::{self, Runtime},
};
use serde_json::{Value, json};
use std::collections::{BTreeSet, VecDeque};

pub struct Walk {
    pub hops: Vec<Value>,
    pub entities: Vec<Value>,
    pub frontier: Vec<String>,
    pub references: Vec<SourceReference>,
    pub truncated: bool,
}
pub fn walk(
    context: &QueryContext<'_>,
    request: &TraceRequest,
    rt: &mut Runtime,
) -> Result<Walk, CoreError> {
    walk_with_root_filters(
        context,
        request,
        rt,
        &Filters::from_request(request.filters.as_ref()),
    )
}

pub(crate) fn walk_with_root_filters(
    context: &QueryContext<'_>,
    request: &TraceRequest,
    rt: &mut Runtime,
    root_filters: &Filters,
) -> Result<Walk, CoreError> {
    walk_selected(context, request, rt, root_filters, |_, _| true)
}

pub(crate) fn walk_selected(
    context: &QueryContext<'_>,
    request: &TraceRequest,
    rt: &mut Runtime,
    root_filters: &Filters,
    accept: impl Fn(&neighbors::Neighbor, &str) -> bool,
) -> Result<Walk, CoreError> {
    let snapshot = crate::query::snapshot_ref(context.database)?;
    let filters = Filters::from_request(request.filters.as_ref());
    let connection = context.database.connection();
    let mut walk = Walk {
        hops: Vec::new(),
        entities: Vec::new(),
        frontier: Vec::new(),
        references: Vec::new(),
        truncated: false,
    };
    let mut queue = VecDeque::new();
    let mut seen = BTreeSet::new();
    let mut edges = BTreeSet::new();
    for root in &request.roots {
        runtime::validate_entity_reference(connection, root, &snapshot)?;
        if !seen.contains(&root.entity_id)
            && runtime::entity_allowed(connection, &root.entity_id, root_filters)?
        {
            if seen.len() as u64 >= rt.budget.max_entities {
                walk.truncated = true;
                break;
            }
            seen.insert(root.entity_id.clone());
            queue.push_back((root.entity_id.clone(), 0));
        }
    }
    let depth = request
        .max_depth
        .unwrap_or(rt.budget.max_depth)
        .min(rt.budget.max_depth);
    while let Some((id, level)) = queue.pop_front() {
        if !rt.visit() {
            queue.push_front((id, level));
            walk.truncated = true;
            break;
        }
        if let Some(entity) = runtime::entity_projection(
            connection,
            &id,
            if level == 0 { root_filters } else { &filters },
            &snapshot,
        )? {
            for a in entity["assertions"].as_array().unwrap() {
                walk.references.extend(runtime::references(
                    connection,
                    a["assertionId"].as_str().unwrap(),
                    &snapshot,
                )?);
            }
            walk.entities.push(entity);
        }
        let current_kind = crate::query::load_entity(connection, &id)?
            .map(|entity| entity.kind.as_str().to_string())
            .unwrap_or_default();
        let row_limit = rt
            .budget
            .max_visited
            .saturating_sub(rt.visited)
            .min(rt.budget.max_relations + 1) as usize
            + 1;
        let rows = neighbors::load(
            connection,
            &id,
            request.direction,
            request.relation_kinds.as_deref().unwrap_or_default(),
            &filters,
            None,
            row_limit,
        )?;
        if rows.len() == row_limit {
            walk.truncated = true;
            walk.frontier.push(id.clone());
        }
        if level >= depth {
            if rows.iter().any(|r| {
                accept(r, &current_kind)
                    && !edges.contains(r.value["assertionId"].as_str().unwrap())
            }) {
                walk.frontier.push(id);
                walk.truncated = true;
            }
            continue;
        }
        for mut row in rows {
            if !rt.visit() {
                walk.frontier.push(id.clone());
                walk.truncated = true;
                break;
            }
            if !accept(&row, &current_kind) {
                continue;
            }
            if walk.hops.len() as u64 >= rt.budget.max_relations {
                walk.frontier.push(id.clone());
                walk.truncated = true;
                break;
            }
            if !runtime::entity_allowed(connection, &row.other, &filters)? {
                continue;
            }
            if !seen.contains(&row.other) && seen.len() as u64 >= rt.budget.max_entities {
                walk.frontier.push(id.clone());
                walk.truncated = true;
                break;
            }
            if !edges.insert(row.value["assertionId"].as_str().unwrap().to_string()) {
                continue;
            }
            row.value["depth"] = json!(level + 1);
            walk.references.extend(neighbors::attach_evidence(
                connection,
                &mut row.value,
                &snapshot,
            )?);
            walk.hops.push(row.value);
            if seen.insert(row.other.clone()) {
                queue.push_back((row.other, level + 1));
            }
        }
        if rt.stopped() {
            walk.truncated = true;
            break;
        }
    }
    walk.frontier.extend(queue.into_iter().map(|(id, _)| id));
    walk.frontier.sort();
    walk.frontier.dedup();
    walk.frontier.truncate(rt.budget.max_entities as usize);
    walk.references
        .sort_by(|a, b| a.evidence_id.cmp(&b.evidence_id));
    walk.references
        .dedup_by(|a, b| a.evidence_id == b.evidence_id);
    Ok(walk)
}
