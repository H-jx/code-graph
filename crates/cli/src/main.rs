//! `project-knowledge` CLI 入口。
//!
//! 目标命令形态见 `docs/NEW-PROJECT-STRUCTURE.md` 第 6 节：
//! `init` / `index` / `status` / `query` / `collect` / `plugin` / `agent` / `generate`。
//!
//! 约定：JSON 输出统一写 stdout，日志写 stderr；成功完整 exit 0，partial/stale
//! exit 2，请求或执行失败 exit 1，取消 exit 130。

mod commands;

use std::process::ExitCode;

use clap::{Parser, Subcommand};

use crate::commands::{EXIT_CANCELLED, EXIT_FAILURE, EXIT_OK, EXIT_PARTIAL, Exit};

/// 项目知识命令行入口。
#[derive(Debug, Parser)]
#[command(
    name = "project-knowledge",
    version,
    about = "项目知识索引与查询：多仓库接入、基础索引、事实检索与证据读取",
    long_about = None,
    disable_help_subcommand = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

/// 子命令。
#[derive(Debug, Subcommand)]
pub enum Command {
    /// 初始化目标项目的配置与索引目录。
    Init(commands::init::InitArgs),
    /// 建立或更新索引。
    Index(commands::index::IndexArgs),
    /// 查看索引新鲜度、覆盖与缺口（只读）。
    Status(commands::status::StatusArgs),
    /// 执行查询。
    Query(commands::query::QueryArgs),
    /// 运行已激活的采集阶段。
    Collect(commands::collect::CollectArgs),
    /// 候选插件的提交、验证、激活与回滚。
    Plugin(commands::plugin::PluginArgs),
    /// 内置 Rig Agent：配置、生成、修复与恢复。
    Agent(commands::agent::AgentArgs),
    /// 导出上下文、导入主题与渲染 Skill。
    Generate(commands::generate::GenerateArgs),
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Init(args) => commands::init::run(args),
        Command::Index(args) => commands::index::run(args),
        Command::Status(args) => commands::status::run(args),
        Command::Query(args) => commands::query::run(args),
        Command::Collect(args) => commands::collect::run(args),
        Command::Plugin(args) => commands::plugin::run(args),
        Command::Agent(args) => commands::agent::run(args),
        Command::Generate(args) => commands::generate::run(args),
    };
    match result {
        Ok(Exit { code, .. }) => match code {
            EXIT_OK => ExitCode::SUCCESS,
            EXIT_PARTIAL => ExitCode::from(2),
            EXIT_FAILURE => ExitCode::from(1),
            EXIT_CANCELLED => ExitCode::from(130),
            other => ExitCode::from(other),
        },
        Err(error) => {
            eprintln!("error: {error}");
            let envelope = serde_json::json!({
                "error": {
                    "code": error.code,
                    "message": error.message,
                    "retryable": error.retryable,
                    "issues": error.issues,
                }
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&envelope).unwrap_or_else(|_| "{}".to_string())
            );
            ExitCode::from(1)
        }
    }
}
