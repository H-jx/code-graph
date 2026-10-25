//! `plugin`：候选插件的提交、验证、激活与回滚。
//!
//! 插件宿主（T07—T09）尚未实现；此命令明确失败，不返回假成功。

use std::path::PathBuf;

use clap::{Args, Subcommand};
use project_knowledge_core::contract::error::CoreError;

use crate::commands::{Exit, not_implemented};

/// `plugin` 参数。
#[derive(Debug, Args)]
pub struct PluginArgs {
    /// 项目配置文件。
    #[arg(long)]
    pub config: PathBuf,
    #[command(subcommand)]
    pub action: PluginAction,
}

/// `plugin` 子动作。
#[derive(Debug, Subcommand)]
pub enum PluginAction {
    /// 提交候选。
    Submit {
        /// 候选目录。
        #[arg(long)]
        path: PathBuf,
    },
    /// 校验候选。
    Validate {
        /// 候选目录。
        #[arg(long)]
        path: PathBuf,
    },
    /// 激活已验证候选。
    Activate {
        /// 插件 ID。
        #[arg(long)]
        plugin: String,
        /// 插件版本。
        #[arg(long)]
        version: String,
    },
    /// 回滚到旧版本（重新验证，不恢复旧批次）。
    Rollback {
        /// 插件 ID。
        #[arg(long)]
        plugin: String,
        /// 目标版本。
        #[arg(long)]
        version: String,
    },
}

/// 执行 `plugin`。
pub fn run(args: PluginArgs) -> Result<Exit, CoreError> {
    let _ = args;
    Err(not_implemented("plugin"))
}
