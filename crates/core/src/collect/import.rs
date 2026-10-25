//! 采集输入导出与结果导入（计划 §3.5、§3.6）。
//!
//! 宿主导出的清单与完整性具有权威性：插件只能看到声明选择器命中的来源，
//! 以及已导出的实体/关系断言。本模块的导出结果是正式快照，不包含未激活的新实体。

use std::collections::BTreeMap;

use rusqlite::Connection;

use crate::contract::collector::{
    AssertionInput, CandidateSet, CollectorInput, EntityInput, EntitySelector, RelationInput,
    SourcePayload, SourceSet, StageSpec,
};
use crate::contract::entity::{Basis, EntityKind, Origin};
use crate::contract::error::{CoreError, codes};
use crate::contract::evidence::SourceClass;
use crate::contract::identity::{
    compute_entity_id, compute_source_id, identity_slots_from_canonical,
};
use crate::contract::relation::{RelationKind, Resolution};
use crate::project::config::LoadedProject;
use crate::project::scan::GlobRules;
use crate::storage;

/// 导出阶段输入。
///
/// `source_snapshot_id` 指向正式基线；`input_snapshot_id` 额外绑定已成功的
/// staging 上游批次。插件引用的 `entityId` 只在本次输入中可引用。
pub fn export_input(
    connection: &Connection,
    project: &LoadedProject,
    stage: &StageSpec,
    source_snapshot_id: &str,
    run_id: &str,
    plugin_id: &str,
    version: &str,
) -> Result<CollectorInput, CoreError> {
    let mut sources: Vec<SourcePayload> = Vec::new();
    let mut source_sets: Vec<SourceSet> = Vec::new();

    for selector in &stage.source_selectors {
        let rules = GlobRules::compile(&selector.include, &selector.exclude)
            .map_err(|reason| CoreError::new(codes::CONFIG_INVALID, reason))?;
        let mut source_ids: Vec<String> = Vec::new();
        for (repository_id, root) in &project.repositories {
            if !selector.repository_ids.is_empty()
                && !selector.repository_ids.iter().any(|id| id == repository_id)
            {
                continue;
            }
            let mut statement = match connection.prepare(
                "SELECT s.id, s.path, s.current_hash, s.selection_status
                   FROM sources s
                   JOIN snapshot_sources ss
                     ON ss.source_id = s.id AND ss.snapshot_id = ?1
                  WHERE s.repository_id = ?2 AND s.selection_status = 'selected'
                  ORDER BY s.path",
            ) {
                Ok(statement) => statement,
                Err(error) => {
                    return Err(CoreError::new(
                        codes::INTERNAL_ERROR,
                        format!("cannot prepare source export: {error}"),
                    ));
                }
            };
            let rows = statement
                .query_map(
                    rusqlite::params![source_snapshot_id, repository_id],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, Option<String>>(2)?,
                            row.get::<_, String>(3)?,
                        ))
                    },
                )
                .map_err(|error| {
                    CoreError::new(
                        codes::INTERNAL_ERROR,
                        format!("cannot query sources: {error}"),
                    )
                })?;
            for row in rows {
                let (source_id, path, current_hash, _status) = row.map_err(|error| {
                    CoreError::new(
                        codes::INTERNAL_ERROR,
                        format!("cannot read source row: {error}"),
                    )
                })?;
                if !rules.is_selected(&path) {
                    continue;
                }
                let Some(content_hash) = current_hash else {
                    continue;
                };
                let Some((bytes, _line_map)) =
                    storage::sources::read_source_version(connection, &source_id, &content_hash)?
                else {
                    continue;
                };
                let content = String::from_utf8(bytes).map_err(|_| {
                    CoreError::new(
                        codes::SOURCE_CHANGED,
                        format!("source `{path}` is not valid UTF-8"),
                    )
                })?;
                let source = crate::contract::evidence::Source {
                    id: source_id.clone(),
                    repository_id: repository_id.clone(),
                    path: path.clone(),
                    source_class: SourceClass::Code,
                    language: None,
                    content_hash: content_hash.clone(),
                    byte_length: content.len() as u64,
                };
                sources.push(SourcePayload { source, content });
                source_ids.push(source_id);
                let _ = root;
            }
            let digest = crate::contract::identity::content_hash(source_ids.join("\n").as_bytes());
            source_sets.push(SourceSet {
                selector_id: selector.id.clone(),
                digest,
                source_ids: source_ids.clone(),
                complete: true,
            });
        }
    }

    // 已存在的实体与关系（来自当前批次）。
    let (entities, relations) = export_entities(connection, &stage.entity_selectors)?;

    Ok(CollectorInput {
        protocol_version: crate::contract::PROTOCOL_VERSION.to_string(),
        run_id: run_id.to_string(),
        plugin_id: plugin_id.to_string(),
        version: version.to_string(),
        stage_id: stage.id.clone(),
        snapshot: crate::contract::evidence::SnapshotRef {
            index_id: String::new(),
            generation: "0".to_string(),
            source_snapshot_id: Some(source_snapshot_id.to_string()),
        },
        input_snapshot_id: source_snapshot_id.to_string(),
        config_subset: serde_json::json!({
            "projectId": project.config.project_id,
        }),
        sources,
        source_sets,
        entities,
        relations,
        candidate_sets: Vec::<CandidateSet>::new(),
        allowed_extensions: Vec::new(),
        limits: crate::contract::collector::Limits::default(),
    })
}

/// 导出已存在的实体与关系断言。
fn export_entities(
    connection: &Connection,
    selectors: &[EntitySelector],
) -> Result<(Vec<EntityInput>, Vec<RelationInput>), CoreError> {
    let mut entities: Vec<EntityInput> = Vec::new();
    let mut relations: Vec<RelationInput> = Vec::new();
    let mut seen_entities: BTreeMap<String, ()> = BTreeMap::new();

    for selector in selectors {
        let mut statement = connection
            .prepare(
                "SELECT e.id, e.kind, e.scope_id, e.identity_json
                   FROM entities e
                   JOIN entity_assertions ea ON ea.entity_id = e.id
                   JOIN current_batches cb ON cb.batch_id = ea.batch_id
                   JOIN batches b ON b.id = cb.batch_id
                  WHERE b.state = 'fresh'
                  ORDER BY e.id",
            )
            .map_err(|error| {
                CoreError::new(
                    codes::INTERNAL_ERROR,
                    format!("cannot prepare entity export: {error}"),
                )
            })?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(|error| {
                CoreError::new(
                    codes::INTERNAL_ERROR,
                    format!("cannot query entities: {error}"),
                )
            })?;
        for row in rows {
            let (entity_id, kind_text, scope_id, identity_json) = row.map_err(|error| {
                CoreError::new(
                    codes::INTERNAL_ERROR,
                    format!("cannot read entity row: {error}"),
                )
            })?;
            let kind = EntityKind::parse(&kind_text).unwrap_or(EntityKind::File);
            if !selector.kinds.is_empty() && !selector.kinds.contains(&kind) {
                continue;
            }
            if !selector.scope_ids.is_empty()
                && !selector.scope_ids.iter().any(|id| id == &scope_id)
            {
                continue;
            }
            if seen_entities.contains_key(&entity_id) {
                continue;
            }
            seen_entities.insert(entity_id.clone(), ());
            let identity = identity_slots_from_canonical(kind, &identity_json).unwrap_or_default();
            let assertions =
                storage::entities::load_entity_assertions(connection, &entity_id, true)?;
            entities.push(EntityInput {
                entity_id: entity_id.clone(),
                kind,
                scope_id,
                identity,
                assertions: assertions
                    .into_iter()
                    .map(|assertion| AssertionInput {
                        assertion_id: assertion.assertion_id,
                        producer_key: assertion.producer_key,
                        batch_id: assertion.batch_id,
                        basis: assertion.basis,
                        origin: assertion.origin,
                        name: assertion.name,
                        attributes: assertion.attributes,
                        evidence_refs: Vec::new(),
                        resolution: None,
                    })
                    .collect(),
            });
        }
    }

    for selector in selectors {
        let mut statement = connection
            .prepare(
                "SELECT r.id, r.kind, r.from_id, r.to_id, r.qualifiers_json
                   FROM relations r
                   JOIN relation_assertions ra ON ra.relation_id = r.id
                   JOIN current_batches cb ON cb.batch_id = ra.batch_id
                   JOIN batches b ON b.id = cb.batch_id
                  WHERE b.state = 'fresh'
                  ORDER BY r.id",
            )
            .map_err(|error| {
                CoreError::new(
                    codes::INTERNAL_ERROR,
                    format!("cannot prepare relation export: {error}"),
                )
            })?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })
            .map_err(|error| {
                CoreError::new(
                    codes::INTERNAL_ERROR,
                    format!("cannot query relations: {error}"),
                )
            })?;
        for row in rows {
            let (relation_id, kind_text, from_id, to_id, qualifiers_json) =
                row.map_err(|error| {
                    CoreError::new(
                        codes::INTERNAL_ERROR,
                        format!("cannot read relation row: {error}"),
                    )
                })?;
            let kind = RelationKind::parse(&kind_text).unwrap_or(RelationKind::References);
            if !selector.kinds.is_empty() && !selector.kinds.contains(&EntityKind::File) {
                // 关系选择器按 kind 过滤；没有匹配时跳过。
                continue;
            }
            let qualifiers: serde_json::Map<String, serde_json::Value> =
                serde_json::from_str(&qualifiers_json).unwrap_or_default();
            let assertions =
                storage::relations::load_relation_assertions(connection, &relation_id, true)?;
            relations.push(RelationInput {
                relation_id: relation_id.clone(),
                kind,
                from_id,
                to_id,
                qualifiers,
                assertions: assertions
                    .into_iter()
                    .map(|assertion| AssertionInput {
                        assertion_id: assertion.assertion_id,
                        producer_key: assertion.producer_key,
                        batch_id: assertion.batch_id,
                        basis: assertion.basis,
                        origin: assertion.origin,
                        name: None,
                        attributes: serde_json::Map::new(),
                        evidence_refs: Vec::new(),
                        resolution: Some(assertion.resolution),
                    })
                    .collect(),
            });
        }
    }

    Ok((entities, relations))
}

/// 导出实体的来源 ID（供证据校验使用）。
pub fn source_id_for(project_id: &str, repository_id: &str, path: &str) -> String {
    compute_source_id(project_id, repository_id, path)
}

/// 导出实体的稳定 ID。
pub fn entity_id_for(
    project_id: &str,
    kind: EntityKind,
    slots: &[crate::contract::identity::IdentitySlot],
) -> Result<String, CoreError> {
    compute_entity_id(project_id, kind, slots)
        .map_err(|error| CoreError::new(codes::IDENTITY_CONFLICT, error.to_string()))
}

/// 依据与来源的辅助别名（保持契约一致性可见）。
pub fn basis_of(value: &str) -> Basis {
    Basis::parse(value)
}

/// 断言来源的辅助别名。
pub fn origin_of(value: &str) -> Origin {
    Origin::parse(value)
}

/// 关系解析状态的辅助别名。
pub fn resolution_of(value: &str) -> Resolution {
    Resolution::parse(value)
}
