//! 从一致快照导出输入；候选集合包含完整断言，零匹配同样有摘要。
use crate::contract::collector::*;
use crate::contract::entity::{Basis, EntityKind, Origin};
use crate::contract::error::{CoreError, codes};
use crate::contract::evidence::{SnapshotRef, Source, SourceClass};
use crate::contract::identity::{
    compute_entity_id, compute_source_id, content_hash, identity_slots_from_canonical,
};
use crate::contract::relation::{RelationKind, Resolution};
use crate::project::{config::LoadedProject, scan::GlobRules};
use crate::storage;
use rusqlite::Connection;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) use crate::storage::database_error;

/// Maintenance references are minted by Core and tied to the exported bytes.
pub fn context_references(
    connection: &Connection,
    input: &CollectorInput,
) -> Result<Vec<crate::contract::evidence::SourceReference>, CoreError> {
    let mut refs = Vec::new();
    for payload in &input.sources {
        if payload.content.is_empty()
            || matches!(
                payload.source.source_class,
                SourceClass::GeneratedKnowledge | SourceClass::DerivedConvention
            )
            || crate::project::scan::is_hard_excluded(&payload.source.path)
        {
            continue;
        }
        let end = payload.content.len() as u64;
        let id = crate::contract::identity::compute_evidence_id(
            &payload.source.id,
            &payload.source.content_hash,
            0,
            end,
            "agent-context",
        );
        let map = crate::project::snapshot::LineMap::build(payload.content.as_bytes());
        storage::insert_evidence(
            connection,
            &crate::contract::evidence::Evidence {
                id: id.clone(),
                source_id: payload.source.id.clone(),
                content_hash: payload.source.content_hash.clone(),
                start_byte: 0,
                end_byte: end,
                start_line: 1,
                end_line: map.end_line_of(0, end),
                role: "agent-context".into(),
            },
        )?;
        refs.push(crate::contract::evidence::SourceReference {
            snapshot: input.snapshot.clone(),
            evidence_id: id,
            source_id: payload.source.id.clone(),
            content_hash: payload.source.content_hash.clone(),
            start_byte: 0,
            end_byte: end,
        });
    }
    Ok(refs)
}

pub fn export_input(
    connection: &Connection,
    project: &LoadedProject,
    stage: &StageSpec,
    source_snapshot_id: &str,
    run_id: &str,
    plugin_id: &str,
    version: &str,
) -> Result<CollectorInput, CoreError> {
    let mut sources = BTreeMap::new();
    let mut source_sets = Vec::new();
    for selector in &stage.source_selectors {
        let rules = GlobRules::compile(&selector.include, &selector.exclude)
            .map_err(|reason| CoreError::new(codes::CONFIG_INVALID, reason))?;
        let mut statement = connection.prepare(
            "SELECT s.id, s.repository_id, s.path, ss.content_hash, ss.selection_status, sv.source_class, sv.content
             FROM snapshot_sources ss JOIN sources s ON s.id = ss.source_id
             LEFT JOIN source_versions sv ON sv.source_id = ss.source_id AND sv.content_hash = ss.content_hash
             WHERE ss.snapshot_id = ?1 ORDER BY s.repository_id, s.path").map_err(database_error)?;
        let rows = statement
            .query_map([source_snapshot_id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, Option<String>>(5)?,
                    r.get::<_, Option<Vec<u8>>>(6)?,
                ))
            })
            .map_err(database_error)?;
        let mut members = Vec::new();
        let mut source_ids = Vec::new();
        let mut complete = true;
        for row in rows {
            let (id, repository_id, path, hash, status, class, bytes) =
                row.map_err(database_error)?;
            if (!selector.repository_ids.is_empty()
                && !selector.repository_ids.contains(&repository_id))
                || !rules.is_selected(&path)
            {
                continue;
            }
            members.push(serde_json::json!([repository_id, path, hash, status]));
            if status != "selected" {
                complete &= status != "skipped";
                continue;
            }
            let hash =
                hash.ok_or_else(|| CoreError::new(codes::SOURCE_CHANGED, "snapshot hash missing"))?;
            let bytes = bytes
                .ok_or_else(|| CoreError::new(codes::SOURCE_CHANGED, "snapshot content missing"))?;
            let content = String::from_utf8(bytes)
                .map_err(|_| CoreError::new(codes::SOURCE_CHANGED, "source is not UTF-8"))?;
            let source_class: SourceClass = serde_json::from_value(serde_json::json!(class))
                .map_err(|_| {
                    CoreError::new(codes::EVIDENCE_INVALID, "unknown stored source class")
                })?;
            sources.insert(
                id.clone(),
                SourcePayload {
                    source: Source {
                        id: id.clone(),
                        repository_id,
                        path,
                        source_class,
                        language: None,
                        content_hash: hash,
                        byte_length: content.len() as u64,
                    },
                    content,
                },
            );
            source_ids.push(id);
        }
        source_ids.sort();
        source_sets.push(SourceSet {
            selector_id: selector.id.clone(),
            digest: digest(&(selector, members)),
            source_ids,
            complete,
        });
    }
    let (entities, relations, candidate_sets) =
        export_entities(connection, &stage.entity_selectors)?;
    let referenced: BTreeSet<_> = entities
        .iter()
        .flat_map(|e| &e.assertions)
        .chain(relations.iter().flat_map(|r| &r.assertions))
        .flat_map(|assertion| &assertion.evidence_refs)
        .collect();
    let mut evidence = Vec::new();
    let mut statement = connection.prepare(
        "SELECT source_id,content_hash,start_byte,end_byte,start_line,end_line,role FROM evidence WHERE id=?1"
    ).map_err(database_error)?;
    for id in referenced {
        let record = statement
            .query_row([id], |row| {
                Ok(crate::contract::evidence::Evidence {
                    id: id.clone(),
                    source_id: row.get(0)?,
                    content_hash: row.get(1)?,
                    start_byte: row.get::<_, i64>(2)? as u64,
                    end_byte: row.get::<_, i64>(3)? as u64,
                    start_line: row.get::<_, i64>(4)? as u64,
                    end_line: row.get::<_, i64>(5)? as u64,
                    role: row.get(6)?,
                })
            })
            .map_err(database_error)?;
        if sources
            .get(&record.source_id)
            .is_some_and(|source| source.source.content_hash == record.content_hash)
        {
            evidence.push(record);
        }
    }
    let snapshot = connection
        .query_row(
            "SELECT index_id, generation FROM index_meta WHERE id = 1",
            [],
            |r| {
                Ok(SnapshotRef {
                    index_id: r.get(0)?,
                    generation: r.get::<_, i64>(1)?.to_string(),
                    source_snapshot_id: Some(source_snapshot_id.to_string()),
                })
            },
        )
        .map_err(database_error)?;
    let input_snapshot_id = format!(
        "input_{}",
        digest(&(
            source_snapshot_id,
            &source_sets,
            &candidate_sets,
            storage::current_batches(connection)?
        ))
    );
    Ok(CollectorInput {
        protocol_version: crate::contract::PROTOCOL_VERSION.to_string(),
        run_id: run_id.to_string(),
        plugin_id: plugin_id.to_string(),
        version: version.to_string(),
        stage_id: stage.id.clone(),
        snapshot,
        input_snapshot_id,
        config_subset: serde_json::to_value(&project.config).expect("serializable config"),
        sources: sources.into_values().collect(),
        source_sets,
        entities,
        relations,
        evidence,
        candidate_sets,
        allowed_extensions: storage::plugins::active_manifests(connection)?
            .iter()
            .flat_map(|m| {
                m.extensions
                    .entities
                    .iter()
                    .map(|e| e.name.as_str().to_string())
                    .chain(
                        m.extensions
                            .relations
                            .iter()
                            .map(|r| r.name.as_str().to_string()),
                    )
            })
            .collect(),
        limits: Default::default(),
    })
}

pub(crate) fn digest(value: &impl serde::Serialize) -> String {
    content_hash(&serde_json::to_vec(value).expect("serializable binding"))
}

fn evidence_refs(connection: &Connection, assertion_id: &str) -> Result<Vec<String>, CoreError> {
    let mut s = connection.prepare("SELECT evidence_id FROM assertion_evidence WHERE assertion_id = ?1 ORDER BY evidence_id").map_err(database_error)?;
    s.query_map([assertion_id], |r| r.get(0))
        .map_err(database_error)?
        .collect::<Result<Vec<String>, _>>()
        .map_err(database_error)
}

fn matches_selector(selector: &EntitySelector, kind: &EntityKind, scope: &str) -> bool {
    (selector.kinds.is_empty() || selector.kinds.contains(kind))
        && (selector.scope_ids.is_empty() || selector.scope_ids.iter().any(|id| id == scope))
}

fn export_entities(
    connection: &Connection,
    selectors: &[EntitySelector],
) -> Result<(Vec<EntityInput>, Vec<RelationInput>, Vec<CandidateSet>), CoreError> {
    let mut entities = Vec::new();
    let mut statement = connection.prepare("SELECT DISTINCT e.id, e.kind, e.scope_id, e.identity_json FROM entities e
        JOIN entity_assertions ea ON ea.entity_id = e.id JOIN current_batches cb ON cb.batch_id = ea.batch_id
        JOIN batches b ON b.id = cb.batch_id WHERE b.state = 'fresh' ORDER BY e.id").map_err(database_error)?;
    let rows = statement
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
            ))
        })
        .map_err(database_error)?;
    for row in rows {
        let (entity_id, kind, scope_id, identity) = row.map_err(database_error)?;
        let kind = EntityKind::parse(&kind)
            .ok_or_else(|| CoreError::new(codes::SCHEMA_INVALID, "unknown entity kind"))?;
        if !selectors
            .iter()
            .any(|s| matches_selector(s, &kind, &scope_id))
        {
            continue;
        }
        let identity = identity_slots_from_canonical(kind.clone(), &identity)
            .map_err(|e| CoreError::new(codes::IDENTITY_CONFLICT, e.to_string()))?;
        let assertions = storage::load_entity_assertions(connection, &entity_id, true)?
            .into_iter()
            .map(|a| {
                Ok(AssertionInput {
                    evidence_refs: evidence_refs(connection, &a.assertion_id)?,
                    assertion_id: a.assertion_id,
                    producer_key: a.producer_key,
                    batch_id: a.batch_id,
                    basis: a.basis,
                    origin: a.origin,
                    name: a.name,
                    attributes: a.attributes,
                    resolution: None,
                })
            })
            .collect::<Result<Vec<_>, CoreError>>()?;
        entities.push(EntityInput {
            entity_id,
            kind,
            scope_id,
            identity,
            assertions,
        });
    }
    let ids: BTreeSet<_> = entities.iter().map(|e| e.entity_id.as_str()).collect();
    let mut relations = Vec::new();
    let mut statement = connection.prepare("SELECT DISTINCT r.id, r.kind, r.from_id, r.to_id, r.qualifiers_json FROM relations r
        JOIN relation_assertions ra ON ra.relation_id = r.id JOIN current_batches cb ON cb.batch_id = ra.batch_id
        JOIN batches b ON b.id = cb.batch_id WHERE b.state = 'fresh' ORDER BY r.id").map_err(database_error)?;
    let rows = statement
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
            ))
        })
        .map_err(database_error)?;
    for row in rows {
        let (relation_id, kind, from_id, to_id, qualifiers) = row.map_err(database_error)?;
        if !ids.contains(from_id.as_str()) || !ids.contains(to_id.as_str()) {
            continue;
        }
        let kind = RelationKind::parse(&kind)
            .ok_or_else(|| CoreError::new(codes::SCHEMA_INVALID, "unknown relation kind"))?;
        let qualifiers = serde_json::from_str(&qualifiers)
            .map_err(|e| CoreError::new(codes::SCHEMA_INVALID, e.to_string()))?;
        let assertions = storage::load_relation_assertions(connection, &relation_id, true)?
            .into_iter()
            .map(|a| {
                Ok(AssertionInput {
                    evidence_refs: evidence_refs(connection, &a.assertion_id)?,
                    assertion_id: a.assertion_id,
                    producer_key: a.producer_key,
                    batch_id: a.batch_id,
                    basis: a.basis,
                    origin: a.origin,
                    name: None,
                    attributes: Default::default(),
                    resolution: Some(a.resolution),
                })
            })
            .collect::<Result<Vec<_>, CoreError>>()?;
        relations.push(RelationInput {
            relation_id,
            kind,
            from_id,
            to_id,
            qualifiers,
            assertions,
        });
    }
    let candidate_sets = selectors
        .iter()
        .map(|selector| {
            let selected: Vec<_> = entities
                .iter()
                .filter(|e| matches_selector(selector, &e.kind, &e.scope_id))
                .collect();
            let ids: BTreeSet<_> = selected.iter().map(|entity| &entity.entity_id).collect();
            let bindings: Vec<_> = relations
                .iter()
                .filter(|relation| ids.contains(&relation.from_id) || ids.contains(&relation.to_id))
                .collect();
            CandidateSet {
                selector_id: selector.id.clone(),
                digest: digest(&(selector, &selected, &bindings)),
                entity_ids: selected.iter().map(|e| e.entity_id.clone()).collect(),
                complete: true,
            }
        })
        .collect();
    Ok((entities, relations, candidate_sets))
}

pub fn source_id_for(project_id: &str, repository_id: &str, path: &str) -> String {
    compute_source_id(project_id, repository_id, path)
}
pub fn entity_id_for(
    project_id: &str,
    kind: EntityKind,
    slots: &[crate::contract::identity::IdentitySlot],
) -> Result<String, CoreError> {
    compute_entity_id(project_id, kind, slots)
        .map_err(|e| CoreError::new(codes::IDENTITY_CONFLICT, e.to_string()))
}
pub fn basis_of(value: &str) -> Basis {
    Basis::parse(value)
}
pub fn origin_of(value: &str) -> Origin {
    Origin::parse(value)
}
pub fn resolution_of(value: &str) -> Resolution {
    Resolution::parse(value)
}
