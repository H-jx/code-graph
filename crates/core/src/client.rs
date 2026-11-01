//! Transport-neutral entry points. All validation, state and algorithms remain in Core.
use std::path::Path;

use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use crate::collect::{registry, staging};
use crate::contract::client::{IndexMode, MaintenanceRequest};
use crate::contract::{CoreError, QueryRequest, codes};
use crate::knowledge::{KnowledgeControl, context, documents, skill};
use crate::project::LoadedProject;
use crate::storage::Database;

pub fn database_path(config: &Path) -> std::path::PathBuf {
    config.with_file_name("index.sqlite")
}

pub fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// Validate against the Rust schema before deserialization; never echo invalid values.
pub fn decode<T: DeserializeOwned + schemars::JsonSchema>(raw: &str) -> Result<T, CoreError> {
    if raw.len() > 1_048_576 {
        return Err(CoreError::new(
            codes::INVALID_REQUEST,
            "request exceeds 1 MiB",
        ));
    }
    let value: Value = serde_json::from_str(raw)
        .map_err(|_| CoreError::new(codes::INVALID_REQUEST, "request must be valid JSON"))?;
    crate::contract::schema::validate::<T>(&value, "")?;
    serde_json::from_value(value)
        .map_err(|_| CoreError::new(codes::INVALID_REQUEST, "invalid request"))
}

pub fn check_cancel(cancel: &CancellationToken) -> Result<(), CoreError> {
    if cancel.is_cancelled() {
        Err(CoreError::new(codes::CANCELLED, "request cancelled"))
    } else {
        Ok(())
    }
}

pub fn execute_query(
    project: &LoadedProject,
    database: &mut Database,
    raw: &str,
    cancel: &CancellationToken,
    request_id: &str,
) -> Result<Value, CoreError> {
    let request = decode::<QueryRequest>(raw)?;
    let response = crate::query::execute_with_request_id(
        &mut crate::query::QueryContext {
            database,
            project: Some(project),
            now: now(),
        },
        &request,
        cancel.clone(),
        request_id.to_string(),
    )?;
    Ok(serde_json::to_value(response).expect("query response"))
}

pub async fn execute_maintenance(
    project: &LoadedProject,
    database: &mut Database,
    raw: &str,
    cancel: &CancellationToken,
) -> Result<Value, CoreError> {
    let request = decode::<MaintenanceRequest>(raw)?;
    check_cancel(cancel)?;
    let now = now();
    let control = KnowledgeControl {
        cancel: cancel.clone(),
        deadline: None,
    };
    match request {
        MaintenanceRequest::Index { mode } => {
            let result = match mode {
                IndexMode::Base => crate::index::incremental::run_base_only_with_cancellation(
                    project, database, cancel, &now,
                )?,
                IndexMode::Incremental => {
                    crate::index::run_incremental(project, database, cancel, &now).await?
                }
            };
            Ok(
                json!({"mode":mode,"indexId":result.base.index_id,"generation":result.generation.to_string(),
                "sourceSnapshotId":result.base.snapshot.id,"sourceSetHash":result.base.snapshot.source_set_hash,
                "entities":result.base.entities,"relations":result.base.relations,"searchDocuments":result.base.documents,
                "gaps":result.base.gaps.len(),"coverage":result.base.coverage,
                "invalidated":result.invalidated,"executed":result.executed,"completedAt":now}),
            )
        }
        MaintenanceRequest::DiscoverPlugins {} => {
            Ok(json!({"candidates":registry::discover(project, database, &now)?}))
        }
        MaintenanceRequest::SubmitPlugin { path } => Ok(json!(registry::submit(
            project,
            database,
            Path::new(&path),
            &now
        )?)),
        MaintenanceRequest::PreparePlugin { candidate_id } => {
            // Dropping preparation also drops its process-tree guard before the owner exits.
            tokio::select! {
                biased;
                _ = cancel.cancelled() => Err(CoreError::new(codes::CANCELLED, "dependency preparation cancelled")),
                value = registry::prepare(project, database, &candidate_id) => value,
            }
        }
        MaintenanceRequest::ValidatePlugin { candidate_ids } => {
            registry::validate_many(project, database, &candidate_ids, cancel, &now).await
        }
        MaintenanceRequest::ActivatePlugin { candidate_id } => {
            registry::activate_with_cancellation(project, database, &candidate_id, cancel, &now)
                .await
        }
        MaintenanceRequest::RollbackPlugin { plugin_id, version } => {
            registry::rollback(project, database, &plugin_id, &version, cancel, &now).await
        }
        MaintenanceRequest::Collect {} => {
            let lock = registry::MaintenanceLock::acquire(project)?;
            lock.recover(project, database, &now)?;
            registry::check_active_artifacts(project, database)?;
            crate::index::incremental::invalidate_for_collect_with_cancellation(
                project, database, cancel, &now,
            )
            .await?;
            let paths = crate::storage::active_version_map(database.connection())?
                .into_iter()
                .map(|(id, version)| {
                    crate::storage::plugins::published_manifest(
                        database.connection(),
                        &id,
                        &version,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            let staged =
                staging::stage_collection(project, database, &paths, None, cancel, &now).await?;
            let mut report = staged.report();
            check_cancel(cancel)?;
            report["generation"] = json!(
                staged
                    .commit_with_cancellation(project, database, cancel, &now)
                    .await?
                    .to_string()
            );
            report["status"] = json!("committed");
            Ok(report)
        }
        MaintenanceRequest::ExportContext { request } => Ok(json!(context::export_with_control(
            database, project, &request, &now, &control
        )?)),
        MaintenanceRequest::ImportTopic { artifact } => {
            documents::import_topic_with_control(database, project, &artifact, &now, &control)
        }
        MaintenanceRequest::ImportConventions { input } => {
            documents::import_conventions_with_control(database, project, &input, &now, &control)
        }
        MaintenanceRequest::GenerateSkill { name, output } => {
            let output = output.map(std::path::PathBuf::from).unwrap_or_else(|| {
                project
                    .base_dir
                    .join(".project-knowledge/skills")
                    .join(&name)
            });
            skill::generate_with_checkpoint(database, project, &name, &output, &now, |_| {
                control.check()
            })
        }
    }
}

/// No config, index, migration, runtime discovery, plugin or model is needed.
pub fn capabilities() -> Value {
    use crate::contract::{entity::ALL_ENTITY_KINDS, relation::ALL_RELATION_KINDS};
    let schema = crate::contract::schema::document();
    let bytes = crate::contract::identity::canonical_config_json(&schema).expect("schema JSON");
    json!({
        "protocolVersion":crate::contract::PROTOCOL_VERSION,
        "schemaHash":crate::contract::identity::content_hash(bytes.as_bytes()),
        "schema":schema,
        "entityKinds":ALL_ENTITY_KINDS.iter().map(|k| k.as_str()).collect::<Vec<_>>(),
        "relationKinds":ALL_RELATION_KINDS.iter().map(|k| k.as_str()).collect::<Vec<_>>(),
        "operations":["status","search","locateFeature","trace","neighbors","codeMap","readSource","page","workflow","report"],
        "maintenanceOperations":["index","discoverPlugins","submitPlugin","preparePlugin","validatePlugin","activatePlugin","rollbackPlugin","collect","exportContext","importTopic","importConventions","generateSkill"],
        "languages":if cfg!(feature="syntax") { vec!["typescript","tsx","javascript","python"] } else { vec![] },
        "providers":[],"pluginRuntimes":["node","python"],"runtimeChecked":false,
        "limits":{"maxEntities":1000,"maxRelations":3000,"maxVisited":20000,"maxDepth":16,"maxResponseChars":200000,"timeoutMs":30000,"pageSize":100}
    })
}
