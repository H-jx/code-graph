//! 与外部 Agent 共用 Core 的候选入口。
use crate::commands::{Exit, database_path, now_timestamp, print_json};
use clap::{Args, Subcommand};
use project_knowledge_core::collect::registry;
use project_knowledge_core::contract::error::{CoreError, codes};
use project_knowledge_core::project::config::LoadedProject;
use project_knowledge_core::storage::{Database, plugins};
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Args)]
pub struct PluginArgs {
    #[arg(long)]
    pub config: PathBuf,
    #[command(subcommand)]
    pub action: PluginAction,
}

#[derive(Debug, Subcommand)]
pub enum PluginAction {
    /// 发现候选并按项目 autoActivate 策略处理（仅维护入口执行）。
    Discover,
    Submit {
        #[arg(long)]
        path: PathBuf,
    },
    /// 显式联网准备锁定依赖；验证和采集仅复用该缓存。
    Prepare {
        #[arg(long)]
        candidate: String,
    },
    Validate {
        #[arg(long, conflicts_with = "path", required_unless_present = "path")]
        candidate: Vec<String>,
        #[arg(long, conflicts_with = "candidate")]
        path: Option<PathBuf>,
    },
    Activate {
        #[arg(long, conflicts_with_all = ["plugin", "version"])]
        candidate: Option<String>,
        #[arg(long, requires = "version", required_unless_present = "candidate")]
        plugin: Option<String>,
        #[arg(long, requires = "plugin")]
        version: Option<String>,
    },
    Rollback {
        #[arg(long)]
        plugin: String,
        #[arg(long)]
        version: String,
    },
}

pub fn run(args: PluginArgs) -> Result<Exit, CoreError> {
    let project = LoadedProject::load(&args.config)?;
    let path = database_path(&args.config);
    if !path.is_file() {
        return Err(CoreError::new(
            codes::INDEX_MISSING,
            "run index --base first",
        ));
    }
    let now = now_timestamp();
    let mut database = Database::open_maintenance(
        &path,
        &project.config.project_id,
        &project.config_hash()?,
        &now,
    )?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| CoreError::new(codes::INTERNAL_ERROR, e.to_string()))?;
    let cancel = CancellationToken::new();
    let response = match args.action {
        PluginAction::Discover => {
            let candidates = registry::discover(&project, &mut database, &now)?;
            let ids = candidates.into_iter().map(|c| c.id).collect::<Vec<_>>();
            if ids.is_empty() {
                serde_json::json!({"results":[]})
            } else {
                runtime.block_on(registry::process_candidates(
                    &project,
                    &mut database,
                    &ids,
                    &cancel,
                    &now,
                ))?
            }
        }
        PluginAction::Submit { path } => {
            serde_json::to_value(registry::submit(&project, &mut database, &path, &now)?)
                .expect("candidate JSON")
        }
        PluginAction::Prepare { candidate } => {
            runtime.block_on(registry::prepare(&project, &database, &candidate))?
        }
        PluginAction::Validate { candidate, path } => {
            let ids = if !candidate.is_empty() {
                candidate
            } else {
                vec![registry::submit(&project, &mut database, &path.expect("clap path"), &now)?.id]
            };
            runtime.block_on(registry::validate_many(
                &project,
                &mut database,
                &ids,
                &cancel,
                &now,
            ))?
        }
        PluginAction::Activate {
            candidate,
            plugin,
            version,
        } => {
            let id = if let Some(id) = candidate {
                id
            } else {
                plugins::validated_candidate_id(
                    database.connection(),
                    &plugin.expect("clap plugin"),
                    &version.expect("clap version"),
                )?
            };
            runtime.block_on(registry::activate(&project, &mut database, &id, &now))?
        }
        PluginAction::Rollback { plugin, version } => runtime.block_on(registry::rollback(
            &project,
            &mut database,
            &plugin,
            &version,
            &cancel,
            &now,
        ))?,
    };
    print_json(&response);
    Ok(Exit::ok())
}
