//! Bounded original material and Core receipts for external explanations.
use crate::{
    contract::{
        SourceReference,
        error::{CoreError, codes},
        knowledge::*,
        query::*,
    },
    project::config::LoadedProject,
    query,
    storage::{self, Database, database_error},
};
use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn digest<T: Serialize>(value: &T) -> String {
    fn ordered(v: Value) -> Value {
        match v {
            Value::Object(m) => {
                let sorted: BTreeMap<_, _> = m.into_iter().map(|(k, v)| (k, ordered(v))).collect();
                serde_json::to_value(sorted).unwrap()
            }
            Value::Array(a) => Value::Array(a.into_iter().map(ordered).collect()),
            v => v,
        }
    }
    crate::contract::identity::content_hash(
        serde_json::to_string(&ordered(serde_json::to_value(value).unwrap()))
            .unwrap()
            .as_bytes(),
    )
}
pub fn prompt_hash() -> String {
    digest(&include_str!("../../../../prompts/knowledge.md"))
}
pub fn template_hash() -> String {
    digest(&(
        include_str!("../../templates/skill.md.hbs"),
        include_str!("../../templates/topic.md.hbs"),
        include_str!("../../templates/references.md.hbs"),
    ))
}
pub(crate) fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 100
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
}
pub(crate) fn invalid(message: &str) -> CoreError {
    CoreError::new(codes::INVALID_REQUEST, message)
}

pub fn export(
    database: &mut Database,
    project: &LoadedProject,
    request: &TopicContextRequest,
    now: &str,
) -> Result<TopicContext, CoreError> {
    export_with_control(
        database,
        project,
        request,
        now,
        &super::KnowledgeControl::default(),
    )
}
pub fn export_with_control(
    database: &mut Database,
    project: &LoadedProject,
    request: &TopicContextRequest,
    now: &str,
    control: &super::KnowledgeControl,
) -> Result<TopicContext, CoreError> {
    control.check()?;
    let _lock = crate::collect::registry::MaintenanceLock::acquire(project)?;
    if !valid_id(&request.topic_id) || !(16_000..=256_000).contains(&request.max_chars) {
        return Err(invalid(
            "topicId must be a safe identifier; maxChars must be 16000..256000",
        ));
    }
    let snapshot = query::snapshot_ref(database)?;
    let mut query_value = serde_json::to_value(&request.query).unwrap();
    if matches!(
        query_value["operation"].as_str(),
        Some("status" | "readSource")
    ) {
        return Err(invalid("topic context requires a fact query"));
    }
    let filters: Option<CommonFilters> = query_value
        .get("filters")
        .filter(|v| !v.is_null())
        .map(|v| serde_json::from_value(v.clone()))
        .transpose()
        .map_err(|_| invalid("invalid filters"))?;
    if query_value.get("budget").is_none_or(Value::is_null) {
        query_value["budget"] = json!({});
    }
    query_value["budget"]["maxResponseChars"] = json!(request.max_chars / 3);
    let bounded: QueryRequest =
        serde_json::from_value(query_value).map_err(|_| invalid("unsupported context query"))?;
    let facts = control.query(database, project, &bounded, now)?;
    if facts.completeness.freshness != Freshness::Fresh {
        return Err(CoreError::new(
            codes::INDEX_STALE,
            "refresh the selected index before exporting context",
        ));
    }
    // topicId names a document; it is not a fact selector. Use the exact
    // selected entities so changing a title cannot redirect its code outline.
    let mut map_ids = BTreeSet::new();
    collect_fact_entity_refs(&facts.data, &facts.snapshot, &mut map_ids);
    let map_roots = map_ids
        .into_iter()
        .map(|entity_id| EntityRef {
            snapshot: snapshot.clone(),
            entity_id,
        })
        .collect();
    let code_map = control.query(
        database,
        project,
        &QueryRequest::CodeMap(CodeMapRequest {
            query: None,
            roots: Some(map_roots),
            max_files: Some(8),
            max_symbols: Some(24),
            filters: filters.clone(),
            budget: Some(Budget {
                max_response_chars: Some(request.max_chars / 6),
                max_entities: None,
                max_relations: None,
                max_visited: None,
                max_depth: None,
                timeout_ms: None,
            }),
        }),
        now,
    )?;
    let mut conventions = storage::knowledge::conventions(database.connection())?;
    let mut convention_gaps = Vec::new();
    let mut references: BTreeMap<String, SourceReference> = BTreeMap::new();
    for r in facts
        .source_references
        .iter()
        .chain(&code_map.source_references)
    {
        references.insert(r.evidence_id.clone(), r.clone());
    }
    for convention in &mut conventions {
        control.check()?;
        let mut original: Vec<SourceReference> =
            serde_json::from_value(convention["sourceReferences"].clone())
                .map_err(|_| invalid("stored convention lacks original references"))?;
        for r in &mut original {
            r.snapshot = snapshot.clone();
        }
        if super::documents::validate_references(database, project, &original).is_ok() {
            for r in &original {
                references.insert(r.evidence_id.clone(), r.clone());
            }
            convention["sourceReferences"] = json!(original);
            convention["freshness"] = json!("fresh");
        } else {
            convention_gaps.push(Gap {
                id: format!(
                    "convention_{}",
                    convention["id"].as_str().unwrap_or("unknown")
                ),
                code: "CONVENTION_EVIDENCE_STALE".into(),
                capability: "knowledge".into(),
                scope_id: String::new(),
                source_refs: vec![],
                candidate_ids: vec![],
                reason: "Derived convention no longer has current original evidence".into(),
                required_evidence: vec![
                    "import a new convention from current original source".into(),
                ],
            });
            *convention = json!({"id":convention["id"],"sourceClass":"derivedConvention","freshness":"stale"});
        }
    }
    // File entities can cite a single declaration byte. Also expose the original
    // method/SQL body, so explanations can cite the content they discuss.
    let originals = references.values().cloned().collect::<Vec<_>>();
    let mut expanded = BTreeSet::new();
    for r in originals {
        control.check()?;
        if !expanded.insert(r.source_id.clone()) {
            continue;
        }
        storage::sources::validate_original_reference(database.connection(), &r)?;
        let Some((bytes, _)) =
            storage::read_source_version(database.connection(), &r.source_id, &r.content_hash)?
        else {
            continue;
        };
        if bytes.is_empty() {
            continue;
        }
        let map = crate::project::snapshot::LineMap::build(&bytes);
        let id = crate::contract::identity::compute_evidence_id(
            &r.source_id,
            &r.content_hash,
            0,
            bytes.len() as u64,
            "topic-context",
        );
        storage::insert_evidence(
            database.connection(),
            &crate::contract::Evidence {
                id: id.clone(),
                source_id: r.source_id.clone(),
                content_hash: r.content_hash.clone(),
                start_byte: 0,
                end_byte: bytes.len() as u64,
                start_line: 1,
                end_line: map.end_line_of(0, bytes.len() as u64),
                role: "topic-context".into(),
            },
        )?;
        references.insert(
            id.clone(),
            SourceReference {
                snapshot: snapshot.clone(),
                evidence_id: id,
                source_id: r.source_id,
                content_hash: r.content_hash,
                start_byte: 0,
                end_byte: bytes.len() as u64,
            },
        );
    }
    let source_references: Vec<_> = references.into_values().collect();
    let reads = control.query(
        database,
        project,
        &QueryRequest::ReadSource(ReadSourceRequest {
            references: source_references.clone(),
            max_response_chars: Some(request.max_chars / 4),
            filters: None,
        }),
        now,
    )?;
    let mut repositories: BTreeSet<String> = filters
        .as_ref()
        .and_then(|f| f.repository_ids.clone())
        .map(|v| v.into_iter().collect())
        .unwrap_or_else(|| project.repositories.keys().cloned().collect());
    for r in &source_references {
        storage::sources::validate_original_reference(database.connection(), r)?;
        if let Some((repo, _)) = storage::source_path(database.connection(), &r.source_id)? {
            repositories.insert(repo);
        }
    }
    let connection = database.connection();
    let mut dependencies = vec![
        KnowledgeDependency {
            kind: "config".into(),
            key: "project".into(),
            digest: project.config_hash()?,
        },
        KnowledgeDependency {
            kind: "prompt".into(),
            key: "knowledge".into(),
            digest: prompt_hash(),
        },
        KnowledgeDependency {
            kind: "template".into(),
            key: "skill".into(),
            digest: template_hash(),
        },
        KnowledgeDependency {
            kind: "conventions".into(),
            key: "all".into(),
            digest: digest(&storage::knowledge::conventions(connection)?),
        },
    ];
    for repo in &repositories {
        dependencies.push(KnowledgeDependency {
            kind: "sourceSet".into(),
            key: repo.clone(),
            digest: indexed_source_set(connection, repo)?,
        });
    }
    dependencies.push(KnowledgeDependency {
        kind: "plugins".into(),
        key: serde_json::to_string(&repositories).unwrap(),
        digest: plugins_digest(connection, &repositories)?,
    });
    dependencies.push(KnowledgeDependency {
        kind: "facts".into(),
        key: serde_json::to_string(&repositories).unwrap(),
        digest: facts_digest(connection, &repositories)?,
    });
    let mut producers = BTreeSet::new();
    collect_strings(&facts.data, "producerKey", &mut producers);
    collect_strings(&code_map.data, "producerKey", &mut producers);
    let mut plugin_ids = BTreeSet::new();
    for producer in producers {
        let key:Option<String>=connection.query_row("SELECT b.replacement_key FROM batches b JOIN current_batches cb ON cb.batch_id=b.id WHERE b.producer_key=?1",[&producer],|r|r.get(0)).optional().map_err(database_error)?;
        if let Some(key) = key
            && !key.starts_with("(core.base,")
        {
            dependencies.push(KnowledgeDependency {
                kind: "batch".into(),
                digest: batch_digest(connection, &key)?,
                key,
            });
        }
        if let Some((id, _)) = producer.split_once('@')
            && id != "core.base"
        {
            plugin_ids.insert(id.to_string());
        }
    }
    for id in plugin_ids {
        dependencies.push(KnowledgeDependency {
            kind: "artifact".into(),
            key: id.clone(),
            digest: artifact_digest(connection, &id)?,
        });
        dependencies.push(KnowledgeDependency {
            kind: "selectors".into(),
            digest: selector_digest(connection, &id)?,
            key: id,
        });
    }
    dependencies.sort_by(|a, b| (&a.kind, &a.key).cmp(&(&b.kind, &b.key)));
    dependencies.dedup_by(|a, b| a.kind == b.kind && a.key == b.key);
    let mut gaps = facts.gaps.clone();
    gaps.extend(convention_gaps);
    for g in code_map.gaps.iter().chain(&reads.gaps) {
        if !gaps.iter().any(|x| x.id == g.id) {
            gaps.push(g.clone());
        }
    }
    if [facts.status, code_map.status, reads.status].contains(&ResponseStatus::Partial) {
        gaps.push(Gap{id:"knowledge_material_partial".into(),code:"KNOWLEDGE_MATERIAL_PARTIAL".into(),capability:"knowledge".into(),scope_id:String::new(),source_refs:vec![],candidate_ids:vec![],reason:"Some selected facts, outlines or source reads are partial; explanation completeness is limited by that material".into(),required_evidence:vec!["inspect completeness reasons and obtain the missing original material".into()]});
    }
    if code_map.completeness.freshness != Freshness::Fresh
        || reads.completeness.freshness != Freshness::Fresh
    {
        gaps.push(Gap {
            id: "knowledge_freshness_unknown".into(),
            code: "KNOWLEDGE_MATERIAL_NOT_CURRENT".into(),
            capability: "knowledge".into(),
            scope_id: String::new(),
            source_refs: vec![],
            candidate_ids: vec![],
            reason: "Some context material could not be verified as current".into(),
            required_evidence: vec!["refresh and verify current original material".into()],
        });
    }
    if facts.completeness.truncated
        || code_map.completeness.truncated
        || reads.completeness.truncated
    {
        gaps.push(Gap {
            id: "knowledge_budget".into(),
            code: "KNOWLEDGE_CONTEXT_TRUNCATED".into(),
            capability: "knowledge".into(),
            scope_id: String::new(),
            source_refs: vec![],
            candidate_ids: vec![],
            reason: "Context is bounded; unreturned material remains unknown".into(),
            required_evidence: vec!["narrow the fact query or increase maxChars".into()],
        });
    }
    let mut coverage = Vec::new();
    for (key, batch) in storage::current_batches(connection)? {
        for (capability, scope, source_set, status) in storage::load_coverage(connection, &batch)? {
            coverage.push(json!({"replacementKey":key,"capability":capability,"scopeId":scope,"sourceSetId":source_set,"status":status}));
        }
    }
    let mut stable_reads = reads.data.clone();
    if let Some(entries) = stable_reads.get_mut("reads").and_then(Value::as_array_mut) {
        for entry in entries {
            if let Some(object) = entry.as_object_mut() {
                object.remove("checkedAt");
            }
        }
    }
    let material_digest = digest(
        &json!({"request":request,"facts":facts.data,"codeMap":code_map.data,"sourceReads":stable_reads,"references":source_references,"dependencies":dependencies,"conventions":conventions,"coverage":coverage,"gaps":gaps,"statuses":[facts.status,code_map.status,reads.status],"truncated":[facts.completeness.truncated,code_map.completeness.truncated,reads.completeness.truncated]}),
    );
    let input_digest = digest(&(&snapshot, &material_digest));
    let context = TopicContext {
        protocol_version: crate::contract::PROTOCOL_VERSION.into(),
        topic_id: request.topic_id.clone(),
        snapshot: snapshot.clone(),
        input_digest,
        material_digest,
        request: request.clone(),
        facts,
        code_map,
        source_reads: reads.data,
        coverage: json!(coverage),
        source_references,
        conventions,
        gaps,
        prompt_hash: prompt_hash(),
        template_hash: template_hash(),
        dependencies,
    };
    if serde_json::to_string(&context).unwrap().chars().count() as u64 > request.max_chars {
        return Err(CoreError::new(
            codes::BUDGET_EXHAUSTED,
            "context metadata and original references exceed maxChars; narrow the query",
        ));
    }
    if query::snapshot_ref(database)? != snapshot {
        return Err(CoreError::new(
            codes::SNAPSHOT_EXPIRED,
            "index changed during context export",
        ));
    }
    control.check()?;
    storage::knowledge::save_receipt(database, &context, now, || control.check())?;
    Ok(context)
}
fn collect_strings(value: &Value, key: &str, output: &mut BTreeSet<String>) {
    match value {
        Value::Object(m) => {
            for (k, v) in m {
                if k == key
                    && let Some(s) = v.as_str()
                {
                    output.insert(s.to_string());
                }
                collect_strings(v, key, output);
            }
        }
        Value::Array(a) => {
            for v in a {
                collect_strings(v, key, output)
            }
        }
        _ => {}
    }
}

fn collect_fact_entity_refs(
    value: &Value,
    snapshot: &crate::contract::SnapshotRef,
    output: &mut BTreeSet<String>,
) {
    match value {
        Value::Object(fields) => {
            if let Some(reference) = fields.get("entityRef")
                && let Ok(reference) = serde_json::from_value::<EntityRef>(reference.clone())
                && &reference.snapshot == snapshot
            {
                output.insert(reference.entity_id);
            }
            for (key, value) in fields {
                // Attribute/qualifier objects are project data, not Core references.
                if !matches!(
                    key.as_str(),
                    "attributes" | "qualifiers" | "identity" | "assertions"
                ) {
                    collect_fact_entity_refs(value, snapshot, output);
                }
            }
        }
        Value::Array(values) => values
            .iter()
            .for_each(|v| collect_fact_entity_refs(v, snapshot, output)),
        _ => {}
    }
}

pub(crate) fn indexed_source_set(connection: &Connection, repo: &str) -> Result<String, CoreError> {
    let mut stmt=connection.prepare("SELECT s.path,ss.content_hash,ss.selection_status FROM snapshot_sources ss JOIN sources s ON s.id=ss.source_id WHERE ss.snapshot_id=(SELECT current_source_snapshot_id FROM index_meta WHERE id=1) AND s.repository_id=?1 ORDER BY s.path").map_err(database_error)?;
    let rows = stmt
        .query_map([repo], |r| {
            Ok(json!([
                r.get::<_, String>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, String>(2)?
            ]))
        })
        .map_err(database_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(database_error)?;
    Ok(digest(&rows))
}
pub(crate) fn artifact_digest(connection: &Connection, id: &str) -> Result<String, CoreError> {
    let row:Option<(String,String,String)>=connection.query_row("SELECT a.version,v.artifact_hash,v.runtime_digest FROM active_plugins a JOIN plugin_versions v ON v.plugin_id=a.plugin_id AND v.version=a.version WHERE a.plugin_id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(database_error)?;
    Ok(digest(&row))
}
pub(crate) fn batch_digest(connection: &Connection, key: &str) -> Result<String, CoreError> {
    let row:Option<(String,String,String)>=connection.query_row("SELECT b.state,b.artifact_hash,b.input_digest FROM batches b JOIN current_batches cb ON cb.batch_id=b.id WHERE cb.replacement_key=?1",[key],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(database_error)?;
    Ok(digest(&row))
}
pub(crate) fn selector_digest(connection: &Connection, id: &str) -> Result<String, CoreError> {
    let mut stmt=connection.prepare("SELECT DISTINCT d.kind,d.key,d.digest FROM dependencies d JOIN current_batches cb ON cb.batch_id=d.batch_id JOIN batches b ON b.id=cb.batch_id WHERE b.producer_key LIKE ?1 AND d.kind IN ('candidateSet','sourceSet') ORDER BY d.kind,d.key,d.digest").map_err(database_error)?;
    let rows = stmt
        .query_map([format!("{id}@%")], |r| {
            Ok(json!([
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?
            ]))
        })
        .map_err(database_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(database_error)?;
    Ok(digest(&rows))
}
pub(crate) fn plugins_digest(
    connection: &Connection,
    repositories: &BTreeSet<String>,
) -> Result<String, CoreError> {
    let mut stmt=connection.prepare("SELECT a.plugin_id,a.version,v.artifact_hash,v.runtime_digest,v.manifest_json FROM active_plugins a JOIN plugin_versions v ON v.plugin_id=a.plugin_id AND v.version=a.version ORDER BY a.plugin_id").map_err(database_error)?;
    let rows = stmt
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
    let mut selected = Vec::new();
    for row in rows {
        let (id, version, hash, runtime, manifest) = row.map_err(database_error)?;
        let manifest: crate::contract::PluginManifest = serde_json::from_str(&manifest)
            .map_err(|_| invalid("invalid active plugin manifest"))?;
        if manifest.stages.iter().any(|s| {
            s.source_selectors.is_empty()
                || s.source_selectors.iter().any(|sel| {
                    sel.repository_ids.is_empty()
                        || sel
                            .repository_ids
                            .iter()
                            .any(|id| repositories.contains(id))
                })
        }) {
            selected.push((id, version, hash, runtime));
        }
    }
    Ok(digest(&selected))
}
pub(crate) fn facts_digest(
    connection: &Connection,
    repositories: &BTreeSet<String>,
) -> Result<String, CoreError> {
    let mut selected = Vec::new();
    for (key, batch) in storage::current_batches(connection)? {
        if key.starts_with("(core.base,") {
            continue;
        }
        let mut stmt=connection.prepare("SELECT DISTINCT s.repository_id FROM assertions a JOIN assertion_evidence ae ON ae.assertion_id=a.id JOIN evidence e ON e.id=ae.evidence_id JOIN sources s ON s.id=e.source_id WHERE a.batch_id=?1 ORDER BY s.repository_id").map_err(database_error)?;
        let repos = stmt
            .query_map([&batch], |r| r.get::<_, String>(0))
            .map_err(database_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(database_error)?;
        if repos.is_empty() || repos.iter().any(|id| repositories.contains(id)) {
            selected.push((key.clone(), batch_digest(connection, &key)?));
        }
    }
    Ok(digest(&selected))
}
/// Bounded read-only verification, including zero-match repository sets.
pub(crate) fn disk_sets(project: &LoadedProject) -> Result<BTreeMap<String, String>, CoreError> {
    disk_sets_with_budget(project, &mut || true)
}
pub(crate) fn disk_sets_with_budget(
    project: &LoadedProject,
    visit: &mut impl FnMut() -> bool,
) -> Result<BTreeMap<String, String>, CoreError> {
    let options = crate::project::scan::ScanOptions {
        include: project.config.selection.include.clone(),
        exclude: project.config.selection.exclude.clone(),
        respect_git_ignore: project.config.selection.respect_git_ignore,
        max_file_bytes: project.config.selection.max_file_bytes,
    };
    let start = std::time::Instant::now();
    let mut visited = 0;
    let mut output = BTreeMap::new();
    for (id, root) in &project.repositories {
        let scan = crate::project::scan::scan_repository_bounded(id, root, &options, &mut || {
            visited += 1;
            visited <= 20_000 && start.elapsed().as_secs() < 5 && visit()
        })
        .map_err(|e| CoreError::new(codes::SOURCE_CHANGED, e))?
        .ok_or_else(|| {
            CoreError::new(
                codes::BUDGET_EXHAUSTED,
                "knowledge source verification exceeded budget",
            )
        })?;
        let mut rows: Vec<_> = scan
            .sources
            .into_iter()
            .map(|s| {
                json!([
                    s.path,
                    if s.selection_status == crate::project::scan::SelectionStatus::Selected {
                        Some(s.content_hash)
                    } else {
                        None
                    },
                    s.selection_status.as_str()
                ])
            })
            .collect();
        rows.sort_by(|a, b| a[0].as_str().cmp(&b[0].as_str()));
        output.insert(id.clone(), digest(&rows));
    }
    Ok(output)
}
