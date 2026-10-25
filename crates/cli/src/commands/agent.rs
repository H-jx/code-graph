//! `agent`：内置 Rig Agent 的配置、生成、修复与恢复。
//!
//! Rig Agent（T18—T19）尚未实现；此命令明确失败，不返回假成功。
//! 基础索引与查询不依赖本命令。

use std::path::PathBuf;

use clap::{Args, Subcommand};
use project_knowledge_core::contract::error::CoreError;

use crate::commands::{Exit, not_implemented};

/// `agent` 参数。
#[derive(Debug, Args)]
pub struct AgentArgs {
    /// 项目配置文件。
    #[arg(long)]
    pub config: PathBuf,
    #[command(subcommand)]
    pub action: AgentAction,
}

/// `agent` 子动作。
#[derive(Debug, Subcommand)]
pub enum AgentAction {
    /// 配置模型与预算。
    Configure,
    /// 调查并生成插件。
    Build {
        /// 采集能力列表。
        #[arg(long)]
        focus: Option<String>,
        /// 任务目标。
        #[arg(long)]
        goal: Option<String>,
        /// 同时生成 Skill。
        #[arg(long)]
        with_skill: bool,
    },
    /// 修复指定插件。
    Repair {
        /// 插件 ID。
        #[arg(long)]
        plugin: String,
    },
    /// 从中断处恢复。
    Resume {
        /// 运行 ID。
        #[arg(long)]
        run: String,
    },
}

/// 执行 `agent`。
pub fn run(args: AgentArgs) -> Result<Exit, CoreError> {
    let _ = args;
    Err(not_implemented("agent"))
}
