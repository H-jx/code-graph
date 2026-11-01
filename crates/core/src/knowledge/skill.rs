//! Strict rendering into immutable complete generations, selected by an atomic pointer.
use super::{
    context::{self, digest, invalid},
    documents,
};
use crate::{
    contract::{
        SourceReference,
        error::{CoreError, codes},
    },
    project::config::LoadedProject,
    query,
    storage::{self, Database},
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    io::Write,
    path::{Path, PathBuf},
};

fn io(error: std::io::Error) -> CoreError {
    CoreError::new(
        codes::INTERNAL_ERROR,
        format!("knowledge output I/O: {error}"),
    )
}

/// Output inside a repository must stay in the scanner's control directories.
fn output_path(project: &LoadedProject, path: &Path) -> Result<PathBuf, CoreError> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().map_err(io)?.join(path)
    };
    let absolute = crate::project::config::normalize_path(&absolute);
    for ancestor in absolute.ancestors() {
        if ancestor.exists()
            && (std::fs::symlink_metadata(ancestor)
                .map_err(io)?
                .file_type()
                .is_symlink()
                || crate::project::scan::is_reparse_point(ancestor))
        {
            return Err(invalid(
                "knowledge output cannot traverse a symlink or reparse point",
            ));
        }
    }
    for root in project.repositories.values() {
        let root = std::fs::canonicalize(root).map_err(io)?;
        // Canonicalize the nearest existing ancestor to resolve Windows path prefixes.
        let mut existing = absolute.as_path();
        let mut suffix = Vec::new();
        while !existing.exists() {
            suffix.push(
                existing
                    .file_name()
                    .ok_or_else(|| invalid("invalid output path"))?
                    .to_os_string(),
            );
            existing = existing
                .parent()
                .ok_or_else(|| invalid("invalid output parent"))?;
        }
        let mut resolved = std::fs::canonicalize(existing).map_err(io)?;
        for part in suffix.iter().rev() {
            resolved.push(part);
        }
        if let Ok(relative) = resolved.strip_prefix(&root) {
            let relative = relative.to_string_lossy().replace('\\', "/");
            if !(relative.starts_with(".agents/skills/")
                || relative.starts_with(".project-knowledge/"))
            {
                return Err(invalid(
                    "repository output must be below .agents/skills or .project-knowledge to prevent generated-source feedback",
                ));
            }
        }
    }
    Ok(absolute)
}

pub fn generate(
    database: &mut Database,
    project: &LoadedProject,
    name: &str,
    output: &Path,
    now: &str,
) -> Result<Value, CoreError> {
    generate_with_checkpoint(database, project, name, output, now, |_| Ok(()))
}

/// Checkpoints allow a host to cancel before publication; previous CURRENT.json is retained.
pub fn generate_with_checkpoint(
    database: &mut Database,
    project: &LoadedProject,
    name: &str,
    output: &Path,
    now: &str,
    mut checkpoint: impl FnMut(&str) -> Result<(), CoreError>,
) -> Result<Value, CoreError> {
    checkpoint("preflight")?;
    let _lock = crate::collect::registry::MaintenanceLock::acquire(project)?;
    if !context::valid_id(name) {
        return Err(invalid("skill name must be a safe identifier"));
    }
    let output = output_path(project, output)?;
    documents::refresh(database, project)?;
    let id = uuid::Uuid::new_v4().to_string();
    storage::knowledge::attempt(database.connection(), &id, &output.to_string_lossy(), now)?;
    let result = (|| {
        let topics = storage::knowledge::topics(database.connection())?;
        if topics.is_empty() {
            return Err(CoreError::new(
                codes::REFERENCE_MISSING,
                "import a validated topic before rendering a Skill",
            ));
        }
        let status = documents::status(database.connection(), Some(project), true)?;
        let fresh = status["topics"]
            .as_array()
            .unwrap()
            .iter()
            .all(|t| t["freshness"] == "fresh");
        let freshness = if fresh {
            "fresh"
        } else if status["topics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["freshness"] == "stale")
        {
            "stale"
        } else {
            "unknown"
        };
        let partial = topics.iter().any(|t| !t.artifact.gaps.is_empty()) || !fresh;
        let snapshot = query::snapshot_ref(database)?;
        let mut handlebars = handlebars::Handlebars::new();
        handlebars.set_strict_mode(true);
        handlebars.register_escape_fn(handlebars::no_escape);
        for (key, text) in [
            ("skill", include_str!("../../templates/skill.md.hbs")),
            ("topic", include_str!("../../templates/topic.md.hbs")),
            (
                "reference",
                include_str!("../../templates/references.md.hbs"),
            ),
        ] {
            handlebars
                .register_template_string(key, text)
                .map_err(|_| invalid("invalid embedded knowledge template"))?;
        }
        let render = |key: &str, value: &Value| {
            handlebars
                .render(key, value)
                .map_err(|_| invalid("knowledge template is missing a required field"))
        };
        let mut files = BTreeMap::<String, String>::new();
        let topic_links = topics
            .iter()
            .map(|t| {
                format!(
                    "- [{}](references/topics/{}.md) — version {}, {}",
                    t.artifact.title,
                    t.artifact.topic_id,
                    t.version,
                    status["topics"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|v| v["topicId"] == t.artifact.topic_id)
                        .unwrap()["freshness"]
                        .as_str()
                        .unwrap()
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        files.insert("SKILL.md".into(),render("skill",&json!({"name":name,"projectId":project.config.project_id,"configPath":project.config_path.to_string_lossy(),"generation":snapshot.generation,"freshness":freshness,"topics":topic_links}))?);
        let mut refs = BTreeMap::<String, SourceReference>::new();
        let mut categories: BTreeMap<&str, Vec<Value>> =
            ["routes", "frontend", "api", "database", "code-map"]
                .into_iter()
                .map(|k| (k, Vec::new()))
                .collect();
        for topic in &topics {
            for r in &topic.artifact.source_references {
                refs.insert(r.evidence_id.clone(), r.clone());
            }
            let claims=topic.artifact.claims.iter().map(|c|{
                let links=c.evidence_refs.iter().map(|id|format!("[{}](../source-refs.md#{})",id,id)).collect::<Vec<_>>().join(", ");
                format!("<a id=\"claim-{}\"></a>\n- {} — basis={}, origin={}, evidence: {}; gaps: {}",c.id,c.text,c.basis.as_str(),c.origin.as_str(),links,c.gap_ids.join(", "))
            }).collect::<Vec<_>>().join("\n");
            let gaps = topic
                .artifact
                .gaps
                .iter()
                .map(|g| {
                    format!(
                        "- {} ({}): {}; required: {}",
                        g.id,
                        g.code,
                        g.reason,
                        g.required_evidence.join("; ")
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            let mut markdown = topic.artifact.markdown.clone();
            for c in &topic.artifact.claims {
                markdown = markdown.replace(
                    &format!("[[claim:{}]]", c.id),
                    &format!("[claim:{}](#claim-{})", c.id, c.id),
                );
            }
            let state = status["topics"]
                .as_array()
                .unwrap()
                .iter()
                .find(|v| v["topicId"] == topic.artifact.topic_id)
                .unwrap();
            files.insert(format!("references/topics/{}.md",topic.artifact.topic_id),render("topic",&json!({"title":topic.artifact.title,"version":topic.version,"snapshot":serde_json::to_string(&topic.artifact.snapshot).unwrap(),"freshness":state["freshness"],"markdown":markdown,"claims":claims,"gaps":gaps}))?);
            categories.get_mut("code-map").unwrap().push(json!({"topicId":topic.artifact.topic_id,"freshness":state["freshness"],"snapshot":topic.artifact.snapshot,"data":topic.context.code_map.data}));
            for (category, kinds) in [
                ("routes", &["route", "title"][..]),
                (
                    "frontend",
                    &["component", "operation", "event", "request"][..],
                ),
                (
                    "api",
                    &[
                        "httpEndpoint",
                        "graphqlOperation",
                        "graphqlField",
                        "service",
                        "symbol",
                    ][..],
                ),
                (
                    "database",
                    &[
                        "dataSource",
                        "dataTable",
                        "dataColumn",
                        "dataKey",
                        "queryImplementation",
                        "reportField",
                    ][..],
                ),
            ] {
                let mut selected = Vec::new();
                for kind in kinds {
                    select_objects(&topic.context.facts.data, kind, &mut selected);
                }
                categories.get_mut(category).unwrap().push(json!({"topicId":topic.artifact.topic_id,"freshness":state["freshness"],"snapshot":topic.artifact.snapshot,"entities":selected,"coverage":topic.context.coverage,"gaps":topic.artifact.gaps}));
            }
        }
        for (category, data) in categories {
            files.insert(format!("references/{category}.md"),render("reference",&json!({"title":category,"freshness":freshness,"body":format!("```json\n{}\n```",serde_json::to_string_pretty(&data).unwrap())}))?);
        }
        files.insert("references/conventions.md".into(),render("reference",&json!({"title":"Original evidence and derived conventions","freshness":freshness,"body":format!("Derived notes retain original references; generated topics are excluded from fact collection.\n\n```json\n{}\n```",serde_json::to_string_pretty(&storage::knowledge::conventions(database.connection())?).unwrap())}))?);
        let mut citations = String::from(
            "# Original source references\n\nReferences describe the original topic version. Check status before reuse.\n\n",
        );
        for (id, r) in &refs {
            let (repo, path) = storage::source_path(database.connection(), &r.source_id)?
                .ok_or_else(|| invalid("topic has missing source"))?;
            citations.push_str(&format!("<a id=\"{id}\"></a>\n\n- {repo}/{path}, UTF-8 bytes [{},{}), hash `{}`, snapshot `{}`.\n\n",r.start_byte,r.end_byte,r.content_hash,serde_json::to_string(&r.snapshot).unwrap()));
        }
        files.insert("references/source-refs.md".into(), citations);
        let versions:Vec<_>=topics.iter().map(|t|json!({"topicId":t.artifact.topic_id,"version":t.version,"snapshot":t.artifact.snapshot,"inputDigest":t.artifact.input_digest,"promptHash":t.artifact.prompt_hash,"templateHash":t.artifact.template_hash,"artifactHash":digest(&t.artifact),"dependencies":t.context.dependencies})).collect();
        let hashes: BTreeMap<_, _> = files
            .iter()
            .map(|(path, text)| {
                (
                    path.clone(),
                    crate::contract::identity::content_hash(text.as_bytes()),
                )
            })
            .collect();
        let manifest = json!({"protocolVersion":crate::contract::PROTOCOL_VERSION,"exportId":id,"snapshot":snapshot,"freshness":freshness,"status":if partial{"partial"}else{"complete"},"promptHash":context::prompt_hash(),"templateHash":context::template_hash(),"activePlugins":storage::active_version_map(database.connection())?,"topics":versions,"sourceReferences":refs.into_values().collect::<Vec<_>>(),"files":hashes});
        files.insert(
            "references/index-manifest.json".into(),
            serde_json::to_string_pretty(&manifest).unwrap() + "\n",
        );
        std::fs::create_dir_all(output.join("versions")).map_err(io)?;
        let stage = tempfile::Builder::new()
            .prefix(".pending-")
            .tempdir_in(&output)
            .map_err(io)?;
        for (path, text) in &files {
            let path = stage.path().join(path);
            std::fs::create_dir_all(path.parent().unwrap()).map_err(io)?;
            let mut file = std::fs::File::create(path).map_err(io)?;
            file.write_all(text.as_bytes()).map_err(io)?;
            file.sync_all().map_err(io)?;
            checkpoint("file-written")?;
        }
        let manifest_hash = crate::contract::identity::content_hash(
            files["references/index-manifest.json"].as_bytes(),
        );
        let mut complete = std::fs::File::create(stage.path().join("COMPLETE.json")).map_err(io)?;
        complete
            .write_all(
                &serde_json::to_vec_pretty(
                    &json!({"manifestSha256":manifest_hash,"files":files.len()}),
                )
                .unwrap(),
            )
            .map_err(io)?;
        complete.sync_all().map_err(io)?;
        drop(complete);
        checkpoint("before-publish")?;
        if query::snapshot_ref(database)? != snapshot {
            return Err(CoreError::new(
                codes::SNAPSHOT_EXPIRED,
                "index changed during Skill rendering",
            ));
        }
        let published = output.join("versions").join(&id);
        std::fs::rename(stage.path(), &published).map_err(io)?;
        checkpoint("before-pointer")?;
        if fresh {
            let disk = context::disk_sets(project)?;
            for topic in &topics {
                if !documents::dependencies_match(
                    database.connection(),
                    Some(project),
                    &topic.context,
                    Some(&disk),
                )? {
                    documents::refresh(database, project)?;
                    return Err(CoreError::new(
                        codes::DEPENDENCY_CHANGED,
                        "knowledge dependencies changed before publication",
                    ));
                }
            }
        }
        let pointer = json!({"exportId":id,"directory":format!("versions/{id}"),"manifestSha256":manifest_hash});
        let mut pending = tempfile::NamedTempFile::new_in(&output).map_err(io)?;
        pending
            .write_all(serde_json::to_string_pretty(&pointer).unwrap().as_bytes())
            .map_err(io)?;
        pending.as_file().sync_all().map_err(io)?;
        // DB registration precedes pointer publication; unselected complete generations are harmless.
        checkpoint("before-commit")?;
        storage::knowledge::save_export(
            database,
            &id,
            &published.to_string_lossy(),
            &manifest,
            now,
            || checkpoint("transaction"),
        )?;
        pending
            .persist(output.join("CURRENT.json"))
            .map_err(|e| io(e.error))?;
        Ok(
            json!({"exportId":id,"directory":published,"pointer":output.join("CURRENT.json"),"freshness":freshness,"status":if partial{"partial"}else{"complete"},"manifestSha256":manifest_hash,"files":files.len()+1}),
        )
    })();
    if let Err(error) = &result {
        storage::knowledge::finish_attempt(database.connection(), &id, Some(&error.code))?;
    }
    result
}
/// None means integrity could not be confirmed within the caller's budget.
pub(crate) fn verify_export(
    directory: &Path,
    expected_digest: &str,
    visit: &mut impl FnMut() -> bool,
) -> Option<bool> {
    if !visit() {
        return None;
    }
    let manifest_path = directory.join("references/index-manifest.json");
    if std::fs::metadata(&manifest_path).ok()?.len() > 2_000_000 {
        return Some(false);
    }
    let bytes = std::fs::read(manifest_path).ok()?;
    let manifest: Value = serde_json::from_slice(&bytes).ok()?;
    let complete_path = directory.join("COMPLETE.json");
    if std::fs::metadata(&complete_path).ok()?.len() > 4096 {
        return Some(false);
    }
    let complete: Value = serde_json::from_slice(&std::fs::read(complete_path).ok()?).ok()?;
    if digest(&manifest) != expected_digest
        || complete["manifestSha256"] != crate::contract::content_hash(&bytes)
    {
        return Some(false);
    }
    let files = manifest["files"].as_object()?;
    if files.len() > 1000 {
        return None;
    }
    let mut total = 0;
    for (name, hash) in files {
        if !visit() {
            return None;
        }
        if crate::project::config::validate_relative_path(name).is_err() {
            return Some(false);
        }
        let path = directory.join(name);
        for ancestor in path.ancestors().take_while(|p| p.starts_with(directory)) {
            if crate::project::scan::is_reparse_point(ancestor)
                || std::fs::symlink_metadata(ancestor)
                    .ok()?
                    .file_type()
                    .is_symlink()
            {
                return Some(false);
            }
        }
        total += std::fs::metadata(&path).ok()?.len();
        if total > 10_000_000 {
            return None;
        }
        let bytes = std::fs::read(path).ok()?;
        if hash != &json!(crate::contract::content_hash(&bytes)) {
            return Some(false);
        }
    }
    Some(true)
}
fn select_objects(value: &Value, kind: &str, selected: &mut Vec<Value>) {
    match value {
        Value::Object(m) => {
            if m.get("kind").and_then(Value::as_str) == Some(kind) {
                selected.push(value.clone());
            } else {
                for v in m.values() {
                    select_objects(v, kind, selected);
                }
            }
        }
        Value::Array(a) => {
            for v in a {
                select_objects(v, kind, selected)
            }
        }
        _ => {}
    }
}
