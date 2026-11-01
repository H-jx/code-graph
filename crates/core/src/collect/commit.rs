//! 采集结果导入与批次替换（计划 §3.6 的结果事务 B）。
//!
//! 事务 B 对目标 `replacementKey` 全量替换断言、证据引用、依赖、coverage、搜索投影与
//! `current_batches`：旧批次被标记为 stale（单向失效，保留用于诊断），新批次成为
//! 当前可见事实；成功空输出同样替换。任一步失败整组回滚。

use rusqlite::{Connection, params};
use serde_json::Value;

use crate::contract::collector::{Coverage, Dependency, Gap};
use crate::contract::entity::{Basis, Entity, Origin};
use crate::contract::error::{CoreError, codes};
use crate::contract::evidence::Evidence;
use crate::contract::identity::canonical_entity_identity;
use crate::contract::relation::{Relation, Resolution};
use crate::search::documents::SearchDocument;
use crate::storage;

/// 一个待提交的批次。
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct StagedBatch {
    pub project_id: String,
    pub batch_id: String,
    pub run_id: String,
    pub producer_key: String,
    pub replacement_key: String,
    pub artifact_hash: String,
    pub input_digest: String,
    pub entities: Vec<StagedEntity>,
    pub relations: Vec<StagedRelation>,
    pub evidence: Vec<Evidence>,
    pub dependencies: Vec<Dependency>,
    pub coverage: Vec<Coverage>,
    pub gaps: Vec<Gap>,
    pub documents: Vec<SearchDocument>,
}

/// 待提交实体及其断言。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StagedEntity {
    pub entity: Entity,
    pub name: Option<String>,
    pub evidence_refs: Vec<String>,
    pub basis: Basis,
    pub origin: Origin,
}

/// 待提交关系及其断言。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StagedRelation {
    pub relation: Relation,
    pub evidence_refs: Vec<String>,
    pub basis: Basis,
    pub origin: Origin,
    pub resolution: Resolution,
}

/// 提交批次（必须在写事务内调用）。
///
/// 顺序：标记旧批次 stale → 写入新批次与全部事实 → 切换 `current_batches`。
pub fn commit_batch(
    connection: &Connection,
    staged: &StagedBatch,
    generation: u64,
    now: &str,
) -> Result<(), CoreError> {
    // 旧批次单向失效。
    let previous = storage::runs::current_batch(connection, &staged.replacement_key)?;
    if let Some(previous) = previous {
        storage::knowledge::invalidate_batches(
            connection,
            std::slice::from_ref(&staged.replacement_key),
        )?;
        connection
            .execute(
                "UPDATE batches SET state = 'stale', invalidated_generation = COALESCE(invalidated_generation, ?2)
                  WHERE id = ?1",
                params![previous, generation as i64],
            )
            .map_err(|error| {
                CoreError::new(
                    codes::INTERNAL_ERROR,
                    format!("cannot invalidate previous batch: {error}"),
                )
            })?;
    }

    storage::runs::create_batch(
        connection,
        &staged.batch_id,
        &staged.run_id,
        &staged.producer_key,
        &staged.replacement_key,
        &staged.artifact_hash,
        &staged.input_digest,
        now,
    )?;

    // 证据。
    for evidence in &staged.evidence {
        storage::entities::insert_evidence(connection, evidence)?;
    }

    // 实体与断言。
    for staged_entity in &staged.entities {
        let identity = canonical_entity_identity(
            &staged.project_id,
            staged_entity.entity.kind.clone(),
            &staged_entity.entity.identity,
        )
        .map_err(|error| {
            CoreError::new(
                codes::IDENTITY_CONFLICT,
                format!("cannot compute entity identity: {error}"),
            )
        })?;
        storage::entities::upsert_entity(
            connection,
            &storage::entities::EntityRow {
                id: &staged_entity.entity.id,
                kind: staged_entity.entity.kind.clone(),
                scope_id: &staged_entity.entity.scope_id,
                identity_json: &identity,
            },
        )?;
        let payload_hash = storage::entities::entity_payload_hash(
            &staged.producer_key,
            &staged.batch_id,
            &staged_entity.entity.id,
            staged_entity.name.as_deref(),
            &staged_entity.entity.attributes,
            staged_entity.basis,
            staged_entity.origin,
        )?;
        let assertion_id =
            storage::entities::assertion_id("entity", &staged.batch_id, &payload_hash);
        storage::entities::insert_entity_assertion(
            connection,
            &storage::entities::EntityAssertionRow {
                assertion_id: &assertion_id,
                entity_id: &staged_entity.entity.id,
                batch_id: &staged.batch_id,
                basis: staged_entity.basis,
                origin: staged_entity.origin,
                name: staged_entity.name.as_deref(),
                attributes: &staged_entity.entity.attributes,
            },
            &payload_hash,
            &staged.producer_key,
        )?;
        for evidence_id in &staged_entity.evidence_refs {
            storage::entities::link_assertion_evidence(connection, &assertion_id, evidence_id)?;
        }
    }

    // 关系与断言。
    for staged_relation in &staged.relations {
        storage::relations::upsert_relation(connection, &staged_relation.relation)?;
        let payload_hash = crate::contract::identity::content_hash(
            format!(
                "{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}",
                staged.batch_id,
                staged_relation.relation.id,
                staged_relation.relation.kind.as_str(),
                staged_relation.relation.from_id,
                staged_relation.relation.to_id,
                staged_relation.resolution.as_str()
            )
            .as_bytes(),
        );
        let assertion_id =
            storage::entities::assertion_id("relation", &staged.batch_id, &payload_hash);
        storage::relations::insert_relation_assertion(
            connection,
            &storage::relations::RelationAssertionRow {
                assertion_id: &assertion_id,
                relation_id: &staged_relation.relation.id,
                batch_id: &staged.batch_id,
                basis: staged_relation.basis,
                origin: staged_relation.origin,
                resolution: staged_relation.resolution,
            },
            &payload_hash,
            &staged.producer_key,
        )?;
        for evidence_id in &staged_relation.evidence_refs {
            storage::entities::link_assertion_evidence(connection, &assertion_id, evidence_id)?;
        }
    }

    // 依赖、覆盖与缺口。
    for dependency in &staged.dependencies {
        storage::insert_dependency(
            connection,
            &staged.batch_id,
            dependency.kind.as_str(),
            &dependency.key,
            &dependency.digest,
        )?;
    }
    for coverage in &staged.coverage {
        storage::insert_coverage(
            connection,
            &staged.batch_id,
            &coverage.capability,
            &coverage.scope_id,
            &coverage.source_set_id,
            coverage.status.as_str(),
            &coverage.counts,
            &coverage.gap_ids,
        )?;
    }
    for gap in &staged.gaps {
        let payload = serde_json::to_value(gap).unwrap_or(Value::Null);
        storage::insert_gap(connection, &gap.id, &staged.batch_id, &gap.code, &payload)?;
    }

    // 搜索投影。
    for document in &staged.documents {
        storage::search::insert_document(connection, document)?;
    }

    // 切换当前批次。
    storage::set_current_batch(connection, &staged.replacement_key, &staged.batch_id)?;
    crate::knowledge::documents::invalidate_changed(connection)?;
    Ok(())
}
