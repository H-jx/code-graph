//! CLI 只解析参数；阶段排序、校验和事务提交由 Core 拥有。
use crate::commands::{Exit, database_path, now_timestamp, print_json};
use clap::Args;
use project_knowledge_core::collect::manifest::{load_manifest, validate_manifest};
use project_knowledge_core::collect::staging::stage_collection;
use project_knowledge_core::contract::collector::StagePhase;
use project_knowledge_core::contract::error::{CoreError, codes};
use project_knowledge_core::project::config::LoadedProject;
use project_knowledge_core::storage::Database;
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Args)]
pub struct CollectArgs {
    #[arg(long)]
    pub config: PathBuf,
    /// all 在暂存库执行完整 extract/link 组；也可显式指定 extract 或 link。
    #[arg(long, default_value = "all")]
    pub phase: String,
    #[arg(long)]
    pub plugin: Option<String>,
    #[arg(long)]
    pub validate_only: bool,
    #[arg(long)]
    pub manifest: Option<PathBuf>,
}

pub fn run(args: CollectArgs) -> Result<Exit, CoreError> {
    let phase = match args.phase.as_str() {
        "all" => None,
        "extract" => Some(StagePhase::Extract),
        "link" => Some(StagePhase::Link),
        _ => {
            return Err(CoreError::new(
                codes::INVALID_REQUEST,
                "phase must be all, extract or link",
            ));
        }
    };
    let project = LoadedProject::load(&args.config)?;
    let _lock = if args.validate_only {
        None
    } else {
        Some(project_knowledge_core::collect::registry::MaintenanceLock::acquire(&project)?)
    };
    let path = database_path(&args.config);
    if !path.is_file() {
        return Err(CoreError::new(
            codes::INDEX_MISSING,
            "run index --base first",
        ));
    }
    if !args.validate_only {
        let mut maintenance = Database::open_maintenance(
            &path,
            &project.config.project_id,
            &project.config_hash()?,
            &now_timestamp(),
        )?;
        _lock.as_ref().expect("maintenance lock").recover(
            &project,
            &maintenance,
            &now_timestamp(),
        )?;
        project_knowledge_core::collect::registry::check_active_artifacts(
            &project,
            &mut maintenance,
        )?;
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|e| CoreError::new(codes::INTERNAL_ERROR, e.to_string()))?
            .block_on(
                project_knowledge_core::index::incremental::invalidate_for_collect(
                    &project,
                    &mut maintenance,
                    &now_timestamp(),
                ),
            )?;
    }
    let database = Database::open_readonly(&path)?;
    let paths = if args.validate_only || args.manifest.is_some() {
        discover_manifests(&project, args.manifest, args.plugin.as_deref())?
    } else {
        project_knowledge_core::storage::active_version_map(database.connection())?
            .into_iter()
            .filter(|(id, _)| args.plugin.as_ref().is_none_or(|wanted| wanted == id))
            .map(|(id, version)| {
                project_knowledge_core::storage::plugins::published_manifest(
                    database.connection(),
                    &id,
                    &version,
                )
            })
            .collect::<Result<Vec<_>, _>>()?
    };
    if !args.validate_only {
        let active = project_knowledge_core::storage::active_version_map(database.connection())?;
        for path in &paths {
            let manifest = load_manifest(path)?;
            if !active.contains(&(manifest.plugin_id.clone(), manifest.version.clone())) {
                return Err(CoreError::new(
                    codes::INVALID_REQUEST,
                    "collect requires an active plugin; submit, validate and activate the candidate first",
                ));
            }
            let published = project_knowledge_core::storage::plugins::published_manifest(
                database.connection(),
                &manifest.plugin_id,
                &manifest.version,
            )?;
            let expected = validate_manifest(
                &load_manifest(&published)?,
                published.parent().expect("published parent"),
                &project.config.runtimes,
            )?;
            let supplied = validate_manifest(
                &manifest,
                path.parent().expect("manifest parent"),
                &project.config.runtimes,
            )?;
            if expected.artifact_hash != supplied.artifact_hash {
                return Err(CoreError::new(
                    codes::ARTIFACT_CHANGED,
                    "collector differs from the active immutable artifact",
                ));
            }
        }
    }
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| CoreError::new(codes::INTERNAL_ERROR, e.to_string()))?;
    let now = now_timestamp();
    let staged = runtime.block_on(stage_collection(
        &project,
        &database,
        &paths,
        phase,
        &CancellationToken::new(),
        &now,
    ))?;
    let mut report = staged.report();
    report["validateOnly"] = serde_json::json!(args.validate_only);
    report["committed"] = serde_json::json!(!args.validate_only);
    if !args.validate_only {
        let mut database = Database::open_maintenance(
            &path,
            &project.config.project_id,
            &project.config_hash()?,
            &now,
        )?;
        let generation = runtime.block_on(staged.commit(&project, &mut database, &now))?;
        report["generation"] = serde_json::json!(generation.to_string());
        report["status"] = serde_json::json!("committed");
    }
    print_json(&report);
    Ok(Exit::ok())
}

fn discover_manifests(
    project: &LoadedProject,
    explicit: Option<PathBuf>,
    plugin: Option<&str>,
) -> Result<Vec<PathBuf>, CoreError> {
    let paths = if let Some(path) = explicit {
        vec![path]
    } else {
        project_knowledge_core::collect::registry::discovery_paths(project)?
    };
    let mut seen = std::collections::BTreeSet::new();
    let mut result = Vec::new();
    for path in paths {
        let manifest = load_manifest(&path)?;
        if plugin.is_some_and(|id| id != manifest.plugin_id) {
            continue;
        }
        let directory = path
            .parent()
            .ok_or_else(|| CoreError::new(codes::CONFIG_INVALID, "manifest has no parent"))?;
        let validated = validate_manifest(&manifest, directory, &project.config.runtimes)?;
        if seen.insert((
            manifest.plugin_id,
            manifest.version,
            validated.artifact_hash,
        )) {
            result.push(path);
        }
    }
    if result.is_empty() {
        return Err(CoreError::new(
            codes::INDEX_MISSING,
            "no collector selected",
        ));
    }
    Ok(result)
}
