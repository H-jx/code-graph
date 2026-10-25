//! 正反向关系、候选与证据（计划 §3.4）。
//!
//! 关系仅在自身断言与两端至少一条匹配 freshness/basis 过滤的当前实体断言都
//! 有效时可见；身份行仍保留但不会暴露悬空边。

use rusqlite::{Connection, params};

use crate::contract::error::{CoreError, codes};
use crate::contract::relation::{Relation, RelationKind, Resolution};

/// 写入（或复用）关系身份行。
pub fn upsert_relation(connection: &Connection, relation: &Relation) -> Result<(), CoreError> {
    let qualifiers_json = serde_json::to_string(&relation.qualifiers).map_err(|error| {
        CoreError::new(
            codes::INTERNAL_ERROR,
            format!("cannot serialize qualifiers: {error}"),
        )
    })?;
    connection
        .execute(
            "INSERT INTO relations (id, kind, from_id, to_id, qualifiers_json)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET kind = excluded.kind,
                                           from_id = excluded.from_id,
                                           to_id = excluded.to_id,
                                           qualifiers_json = excluded.qualifiers_json",
            params![
                relation.id,
                relation.kind.as_str(),
                relation.from_id,
                relation.to_id,
                qualifiers_json
            ],
        )
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot upsert relation `{}`: {error}", relation.id),
            )
        })?;
    Ok(())
}

/// 关系断言写入载荷。
pub struct RelationAssertionRow<'a> {
    pub assertion_id: &'a str,
    pub relation_id: &'a str,
    pub batch_id: &'a str,
    pub basis: crate::contract::entity::Basis,
    pub origin: crate::contract::entity::Origin,
    pub resolution: Resolution,
}

/// 写入关系断言（父表 + 子表）。
pub fn insert_relation_assertion(
    connection: &Connection,
    row: &RelationAssertionRow<'_>,
    payload_hash: &str,
    producer_key: &str,
) -> Result<(), CoreError> {
    connection
        .execute(
            "INSERT INTO assertions (id, kind, batch_id, payload_hash, producer_key, basis, origin)
             VALUES (?1, 'relation', ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO NOTHING",
            params![
                row.assertion_id,
                row.batch_id,
                payload_hash,
                producer_key,
                row.basis.as_str(),
                row.origin.as_str()
            ],
        )
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot insert relation assertion: {error}"),
            )
        })?;
    connection
        .execute(
            "INSERT INTO relation_assertions
               (id, relation_id, batch_id, basis, origin, resolution, payload_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO NOTHING",
            params![
                row.assertion_id,
                row.relation_id,
                row.batch_id,
                row.basis.as_str(),
                row.origin.as_str(),
                row.resolution.as_str(),
                payload_hash
            ],
        )
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot insert relation assertion: {error}"),
            )
        })?;
    Ok(())
}

/// 关系断言记录。
pub struct StoredRelationAssertion {
    pub assertion_id: String,
    pub relation_id: String,
    pub kind: RelationKind,
    pub from_id: String,
    pub to_id: String,
    pub producer_key: String,
    pub batch_id: String,
    pub basis: crate::contract::entity::Basis,
    pub origin: crate::contract::entity::Origin,
    pub resolution: Resolution,
    pub qualifiers_json: String,
}

/// 读取关系的断言集合。
pub fn load_relation_assertions(
    connection: &Connection,
    relation_id: &str,
    only_fresh: bool,
) -> Result<Vec<StoredRelationAssertion>, CoreError> {
    let mut statement = connection
        .prepare(
            "SELECT ra.id, ra.relation_id, r.kind, r.from_id, r.to_id,
                    a.producer_key, a.batch_id, ra.basis, ra.origin, ra.resolution, r.qualifiers_json
               FROM relation_assertions ra
               JOIN assertions a ON a.id = ra.id
               JOIN relations r ON r.id = ra.relation_id
              WHERE ra.relation_id = ?1
                AND (?2 = 0 OR EXISTS (
                      SELECT 1 FROM current_batches cb
                       JOIN batches b ON b.id = cb.batch_id
                      WHERE b.id = ra.batch_id AND b.state = 'fresh'))",
        )
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot prepare relation assertion query: {error}"),
            )
        })?;
    let rows = statement
        .query_map(
            params![relation_id, if only_fresh { 1 } else { 0 }],
            |row| {
                let kind_text: String = row.get(2)?;
                let basis_text: String = row.get(7)?;
                let origin_text: String = row.get(8)?;
                let resolution_text: String = row.get(9)?;
                Ok(StoredRelationAssertion {
                    assertion_id: row.get(0)?,
                    relation_id: row.get(1)?,
                    kind: RelationKind::parse(&kind_text).unwrap_or(RelationKind::References),
                    from_id: row.get(3)?,
                    to_id: row.get(4)?,
                    producer_key: row.get(5)?,
                    batch_id: row.get(6)?,
                    basis: crate::contract::entity::Basis::parse(&basis_text),
                    origin: crate::contract::entity::Origin::parse(&origin_text),
                    resolution: Resolution::parse(&resolution_text),
                    qualifiers_json: row.get(10)?,
                })
            },
        )
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot query relation assertions: {error}"),
            )
        })?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot read relation assertion: {error}"),
            )
        })?);
    }
    Ok(result)
}

/// 邻接查询结果。
pub struct NeighborRow {
    pub relation_id: String,
    pub kind: RelationKind,
    pub from_id: String,
    pub to_id: String,
    pub resolution: Resolution,
    pub basis: crate::contract::entity::Basis,
    pub assertion_id: String,
    pub batch_id: String,
    pub qualifiers_json: String,
}

/// 按方向读取邻接关系。
///
/// `direction`：`out` 表示 `from_id = entity`，`in` 表示 `to_id = entity`。
/// 只有两端实体都存在当前有效断言的关系才会返回，避免悬空边。
pub fn load_neighbors(
    connection: &Connection,
    entity_id: &str,
    direction: &str,
    relation_kinds: &[RelationKind],
    include_candidates: bool,
    only_fresh: bool,
) -> Result<Vec<NeighborRow>, CoreError> {
    let (sql, params): (String, Vec<String>) = match direction {
        "out" => (
            "SELECT r.id, r.kind, r.from_id, r.to_id, ra.resolution, ra.basis, ra.id, ra.batch_id, r.qualifiers_json
               FROM relations r
               JOIN relation_assertions ra ON ra.relation_id = r.id
               JOIN current_batches cb ON cb.batch_id = ra.batch_id
               JOIN batches b ON b.id = cb.batch_id
              WHERE r.from_id = ?1
                AND (?2 = 0 OR b.state = 'fresh')
                AND EXISTS (SELECT 1 FROM entity_assertions ea
                              JOIN current_batches cb3 ON cb3.batch_id = ea.batch_id
                              JOIN batches b3 ON b3.id = cb3.batch_id
                             WHERE ea.entity_id = r.to_id AND (?2 = 0 OR b3.state = 'fresh'))"
                .to_string(),
            vec![entity_id.to_string()],
        ),
        "in" => (
            "SELECT r.id, r.kind, r.from_id, r.to_id, ra.resolution, ra.basis, ra.id, ra.batch_id, r.qualifiers_json
               FROM relations r
               JOIN relation_assertions ra ON ra.relation_id = r.id
               JOIN current_batches cb ON cb.batch_id = ra.batch_id
               JOIN batches b ON b.id = cb.batch_id
              WHERE r.to_id = ?1
                AND (?2 = 0 OR b.state = 'fresh')
                AND EXISTS (SELECT 1 FROM entity_assertions ea
                              JOIN current_batches cb3 ON cb3.batch_id = ea.batch_id
                              JOIN batches b3 ON b3.id = cb3.batch_id
                             WHERE ea.entity_id = r.from_id AND (?2 = 0 OR b3.state = 'fresh'))"
                .to_string(),
            vec![entity_id.to_string()],
        ),
        _ => {
            return Err(CoreError::new(
                codes::INVALID_REQUEST,
                format!("unknown traversal direction `{direction}`"),
            ))
        }
    };

    let mut statement = connection.prepare(&sql).map_err(|error| {
        CoreError::new(
            codes::INTERNAL_ERROR,
            format!("cannot prepare neighbor query: {error}"),
        )
    })?;
    let rows = statement
        .query_map(
            rusqlite::params![params[0], if only_fresh { 1 } else { 0 }],
            |row| {
                let kind_text: String = row.get(1)?;
                let resolution_text: String = row.get(4)?;
                let basis_text: String = row.get(5)?;
                Ok(NeighborRow {
                    relation_id: row.get(0)?,
                    kind: RelationKind::parse(&kind_text).unwrap_or(RelationKind::References),
                    from_id: row.get(2)?,
                    to_id: row.get(3)?,
                    resolution: Resolution::parse(&resolution_text),
                    basis: crate::contract::entity::Basis::parse(&basis_text),
                    assertion_id: row.get(6)?,
                    batch_id: row.get(7)?,
                    qualifiers_json: row.get(8)?,
                })
            },
        )
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot query neighbors: {error}"),
            )
        })?;

    let mut result = Vec::new();
    for row in rows {
        let row = row.map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot read neighbor row: {error}"),
            )
        })?;
        if !relation_kinds.is_empty() && !relation_kinds.contains(&row.kind) {
            continue;
        }
        if !include_candidates && row.resolution == Resolution::Candidate {
            continue;
        }
        result.push(row);
    }
    result.sort_by(|left, right| {
        left.kind
            .as_str()
            .cmp(right.kind.as_str())
            .then(left.relation_id.cmp(&right.relation_id))
            .then(left.assertion_id.cmp(&right.assertion_id))
    });
    Ok(result)
}
