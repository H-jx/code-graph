//! External topic validation, dependency verification and cited conventions.
use super::context::{self, digest, invalid};
use crate::{
    contract::{
        Basis, Origin, SourceReference,
        error::{CoreError, codes},
        knowledge::*,
    },
    project::config::{LoadedProject, apply_identity_delta},
    query,
    storage::{self, Database, database_error},
};
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Write,
};

pub(crate) fn dependencies_match(
    connection: &Connection,
    project: Option<&LoadedProject>,
    context: &TopicContext,
    disk: Option<&BTreeMap<String, String>>,
) -> Result<bool, CoreError> {
    for d in &context.dependencies {
        let actual = match d.kind.as_str() {
            "config" => project.map(|p| p.config_hash()).transpose()?.unwrap_or(
                connection
                    .query_row("SELECT config_hash FROM index_meta WHERE id=1", [], |r| {
                        r.get(0)
                    })
                    .map_err(database_error)?,
            ),
            "prompt" => context::prompt_hash(),
            "template" => context::template_hash(),
            "conventions" => digest(&storage::knowledge::conventions(connection)?),
            "sourceSet" => {
                let indexed = context::indexed_source_set(connection, &d.key)?;
                if let Some(disk) = disk
                    && disk.get(&d.key) != Some(&d.digest)
                {
                    return Ok(false);
                }
                indexed
            }
            "artifact" => context::artifact_digest(connection, &d.key)?,
            "batch" => context::batch_digest(connection, &d.key)?,
            "selectors" => context::selector_digest(connection, &d.key)?,
            "plugins" => context::plugins_digest(
                connection,
                &serde_json::from_str(&d.key)
                    .map_err(|_| invalid("invalid plugin repository dependency"))?,
            )?,
            "facts" => context::facts_digest(
                connection,
                &serde_json::from_str(&d.key)
                    .map_err(|_| invalid("invalid fact repository dependency"))?,
            )?,
            _ => return Ok(false),
        };
        if actual != d.digest {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Read-only status: dependency checks do not rewrite the generation of a topic.
pub fn status(
    connection: &Connection,
    project: Option<&LoadedProject>,
    check_disk: bool,
) -> Result<Value, CoreError> {
    status_with_budget(connection, project, check_disk, &mut || true)
}
pub(crate) fn status_with_budget(
    connection: &Connection,
    project: Option<&LoadedProject>,
    check_disk: bool,
    visit: &mut impl FnMut() -> bool,
) -> Result<Value, CoreError> {
    let topics = storage::knowledge::topics(connection)?;
    let disk = if check_disk && !topics.is_empty() {
        project
            .map(|p| context::disk_sets_with_budget(p, visit))
            .transpose()
    } else {
        Ok(None)
    };
    let disk_available = matches!(&disk, Ok(Some(_)));
    let disk = disk.unwrap_or(None);
    let mut states = Vec::new();
    for t in topics {
        let matches = dependencies_match(connection, project, &t.context, disk.as_ref())?;
        let state = if t.state == "stale" || !matches {
            "stale"
        } else if disk_available {
            "fresh"
        } else {
            "unknown"
        };
        states.push(json!({"topicId":t.artifact.topic_id,"version":t.version,"freshness":state,"snapshot":t.artifact.snapshot,"inputDigest":t.artifact.input_digest,"dependenciesRechecked":disk_available&&matches,"gapIds":t.artifact.gaps.iter().map(|g|&g.id).collect::<Vec<_>>()}));
    }
    let mut stmt=connection.prepare("SELECT id,output_path,state,topic_versions_json,output_hash FROM knowledge_exports ORDER BY created_at,id").map_err(database_error)?;
    let exports=stmt.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?))).map_err(database_error)?.map(|r|{
        let(id,path,state,versions,hash)=r.map_err(database_error)?;let versions:Vec<Value>=serde_json::from_str(&versions).map_err(|_|invalid("invalid stored export versions"))?;
        let stale=versions.iter().any(|v|!states.iter().any(|s|s["topicId"]==v["topicId"]&&s["version"]==v["version"]&&s["freshness"]!="stale"));
        let integrity=if check_disk{super::skill::verify_export(std::path::Path::new(&path),&hash,visit)}else{None};
        Ok(json!({"id":id,"path":path,"integrity":integrity,"freshness":if state=="stale"||stale||integrity==Some(false){"stale"}else if disk_available&&integrity==Some(true){"fresh"}else{"unknown"}}))
    }).collect::<Result<Vec<_>,CoreError>>()?;
    let mut stmt=connection.prepare("SELECT id,output_path,state,error_code FROM knowledge_export_attempts ORDER BY created_at,id").map_err(database_error)?;
    let attempts=stmt.query_map([],|r|Ok(json!({"id":r.get::<_,String>(0)?,"path":r.get::<_,String>(1)?,"state":r.get::<_,String>(2)?,"errorCode":r.get::<_,Option<String>>(3)?}))).map_err(database_error)?.collect::<Result<Vec<_>,_>>().map_err(database_error)?;
    Ok(json!({"topics":states,"exports":exports,"attempts":attempts,"diskChecked":disk_available}))
}

/// Maintenance persists staleness before running work which may fail or be cancelled.
pub(crate) fn invalidate_changed(connection: &Connection) -> Result<(), CoreError> {
    for topic in storage::knowledge::topics(connection)? {
        if !dependencies_match(connection, None, &topic.context, None)? {
            storage::knowledge::mark_stale(connection, &topic.artifact.topic_id, topic.version)?;
        }
    }
    Ok(())
}

pub fn refresh(database: &mut Database, project: &LoadedProject) -> Result<(), CoreError> {
    if storage::knowledge::topics(database.connection())?.is_empty() {
        return Ok(());
    }
    let report = status(database.connection(), Some(project), true)?;
    let tx = database.write_transaction()?;
    for topic in report["topics"].as_array().unwrap() {
        if topic["freshness"] == "stale" {
            storage::knowledge::mark_stale(
                &tx,
                topic["topicId"].as_str().unwrap(),
                topic["version"].as_u64().unwrap(),
            )?;
        }
    }
    tx.commit().map_err(database_error)
}

pub(crate) fn validate_references(
    database: &Database,
    project: &LoadedProject,
    references: &[SourceReference],
) -> Result<(), CoreError> {
    let snapshot = query::snapshot_ref(database)?;
    let mut seen = BTreeSet::new();
    for r in references {
        if r.snapshot != snapshot {
            return Err(CoreError::new(
                codes::SNAPSHOT_EXPIRED,
                "original reference belongs to another snapshot",
            ));
        }
        if !seen.insert(&r.evidence_id) {
            return Err(invalid("duplicate evidence reference"));
        }
        storage::sources::validate_original_reference(database.connection(), r)?;
        let (repo, path) = storage::source_path(database.connection(), &r.source_id)?
            .ok_or_else(|| invalid("missing source"))?;
        let root = project
            .repositories
            .get(&repo)
            .ok_or_else(|| invalid("missing repository"))?;
        let read = crate::evidence::read::read_source(
            r,
            root,
            &path,
            None,
            false,
            "knowledge-validation",
        )?;
        if read.status != crate::contract::SourceReadStatus::Current {
            return Err(CoreError::new(
                codes::SOURCE_CHANGED,
                "original reference is no longer current",
            ));
        }
    }
    Ok(())
}

pub fn import_topic(
    database: &mut Database,
    project: &LoadedProject,
    artifact: &TopicArtifact,
    now: &str,
) -> Result<Value, CoreError> {
    import_topic_with_control(
        database,
        project,
        artifact,
        now,
        &super::KnowledgeControl::default(),
    )
}
pub fn import_topic_with_control(
    database: &mut Database,
    project: &LoadedProject,
    artifact: &TopicArtifact,
    now: &str,
    control: &super::KnowledgeControl,
) -> Result<Value, CoreError> {
    control.check()?;
    let _lock = crate::collect::registry::MaintenanceLock::acquire(project)?;
    refresh(database, project)?;
    crate::contract::check_protocol_version(&artifact.protocol_version)?;
    if artifact.snapshot != query::snapshot_ref(database)? {
        return Err(CoreError::new(
            codes::SNAPSHOT_EXPIRED,
            "topic must use the current index snapshot",
        ));
    }
    let context = storage::knowledge::receipt(database.connection(), &artifact.input_digest)?;
    if artifact.topic_id != context.topic_id
        || artifact.snapshot != context.snapshot
        || artifact.prompt_hash != context.prompt_hash
        || artifact.template_hash != context.template_hash
    {
        return Err(invalid("topic does not match its Core context receipt"));
    }
    let disk = context::disk_sets_with_budget(project, &mut || control.check().is_ok());
    control.check()?;
    let disk = disk?;
    if !dependencies_match(database.connection(), Some(project), &context, Some(&disk))? {
        return Err(CoreError::new(
            codes::DEPENDENCY_CHANGED,
            "topic input dependencies have changed",
        ));
    }
    if !context::valid_id(&artifact.topic_id)
        || artifact.title.trim().is_empty()
        || artifact.claims.is_empty()
        || artifact.markdown.len() > 256_000
    {
        return Err(invalid(
            "topic needs a safe id, title and bounded cited claims",
        ));
    }
    for reference in &artifact.source_references {
        control.check()?;
        validate_references(database, project, std::slice::from_ref(reference))?;
    }
    let unique: BTreeSet<_> = artifact
        .source_references
        .iter()
        .map(|r| &r.evidence_id)
        .collect();
    if unique.len() != artifact.source_references.len() {
        return Err(invalid("duplicate evidence reference"));
    }
    for r in &artifact.source_references {
        if !context.source_references.contains(r) {
            return Err(CoreError::new(
                codes::EVIDENCE_INVALID,
                "topic references must come from the exported context",
            ));
        }
    }
    // A flat expansion to original references makes derived/topic cycles unrepresentable.
    let refs: BTreeSet<_> = artifact
        .source_references
        .iter()
        .map(|r| r.evidence_id.as_str())
        .collect();
    let gap_ids: BTreeSet<_> = artifact.gaps.iter().map(|g| g.id.as_str()).collect();
    if gap_ids.len() != artifact.gaps.len()
        || context.gaps.iter().any(|g| !artifact.gaps.contains(g))
    {
        return Err(invalid(
            "topic must retain all context gaps without rewriting them",
        ));
    }
    let mut ids = BTreeSet::new();
    let markdown_link = regex::Regex::new(r"\]\s*\(|<[^>]+>").unwrap();
    if artifact.title.contains(['\n', '\r']) || markdown_link.is_match(&artifact.title) {
        return Err(invalid("topic title must be plain text"));
    }
    for c in &artifact.claims {
        control.check()?;
        if c.text.contains(['\n', '\r']) || markdown_link.is_match(&c.text) {
            return Err(invalid(
                "claim text must be plain text; use structured original references for citations",
            ));
        }
        if !context::valid_id(&c.id) || !ids.insert(c.id.as_str()) || c.text.trim().is_empty() {
            return Err(invalid(
                "claim ids must be unique safe identifiers with nonempty text",
            ));
        }
        if c.evidence_refs.iter().any(|id| !refs.contains(id.as_str()))
            || c.gap_ids.iter().any(|id| !gap_ids.contains(id.as_str()))
        {
            return Err(CoreError::new(
                codes::REFERENCE_MISSING,
                "claim refers to unknown original evidence or gap",
            ));
        }
        if (c.origin == Origin::Ai && c.basis != Basis::Inferred)
            || (c.basis != Basis::Inferred && c.evidence_refs.is_empty())
            || (c.basis == Basis::Inferred && c.gap_ids.is_empty())
        {
            return Err(invalid(
                "AI claims must be inferred; facts need original evidence; inferences need explicit gaps",
            ));
        }
        if !artifact.markdown.contains(&format!("[[claim:{}]]", c.id)) {
            return Err(invalid(
                "each claim needs a [[claim:id]] marker in markdown",
            ));
        }
    }
    let marker = regex::Regex::new(r"\[\[claim:([A-Za-z0-9_-]+)\]\]").unwrap();
    let mut in_mermaid = false;
    for line in artifact.markdown.lines() {
        let line = line.trim();
        if markdown_link.is_match(line) {
            return Err(invalid(
                "topic links are generated from [[claim:id]] and original references; embedded links and HTML are not accepted",
            ));
        }
        if line.starts_with("```mermaid") {
            in_mermaid = true;
            continue;
        }
        if line.starts_with("```") {
            if !in_mermaid {
                return Err(invalid(
                    "only mapped Mermaid code blocks are supported in topics",
                ));
            }
            in_mermaid = false;
            continue;
        }
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if in_mermaid && (line.starts_with("flowchart ") || line.starts_with("graph ")) {
            continue;
        }
        let markers: Vec<_> = marker.captures_iter(line).collect();
        if markers.is_empty()
            || markers
                .iter()
                .any(|c| !ids.contains(c.get(1).unwrap().as_str()))
        {
            return Err(invalid(
                "every explanation line and Mermaid element needs a known [[claim:id]] citation",
            ));
        }
        if in_mermaid && !line.contains("%%") {
            return Err(invalid(
                "Mermaid node/edge mappings must appear in a %% [[claim:id]] comment",
            ));
        }
        if line.contains("topic:") || line.contains("generatedKnowledge:") {
            return Err(invalid(
                "topics and generated knowledge cannot prove claims or other topics",
            ));
        }
    }
    if in_mermaid {
        return Err(invalid("unterminated Mermaid block"));
    }
    control.check()?;
    let version =
        storage::knowledge::save_topic(database, artifact, &context, now, || control.check())?;
    Ok(
        json!({"topicId":artifact.topic_id,"version":version,"snapshot":artifact.snapshot,"freshness":"fresh","status":if artifact.gaps.is_empty(){"complete"}else{"partial"},"artifactHash":digest(artifact)}),
    )
}

pub fn import_conventions(
    database: &mut Database,
    project: &LoadedProject,
    input: &ConventionsImport,
    now: &str,
) -> Result<Value, CoreError> {
    import_conventions_with_control(
        database,
        project,
        input,
        now,
        &super::KnowledgeControl::default(),
    )
}
pub fn import_conventions_with_control(
    database: &mut Database,
    project: &LoadedProject,
    input: &ConventionsImport,
    now: &str,
    control: &super::KnowledgeControl,
) -> Result<Value, CoreError> {
    control.check()?;
    let _lock = crate::collect::registry::MaintenanceLock::acquire(project)?;
    let current = LoadedProject::load(&project.config_path)?;
    if input.notes.trim().is_empty()
        || input.notes.len() > 80_000
        || input.source_references.is_empty()
    {
        return Err(invalid(
            "conventions need bounded notes and original evidence",
        ));
    }
    for reference in &input.source_references {
        control.check()?;
        validate_references(database, &current, std::slice::from_ref(reference))?;
    }
    let unique: BTreeSet<_> = input
        .source_references
        .iter()
        .map(|r| &r.evidence_id)
        .collect();
    if unique.len() != input.source_references.len() {
        return Err(invalid("duplicate evidence reference"));
    }
    let next = apply_identity_delta(
        &current.config,
        &input.identity_delta,
        &input.expected_config_hash,
    )?;
    let refs: BTreeSet<_> = input
        .source_references
        .iter()
        .map(|r| r.evidence_id.as_str())
        .collect();
    for binding in &input.identity_delta.bindings {
        if binding.evidence_refs.is_empty()
            || binding
                .evidence_refs
                .iter()
                .any(|r| !refs.contains(r.as_str()))
        {
            return Err(invalid("identity bindings need imported original evidence"));
        }
    }
    let mut checked = current.clone();
    checked.config = next.config.clone();
    checked.validate()?;
    let id = digest(input);
    let changed = next.config_hash != next.previous_config_hash;
    let mut pending = tempfile::NamedTempFile::new_in(&current.base_dir)
        .map_err(|_| invalid("cannot stage conventions config"))?;
    pending
        .write_all(
            serde_json::to_string_pretty(&next.config)
                .unwrap()
                .as_bytes(),
        )
        .map_err(|_| invalid("cannot write conventions config"))?;
    pending
        .as_file()
        .sync_all()
        .map_err(|_| invalid("cannot sync conventions config"))?;
    control.check()?;
    let tx = database.write_transaction()?;
    control.check()?;
    tx.execute("INSERT OR IGNORE INTO conventions VALUES(?1,?2,?3)",params![id,serde_json::to_string(&json!({"id":id,"sourceClass":"derivedConvention","notes":input.notes,"sourceReferences":input.source_references,"configHash":next.config_hash})).unwrap(),now]).map_err(database_error)?;
    if changed {
        // File precedes DB commit: a crash leaves a detectable config mismatch, never false fresh facts.
        if LoadedProject::load(&current.config_path)?.config_hash()? != input.expected_config_hash {
            return Err(CoreError::new(
                codes::ACTIVATION_CONFLICT,
                "configuration changed during convention import",
            ));
        }
        pending
            .persist(&current.config_path)
            .map_err(|_| invalid("cannot publish conventions config"))?;
        let generation: i64 = tx
            .query_row("SELECT generation+1 FROM index_meta WHERE id=1", [], |r| {
                r.get(0)
            })
            .map_err(database_error)?;
        let keys = storage::current_batches(&tx)?
            .into_iter()
            .map(|(k, _)| k)
            .collect::<Vec<_>>();
        storage::invalidate_batches(&tx, &keys, generation as u64)?;
        tx.execute(
            "UPDATE index_meta SET generation=?1,config_hash=?2 WHERE id=1",
            params![generation, next.config_hash],
        )
        .map_err(database_error)?;
    }
    tx.execute("UPDATE topics SET state='stale' WHERE state<>'stale'", [])
        .map_err(database_error)?;
    tx.execute(
        "UPDATE knowledge_exports SET state='stale' WHERE state<>'stale'",
        [],
    )
    .map_err(database_error)?;
    tx.commit().map_err(database_error)?;
    Ok(
        json!({"conventionId":id,"sourceClass":"derivedConvention","originalReferences":input.source_references,"configHash":next.config_hash,"snapshot":query::snapshot_ref(database)?,"indexInvalidated":changed}),
    )
}
