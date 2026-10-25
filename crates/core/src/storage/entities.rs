//! 实体、业务属性与作用域（计划 §3.4）。
//!
//! 身份表只存不冲突的身份字段；未被当前批次引用的身份行不参与查询。

use rusqlite::{Connection, params};

use crate::contract::entity::{Attributes, Basis, Entity, EntityKind, Origin};
use crate::contract::error::{CoreError, codes};
use crate::contract::evidence::Evidence;
use crate::contract::identity::content_hash;

/// 实体身份行的写入载荷。
#[derive(Debug, Clone)]
pub struct EntityRow<'a> {
    pub id: &'a str,
    pub kind: EntityKind,
    pub scope_id: &'a str,
    /// 规范身份 JSON（含 projectId 与 kind），在库内唯一。
    pub identity_json: &'a str,
}

/// 写入（或复用）实体身份行。
pub fn upsert_entity(connection: &Connection, row: &EntityRow<'_>) -> Result<(), CoreError> {
    connection
        .execute(
            "INSERT INTO entities (id, kind, scope_id, identity_json) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET kind = excluded.kind,
                                           scope_id = excluded.scope_id,
                                           identity_json = excluded.identity_json",
            params![row.id, row.kind.as_str(), row.scope_id, row.identity_json],
        )
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot upsert entity `{}`: {error}", row.id),
            )
        })?;
    Ok(())
}

/// 实体断言写入载荷。
pub struct EntityAssertionRow<'a> {
    pub assertion_id: &'a str,
    pub entity_id: &'a str,
    pub batch_id: &'a str,
    pub basis: Basis,
    pub origin: Origin,
    pub name: Option<&'a str>,
    pub attributes: &'a Attributes,
}

/// 实体断言的载荷哈希：`(producerKey, batchId, entityId, payloadHash)` 的基础部分。
pub fn entity_payload_hash(
    producer_key: &str,
    batch_id: &str,
    entity_id: &str,
    name: Option<&str>,
    attributes: &Attributes,
    basis: Basis,
    origin: Origin,
) -> Result<String, CoreError> {
    let attributes_json = serde_json::to_string(attributes).map_err(|error| {
        CoreError::new(
            codes::INTERNAL_ERROR,
            format!("cannot serialize attributes: {error}"),
        )
    })?;
    let canonical = format!(
        "{producer_key}\u{1f}{batch_id}\u{1f}{entity_id}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{attributes_json}",
        name.unwrap_or(""),
        basis.as_str(),
        origin.as_str()
    );
    Ok(content_hash(canonical.as_bytes()))
}

/// 写入实体断言（父表 + 子表）。
pub fn insert_entity_assertion(
    connection: &Connection,
    row: &EntityAssertionRow<'_>,
    payload_hash: &str,
    producer_key: &str,
) -> Result<(), CoreError> {
    let attributes_json = serde_json::to_string(row.attributes).map_err(|error| {
        CoreError::new(
            codes::INTERNAL_ERROR,
            format!("cannot serialize attributes: {error}"),
        )
    })?;
    connection
        .execute(
            "INSERT INTO assertions (id, kind, batch_id, payload_hash, producer_key, basis, origin)
             VALUES (?1, 'entity', ?2, ?3, ?4, ?5, ?6)
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
                format!("cannot insert assertion `{}`: {error}", row.assertion_id),
            )
        })?;
    connection
        .execute(
            "INSERT INTO entity_assertions
               (id, entity_id, batch_id, basis, origin, name, attributes_json, payload_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(id) DO NOTHING",
            params![
                row.assertion_id,
                row.entity_id,
                row.batch_id,
                row.basis.as_str(),
                row.origin.as_str(),
                row.name,
                attributes_json,
                payload_hash
            ],
        )
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!(
                    "cannot insert entity assertion `{}`: {error}",
                    row.assertion_id
                ),
            )
        })?;
    Ok(())
}

/// 写入证据。
pub fn insert_evidence(connection: &Connection, evidence: &Evidence) -> Result<(), CoreError> {
    connection
        .execute(
            "INSERT INTO evidence
               (id, source_id, content_hash, start_byte, end_byte, start_line, end_line, role)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(id) DO NOTHING",
            params![
                evidence.id,
                evidence.source_id,
                evidence.content_hash,
                evidence.start_byte as i64,
                evidence.end_byte as i64,
                evidence.start_line as i64,
                evidence.end_line as i64,
                evidence.role
            ],
        )
        .map_err(|error| {
            CoreError::new(
                codes::EVIDENCE_INVALID,
                format!("cannot insert evidence `{}`: {error}", evidence.id),
            )
        })?;
    Ok(())
}

/// 关联断言与证据。
pub fn link_assertion_evidence(
    connection: &Connection,
    assertion_id: &str,
    evidence_id: &str,
) -> Result<(), CoreError> {
    connection
        .execute(
            "INSERT INTO assertion_evidence (assertion_id, evidence_id) VALUES (?1, ?2)
             ON CONFLICT(assertion_id, evidence_id) DO NOTHING",
            params![assertion_id, evidence_id],
        )
        .map_err(|error| {
            CoreError::new(
                codes::EVIDENCE_INVALID,
                format!("cannot link evidence to assertion: {error}"),
            )
        })?;
    Ok(())
}

/// 实体的全部断言（含批次与依据）。
#[derive(Debug, Clone)]
pub struct StoredEntityAssertion {
    pub assertion_id: String,
    pub entity_id: String,
    pub kind: EntityKind,
    pub scope_id: String,
    pub identity_json: String,
    pub producer_key: String,
    pub batch_id: String,
    pub basis: Basis,
    pub origin: Origin,
    pub name: Option<String>,
    pub attributes: Attributes,
}

/// 读取实体的断言集合。`include_candidates` 为 `false` 时不返回 candidate 关系。
pub fn load_entity_assertions(
    connection: &Connection,
    entity_id: &str,
    only_fresh: bool,
) -> Result<Vec<StoredEntityAssertion>, CoreError> {
    let mut statement = connection
        .prepare(
            "SELECT ea.id, ea.entity_id, e.kind, e.scope_id, e.identity_json,
                    a.producer_key, a.batch_id, ea.basis, ea.origin, ea.name, ea.attributes_json
               FROM entity_assertions ea
               JOIN assertions a ON a.id = ea.id
               JOIN entities e ON e.id = ea.entity_id
              WHERE ea.entity_id = ?1
                AND (?2 = 0 OR EXISTS (
                      SELECT 1 FROM current_batches cb
                       JOIN batches b ON b.id = cb.batch_id
                      WHERE b.id = ea.batch_id AND b.state = 'fresh'))",
        )
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot prepare entity assertion query: {error}"),
            )
        })?;
    let rows = statement
        .query_map(params![entity_id, if only_fresh { 1 } else { 0 }], |row| {
            let kind_text: String = row.get(2)?;
            let basis_text: String = row.get(7)?;
            let origin_text: String = row.get(8)?;
            let attributes_text: String = row.get(10)?;
            Ok(StoredEntityAssertion {
                assertion_id: row.get(0)?,
                entity_id: row.get(1)?,
                kind: EntityKind::parse(&kind_text).unwrap_or(EntityKind::File),
                scope_id: row.get(3)?,
                identity_json: row.get(4)?,
                producer_key: row.get(5)?,
                batch_id: row.get(6)?,
                basis: Basis::parse(&basis_text),
                origin: Origin::parse(&origin_text),
                name: row.get(9)?,
                attributes: serde_json::from_str(&attributes_text).unwrap_or_default(),
            })
        })
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot query entity assertions: {error}"),
            )
        })?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot read entity assertion: {error}"),
            )
        })?);
    }
    Ok(result)
}

/// 读取断言引用的证据 ID。
pub fn load_assertion_evidence(
    connection: &Connection,
    assertion_id: &str,
) -> Result<Vec<String>, CoreError> {
    let mut statement = connection
        .prepare(
            "SELECT evidence_id FROM assertion_evidence WHERE assertion_id = ?1 ORDER BY evidence_id",
        )
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot prepare assertion evidence query: {error}"),
            )
        })?;
    let rows = statement
        .query_map(params![assertion_id], |row| row.get::<_, String>(0))
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot query assertion evidence: {error}"),
            )
        })?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot read assertion evidence: {error}"),
            )
        })?);
    }
    Ok(result)
}

/// 断言 ID：`(kind, batchId, payloadHash)` 的稳定派生。
pub fn assertion_id(kind: &str, batch_id: &str, payload_hash: &str) -> String {
    format!(
        "as_{}",
        content_hash(format!("{kind}\u{1f}{batch_id}\u{1f}{payload_hash}").as_bytes())
    )
}

/// 构造实体身份行载荷。
pub fn entity_row<'a>(entity: &'a Entity, identity_json: &'a str) -> EntityRow<'a> {
    EntityRow {
        id: &entity.id,
        kind: entity.kind,
        scope_id: &entity.scope_id,
        identity_json,
    }
}
