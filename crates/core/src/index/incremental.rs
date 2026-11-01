//! 无模型增量：比较完整输入，先持久失效，再在暂存库按 DAG 运行，业务组一次提交。
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use rusqlite::Connection;
use tokio_util::sync::CancellationToken;

use crate::collect::import::{digest, export_input};
use crate::collect::manifest::validate_stage_dag;
use crate::collect::registry::{MaintenanceLock, check_active_artifacts};
use crate::collect::staging::{
    StagedCollection, resolve_runtime, stage_selected_collection, store_batch,
};
use crate::collect::workspace::StagingWorkspace;
use crate::contract::collector::{PluginManifest, StagePhase, StageSpec};
use crate::contract::error::{CoreError, codes};
use crate::index::build::BaseIndexResult;
use crate::index::dependencies::{
    DependencyChange, DependencyKindTag, batch_dependencies, dependency_closure,
};
use crate::project::config::LoadedProject;
use crate::storage::{self, Database, database_error};

#[derive(Debug, Clone)]
pub struct IncrementalResult {
    pub base: BaseIndexResult,
    pub invalidated: Vec<String>,
    pub executed: Vec<String>,
    pub generation: u64,
}

struct Plan {
    manifests: Vec<PluginManifest>,
    paths: Vec<PathBuf>,
    stages: BTreeMap<String, (usize, StageSpec)>,
    runtimes: BTreeMap<String, String>,
    base: BaseIndexResult,
    affected: BTreeSet<String>,
}

/// Core 拥有整个维护生命周期；查询不会调用本入口。
pub async fn run_incremental(
    project: &LoadedProject,
    database: &mut Database,
    cancel: &CancellationToken,
    now: &str,
) -> Result<IncrementalResult, CoreError> {
    crate::client::check_cancel(cancel)?;
    let lock = MaintenanceLock::acquire(project)?;
    lock.recover(project, database, now)?;
    let work = StagingWorkspace::new(project)?;
    let path = work.path().join("incremental.sqlite");
    storage::backup_to(database.path(), &path)?;
    let mut preview = Database::open_maintenance(
        &path,
        &project.config.project_id,
        &project.config_hash()?,
        now,
    )?;
    let plan = prepare_plan(project, database, &mut preview, now, cancel).await?;
    let mut invalidated = plan.affected.clone();
    persist_invalidation(project, database, &invalidated)?;
    if cancel.is_cancelled() {
        return Err(CoreError::new(
            codes::CANCELLED,
            "incremental cancelled after invalidation",
        ));
    }
    // 基础层只重建改变的仓库；业务旧批次在基础结果可见之前已失效。
    let mut base = if database.current_source_snapshot_id()?.as_deref()
        != Some(&plan.base.snapshot.id)
        || base_changed(database.connection(), preview.connection())?
    {
        crate::index::build::build_base_index_with_cancellation(project, database, now, cancel)?
    } else {
        let mut base = plan.base.clone();
        base.generation = database.generation()?;
        base
    };
    if base.snapshot.id != plan.base.snapshot.id {
        let keys = business_keys(database.connection())?;
        persist_invalidation(project, database, &keys)?;
        return Err(CoreError::new(
            codes::SOURCE_CHANGED,
            "sources changed while preparing incremental update",
        ));
    }
    // 重新复制已提交的基础快照。逐阶段把完整结果写入私有库，正式业务只切换一次。
    drop(preview);
    let path = work.path().join("execution.sqlite");
    storage::backup_to(database.path(), &path)?;
    let mut staging = Database::open_maintenance(
        &path,
        &project.config.project_id,
        &project.config_hash()?,
        now,
    )?;
    let mut pending: BTreeSet<_> = plan.stages.keys().cloned().collect();
    let mut executed = Vec::new();
    let mut combined: Option<StagedCollection> = None;
    let mut runs = Vec::new();
    let mut expected_generation = database.generation()?;
    while !pending.is_empty() {
        let key = pending
            .iter()
            .filter(|key| {
                plan.stages[*key]
                    .1
                    .depends_on
                    .iter()
                    .all(|d| !pending.contains(d))
            })
            .min_by_key(|key| {
                (
                    if plan.stages[*key].1.phase == StagePhase::Extract {
                        0
                    } else {
                        1
                    },
                    *key,
                )
            })
            .cloned()
            .ok_or_else(|| CoreError::new(codes::CONFIG_INVALID, "stage dependency cycle"))?;
        pending.remove(&key);
        let replacement = replacement_key(project, &key);
        if !invalidated.contains(&replacement) && !stage_changed(project, &staging, &plan, &key)? {
            continue;
        }
        let affected = affected_closure(
            project,
            database.connection(),
            &plan,
            BTreeSet::from([replacement.clone()]),
        )?;
        if database.generation()? != expected_generation {
            return Err(CoreError::new(
                codes::ACTIVATION_CONFLICT,
                "index changed during incremental execution",
            ));
        }
        persist_invalidation(project, database, &affected)?;
        expected_generation = database.generation()?;
        invalidated.extend(affected);
        let run_id = format!("run_{}", uuid::Uuid::new_v4());
        storage::create_run(
            database.connection(),
            &run_id,
            "index.incremental",
            &key,
            Some(&base.snapshot.id),
            now,
        )?;
        runs.push(run_id.clone());
        // 不把自身与显式下游旧断言喂回本阶段，与首次整组 extract/link 一致。
        let result = stage_selected_collection(
            project,
            &staging,
            &plan.paths,
            None,
            Some(&BTreeSet::from([key.clone()])),
            cancel,
            now,
        )
        .await;
        let group = match result {
            Ok(group) => group,
            Err(error) => {
                for run in &runs {
                    storage::finish_run(
                        database.connection(),
                        run,
                        if error.code == codes::CANCELLED {
                            storage::RunStatus::Cancelled
                        } else {
                            storage::RunStatus::Failed
                        },
                        Some(&serde_json::to_value(&error).expect("error JSON")),
                        now,
                    )?;
                }
                return Err(error.with_phase(&key).with_run_id(&run_id));
            }
        };
        let transaction = staging.write_transaction()?;
        for batch in &group.batches {
            store_batch(&transaction, batch, base.generation + 1, now)?;
        }
        transaction.commit().map_err(database_error)?;
        if let Some(combined) = &mut combined {
            combined.batches.extend(group.batches);
            combined.cases.extend(group.cases);
        } else {
            combined = Some(group);
        }
        executed.push(key);
    }
    if cancel.is_cancelled() {
        for run in &runs {
            storage::finish_run(
                database.connection(),
                run,
                storage::RunStatus::Cancelled,
                None,
                now,
            )?;
        }
        return Err(CoreError::new(
            codes::CANCELLED,
            "incremental cancelled before result commit",
        ));
    }
    if let Some(mut group) = combined {
        group.expected_generation = expected_generation;
        group.result_hash = digest(&(&group.batches, &group.cases));
        if let Err(error) = group
            .commit_with_cancellation(project, database, cancel, now)
            .await
        {
            if matches!(
                error.code.as_str(),
                codes::SOURCE_CHANGED | codes::DEPENDENCY_CHANGED | codes::ARTIFACT_CHANGED
            ) {
                persist_invalidation(project, database, &business_keys(database.connection())?)?;
            }
            for run in &runs {
                storage::finish_run(
                    database.connection(),
                    run,
                    storage::RunStatus::Failed,
                    Some(&serde_json::to_value(&error).expect("error JSON")),
                    now,
                )?;
            }
            return Err(error);
        }
    }
    if let Err(error) = crate::collect::staging::verify_sources(project, &base.snapshot.id) {
        persist_invalidation(project, database, &business_keys(database.connection())?)?;
        for run in &runs {
            storage::finish_run(
                database.connection(),
                run,
                storage::RunStatus::Failed,
                Some(&serde_json::to_value(&error).expect("error JSON")),
                now,
            )?;
        }
        return Err(error);
    }
    for run in &runs {
        storage::finish_run(
            database.connection(),
            run,
            storage::RunStatus::Succeeded,
            None,
            now,
        )?;
    }
    base.generation = database.generation()?;
    Ok(IncrementalResult {
        generation: base.generation,
        base,
        invalidated: invalidated.into_iter().collect(),
        executed,
    })
}

/// 显式 collect 不刷新基础索引，但在任何失败前登记已确认的输入变化。调用方持锁。
pub async fn invalidate_for_collect(
    project: &LoadedProject,
    database: &mut Database,
    now: &str,
) -> Result<Vec<String>, CoreError> {
    invalidate_for_collect_with_cancellation(project, database, &CancellationToken::new(), now)
        .await
}

pub async fn invalidate_for_collect_with_cancellation(
    project: &LoadedProject,
    database: &mut Database,
    cancel: &CancellationToken,
    now: &str,
) -> Result<Vec<String>, CoreError> {
    let work = StagingWorkspace::new(project)?;
    let path = work.path().join("plan.sqlite");
    storage::backup_to(database.path(), &path)?;
    let mut preview = Database::open_maintenance(
        &path,
        &project.config.project_id,
        &project.config_hash()?,
        now,
    )?;
    let plan = prepare_plan(project, database, &mut preview, now, cancel).await?;
    persist_invalidation(project, database, &plan.affected)?;
    Ok(plan.affected.into_iter().collect())
}

/// --base 保留无 Node/Python 的独立可用性，不解析或执行插件。
pub fn run_base_only(
    project: &LoadedProject,
    database: &mut Database,
    now: &str,
) -> Result<IncrementalResult, CoreError> {
    run_base_only_with_cancellation(project, database, &CancellationToken::new(), now)
}

pub fn run_base_only_with_cancellation(
    project: &LoadedProject,
    database: &mut Database,
    cancel: &CancellationToken,
    now: &str,
) -> Result<IncrementalResult, CoreError> {
    crate::client::check_cancel(cancel)?;
    let lock = MaintenanceLock::acquire(project)?;
    lock.recover(project, database, now)?;
    let keys = business_keys(database.connection())?;
    persist_invalidation(project, database, &keys)?;
    let base =
        crate::index::build::build_base_index_with_cancellation(project, database, now, cancel)?;
    Ok(IncrementalResult {
        generation: base.generation,
        base,
        invalidated: keys.into_iter().collect(),
        executed: Vec::new(),
    })
}

async fn prepare_plan(
    project: &LoadedProject,
    database: &mut Database,
    preview: &mut Database,
    now: &str,
    cancel: &CancellationToken,
) -> Result<Plan, CoreError> {
    check_active_artifacts(project, database)?;
    let base = match crate::index::build::build_base_index_with_cancellation(
        project, preview, now, cancel,
    ) {
        Ok(base) => base,
        Err(error) => {
            persist_invalidation(project, database, &business_keys(database.connection())?)?;
            return Err(error);
        }
    };
    let manifests = storage::plugins::active_manifests(database.connection())?;
    validate_stage_dag(&manifests)?;
    let mut plan = Plan {
        manifests,
        paths: Vec::new(),
        stages: BTreeMap::new(),
        runtimes: BTreeMap::new(),
        base,
        affected: BTreeSet::new(),
    };
    for (index, manifest) in plan.manifests.iter().enumerate() {
        plan.paths.push(storage::plugins::published_manifest(
            database.connection(),
            &manifest.plugin_id,
            &manifest.version,
        )?);
        for stage in &manifest.stages {
            plan.stages.insert(
                format!("{}:{}", manifest.plugin_id, stage.id),
                (index, stage.clone()),
            );
        }
        let runtime = match resolve_runtime(manifest, project).await {
            Ok(runtime) => runtime,
            Err(error) => {
                persist_invalidation(project, database, &business_keys(database.connection())?)?;
                return Err(error);
            }
        };
        plan.runtimes.insert(
            manifest.plugin_id.clone(),
            digest(&(runtime.executable.to_string_lossy(), &runtime.version)),
        );
        if manifest.dependencies.is_some() {
            let directory = plan
                .paths
                .last()
                .expect("published path")
                .parent()
                .expect("manifest parent");
            let validated = crate::collect::manifest::validate_manifest(
                manifest,
                directory,
                &project.config.runtimes,
            )?;
            let prepared = crate::collect::deps::plugin_plan(&validated, &runtime).and_then(
                |(plan, inputs)| crate::collect::deps::prepared_dependencies(&plan, &inputs),
            );
            match prepared {
                Ok(prepared) => {
                    plan.runtimes.insert(
                        format!("{}:dependencies", manifest.plugin_id),
                        digest(&(&prepared.digest, &prepared.integrity)),
                    );
                }
                Err(error) => {
                    persist_invalidation(
                        project,
                        database,
                        &business_keys(database.connection())?,
                    )?;
                    return Err(error);
                }
            }
        }
    }
    for key in plan.stages.keys() {
        if stage_changed(project, preview, &plan, key)? {
            plan.affected.insert(replacement_key(project, key));
        }
    }
    for (key, id) in storage::current_batches(database.connection())? {
        if key.starts_with("(core.base,")
            && storage::current_batch(preview.connection(), &key)?.as_ref() != Some(&id)
        {
            plan.affected.insert(key);
        }
    }
    plan.affected = affected_closure(project, database.connection(), &plan, plan.affected.clone())?;
    Ok(plan)
}

fn stage_changed(
    project: &LoadedProject,
    database: &Database,
    plan: &Plan,
    key: &str,
) -> Result<bool, CoreError> {
    let replacement = replacement_key(project, key);
    let Some(batch) = storage::current_batch(database.connection(), &replacement)? else {
        return Ok(true);
    };
    if storage::batch_state(database.connection(), &batch)? != Some(storage::BatchState::Fresh) {
        return Ok(true);
    }
    let previous = batch_dependencies(database.connection(), &batch)?;
    let (index, stage) = &plan.stages[key];
    let manifest = &plan.manifests[*index];
    mask_stage(project, database.connection(), plan, key)?;
    let result = export_input(
        database.connection(),
        project,
        stage,
        &plan.base.snapshot.id,
        "incremental-plan",
        &manifest.plugin_id,
        &manifest.version,
    );
    unmask_stage(database.connection())?;
    let input = result?;
    let mut current = BTreeMap::new();
    current.insert(
        ("config".into(), project.config.project_id.clone()),
        project.config_hash()?,
    );
    for (id, digest) in &plan.runtimes {
        current.insert(("runtime".into(), id.clone()), digest.clone());
    }
    for source in input.sources {
        current.insert(
            ("source".into(), source.source.id),
            source.source.content_hash,
        );
    }
    for (kind, sets) in [
        (
            "sourceSet",
            input
                .source_sets
                .iter()
                .map(|s| (&s.selector_id, &s.digest))
                .collect::<Vec<_>>(),
        ),
        (
            "candidateSet",
            input
                .candidate_sets
                .iter()
                .map(|s| (&s.selector_id, &s.digest))
                .collect(),
        ),
    ] {
        for (id, digest) in sets {
            current.insert(
                (
                    kind.to_string(),
                    format!("{}:{}:{id}", manifest.plugin_id, stage.id),
                ),
                digest.clone(),
            );
        }
    }
    for assertion in input
        .entities
        .iter()
        .flat_map(|e| &e.assertions)
        .chain(input.relations.iter().flat_map(|r| &r.assertions))
    {
        current.insert(
            ("upstreamBatch".into(), assertion.batch_id.clone()),
            assertion.batch_id.clone(),
        );
    }
    for dependency in &stage.depends_on {
        if let Some(batch) =
            storage::current_batch(database.connection(), &replacement_key(project, dependency))?
        {
            current.insert(("upstreamBatch".into(), batch.clone()), batch);
        }
    }
    Ok(previous.iter().any(|((kind, key), value)| {
        kind != "artifact" && current.get(&(kind.clone(), key.clone())) != Some(value)
    }) || current.iter().any(|((kind, key), value)| {
        kind != "runtime" && previous.get(&(kind.clone(), key.clone())) != Some(value)
    }))
}

fn mask_stage(
    project: &LoadedProject,
    connection: &Connection,
    plan: &Plan,
    key: &str,
) -> Result<(), CoreError> {
    let mut masked = BTreeSet::from([key.to_string()]);
    loop {
        let before = masked.len();
        for (key, (_, stage)) in &plan.stages {
            if stage.depends_on.iter().any(|d| masked.contains(d)) {
                masked.insert(key.clone());
            }
        }
        if masked.len() == before {
            break;
        }
    }
    connection
        .execute_batch("SAVEPOINT incremental_input")
        .map_err(database_error)?;
    for key in masked {
        connection
            .execute(
                "DELETE FROM current_batches WHERE replacement_key=?1",
                [replacement_key(project, &key)],
            )
            .map_err(database_error)?;
    }
    Ok(())
}
fn unmask_stage(connection: &Connection) -> Result<(), CoreError> {
    connection
        .execute_batch("ROLLBACK TO incremental_input; RELEASE incremental_input")
        .map_err(database_error)
}
fn replacement_key(project: &LoadedProject, key: &str) -> String {
    let (plugin, stage) = key.split_once(':').expect("validated stage key");
    storage::plugin_replacement_key(plugin, stage, &project.config.project_id)
}
fn business_keys(connection: &Connection) -> Result<BTreeSet<String>, CoreError> {
    Ok(storage::current_batches(connection)?
        .into_iter()
        .filter(|(k, _)| !k.starts_with("(core.base,"))
        .map(|(k, _)| k)
        .collect())
}
fn base_changed(before: &Connection, after: &Connection) -> Result<bool, CoreError> {
    Ok(storage::current_batches(before)?
        .into_iter()
        .filter(|(k, _)| k.starts_with("(core.base,"))
        .collect::<Vec<_>>()
        != storage::current_batches(after)?
            .into_iter()
            .filter(|(k, _)| k.starts_with("(core.base,"))
            .collect::<Vec<_>>())
}
fn affected_closure(
    project: &LoadedProject,
    connection: &Connection,
    plan: &Plan,
    mut keys: BTreeSet<String>,
) -> Result<BTreeSet<String>, CoreError> {
    loop {
        let before = keys.len();
        let changes = storage::current_batches(connection)?
            .into_iter()
            .filter(|(k, _)| keys.contains(k))
            .map(|(_, id)| DependencyChange {
                kind: DependencyKindTag::UpstreamBatch,
                key: id,
                digest: String::new(),
            })
            .collect::<Vec<_>>();
        keys.extend(
            dependency_closure(connection, &changes)?
                .into_iter()
                .map(|(_, key)| key),
        );
        for (key, (_, stage)) in &plan.stages {
            if stage
                .depends_on
                .iter()
                .any(|d| keys.contains(&replacement_key(project, d)))
            {
                keys.insert(replacement_key(project, key));
            }
        }
        if before == keys.len() {
            return Ok(keys);
        }
    }
}
fn persist_invalidation(
    project: &LoadedProject,
    database: &mut Database,
    keys: &BTreeSet<String>,
) -> Result<(), CoreError> {
    crate::knowledge::documents::refresh(database, project)?;
    let generation = database.generation()? + 1;
    let transaction = database.write_transaction()?;
    if storage::invalidate_batches(
        &transaction,
        &keys.iter().cloned().collect::<Vec<_>>(),
        generation,
    )? > 0
    {
        transaction
            .execute(
                "UPDATE index_meta SET generation=?1 WHERE id=1",
                [generation as i64],
            )
            .map_err(database_error)?;
        storage::knowledge::invalidate_batches(
            &transaction,
            &keys.iter().cloned().collect::<Vec<_>>(),
        )?;
    }
    transaction.commit().map_err(database_error)
}

pub fn current_freshness(
    connection: &Connection,
) -> Result<Vec<(String, String, Option<u64>)>, CoreError> {
    storage::current_batches(connection)?
        .into_iter()
        .map(|(key, id)| {
            Ok((
                key,
                storage::batch_state(connection, &id)?
                    .map(|s| s.as_str().to_string())
                    .unwrap_or_default(),
                storage::batch_invalidated_generation(connection, &id)?,
            ))
        })
        .collect()
}
