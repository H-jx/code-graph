//! 候选登记、绑定验证、不可变发布与事务 C。数据库是活动指针的唯一来源。
use super::commit::StagedBatch;
use super::import::{database_error, digest};
use super::manifest::{ValidatedManifest, load_manifest, validate_manifest};
use super::runner::ResolvedRuntime;
use super::staging::{
    CaseResult, StagedCollection, invalidate_downstream, stage_collection,
    stage_selected_collection, store_batch,
};
use crate::contract::error::{CoreError, codes};
use crate::project::config::LoadedProject;
use crate::storage::plugins::{Candidate, candidate, insert_candidate, update_candidate};
use crate::storage::{self, Database};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use tokio_util::sync::CancellationToken;

/// 进程退出自动释放；锁文件本身保留，不能以删除文件的方式绕开互斥。
pub struct MaintenanceLock {
    _file: File,
}
impl Drop for MaintenanceLock {
    fn drop(&mut self) {
        // flock 的 open-file-description 可在其他线程 fork 时被短暂继承。
        // 显式解锁，不能仅依赖最后一个 fd 的关闭时间。
        let _ = self._file.unlock();
    }
}
impl MaintenanceLock {
    pub fn acquire(project: &LoadedProject) -> Result<Self, CoreError> {
        let directory = project.base_dir.join(".project-knowledge/work");
        std::fs::create_dir_all(&directory).map_err(io_error)?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(directory.join("maintenance.lock"))
            .map_err(io_error)?;
        file.try_lock().map_err(|_| {
            CoreError::new(
                codes::BUSY,
                "another project maintenance operation owns the lock",
            )
        })?;
        Ok(Self { _file: file })
    }
    pub fn recover(
        &self,
        project: &LoadedProject,
        database: &Database,
        now: &str,
    ) -> Result<usize, CoreError> {
        let records = storage::plugins::recover_candidates(database.connection(), now)?;
        Ok(records + super::workspace::cleanup(project)?)
    }
}

fn io_error(error: std::io::Error) -> CoreError {
    CoreError::new(
        codes::INTERNAL_ERROR,
        format!("plugin artifact I/O: {error}"),
    )
}

#[derive(Serialize, Deserialize)]
struct Binding {
    #[serde(default)]
    candidate_ids: Vec<String>,
    expected_generation: u64,
    source_snapshot_id: String,
    active_versions: Vec<(String, String)>,
    config_hash: String,
    batches: Vec<StagedBatch>,
    manifests: Vec<ValidatedManifest>,
    runtimes: Vec<ResolvedRuntime>,
    #[serde(default)]
    prepared: Vec<Option<super::deps::PreparedDependencies>>,
    result_hash: String,
    cases: Vec<CaseResult>,
    run_id: String,
}
impl Binding {
    fn from_group(group: StagedCollection, candidate_ids: &[String]) -> Self {
        Self {
            candidate_ids: candidate_ids.to_vec(),
            expected_generation: group.expected_generation,
            source_snapshot_id: group.source_snapshot_id,
            active_versions: group.active_versions,
            config_hash: group.config_hash,
            batches: group.batches,
            manifests: group.manifests,
            runtimes: group.runtimes,
            prepared: group.prepared,
            result_hash: group.result_hash,
            cases: group.cases,
            run_id: group.run_id,
        }
    }
    fn into_group(self) -> StagedCollection {
        StagedCollection {
            expected_generation: self.expected_generation,
            source_snapshot_id: self.source_snapshot_id,
            active_versions: self.active_versions,
            config_hash: self.config_hash,
            batches: self.batches,
            manifests: self.manifests,
            runtimes: self.runtimes,
            prepared: self.prepared,
            result_hash: self.result_hash,
            cases: self.cases,
            run_id: self.run_id,
        }
    }
}

pub fn recover(
    project: &LoadedProject,
    database: &mut Database,
    now: &str,
) -> Result<usize, CoreError> {
    let _lock = MaintenanceLock::acquire(project)?;
    _lock.recover(project, database, now)
}

pub fn submit(
    project: &LoadedProject,
    database: &mut Database,
    path: &Path,
    now: &str,
) -> Result<Candidate, CoreError> {
    let _lock = MaintenanceLock::acquire(project)?;
    _lock.recover(project, database, now)?;
    submit_locked(project, database, path, now)
}

/// 只在维护入口调用。扫描不等于执行，发现的制品仍进入 generated 状态。
pub fn discover(
    project: &LoadedProject,
    database: &mut Database,
    now: &str,
) -> Result<Vec<Candidate>, CoreError> {
    if !project.config.plugins.auto_load {
        return Ok(Vec::new());
    }
    let _lock = MaintenanceLock::acquire(project)?;
    _lock.recover(project, database, now)?;
    let paths = discovery_paths(project)?;
    let mut manifests = std::collections::BTreeMap::new();
    for path in paths {
        let m = load_manifest(&path)?;
        let checked = validate_manifest(
            &m,
            path.parent().expect("manifest parent"),
            &project.config.runtimes,
        )?;
        let key = (m.plugin_id, m.version);
        if let Some((hash, _)) = manifests.get(&key) {
            if hash != &checked.artifact_hash {
                return Err(CoreError::new(
                    codes::ARTIFACT_CHANGED,
                    "same plugin version discovered with conflicting artifacts",
                ));
            }
        } else {
            manifests.insert(key, (checked.artifact_hash, path));
        }
    }
    let mut found = Vec::new();
    for ((plugin, version), (hash, path)) in manifests {
        let known:bool=database.connection().query_row("SELECT EXISTS(SELECT 1 FROM plugin_candidates WHERE plugin_id=?1 AND version=?2 AND artifact_hash=?3)",rusqlite::params![plugin,version,hash],|r|r.get(0)).map_err(database_error)?;
        if !known {
            found.push(submit_locked(project, database, &path, now)?);
        }
    }
    Ok(found)
}

pub fn discovery_paths(project: &LoadedProject) -> Result<Vec<PathBuf>, CoreError> {
    let mut paths = project.collector_manifest_paths();
    if project.config.plugins.auto_load {
        for directory in project.plugin_directories() {
            discover_directory(&directory, 0, &mut paths)?;
        }
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn discover_directory(
    directory: &Path,
    depth: usize,
    paths: &mut Vec<PathBuf>,
) -> Result<(), CoreError> {
    if depth > 3 || !directory.exists() {
        return Ok(());
    }
    let metadata = std::fs::symlink_metadata(directory).map_err(io_error)?;
    if metadata.file_type().is_symlink() {
        return Err(CoreError::new(
            codes::CONFIG_INVALID,
            "plugin discovery refuses symlink directories",
        ));
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err(CoreError::new(
                codes::CONFIG_INVALID,
                "plugin discovery refuses reparse points",
            ));
        }
    }
    if directory.join("manifest.json").is_file() {
        paths.push(directory.join("manifest.json"));
        return Ok(());
    }
    for entry in std::fs::read_dir(directory).map_err(io_error)? {
        let entry = entry.map_err(io_error)?;
        if entry.file_type().map_err(io_error)?.is_dir() {
            discover_directory(&entry.path(), depth + 1, paths)?;
        }
    }
    Ok(())
}

fn submit_locked(
    project: &LoadedProject,
    database: &Database,
    path: &Path,
    now: &str,
) -> Result<Candidate, CoreError> {
    let directory = if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent()
            .ok_or_else(|| CoreError::new(codes::CONFIG_INVALID, "candidate has no parent"))?
            .to_path_buf()
    };
    let manifest = load_manifest(&directory.join("manifest.json"))?;
    for part in [&manifest.plugin_id, &manifest.version] {
        if part.is_empty()
            || part == "."
            || part == ".."
            || !part
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        {
            return Err(CoreError::new(
                codes::CONFIG_INVALID,
                "plugin ID and version must be safe path components",
            ));
        }
    }
    let validated = validate_manifest(&manifest, &directory, &project.config.runtimes)?;
    let existing = storage::plugins::published_manifest(
        database.connection(),
        &manifest.plugin_id,
        &manifest.version,
    );
    if let Ok(path) = existing {
        let published = load_manifest(&path)?;
        let published = validate_manifest(
            &published,
            path.parent().expect("published parent"),
            &project.config.runtimes,
        )?;
        if published.artifact_hash != validated.artifact_hash {
            return Err(CoreError::new(
                codes::ARTIFACT_CHANGED,
                "published version is immutable; use a new version",
            ));
        }
    }
    let id = format!("candidate_{}", uuid::Uuid::new_v4());
    let destination = project
        .base_dir
        .join(".project-knowledge/work/candidates")
        .join(&id);
    copy_artifact(&validated, &destination, &project.config.runtimes)?;
    let result = Candidate {
        id,
        plugin_id: manifest.plugin_id,
        version: manifest.version,
        artifact_hash: validated.artifact_hash,
        directory: destination.to_string_lossy().to_string(),
        state: "generated".into(),
        validation_id: None,
        staged_json: None,
    };
    insert_candidate(database.connection(), &result, now)?;
    Ok(result)
}

fn copy_artifact(
    manifest: &ValidatedManifest,
    destination: &Path,
    runtimes: &[crate::project::config::RuntimeConfig],
) -> Result<(), CoreError> {
    let parent = destination
        .parent()
        .ok_or_else(|| CoreError::new(codes::CONFIG_INVALID, "artifact has no parent"))?;
    std::fs::create_dir_all(parent).map_err(io_error)?;
    if destination.exists() {
        let current = load_manifest(&destination.join("manifest.json"))?;
        let copied = validate_manifest(&current, destination, runtimes)?;
        if copied.artifact_hash != manifest.artifact_hash {
            return Err(CoreError::new(
                codes::ARTIFACT_CHANGED,
                "immutable destination has different content",
            ));
        }
        return Ok(());
    }
    let temporary = tempfile::Builder::new()
        .prefix(".publishing-")
        .tempdir_in(parent)
        .map_err(io_error)?;
    for file in &manifest.files {
        let output = temporary.path().join(&file.relative_path);
        std::fs::create_dir_all(output.parent().expect("artifact parent")).map_err(io_error)?;
        let bytes =
            std::fs::read(manifest.directory.join(&file.relative_path)).map_err(io_error)?;
        if crate::contract::identity::content_hash(&bytes) != file.content_hash {
            return Err(CoreError::new(
                codes::ARTIFACT_CHANGED,
                "artifact changed while publishing",
            ));
        }
        let mut handle = File::create(&output).map_err(io_error)?;
        std::io::Write::write_all(&mut handle, &bytes).map_err(io_error)?;
        handle.sync_all().map_err(io_error)?;
    }
    std::fs::rename(temporary.path(), destination).map_err(io_error)?;
    Ok(())
}

pub async fn validate(
    project: &LoadedProject,
    database: &mut Database,
    id: &str,
    cancel: &CancellationToken,
    now: &str,
) -> Result<serde_json::Value, CoreError> {
    validate_many(project, database, &[id.to_string()], cancel, now).await
}

/// 唯一联网的依赖入口；validate/collect/activate 不会调用本函数。
pub async fn prepare(
    project: &LoadedProject,
    database: &Database,
    id: &str,
) -> Result<serde_json::Value, CoreError> {
    let candidate = candidate(database.connection(), id)?;
    let directory = PathBuf::from(&candidate.directory);
    let manifest = load_manifest(&directory.join("manifest.json"))?;
    let checked = validate_manifest(&manifest, &directory, &project.config.runtimes)?;
    if checked.artifact_hash != candidate.artifact_hash {
        return Err(CoreError::new(
            codes::ARTIFACT_CHANGED,
            "submitted candidate changed before prepare",
        ));
    }
    if manifest.dependencies.is_none() {
        return Ok(serde_json::json!({"candidateId":id,"status":"noDependencies"}));
    }
    let runtime = super::staging::resolve_runtime(&manifest, project).await?;
    let (plan, inputs) = super::deps::plugin_plan(&checked, &runtime)?;
    let prepared = super::deps::prepare_dependencies(&plan, &inputs).await?;
    Ok(serde_json::json!({"candidateId":id,"status":"dependenciesPrepared","cache":prepared}))
}

pub async fn validate_many(
    project: &LoadedProject,
    database: &mut Database,
    ids: &[String],
    cancel: &CancellationToken,
    now: &str,
) -> Result<serde_json::Value, CoreError> {
    let id = ids
        .first()
        .ok_or_else(|| CoreError::new(codes::INVALID_REQUEST, "candidate group is empty"))?;
    if ids.iter().collect::<BTreeSet<_>>().len() != ids.len() {
        return Err(CoreError::new(
            codes::INVALID_REQUEST,
            "candidate IDs must be unique",
        ));
    }
    let _lock = MaintenanceLock::acquire(project)?;
    _lock.recover(project, database, now)?;
    check_active_artifacts(project, database)?;
    let mut candidates = Vec::new();
    let mut paths = Vec::new();
    for id in ids {
        let candidate = candidate(database.connection(), id)?;
        if matches!(candidate.state.as_str(), "active" | "superseded") {
            return Err(CoreError::new(
                codes::INVALID_REQUEST,
                "resubmit an active artifact to obtain a new candidate",
            ));
        }
        let path = PathBuf::from(&candidate.directory).join("manifest.json");
        let manifest = load_manifest(&path)?;
        let checked = validate_manifest(
            &manifest,
            path.parent().expect("candidate directory"),
            &project.config.runtimes,
        )?;
        if checked.artifact_hash != candidate.artifact_hash {
            return Err(CoreError::new(
                codes::ARTIFACT_CHANGED,
                "submitted candidate changed",
            ));
        }
        candidates.push(candidate);
        paths.push(path);
    }
    let candidate = &candidates[0];
    for id in ids {
        update_candidate(database.connection(), id, "prepared", None, None, None)?;
    }
    let result = validate_group(project, database, &paths, cancel, now).await;
    let group = match result {
        Ok(group) => group,
        Err(error) => {
            persist_input_change(database, &error)?;
            for id in ids {
                update_candidate(
                    database.connection(),
                    id,
                    "failed",
                    None,
                    None,
                    Some(&serde_json::to_string(&error).expect("error JSON")),
                )?;
            }
            return Err(error);
        }
    };
    let report = group.report();
    let validation_id = format!("validation_{}", uuid::Uuid::new_v4());
    let candidate_digest = digest(
        &group
            .batches
            .iter()
            .flat_map(|b| &b.dependencies)
            .filter(|d| d.kind == crate::contract::collector::DependencyKind::CandidateSet)
            .collect::<Vec<_>>(),
    );
    let truth_hash = digest(&group.cases);
    let input_digest = digest(
        &group
            .batches
            .iter()
            .map(|b| &b.input_digest)
            .collect::<Vec<_>>(),
    );
    let map = serde_json::json!(group.active_versions);
    let hash = group.result_hash.clone();
    let staged_json =
        serde_json::to_string(&Binding::from_group(group, ids)).expect("binding JSON");
    crate::client::check_cancel(cancel)?;
    let transaction = database.write_transaction()?;
    storage::record_validation(
        &transaction,
        &validation_id,
        &candidate.artifact_hash,
        &input_digest,
        &map,
        &candidate_digest,
        &truth_hash,
        "passed",
        Some(&hash),
        now,
    )?;
    for id in ids {
        update_candidate(
            &transaction,
            id,
            "validated",
            Some(&validation_id),
            Some(&staged_json),
            None,
        )?;
    }
    transaction.commit().map_err(database_error)?;
    Ok(
        serde_json::json!({"candidateId":id,"candidateIds":ids,"validationId":validation_id,"report":report}),
    )
}

async fn validate_group(
    project: &LoadedProject,
    database: &Database,
    primary: &[PathBuf],
    cancel: &CancellationToken,
    now: &str,
) -> Result<StagedCollection, CoreError> {
    let group = stage_collection(project, database, primary, None, cancel, now).await?;
    let plugins = group
        .manifests
        .iter()
        .map(|m| m.manifest.plugin_id.clone())
        .collect::<BTreeSet<_>>();
    let mut selected: BTreeSet<String> = group
        .manifests
        .iter()
        .flat_map(|m| {
            m.manifest
                .stages
                .iter()
                .map(|s| format!("{}:{}", m.manifest.plugin_id, s.id))
        })
        .collect();
    let mut paths = primary.to_vec();
    let mut active = Vec::new();
    for (id, version) in &group.active_versions {
        if plugins.contains(id) {
            continue;
        }
        let path = storage::plugins::published_manifest(database.connection(), id, version)?;
        let manifest = load_manifest(&path)?;
        active.push((path, manifest));
    }
    // 除旧 upstreamBatch 的反向闭包外，也覆盖零匹配 selector 新增候选的情况。
    let mut affected_scopes = Vec::new();
    for b in &group.batches {
        for e in &b.entities {
            affected_scopes.push((e.entity.kind.clone(), e.entity.scope_id.clone()));
        }
    }
    let mut changes = Vec::new();
    for batch in &group.batches {
        if let Some(old) = storage::current_batch(database.connection(), &batch.replacement_key)? {
            changes.push(crate::index::dependencies::DependencyChange {
                kind: crate::index::dependencies::DependencyKindTag::UpstreamBatch,
                key: old.clone(),
                digest: String::new(),
            });
            let mut query = database.connection().prepare("SELECT e.kind,e.scope_id FROM entities e JOIN entity_assertions ea ON ea.entity_id = e.id WHERE ea.batch_id = ?1").map_err(database_error)?;
            let rows = query
                .query_map([old], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
                })
                .map_err(database_error)?;
            for row in rows {
                let (kind, scope) = row.map_err(database_error)?;
                if let Some(kind) = crate::contract::entity::EntityKind::parse(&kind) {
                    affected_scopes.push((kind, scope));
                }
            }
        }
    }
    let closure: BTreeSet<_> =
        crate::index::dependencies::dependency_closure(database.connection(), &changes)?
            .into_iter()
            .map(|(_, key)| key)
            .collect();
    loop {
        let before = selected.len();
        for (_, manifest) in &active {
            for stage in &manifest.stages {
                let key = format!("{}:{}", manifest.plugin_id, stage.id);
                if closure.contains(&storage::plugin_replacement_key(
                    &manifest.plugin_id,
                    &stage.id,
                    &project.config.project_id,
                )) || stage.depends_on.iter().any(|d| selected.contains(d))
                    || stage.entity_selectors.iter().any(|s| {
                        affected_scopes.iter().any(|(k, scope)| {
                            (s.kinds.is_empty() || s.kinds.contains(k))
                                && (s.scope_ids.is_empty() || s.scope_ids.contains(scope))
                        })
                    })
                {
                    selected.insert(key);
                }
            }
        }
        if selected.len() == before {
            break;
        }
    }
    for (path, manifest) in active {
        if manifest
            .stages
            .iter()
            .any(|s| selected.contains(&format!("{}:{}", manifest.plugin_id, s.id)))
        {
            paths.push(path);
        }
    }
    if paths.len() == primary.len() {
        return Ok(group);
    }
    stage_selected_collection(
        project,
        database,
        &paths,
        None,
        Some(&selected),
        cancel,
        now,
    )
    .await
}

/// 故障只由调用方显式传入；普通入口固定 None，不读取环境开关。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FaultPoint {
    None,
    BeforePublish,
    AfterPublish,
    DuringCommit(usize),
    /// 仅显式故障调用使用；写屏障后等待父测试杀死本次 owned 宿主。
    HoldDuringCommit {
        step: usize,
        marker: PathBuf,
    },
    AfterCommit,
}
fn fault() -> CoreError {
    CoreError::new(codes::INTERNAL_ERROR, "injected activation interruption")
}

pub async fn activate(
    project: &LoadedProject,
    database: &mut Database,
    id: &str,
    now: &str,
) -> Result<serde_json::Value, CoreError> {
    activate_with_fault(project, database, id, FaultPoint::None, now).await
}

pub async fn activate_with_fault(
    project: &LoadedProject,
    database: &mut Database,
    id: &str,
    point: FaultPoint,
    now: &str,
) -> Result<serde_json::Value, CoreError> {
    activate_controlled(project, database, id, point, &CancellationToken::new(), now).await
}

pub async fn activate_with_cancellation(
    project: &LoadedProject,
    database: &mut Database,
    id: &str,
    cancel: &CancellationToken,
    now: &str,
) -> Result<serde_json::Value, CoreError> {
    activate_controlled(project, database, id, FaultPoint::None, cancel, now).await
}

async fn activate_controlled(
    project: &LoadedProject,
    database: &mut Database,
    id: &str,
    point: FaultPoint,
    cancel: &CancellationToken,
    now: &str,
) -> Result<serde_json::Value, CoreError> {
    crate::client::check_cancel(cancel)?;
    let _lock = MaintenanceLock::acquire(project)?;
    _lock.recover(project, database, now)?;
    check_active_artifacts(project, database)?;
    let candidate = candidate(database.connection(), id)?;
    if candidate.state == "active" {
        return Ok(
            serde_json::json!({"candidateId":id,"status":"active","generation":database.generation_string()?,"alreadyCommitted":true}),
        );
    }
    if !matches!(candidate.state.as_str(), "validated" | "published") {
        return Err(CoreError::new(
            codes::INVALID_REQUEST,
            "candidate must pass validation first",
        ));
    }
    let binding: Binding = serde_json::from_str(
        candidate
            .staged_json
            .as_deref()
            .ok_or_else(|| CoreError::new(codes::INVALID_REQUEST, "validation binding missing"))?,
    )
    .map_err(|e| CoreError::new(codes::SCHEMA_INVALID, e.to_string()))?;
    let candidate_ids = if binding.candidate_ids.is_empty() {
        vec![id.to_string()]
    } else {
        binding.candidate_ids.clone()
    };
    for other_id in &candidate_ids {
        let other = storage::plugins::candidate(database.connection(), other_id)?;
        if other.validation_id != candidate.validation_id
            || !matches!(other.state.as_str(), "validated" | "published")
        {
            return Err(CoreError::new(
                codes::ACTIVATION_CONFLICT,
                "candidate group validation changed",
            ));
        }
    }
    let mut group = binding.into_group();
    if digest(&(&group.batches, &group.cases)) != group.result_hash {
        return Err(CoreError::new(
            codes::DEPENDENCY_CHANGED,
            "staged result hash mismatch",
        ));
    }
    if let Err(error) = group.recheck_with_cancellation(project, cancel).await {
        persist_input_change(database, &error)?;
        return Err(error);
    }
    group.check_database(database.connection())?;
    if point == FaultPoint::BeforePublish {
        return Err(fault());
    }
    for m in &mut group.manifests {
        let destination = project
            .base_dir
            .join(".project-knowledge/collectors")
            .join(&m.manifest.plugin_id)
            .join(&m.manifest.version)
            .join(&m.artifact_hash);
        if m.directory != destination {
            copy_artifact(m, &destination, &project.config.runtimes)?;
            m.directory = destination;
        }
    }
    for id in &candidate_ids {
        update_candidate(database.connection(), id, "published", None, None, None)?;
    }
    if point == FaultPoint::AfterPublish {
        return Err(fault());
    }
    crate::client::check_cancel(cancel)?;
    // 发布后、事务前再次验证，防止复制期间发生来源或运行时变化。
    if let Err(error) = group.recheck_with_cancellation(project, cancel).await {
        persist_input_change(database, &error)?;
        return Err(error);
    }
    let activation_id = format!(
        "activation_{}",
        candidate.validation_id.as_deref().expect("validated ID")
    );
    storage::record_activation(
        database.connection(),
        &activation_id,
        group.expected_generation,
        &serde_json::json!({"candidateId":id}),
        "pending",
        Some(&group.run_id),
        now,
    )?;
    let transaction = database.write_transaction()?;
    group.check_database(&transaction)?;
    let generation = group.expected_generation + 1;
    invalidate_downstream(&transaction, &group.batches, generation)?;
    for (index, batch) in group.batches.iter().enumerate() {
        crate::client::check_cancel(cancel)?;
        store_batch(&transaction, batch, generation, now)?;
        if point == FaultPoint::DuringCommit(index) {
            return Err(fault());
        }
        if let FaultPoint::HoldDuringCommit { step, marker } = &point
            && *step == index
        {
            let mut file = File::create(marker).map_err(io_error)?;
            std::io::Write::write_all(
                &mut file,
                format!("{}:{index}", std::process::id()).as_bytes(),
            )
            .map_err(io_error)?;
            file.sync_all().map_err(io_error)?;
            drop(file);
            loop {
                std::thread::park();
            }
        }
    }
    for (m, r) in group.manifests.iter().zip(&group.runtimes) {
        storage::register_plugin_version(
            &transaction,
            &m.manifest.plugin_id,
            &m.manifest.version,
            &m.artifact_hash,
            &serde_json::to_string(&m.manifest).expect("manifest JSON"),
            &digest(&(r.executable.to_string_lossy(), &r.version)),
            &m.directory.to_string_lossy(),
            candidate.validation_id.as_deref(),
            now,
        )?;
        storage::set_active_plugin(&transaction, &m.manifest.plugin_id, &m.manifest.version)?;
    }
    transaction
        .execute(
            "UPDATE index_meta SET generation = ?1 WHERE id = 1",
            [generation as i64],
        )
        .map_err(database_error)?;
    crate::knowledge::documents::invalidate_changed(&transaction)?;
    for id in &candidate_ids {
        let primary = storage::plugins::candidate(&transaction, id)?;
        transaction.execute("UPDATE plugin_candidates SET state = 'superseded' WHERE plugin_id = ?1 AND id <> ?2 AND state = 'active'",rusqlite::params![primary.plugin_id,id]).map_err(database_error)?;
        update_candidate(&transaction, id, "active", None, None, None)?;
    }
    storage::record_activation(
        &transaction,
        &activation_id,
        group.expected_generation,
        &serde_json::json!({"candidateId":id}),
        "committed",
        Some(&group.run_id),
        now,
    )?;
    crate::client::check_cancel(cancel)?;
    transaction.commit().map_err(database_error)?;
    if point == FaultPoint::AfterCommit {
        return Err(fault());
    }
    Ok(
        serde_json::json!({"candidateId":id,"activationId":activation_id,"status":"active","generation":generation.to_string()}),
    )
}

/// 回退重新提交并校验当前输入，绝不恢复旧批次。
pub async fn rollback(
    project: &LoadedProject,
    database: &mut Database,
    plugin: &str,
    version: &str,
    cancel: &CancellationToken,
    now: &str,
) -> Result<serde_json::Value, CoreError> {
    let path = storage::plugins::published_manifest(database.connection(), plugin, version)?;
    let next = submit(project, database, &path, now)?;
    validate(project, database, &next.id, cancel, now).await?;
    activate_with_cancellation(project, database, &next.id, cancel, now).await
}

/// 仅自动入口尊重 autoActivate；显式 activate 是用户主动维护操作。
pub async fn process_candidate(
    project: &LoadedProject,
    database: &mut Database,
    id: &str,
    cancel: &CancellationToken,
    now: &str,
) -> Result<serde_json::Value, CoreError> {
    process_candidates(project, database, &[id.to_string()], cancel, now).await
}

pub async fn process_candidates(
    project: &LoadedProject,
    database: &mut Database,
    ids: &[String],
    cancel: &CancellationToken,
    now: &str,
) -> Result<serde_json::Value, CoreError> {
    let report = validate_many(project, database, ids, cancel, now).await?;
    if project.config.plugins.auto_activate {
        activate(project, database, &ids[0], now).await
    } else {
        Ok(report)
    }
}

// 已确认的输入变化先持久失效。失败候选、内容改回和重启都不能复活旧断言。
fn persist_input_change(database: &mut Database, error: &CoreError) -> Result<(), CoreError> {
    if !matches!(
        error.code.as_str(),
        codes::SOURCE_CHANGED | codes::DEPENDENCY_CHANGED | codes::INDEX_STALE
    ) {
        return Ok(());
    }
    let keys = storage::current_batches(database.connection())?
        .into_iter()
        .filter(|(key, _)| !key.starts_with("(core.base,"))
        .map(|(key, _)| key)
        .collect::<Vec<_>>();
    let generation = database.generation()? + 1;
    let transaction = database.write_transaction()?;
    if storage::invalidate_batches(&transaction, &keys, generation)? > 0 {
        transaction
            .execute(
                "UPDATE index_meta SET generation=?1 WHERE id=1",
                [generation as i64],
            )
            .map_err(database_error)?;
        storage::knowledge::invalidate_batches(&transaction, &keys)?;
    }
    transaction.commit().map_err(database_error)?;
    Ok(())
}

pub fn check_active_artifacts(
    project: &LoadedProject,
    database: &mut Database,
) -> Result<(), CoreError> {
    let mut changed = Vec::new();
    for (plugin, version) in storage::active_version_map(database.connection())? {
        let path = storage::plugins::published_manifest(database.connection(), &plugin, &version)?;
        let expected: String = database
            .connection()
            .query_row(
                "SELECT artifact_hash FROM plugin_versions WHERE plugin_id = ?1 AND version = ?2",
                rusqlite::params![plugin, version],
                |r| r.get(0),
            )
            .map_err(database_error)?;
        let actual = load_manifest(&path).and_then(|m| {
            validate_manifest(
                &m,
                path.parent().expect("artifact directory"),
                &project.config.runtimes,
            )
        });
        if actual.map(|a| a.artifact_hash != expected).unwrap_or(true) {
            changed.push(plugin);
        }
    }
    if changed.is_empty() {
        return Ok(());
    }
    let generation = database.generation()? + 1;
    let transaction = database.write_transaction()?;
    let current = storage::current_batches(&transaction)?;
    let mut keys = current
        .clone()
        .into_iter()
        .filter(|(k, _)| changed.iter().any(|p| k.starts_with(&format!("({p},"))))
        .map(|(k, _)| k)
        .collect::<Vec<_>>();
    let changes = current
        .into_iter()
        .filter(|(key, _)| keys.contains(key))
        .map(|(_, id)| crate::index::dependencies::DependencyChange {
            kind: crate::index::dependencies::DependencyKindTag::UpstreamBatch,
            key: id,
            digest: String::new(),
        })
        .collect::<Vec<_>>();
    keys.extend(
        crate::index::dependencies::dependency_closure(&transaction, &changes)?
            .into_iter()
            .map(|(_, key)| key),
    );
    storage::invalidate_batches(&transaction, &keys, generation)?;
    storage::knowledge::invalidate_batches(&transaction, &keys)?;
    transaction
        .execute(
            "UPDATE index_meta SET generation = ?1 WHERE id = 1",
            [generation as i64],
        )
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)?;
    Err(CoreError::new(
        codes::ARTIFACT_CHANGED,
        "active artifact changed or is missing; current batches are stale",
    ))
}
