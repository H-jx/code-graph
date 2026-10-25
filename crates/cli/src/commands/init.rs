//! `init`：初始化目标项目的配置与索引目录。
//!
//! 已有配置不覆盖（除非显式 `--force`）。

use std::path::PathBuf;

use clap::Args;
use project_knowledge_core::contract::error::CoreError;
use project_knowledge_core::project::config::ProjectConfig;
use serde_json::json;

use crate::commands::{Exit, now_timestamp, print_json};

/// `init` 参数。
#[derive(Debug, Args)]
pub struct InitArgs {
    /// 目标项目根目录。
    #[arg(long)]
    pub root: PathBuf,
    /// 项目 ID；缺省使用目录名。
    #[arg(long)]
    pub project_id: Option<String>,
    /// 覆盖已有配置。
    #[arg(long)]
    pub force: bool,
}

/// 执行 `init`。
pub fn run(args: InitArgs) -> Result<Exit, CoreError> {
    let root = args
        .root
        .canonicalize()
        .unwrap_or_else(|_| args.root.clone());
    let workspace = root.join(".project-knowledge");
    let config_path = workspace.join("project.json");
    if config_path.exists() && !args.force {
        return Err(CoreError::new(
            project_knowledge_core::contract::error::codes::CONFIG_INVALID,
            format!(
                "config `{}` already exists; use --force to overwrite",
                config_path.display()
            ),
        ));
    }
    std::fs::create_dir_all(&workspace).map_err(|error| {
        CoreError::new(
            project_knowledge_core::contract::error::codes::INTERNAL_ERROR,
            format!("cannot create `{}`: {error}", workspace.display()),
        )
    })?;
    for sub in [
        "work",
        "work/candidates",
        "work/contexts",
        "collectors",
        "prompts",
    ] {
        std::fs::create_dir_all(workspace.join(sub)).map_err(|error| {
            CoreError::new(
                project_knowledge_core::contract::error::codes::INTERNAL_ERROR,
                format!("cannot create `{sub}`: {error}"),
            )
        })?;
    }

    let project_id = args
        .project_id
        .clone()
        .or_else(|| {
            root.file_name()
                .map(|name| name.to_string_lossy().to_string())
                .filter(|name| !name.is_empty())
        })
        .unwrap_or_else(|| "project".to_string());

    let config = ProjectConfig {
        schema_version: project_knowledge_core::contract::CONFIG_SCHEMA_VERSION,
        project_id: project_id.clone(),
        repositories: vec![project_knowledge_core::project::config::RepositoryConfig {
            id: "repo.main".to_string(),
            // 配置位于 `.project-knowledge/project.json`，仓库根是它的上一级。
            root: "..".to_string(),
        }],
        selection: project_knowledge_core::project::config::SelectionConfig {
            include: vec!["**/*".to_string()],
            exclude: vec![
                ".git/**".to_string(),
                "node_modules/**".to_string(),
                "target/**".to_string(),
                "dist/**".to_string(),
                ".project-knowledge/**".to_string(),
                ".agents/skills/**".to_string(),
            ],
            respect_git_ignore: true,
            max_file_bytes: project_knowledge_core::project::config::DEFAULT_MAX_FILE_BYTES,
        },
        scopes: Vec::new(),
        services: Vec::new(),
        data_sources: Vec::new(),
        bindings: Vec::new(),
        knowledge_inputs: Vec::new(),
        plugins: project_knowledge_core::project::config::PluginDiscoveryConfig {
            directories: vec!["collectors".to_string()],
            auto_load: true,
            auto_activate: true,
        },
        collectors: Vec::new(),
        agent: None,
        runtimes: Vec::new(),
    };

    let serialized = serde_json::to_string_pretty(&config).map_err(|error| {
        CoreError::new(
            project_knowledge_core::contract::error::codes::INTERNAL_ERROR,
            format!("cannot serialize config: {error}"),
        )
    })?;
    std::fs::write(&config_path, format!("{serialized}\n")).map_err(|error| {
        CoreError::new(
            project_knowledge_core::contract::error::codes::INTERNAL_ERROR,
            format!("cannot write `{}`: {error}", config_path.display()),
        )
    })?;

    let payload = json!({
        "configPath": config_path.to_string_lossy(),
        "projectId": project_id,
        "root": root.to_string_lossy(),
        "createdAt": now_timestamp(),
        "indexDatabase": workspace.join("index.sqlite").to_string_lossy(),
        "next": ["project-knowledge index --config <config> --base"],
    });
    print_json(&payload);
    Ok(Exit::ok())
}
