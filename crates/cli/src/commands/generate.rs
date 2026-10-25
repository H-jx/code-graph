//! `generate`：导出上下文、导入主题与渲染 Skill。
//!
//! 知识输出（T17）尚未实现；此命令明确失败，不返回假成功。

use std::path::PathBuf;

use clap::{Args, Subcommand};
use project_knowledge_core::contract::error::CoreError;

use crate::commands::{Exit, not_implemented};

/// `generate` 参数。
#[derive(Debug, Args)]
pub struct GenerateArgs {
    /// 项目配置文件。
    #[arg(long)]
    pub config: PathBuf,
    #[command(subcommand)]
    pub action: GenerateAction,
}

/// `generate` 子动作。
#[derive(Debug, Subcommand)]
pub enum GenerateAction {
    /// 导出主题上下文。
    Context {
        /// 主题。
        #[arg(long)]
        topic: String,
        /// 输出文件。
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// 导入主题产物。
    Docs {
        /// 输入文件。
        #[arg(long)]
        input: PathBuf,
    },
    /// 渲染 Skill。
    Skill {
        /// Skill 名称。
        #[arg(long)]
        name: String,
        /// 输出目录。
        #[arg(long)]
        output: Option<PathBuf>,
    },
}

/// 执行 `generate`。
pub fn run(args: GenerateArgs) -> Result<Exit, CoreError> {
    let _ = args;
    Err(not_implemented("generate"))
}
