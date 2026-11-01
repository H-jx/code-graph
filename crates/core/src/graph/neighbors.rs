//! 基于两侧索引的有界邻接；过滤关系断言与两端当前实体，不按名字补边。
use crate::contract::error::{CoreError, codes};
use crate::contract::query::{Direction, FreshnessPolicy};
use crate::contract::relation::RelationKind;
use crate::query::{Filters, runtime};
use crate::storage;
use rusqlite::{Connection, params};
use serde_json::{Value, json};

/// 稳定 keyset 键，不包含深度或 BFS 队列。
pub type NeighborKey = (String, String, String, String);
pub struct Neighbor {
    pub key: NeighborKey,
    pub other: String,
    pub value: Value,
}

#[allow(clippy::too_many_arguments)]
pub fn load(
    connection: &Connection,
    root: &str,
    direction: Direction,
    kinds: &[RelationKind],
    filters: &Filters,
    after: Option<&NeighborKey>,
    limit: usize,
) -> Result<Vec<Neighbor>, CoreError> {
    let mut output = Vec::new();
    let directions: &[&str] = match direction {
        Direction::In => &["in"],
        Direction::Out => &["out"],
        Direction::Both => &["in", "out"],
    };
    let basis = serde_json::to_string(&filters.basis_filter.clone().unwrap_or_else(|| {
        vec![
            crate::contract::entity::Basis::Observed,
            crate::contract::entity::Basis::Declared,
        ]
    }))
    .unwrap();
    let kinds = serde_json::to_string(kinds).unwrap();
    for dir in directions {
        if after.is_some_and(|key| dir < &key.0.as_str()) {
            continue;
        }
        // The endpoint equality is deliberately first; SQLite uses idx_relations_{from,to}.
        let endpoint = if *dir == "out" { "from_id" } else { "to_id" };
        let sql=format!("SELECT r.id,r.kind,r.from_id,r.to_id,ra.id,ra.basis,ra.origin,ra.resolution,ra.batch_id,a.producer_key,r.qualifiers_json
            FROM relations r JOIN relation_assertions ra ON ra.relation_id=r.id JOIN assertions a ON a.id=ra.id
            JOIN current_batches cb ON cb.batch_id=ra.batch_id JOIN batches b ON b.id=cb.batch_id
            WHERE r.{endpoint}=?1 AND (?2 OR b.state='fresh') AND (?3 OR ra.resolution='confirmed')
              AND ra.basis IN (SELECT value FROM json_each(?4))
              AND (json_array_length(?5)=0 OR r.kind IN (SELECT value FROM json_each(?5)))
              AND (?6='' OR (r.kind,r.id,ra.id)>(?7,?8,?9))
              AND EXISTS(SELECT 1 FROM entity_assertions ea JOIN current_batches c ON c.batch_id=ea.batch_id JOIN batches b ON b.id=c.batch_id WHERE ea.entity_id=r.from_id AND (?2 OR b.state='fresh') AND ea.basis IN (SELECT value FROM json_each(?4)))
              AND EXISTS(SELECT 1 FROM entity_assertions ea JOIN current_batches c ON c.batch_id=ea.batch_id JOIN batches b ON b.id=c.batch_id WHERE ea.entity_id=r.to_id AND (?2 OR b.state='fresh') AND ea.basis IN (SELECT value FROM json_each(?4)))
            ORDER BY r.kind,r.id,ra.id LIMIT ?10");
        let key = after.filter(|key| key.0 == *dir);
        let mut stmt = connection.prepare(&sql).map_err(storage::database_error)?;
        let rows=stmt.query_map(params![root,filters.freshness_policy==FreshnessPolicy::AllowStale,filters.include_candidates,basis,kinds,key.map_or("",|k|k.0.as_str()),key.map_or("",|k|k.1.as_str()),key.map_or("",|k|k.2.as_str()),key.map_or("",|k|k.3.as_str()),limit as i64],|r| {
            let id:String=r.get(0)?; let kind:String=r.get(1)?;let from:String=r.get(2)?;let to:String=r.get(3)?;let assertion:String=r.get(4)?;
            let qualifiers:String=r.get(10)?;
            Ok(Neighbor {key:((*dir).into(),kind.clone(),id.clone(),assertion.clone()),other:if from==root {to.clone()} else {from.clone()},value:json!({"relationId":id,"kind":kind,"fromId":from,"toId":to,"direction":dir,"assertionId":assertion,"basis":r.get::<_,String>(5)?,"origin":r.get::<_,String>(6)?,"resolution":r.get::<_,String>(7)?,"batchId":r.get::<_,String>(8)?,"producerKey":r.get::<_,String>(9)?,"qualifiers":serde_json::from_str::<Value>(&qualifiers).map_err(|_|rusqlite::Error::InvalidQuery)?})})
        }).map_err(storage::database_error)?;
        for row in rows {
            output.push(row.map_err(storage::database_error)?);
        }
    }
    output.sort_by(|a, b| a.key.cmp(&b.key));
    output.truncate(limit);
    Ok(output)
}

pub fn decode_key(value: Value) -> Result<NeighborKey, CoreError> {
    let key: NeighborKey = serde_json::from_value(value)
        .map_err(|_| CoreError::new(codes::INVALID_REQUEST, "invalid neighbor cursor key"))?;
    if !matches!(key.0.as_str(), "in" | "out")
        || RelationKind::parse(&key.1).is_none()
        || key.2.len() > 128
        || key.3.len() > 128
    {
        return Err(CoreError::new(
            codes::INVALID_REQUEST,
            "invalid neighbor cursor key",
        ));
    }
    Ok(key)
}

pub fn attach_evidence(
    connection: &Connection,
    value: &mut Value,
    snapshot: &crate::contract::evidence::SnapshotRef,
) -> Result<Vec<crate::contract::evidence::SourceReference>, CoreError> {
    let assertion = value["assertionId"]
        .as_str()
        .ok_or_else(|| CoreError::new(codes::INTERNAL_ERROR, "missing assertion"))?;
    let refs = runtime::references(connection, assertion, snapshot)?;
    value["evidenceRefs"] = json!(refs.iter().map(|r| &r.evidence_id).collect::<Vec<_>>());
    value["relationRef"] = json!({"snapshot":snapshot,"relationId":value["relationId"]});
    Ok(refs)
}
