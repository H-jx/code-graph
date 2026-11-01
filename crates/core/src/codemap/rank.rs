//! 入口、直接依赖和同文件轮廓的确定性排序，不给业务正确性分数。
use crate::contract::{error::CoreError, query::Direction, relation::RelationKind};
use crate::query::{
    Filters, QueryContext,
    runtime::{self, Runtime},
};
use crate::storage;
use std::collections::{BTreeMap, BTreeSet};

pub struct Ranked {
    pub entities: Vec<(String, u32)>,
    pub truncated: bool,
}
pub fn select(
    context: &QueryContext<'_>,
    roots: &BTreeSet<String>,
    filters: &Filters,
    rt: &mut Runtime,
) -> Result<Ranked, CoreError> {
    let connection = context.database.connection();
    let mut selected = BTreeMap::new();
    let mut truncated = false;
    for id in roots {
        if !rt.visit() {
            truncated = true;
            break;
        }
        if !runtime::entity_allowed(connection, id, filters)? {
            continue;
        }
        selected.insert(id.clone(), 0);
        let rows = crate::graph::neighbors::load(
            connection,
            id,
            Direction::Both,
            &[
                RelationKind::Calls,
                RelationKind::Imports,
                RelationKind::Contains,
                RelationKind::Renders,
            ],
            filters,
            None,
            rt.budget.max_relations as usize + 1,
        )?;
        for row in rows {
            if !rt.visit() {
                truncated = true;
                break;
            }
            if runtime::entity_allowed(connection, &row.other, filters)? {
                selected.entry(row.other).or_insert(1);
            }
        }
        // Outlines are linked by original evidence source and scope, never a globally equal path.
        let mut stmt=connection.prepare("SELECT DISTINCT other.entity_id FROM entity_assertions root JOIN assertion_evidence re ON re.assertion_id=root.id JOIN evidence source ON source.id=re.evidence_id JOIN evidence e ON e.source_id=source.source_id AND e.content_hash=source.content_hash JOIN assertion_evidence ae ON ae.evidence_id=e.id JOIN entity_assertions other ON other.id=ae.assertion_id JOIN entities er ON er.id=root.entity_id JOIN entities eo ON eo.id=other.entity_id AND eo.scope_id=er.scope_id WHERE root.entity_id=?1 ORDER BY other.entity_id LIMIT ?2").map_err(storage::database_error)?;
        let rows = stmt
            .query_map(
                rusqlite::params![id, (rt.budget.max_visited + 1) as i64],
                |r| r.get::<_, String>(0),
            )
            .map_err(storage::database_error)?;
        for row in rows {
            if !rt.visit() {
                truncated = true;
                break;
            }
            let id = row.map_err(storage::database_error)?;
            if runtime::entity_allowed(connection, &id, filters)? {
                selected.entry(id).or_insert(2);
            }
        }
    }
    let mut entities = selected.into_iter().collect::<Vec<_>>();
    entities.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)));
    if entities.len() as u64 > rt.budget.max_entities {
        truncated = true;
        entities.truncate(rt.budget.max_entities as usize);
    }
    Ok(Ranked {
        entities,
        truncated,
    })
}
