//! extract → 再导出 → link → case；所有脚本在正式事务之外运行。
use super::commit::{StagedBatch, StagedEntity, StagedRelation, commit_batch};
use super::import::{database_error, digest, export_input};
use super::manifest::{ValidatedManifest, load_manifest, validate_manifest, validate_stage_dag};
use super::runner::{PluginRunner, ResolvedRuntime, RunRequest};
use super::validate::{ValidationContext, validate_output};
use crate::contract::collector::*;
use crate::contract::entity::Entity;
use crate::contract::error::{CoreError, Issue, codes};
use crate::contract::relation::Relation;
use crate::project::config::LoadedProject;
use crate::storage::{self, Database};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use tokio_util::sync::CancellationToken;

/// 只有 Core 在完整校验后构造的暂存组；不允许调用方填入任意“已验证”批次。
pub struct StagedCollection {
    pub(crate) expected_generation: u64,
    pub(crate) source_snapshot_id: String,
    pub(crate) active_versions: Vec<(String, String)>,
    pub(crate) config_hash: String,
    pub(crate) batches: Vec<StagedBatch>,
    pub(crate) manifests: Vec<ValidatedManifest>,
    pub(crate) runtimes: Vec<ResolvedRuntime>,
    pub(crate) prepared: Vec<Option<super::deps::PreparedDependencies>>,
    pub(crate) result_hash: String,
    pub(crate) cases: Vec<CaseResult>,
    pub(crate) run_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseResult {
    pub id: String,
    pub passed: bool,
    pub truth_hash: Option<String>,
    /// 候选自带的 case 不能冒充独立业务真值。
    pub origin: String,
}

impl StagedCollection {
    pub fn report(&self) -> serde_json::Value {
        serde_json::json!({"status":"validated", "runId":self.run_id,
            "expectedGeneration":self.expected_generation.to_string(), "stagedResultHash":self.result_hash,
            "cases":self.cases, "stages":self.batches.iter().map(|b| serde_json::json!({
                "replacementKey":b.replacement_key,"batchId":b.batch_id,"entities":b.entities.len(),
                "relations":b.relations.len(),"dependencies":b.dependencies.len()
            })).collect::<Vec<_>>()})
    }

    /// 显式 collect 的结果事务 B；活动版本切换由 registry 的事务 C 拥有。
    pub async fn commit(
        self,
        project: &LoadedProject,
        database: &mut Database,
        now: &str,
    ) -> Result<u64, CoreError> {
        self.commit_with_cancellation(project, database, &CancellationToken::new(), now)
            .await
    }

    pub async fn commit_with_cancellation(
        self,
        project: &LoadedProject,
        database: &mut Database,
        cancel: &CancellationToken,
        now: &str,
    ) -> Result<u64, CoreError> {
        self.recheck_with_cancellation(project, cancel).await?;
        crate::client::check_cancel(cancel)?;
        let transaction = database.write_transaction()?;
        self.check_database(&transaction)?;
        let generation = self.expected_generation + 1;
        invalidate_downstream(&transaction, &self.batches, generation)?;
        for batch in &self.batches {
            store_batch(&transaction, batch, generation, now)?;
        }
        transaction
            .execute(
                "UPDATE index_meta SET generation = ?1 WHERE id = 1",
                [generation as i64],
            )
            .map_err(database_error)?;
        crate::client::check_cancel(cancel)?;
        transaction.commit().map_err(database_error)?;
        Ok(generation)
    }

    pub(crate) fn check_database(
        &self,
        connection: &rusqlite::Connection,
    ) -> Result<(), CoreError> {
        let (generation, snapshot, config): (i64, String, String) = connection.query_row(
            "SELECT generation, current_source_snapshot_id, config_hash FROM index_meta WHERE id = 1", [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).map_err(database_error)?;
        if generation != self.expected_generation as i64
            || snapshot != self.source_snapshot_id
            || config != self.config_hash
            || storage::active_version_map(connection)? != self.active_versions
        {
            return Err(CoreError::new(
                codes::ACTIVATION_CONFLICT,
                "index or active plugins changed after staging",
            ));
        }
        Ok(())
    }

    pub(crate) async fn recheck_with_cancellation(
        &self,
        project: &LoadedProject,
        cancel: &CancellationToken,
    ) -> Result<(), CoreError> {
        verify_sources_with_cancellation(project, &self.source_snapshot_id, cancel)?;
        for (manifest, runtime) in self.manifests.iter().zip(&self.runtimes) {
            crate::client::check_cancel(cancel)?;
            let current = load_manifest(&manifest.directory.join("manifest.json"))?;
            let current =
                validate_manifest(&current, &manifest.directory, &project.config.runtimes)?;
            if current.artifact_hash != manifest.artifact_hash
                || current.manifest != manifest.manifest
            {
                return Err(CoreError::new(
                    codes::ARTIFACT_CHANGED,
                    "candidate changed after staging",
                ));
            }
            if resolve_runtime(&manifest.manifest, project).await? != *runtime {
                return Err(CoreError::new(
                    codes::DEPENDENCY_CHANGED,
                    "runtime changed after staging",
                ));
            }
        }
        for prepared in self.prepared.iter().flatten() {
            super::deps::verify_cache(prepared)?;
        }
        Ok(())
    }
}

/// phase=None 用于候选整组验证；Some 用于显式分阶段 collect。
pub async fn stage_collection(
    project: &LoadedProject,
    database: &Database,
    paths: &[PathBuf],
    phase: Option<StagePhase>,
    cancel: &CancellationToken,
    now: &str,
) -> Result<StagedCollection, CoreError> {
    stage_selected_collection(project, database, paths, phase, None, cancel, now).await
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn stage_selected_collection(
    project: &LoadedProject,
    database: &Database,
    paths: &[PathBuf],
    phase: Option<StagePhase>,
    selected: Option<&BTreeSet<String>>,
    cancel: &CancellationToken,
    now: &str,
) -> Result<StagedCollection, CoreError> {
    let work = super::workspace::StagingWorkspace::new(project)?;
    let staged_path = work.path().join("staging.sqlite");
    storage::backup_to(database.path(), &staged_path)?;
    let config_hash = project.config_hash()?;
    let mut staging =
        Database::open_maintenance(&staged_path, &project.config.project_id, &config_hash, now)?;
    if staging.project_id()? != project.config.project_id || staging.config_hash()? != config_hash {
        return Err(CoreError::new(
            codes::INDEX_STALE,
            "configuration differs from base index; run index --base",
        ));
    }
    let source_snapshot_id = staging
        .current_source_snapshot_id()?
        .ok_or_else(|| CoreError::new(codes::INDEX_MISSING, "run index --base first"))?;
    verify_sources_with_cancellation(project, &source_snapshot_id, cancel)?;
    let mut manifests = Vec::new();
    let mut ids = BTreeSet::new();
    for path in paths {
        let path = portable_path(
            std::fs::canonicalize(path)
                .map_err(|e| CoreError::new(codes::CONFIG_INVALID, e.to_string()))?,
        );
        let manifest = load_manifest(&path)?;
        let validated = validate_manifest(
            &manifest,
            path.parent().expect("manifest parent"),
            &project.config.runtimes,
        )?;
        if !ids.insert(manifest.plugin_id.clone()) {
            return Err(CoreError::new(
                codes::ARTIFACT_CHANGED,
                "select exactly one candidate per plugin ID",
            ));
        }
        // 收集期间绝不隐式安装或使用业务项目 node_modules。
        manifests.push(validated);
    }
    validate_stage_dag(
        &manifests
            .iter()
            .map(|m| m.manifest.clone())
            .collect::<Vec<_>>(),
    )?;
    let mut runtimes = Vec::new();
    for m in &manifests {
        runtimes.push(resolve_runtime(&m.manifest, project).await?);
    }
    let prepared = manifests
        .iter()
        .zip(&runtimes)
        .map(|(manifest, runtime)| {
            if manifest.manifest.dependencies.is_none() {
                return Ok(None);
            }
            let (plan, inputs) = super::deps::plugin_plan(manifest, runtime)?;
            super::deps::prepared_dependencies(&plan, &inputs).map(Some)
        })
        .collect::<Result<Vec<_>, CoreError>>()?;
    let mut group = StagedCollection {
        expected_generation: staging.generation()?,
        active_versions: storage::active_version_map(staging.connection())?,
        source_snapshot_id,
        config_hash,
        batches: Vec::new(),
        manifests,
        runtimes,
        prepared,
        result_hash: String::new(),
        cases: Vec::new(),
        run_id: format!("run_{}", uuid::Uuid::new_v4()),
    };
    let active_manifests = storage::plugins::active_manifests(staging.connection())?;
    let registry = crate::contract::extensions::ExtensionRegistry::from_manifests(
        group.manifests.iter().map(|m| &m.manifest).chain(
            active_manifests
                .iter()
                .filter(|m| !ids.contains(&m.plugin_id)),
        ),
    )?;
    let mut pending = BTreeMap::new();
    for (index, m) in group.manifests.iter().enumerate() {
        for stage in &m.manifest.stages {
            if phase.is_none_or(|p| stage.phase == p)
                && selected.is_none_or(|keys| {
                    keys.contains(&format!("{}:{}", m.manifest.plugin_id, stage.id))
                })
            {
                pending.insert(
                    format!("{}:{}", m.manifest.plugin_id, stage.id),
                    (index, stage.clone()),
                );
            }
        }
    }
    if pending.is_empty() {
        return Err(CoreError::new(
            codes::INVALID_REQUEST,
            "no stages match requested phase",
        ));
    }
    // 移除目标组及其显式下游旧指针，避免分阶段重跑读到自身派生的旧候选。
    let mut masked: BTreeSet<_> = pending.keys().cloned().collect();
    loop {
        let before = masked.len();
        for manifest in &group.manifests {
            for stage in &manifest.manifest.stages {
                if stage.depends_on.iter().any(|d| masked.contains(d)) {
                    masked.insert(format!("{}:{}", manifest.manifest.plugin_id, stage.id));
                }
            }
        }
        if masked.len() == before {
            break;
        }
    }
    let transaction = staging.write_transaction()?;
    for stage in masked {
        let (plugin, stage) = stage.split_once(':').expect("validated stage key");
        let key = storage::plugin_replacement_key(plugin, stage, &project.config.project_id);
        transaction
            .execute(
                "DELETE FROM current_batches WHERE replacement_key = ?1",
                [key],
            )
            .map_err(database_error)?;
    }
    transaction.commit().map_err(database_error)?;
    let runner = PluginRunner::default();
    while !pending.is_empty() {
        let key = pending
            .iter()
            .filter(|(_, (_, s))| s.depends_on.iter().all(|d| !pending.contains_key(d)))
            .min_by_key(|(key, (_, s))| (if s.phase == StagePhase::Extract { 0 } else { 1 }, *key))
            .map(|(k, _)| k.clone())
            .ok_or_else(|| CoreError::new(codes::CONFIG_INVALID, "stage dependency cycle"))?;
        let (index, stage) = pending.remove(&key).expect("selected pending stage");
        let manifest = &group.manifests[index];
        let upstream = upstream_dependencies(staging.connection(), project, &stage)?;
        let mut input = export_input(
            staging.connection(),
            project,
            &stage,
            &group.source_snapshot_id,
            &format!("{}-{}", group.run_id, group.batches.len()),
            &manifest.manifest.plugin_id,
            &manifest.manifest.version,
        )?;
        input.limits = manifest.manifest.limits.clone().unwrap_or_default();
        input.allowed_extensions = registry.names();
        let output = run_stage(
            &runner,
            manifest,
            &group.runtimes[index],
            &stage,
            input.clone(),
            cancel,
            &registry,
            group.prepared[index].as_ref(),
        )
        .await?;
        let mut batch = to_batch(
            project,
            manifest,
            &group.runtimes[index],
            &stage,
            &input,
            &output,
            upstream,
        )?;
        if let Some(prepared) = &group.prepared[index] {
            batch.dependencies.push(Dependency {
                kind: DependencyKind::Runtime,
                key: format!("{}:dependencies", manifest.manifest.plugin_id),
                digest: digest(&(&prepared.digest, &prepared.integrity)),
            });
        }
        let transaction = staging.write_transaction()?;
        store_batch(&transaction, &batch, group.expected_generation + 1, now)?;
        transaction.commit().map_err(database_error)?;
        group.batches.push(batch);
    }
    for ((m, runtime), prepared) in group
        .manifests
        .iter()
        .zip(&group.runtimes)
        .zip(&group.prepared)
    {
        for case_path in &m.manifest.acceptance_cases {
            let case: AcceptanceCase = read_json(&m.directory.join(case_path))?;
            let mut input: CollectorInput = read_json(&m.directory.join(&case.input_fixture))?;
            let stage = m
                .manifest
                .stages
                .iter()
                .find(|s| s.id == input.stage_id)
                .ok_or_else(|| {
                    CoreError::new(
                        codes::SCHEMA_INVALID,
                        "case fixture references unknown stage",
                    )
                })?;
            if selected.is_some_and(|keys| {
                !keys.contains(&format!("{}:{}", m.manifest.plugin_id, stage.id))
            }) {
                continue;
            }
            input.plugin_id = m.manifest.plugin_id.clone();
            input.version = m.manifest.version.clone();
            // Cases use the same host-owned configuration as live extraction.
            // Dropping scopes/services/bindings makes valid case identities and
            // client bindings differ from their live run. Never trust fixture configuration.
            input.config_subset =
                serde_json::to_value(&project.config).expect("serializable config");
            input.run_id = format!("{}-case-{}", group.run_id, case.id);
            input.allowed_extensions = registry.names();
            let output = run_stage(
                &runner,
                m,
                runtime,
                stage,
                input,
                cancel,
                &registry,
                prepared.as_ref(),
            )
            .await?;
            check_case(&case, &output)?;
            group.cases.push(CaseResult {
                id: case.id,
                passed: true,
                truth_hash: case.truth_hash,
                origin: "candidate".to_string(),
            });
        }
    }
    group.result_hash = digest(&(&group.batches, &group.cases));
    group.recheck_with_cancellation(project, cancel).await?;
    Ok(group)
}

pub(crate) fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, CoreError> {
    let bytes =
        std::fs::read(path).map_err(|e| CoreError::new(codes::CONFIG_INVALID, e.to_string()))?;
    serde_json::from_slice(&bytes)
        .map_err(|e| CoreError::new(codes::SCHEMA_INVALID, format!("{}: {e}", path.display())))
}

#[allow(clippy::too_many_arguments)]
async fn run_stage(
    runner: &PluginRunner,
    manifest: &ValidatedManifest,
    runtime: &ResolvedRuntime,
    stage: &StageSpec,
    input: CollectorInput,
    cancel: &CancellationToken,
    registry: &crate::contract::extensions::ExtensionRegistry,
    prepared: Option<&super::deps::PreparedDependencies>,
) -> Result<CollectorOutput, CoreError> {
    let project_id = input
        .config_subset
        .get("projectId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CoreError::new(codes::CONFIG_INVALID, "stage input is missing projectId"))?;
    for (index, source) in input.sources.iter().enumerate() {
        if source.source.content_hash
            != crate::contract::identity::content_hash(source.content.as_bytes())
            || source.source.byte_length != source.content.len() as u64
            || source.source.id
                != crate::contract::identity::compute_source_id(
                    project_id,
                    &source.source.repository_id,
                    &source.source.path,
                )
        {
            return Err(CoreError::new(
                codes::EVIDENCE_INVALID,
                "input fixture source content or identity is invalid",
            )
            .with_issues(vec![
                Issue::new(codes::EVIDENCE_INVALID, &format!("/input/sources/{index}"))
                    .with_hint("按原始字节重算来源哈希和身份"),
            ]));
        }
    }
    let workspace = tempfile::Builder::new()
        .prefix("pk-plugin-")
        .tempdir()
        .map_err(|e| CoreError::new(codes::INTERNAL_ERROR, e.to_string()))?;
    let (executable, entry, env) = if let Some(prepared) = prepared {
        super::deps::verify_cache(prepared)?;
        (
            prepared.executable.clone(),
            prepared.directory.join("artifact").join(&stage.entry),
            prepared.env.clone(),
        )
    } else {
        super::deps::copy_artifact(manifest, workspace.path())?;
        (
            runtime.executable.clone(),
            workspace.path().join(&stage.entry),
            Default::default(),
        )
    };
    let outcome = runner
        .run(
            &RunRequest {
                runtime: ResolvedRuntime {
                    executable,
                    version: runtime.version.clone(),
                },
                entry,
                args: stage.args.clone(),
                directory: workspace.path().to_path_buf(),
                limits: manifest.manifest.limits.clone().unwrap_or_default(),
                input: input.clone(),
                extra_env: env,
            },
            cancel,
        )
        .await?;
    if let Some(prepared) = prepared {
        super::deps::verify_cache(prepared)?;
    }
    validate_output(
        &ValidationContext {
            project_id: input
                .config_subset
                .get("projectId")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
            input: &input,
            manifest: &manifest.manifest,
            stage,
            registry: Some(registry),
        },
        &outcome.output,
    )?;
    Ok(outcome.output)
}

fn check_case(case: &AcceptanceCase, output: &CollectorOutput) -> Result<(), CoreError> {
    let mut issues = Vec::new();
    for (field, expected, actual) in [
        (
            "entities",
            &case.expected.entities,
            output
                .entities
                .iter()
                .map(|e| &e.entity_id)
                .collect::<Vec<_>>(),
        ),
        (
            "relations",
            &case.expected.relations,
            output.relations.iter().map(|r| &r.relation_id).collect(),
        ),
        (
            "gapCodes",
            &case.expected.gap_codes,
            output.gaps.iter().map(|g| &g.code).collect(),
        ),
    ] {
        for (i, id) in expected.iter().enumerate() {
            if !actual.contains(&id) {
                issues.push(
                    Issue::new(codes::SCHEMA_INVALID, &format!("/expected/{field}/{i}"))
                        .with_actual(id)
                        .with_hint("修复采集输出以满足验收用例"),
                );
            }
        }
    }
    for (i, id) in case.expected.forbidden_relations.iter().enumerate() {
        if output.relations.iter().any(|r| &r.relation_id == id) {
            issues.push(
                Issue::new(
                    codes::SCHEMA_INVALID,
                    &format!("/expected/forbiddenRelations/{i}"),
                )
                .with_actual(id)
                .with_hint("移除无依据或错误绑定的关系"),
            );
        }
    }
    if issues.is_empty() {
        Ok(())
    } else {
        Err(CoreError::new(
            codes::SCHEMA_INVALID,
            format!("acceptance case `{}` failed", case.id),
        )
        .with_issues(issues))
    }
}

fn upstream_dependencies(
    connection: &rusqlite::Connection,
    project: &LoadedProject,
    stage: &StageSpec,
) -> Result<Vec<Dependency>, CoreError> {
    let mut result = Vec::new();
    for dependency in &stage.depends_on {
        let (plugin, stage) = dependency
            .split_once(':')
            .ok_or_else(|| CoreError::new(codes::CONFIG_INVALID, "invalid stage dependency"))?;
        let key = storage::plugin_replacement_key(plugin, stage, &project.config.project_id);
        let id = storage::current_batch(connection, &key)?.ok_or_else(|| {
            CoreError::new(
                codes::REFERENCE_MISSING,
                format!("upstream `{dependency}` is missing"),
            )
        })?;
        if storage::batch_state(connection, &id)? != Some(storage::BatchState::Fresh) {
            return Err(CoreError::new(
                codes::INDEX_STALE,
                format!("upstream `{dependency}` is stale"),
            ));
        }
        result.push(Dependency {
            kind: DependencyKind::UpstreamBatch,
            key: id.clone(),
            digest: id,
        });
    }
    Ok(result)
}

fn to_batch(
    project: &LoadedProject,
    manifest: &ValidatedManifest,
    runtime: &ResolvedRuntime,
    stage: &StageSpec,
    input: &CollectorInput,
    output: &CollectorOutput,
    mut dependencies: Vec<Dependency>,
) -> Result<StagedBatch, CoreError> {
    let plugin = &manifest.manifest.plugin_id;
    let replacement_key =
        storage::plugin_replacement_key(plugin, &stage.id, &project.config.project_id);
    let batch_id = storage::batch_id(&input.run_id, &replacement_key);
    let producer_key = format!(
        "{}@{}:{}:{}",
        plugin, manifest.manifest.version, manifest.artifact_hash, stage.id
    );
    for (kind, key, value) in [
        (
            DependencyKind::Config,
            project.config.project_id.clone(),
            project.config_hash()?,
        ),
        (
            DependencyKind::Artifact,
            plugin.clone(),
            manifest.artifact_hash.clone(),
        ),
        (
            DependencyKind::Runtime,
            plugin.clone(),
            digest(&(runtime.executable.to_string_lossy(), &runtime.version)),
        ),
    ] {
        dependencies.push(Dependency {
            kind,
            key,
            digest: value,
        });
    }
    for set in &input.source_sets {
        dependencies.push(Dependency {
            kind: DependencyKind::SourceSet,
            key: format!("{plugin}:{}:{}", stage.id, set.selector_id),
            digest: set.digest.clone(),
        });
    }
    for set in &input.candidate_sets {
        dependencies.push(Dependency {
            kind: DependencyKind::CandidateSet,
            key: format!("{plugin}:{}:{}", stage.id, set.selector_id),
            digest: set.digest.clone(),
        });
    }
    for source in &input.sources {
        dependencies.push(Dependency {
            kind: DependencyKind::Source,
            key: source.source.id.clone(),
            digest: source.source.content_hash.clone(),
        });
    }
    for assertion in input
        .entities
        .iter()
        .flat_map(|e| &e.assertions)
        .chain(input.relations.iter().flat_map(|r| &r.assertions))
    {
        dependencies.push(Dependency {
            kind: DependencyKind::UpstreamBatch,
            key: assertion.batch_id.clone(),
            digest: assertion.batch_id.clone(),
        });
    }
    let mut automatic: BTreeMap<(String, String), Dependency> = dependencies
        .into_iter()
        .map(|d| ((d.kind.as_str().to_string(), d.key.clone()), d))
        .collect();
    for (index, d) in output.dependencies.iter().enumerate() {
        let key = (d.kind.as_str().to_string(), d.key.clone());
        if automatic
            .get(&key)
            .is_none_or(|expected| expected.digest != d.digest)
        {
            return Err(CoreError::new(
                codes::DEPENDENCY_CHANGED,
                "plugin dependency is not bound to host input",
            )
            .with_issues(vec![
                Issue::new(codes::DEPENDENCY_CHANGED, &format!("/dependencies/{index}"))
                    .with_hint("仅声明宿主已导出的来源与集合摘要"),
            ]));
        }
        automatic.insert(key, d.clone());
    }
    let entities = output
        .entities
        .iter()
        .map(|e| StagedEntity {
            entity: Entity {
                id: e.entity_id.clone(),
                kind: e.kind.clone(),
                scope_id: e.scope_id.clone(),
                identity: e.identity.clone(),
                attributes: e.assertion.attributes.clone(),
                basis: e.assertion.basis,
                origin: e.assertion.origin,
                evidence_refs: e.assertion.evidence_refs.clone(),
                extensions: Default::default(),
            },
            name: e.assertion.name.clone(),
            evidence_refs: e.assertion.evidence_refs.clone(),
            basis: e.assertion.basis,
            origin: e.assertion.origin,
        })
        .collect::<Vec<_>>();
    let documents = entities
        .iter()
        .map(|e| {
            let payload = storage::entity_payload_hash(
                &producer_key,
                &batch_id,
                &e.entity.id,
                e.name.as_deref(),
                &e.entity.attributes,
                e.basis,
                e.origin,
            )?;
            Ok(crate::search::documents::entity_projection(
                &storage::assertion_id("entity", &batch_id, &payload),
                &e.entity.id,
                &e.entity.scope_id,
                e.name.as_deref().unwrap_or(""),
                e.entity.kind.as_str(),
                "",
                "",
                &serde_json::json!(e.entity.attributes),
                if e.entity.kind == crate::contract::EntityKind::Route {
                    e.entity
                        .identity
                        .iter()
                        .find(|slot| slot.name == "declaredPath")
                        .and_then(|slot| slot.value.as_ref())
                        .and_then(|v| v.as_text())
                        .map(str::to_string)
                } else {
                    None
                },
            ))
        })
        .collect::<Result<Vec<_>, CoreError>>()?;
    let relations = output
        .relations
        .iter()
        .map(|r| StagedRelation {
            relation: Relation {
                id: r.relation_id.clone(),
                kind: r.kind.clone(),
                from_id: r.from_id.clone(),
                to_id: r.to_id.clone(),
                qualifiers: r.qualifiers.clone(),
                basis: r.assertion.basis,
                origin: r.assertion.origin,
                resolution: r.resolution,
                evidence_refs: r.assertion.evidence_refs.clone(),
                extensions: Default::default(),
            },
            evidence_refs: r.assertion.evidence_refs.clone(),
            basis: r.assertion.basis,
            origin: r.assertion.origin,
            resolution: r.resolution,
        })
        .collect();
    Ok(StagedBatch {
        project_id: project.config.project_id.clone(),
        batch_id,
        run_id: input.run_id.clone(),
        producer_key,
        replacement_key,
        artifact_hash: manifest.artifact_hash.clone(),
        input_digest: digest(input),
        entities,
        relations,
        evidence: output.evidence.clone(),
        dependencies: automatic.into_values().collect(),
        coverage: output.coverage.clone(),
        gaps: output.gaps.clone(),
        documents,
    })
}

pub(crate) fn store_batch(
    connection: &rusqlite::Connection,
    batch: &StagedBatch,
    generation: u64,
    now: &str,
) -> Result<(), CoreError> {
    storage::create_run(
        connection,
        &batch.run_id,
        "collect",
        &batch.replacement_key,
        Some(&batch.input_digest),
        now,
    )?;
    commit_batch(connection, batch, generation, now)?;
    storage::finish_run(
        connection,
        &batch.run_id,
        storage::RunStatus::Succeeded,
        None,
        now,
    )
}

pub(crate) fn invalidate_downstream(
    connection: &rusqlite::Connection,
    batches: &[StagedBatch],
    generation: u64,
) -> Result<(), CoreError> {
    let mut changes = Vec::new();
    for batch in batches {
        if let Some(previous) = storage::current_batch(connection, &batch.replacement_key)? {
            changes.push(crate::index::dependencies::DependencyChange {
                kind: crate::index::dependencies::DependencyKindTag::UpstreamBatch,
                key: previous,
                digest: String::new(),
            });
        }
        for d in batch
            .dependencies
            .iter()
            .filter(|d| matches!(d.kind, DependencyKind::CandidateSet))
        {
            changes.push(crate::index::dependencies::DependencyChange {
                kind: crate::index::dependencies::DependencyKindTag::CandidateSet,
                key: d.key.clone(),
                digest: d.digest.clone(),
            });
        }
    }
    let affected = crate::index::dependencies::dependency_closure(connection, &changes)?;
    storage::invalidate_batches(
        connection,
        &affected.into_iter().map(|(_, key)| key).collect::<Vec<_>>(),
        generation,
    )?;
    Ok(())
}

pub(crate) fn verify_sources(project: &LoadedProject, snapshot_id: &str) -> Result<(), CoreError> {
    verify_sources_with_cancellation(project, snapshot_id, &CancellationToken::new())
}

pub(crate) fn verify_sources_with_cancellation(
    project: &LoadedProject,
    snapshot_id: &str,
    cancel: &CancellationToken,
) -> Result<(), CoreError> {
    let current = LoadedProject::load(&project.config_path)?;
    if current.config_hash()? != project.config_hash()? {
        return Err(CoreError::new(
            codes::SOURCE_CHANGED,
            "project configuration changed",
        ));
    }
    let options = crate::project::scan::ScanOptions {
        respect_git_ignore: current.config.selection.respect_git_ignore,
        max_file_bytes: current.config.selection.max_file_bytes,
        include: current.config.selection.include.clone(),
        exclude: current.config.selection.exclude.clone(),
    };
    let scans = current
        .repositories
        .iter()
        .map(|(id, root)| {
            crate::project::scan::scan_repository_bounded(id, root, &options, &mut || {
                !cancel.is_cancelled()
            })
            .map_err(|e| CoreError::new(codes::SOURCE_CHANGED, e))
            .and_then(|result| {
                result.ok_or_else(|| {
                    CoreError::new(codes::CANCELLED, "source verification cancelled")
                })
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let snapshot = crate::project::snapshot::build_snapshot(
        &current.config.project_id,
        &current.config_hash()?,
        &scans,
        "verify",
    );
    if snapshot.id != snapshot_id {
        return Err(CoreError::new(
            codes::SOURCE_CHANGED,
            "source selection changed; run index --base",
        ));
    }
    Ok(())
}

pub async fn resolve_runtime(
    manifest: &PluginManifest,
    project: &LoadedProject,
) -> Result<ResolvedRuntime, CoreError> {
    let kind = match manifest.runtime.kind {
        RuntimeKind::Node => "node",
        RuntimeKind::Python => "python",
        RuntimeKind::Executable => "executable",
    };
    let configured = project
        .config
        .runtimes
        .iter()
        .find(|r| match manifest.runtime.kind {
            RuntimeKind::Executable => Some(&r.id) == manifest.runtime.executable_id.as_ref(),
            _ => r.kind == kind,
        });
    let executable = if let Some(config) = configured {
        let path = PathBuf::from(&config.executable);
        if !path.is_absolute() {
            return Err(CoreError::new(
                codes::CONFIG_INVALID,
                "runtime executable must be absolute",
            ));
        }
        path
    } else if manifest.runtime.kind == RuntimeKind::Executable {
        return Err(CoreError::new(
            codes::RUNTIME_MISSING,
            "executable runtime is not configured",
        ));
    } else {
        let paths = std::env::var_os("PATH").unwrap_or_default();
        std::env::split_paths(&paths)
            .map(|p| {
                p.join(if cfg!(windows) {
                    format!("{kind}.exe")
                } else {
                    kind.to_string()
                })
            })
            .find(|p| p.is_file())
            .ok_or_else(|| {
                CoreError::new(codes::RUNTIME_MISSING, format!("{kind} runtime missing"))
            })?
    };
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        tokio::process::Command::new(&executable)
            .arg("--version")
            .kill_on_drop(true)
            .output(),
    )
    .await
    .map_err(|_| CoreError::new(codes::RUNTIME_MISSING, "runtime version probe timed out"))?
    .map_err(|e| CoreError::new(codes::RUNTIME_MISSING, e.to_string()))?;
    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !output.status.success() || version.is_empty() {
        return Err(CoreError::new(
            codes::RUNTIME_MISSING,
            "runtime version probe failed",
        ));
    }
    let number = version.trim_start_matches(|c: char| !c.is_ascii_digit());
    let actual = version_tuple(number)?;
    if (kind == "node" && actual.0 != 24) || (kind == "python" && (actual.0, actual.1) != (3, 12)) {
        return Err(CoreError::new(
            codes::RUNTIME_MISSING,
            format!("unsupported {kind} runtime {version}"),
        ));
    }
    if let Some(range) = &manifest.runtime.version_range {
        for constraint in range.split_whitespace() {
            let value = constraint.trim_start_matches(['>', '<', '=']);
            let target = version_tuple(value)?;
            let valid = if constraint.starts_with(">=") {
                actual >= target
            } else if constraint.starts_with("<=") {
                actual <= target
            } else if constraint.starts_with('>') {
                actual > target
            } else if constraint.starts_with('<') {
                actual < target
            } else {
                actual == target
            };
            if !valid {
                return Err(CoreError::new(
                    codes::RUNTIME_MISSING,
                    format!("{version} does not satisfy {range}"),
                ));
            }
        }
    }
    Ok(ResolvedRuntime {
        executable: portable_path(
            std::fs::canonicalize(&executable)
                .map_err(|e| CoreError::new(codes::RUNTIME_MISSING, e.to_string()))?,
        ),
        version,
    })
}

// Node 的主脚本解析不接受 Windows canonicalize 产生的 verbatim 路径。
pub(crate) fn portable_path(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        let text = path.to_string_lossy();
        if let Some(tail) = text.strip_prefix(r"\\?\UNC\") {
            return PathBuf::from(format!(r"\\{tail}"));
        }
        if let Some(tail) = text.strip_prefix(r"\\?\") {
            return PathBuf::from(tail);
        }
    }
    path
}

fn version_tuple(value: &str) -> Result<(u64, u64, u64), CoreError> {
    let values = value
        .split('.')
        .map(str::parse::<u64>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| {
            CoreError::new(
                codes::CONFIG_INVALID,
                "runtime versionRange requires numeric comparison clauses",
            )
        })?;
    if values.is_empty() || values.len() > 3 {
        return Err(CoreError::new(
            codes::CONFIG_INVALID,
            "invalid runtime version",
        ));
    }
    Ok((
        values[0],
        *values.get(1).unwrap_or(&0),
        *values.get(2).unwrap_or(&0),
    ))
}
