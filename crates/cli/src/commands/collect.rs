//! `collect`：运行已激活的采集阶段。
//!
//! T07/T08：宿主校验 manifest、准备依赖、运行子进程并校验输出。
//! 正式写入事实需要校验与导入流程（T08/T09），本命令当前只做“运行 + 校验”。

use std::path::{Path, PathBuf};

use clap::Args;
use project_knowledge_core::collect::import::export_input;
use project_knowledge_core::collect::manifest::{load_manifest, validate_manifest};
use project_knowledge_core::collect::runner::{PluginRunner, ResolvedRuntime, RunRequest};
use project_knowledge_core::collect::validate::validate_output;
use project_knowledge_core::contract::collector::{PluginManifest, StagePhase};
use project_knowledge_core::contract::error::{CoreError, codes};
use project_knowledge_core::project::config::LoadedProject;
use project_knowledge_core::storage::database::Database;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use crate::commands::{Exit, database_path, now_timestamp, print_json};

/// `collect` 参数。
#[derive(Debug, Args)]
pub struct CollectArgs {
    /// 项目配置文件。
    #[arg(long)]
    pub config: PathBuf,
    /// 采集阶段：`extract` 或 `link`。
    #[arg(long)]
    pub phase: String,
    /// 只运行指定插件。
    #[arg(long)]
    pub plugin: Option<String>,
    /// 只校验，不提交事实。
    #[arg(long)]
    pub validate_only: bool,
    /// 直接指定候选 manifest 路径（用于尚未登记的候选）。
    #[arg(long)]
    pub manifest: Option<PathBuf>,
}

/// 执行 `collect`。
pub fn run(args: CollectArgs) -> Result<Exit, CoreError> {
    let phase = match args.phase.as_str() {
        "extract" => StagePhase::Extract,
        "link" => StagePhase::Link,
        other => {
            return Err(CoreError::new(
                codes::INVALID_REQUEST,
                format!("unknown phase `{other}` (expected extract or link)"),
            ));
        }
    };

    let project = LoadedProject::load(&args.config)?;
    let now = now_timestamp();
    let database_path = database_path(&args.config);
    let database = Database::open_readonly(&database_path)?;
    let source_snapshot_id = database.current_source_snapshot_id()?.ok_or_else(|| {
        CoreError::new(
            codes::INDEX_MISSING,
            "index has no source snapshot; run `index --base` first",
        )
    })?;

    let manifests = discover_manifests(&project, args.manifest.as_deref())?;
    // 同一插件可能同时出现在显式登记与插件目录中，按 (ID, 版本, 制品哈希) 去重。
    let mut seen: Vec<(String, String, String)> = Vec::new();
    let mut selected: Vec<(PathBuf, PluginManifest)> = Vec::new();
    for (path, manifest) in manifests {
        let directory = path.parent().map(Path::to_path_buf).unwrap_or_default();
        let artifact_hash = validate_manifest(&manifest, &directory, &project.config.runtimes)
            .map(|validated| validated.artifact_hash)
            .unwrap_or_default();
        let key = (
            manifest.plugin_id.clone(),
            manifest.version.clone(),
            artifact_hash,
        );
        if seen.contains(&key) {
            continue;
        }
        seen.push(key);
        if args
            .plugin
            .as_ref()
            .map(|plugin| manifest.plugin_id == *plugin)
            .unwrap_or(true)
        {
            selected.push((path, manifest));
        }
    }
    if selected.is_empty() {
        return Err(CoreError::new(
            codes::INDEX_MISSING,
            "no collector manifest found; register one under `collectors` or `plugins.directories`",
        ));
    }

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot start async runtime: {error}"),
            )
        })?;

    let mut results: Vec<Value> = Vec::new();
    let mut failures = 0usize;
    for (manifest_path, manifest) in selected {
        let directory = manifest_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let validated = validate_manifest(&manifest, &directory, &project.config.runtimes)?;
        for stage in manifest.stages.iter().filter(|stage| stage.phase == phase) {
            let run_id = format!("run_{}", uuid::Uuid::new_v4());
            let input = export_input(
                database.connection(),
                &project,
                stage,
                &source_snapshot_id,
                &run_id,
                &manifest.plugin_id,
                &manifest.version,
            )?;
            let executable = resolve_runtime(&manifest, &project)?;
            let entry = directory.join(&stage.entry);
            let request = RunRequest {
                runtime: ResolvedRuntime {
                    executable: executable.clone(),
                    version: "runtime".to_string(),
                },
                entry,
                args: stage.args.clone(),
                directory: directory.clone(),
                input,
                limits: manifest.limits.clone().unwrap_or_default(),
                extra_env: Default::default(),
            };
            let runner = PluginRunner::default();
            match runtime.block_on(runner.run(&request, &CancellationToken::new())) {
                Ok(outcome) => {
                    let report = validate_output(
                        &project_knowledge_core::collect::validate::ValidationContext {
                            project_id: &project.config.project_id,
                            input: &request.input,
                            manifest: &manifest,
                            stage,
                        },
                        &outcome.output,
                    )
                    .map(|issues| json!({ "issues": issues }))
                    .unwrap_or_else(|error| json!({ "issues": error.issues }));
                    let mut entry_value = json!({
                        "pluginId": manifest.plugin_id,
                        "version": manifest.version,
                        "stageId": stage.id,
                        "artifactHash": validated.artifact_hash,
                        "stdoutBytes": outcome.stdout_bytes,
                        "stderrBytes": outcome.stderr_bytes,
                        "durationMs": outcome.duration_ms,
                        "entities": outcome.output.entities.len(),
                        "relations": outcome.output.relations.len(),
                        "evidence": outcome.output.evidence.len(),
                        "gaps": outcome.output.gaps.len(),
                        "validation": report,
                        "committed": false,
                    });
                    entry_value["status"] = json!("validated");
                    results.push(entry_value);
                }
                Err(error) => {
                    failures += 1;
                    results.push(json!({
                        "pluginId": manifest.plugin_id,
                        "version": manifest.version,
                        "stageId": stage.id,
                        "status": "failed",
                        "error": {
                            "code": error.code,
                            "message": error.message,
                            "issues": error.issues,
                        },
                    }));
                }
            }
        }
    }

    let payload = json!({
        "phase": args.phase,
        "validateOnly": args.validate_only,
        "committed": false,
        "results": results,
        "note": "正式提交事实需要校验与导入流程（T08/T09）；本命令只运行并校验阶段输出。",
        "completedAt": now,
    });
    print_json(&payload);
    Ok(if failures == 0 {
        Exit::ok()
    } else {
        crate::commands::Exit { code: 1 }
    })
}

/// 发现候选 manifest：显式登记的 `collectors` 路径与插件目录。
fn discover_manifests(
    project: &LoadedProject,
    explicit: Option<&Path>,
) -> Result<Vec<(PathBuf, PluginManifest)>, CoreError> {
    let mut paths: Vec<PathBuf> = Vec::new();
    if let Some(path) = explicit {
        paths.push(path.to_path_buf());
    } else {
        paths.extend(project.collector_manifest_paths());
        for directory in project.plugin_directories() {
            let Ok(entries) = std::fs::read_dir(&directory) else {
                continue;
            };
            for entry in entries.flatten() {
                let plugin_dir = entry.path();
                if !plugin_dir.is_dir() {
                    continue;
                }
                let direct = plugin_dir.join("manifest.json");
                if direct.is_file() {
                    paths.push(direct);
                    continue;
                }
                let Ok(versions) = std::fs::read_dir(&plugin_dir) else {
                    continue;
                };
                for version in versions.flatten() {
                    let candidate = version.path().join("manifest.json");
                    if candidate.is_file() {
                        paths.push(candidate);
                    }
                }
            }
        }
    }

    let mut result: Vec<(PathBuf, PluginManifest)> = Vec::new();
    for path in paths {
        let manifest = load_manifest(&path)?;
        result.push((path, manifest));
    }
    Ok(result)
}

/// 解析运行时可执行文件。
fn resolve_runtime(
    manifest: &PluginManifest,
    project: &LoadedProject,
) -> Result<PathBuf, CoreError> {
    use project_knowledge_core::contract::collector::RuntimeKind;
    match manifest.runtime.kind {
        RuntimeKind::Node => Ok(PathBuf::from("node")),
        RuntimeKind::Python => Ok(PathBuf::from("python")),
        RuntimeKind::Executable => {
            let id = manifest.runtime.executable_id.as_deref().ok_or_else(|| {
                CoreError::new(
                    codes::CONFIG_INVALID,
                    "executable runtime requires `executableId`",
                )
            })?;
            project
                .config
                .runtimes
                .iter()
                .find(|runtime| runtime.id == id)
                .map(|runtime| PathBuf::from(&runtime.executable))
                .ok_or_else(|| {
                    CoreError::new(
                        codes::RUNTIME_MISSING,
                        format!("runtime `{id}` is not configured in this project"),
                    )
                })
        }
    }
}
