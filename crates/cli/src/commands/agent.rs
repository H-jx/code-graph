//! `agent`：内置 Rig Agent 的配置、生成、修复与恢复。
//!
//! 模型凭据只在 Agent 入口读取；日常索引和查询不加载 dotenv 或模型客户端。

use std::path::PathBuf;

use clap::{Args, Subcommand};
use project_knowledge_core::contract::error::CoreError;

use crate::commands::Exit;

/// `agent` 参数。
#[derive(Debug, Args)]
pub struct AgentArgs {
    /// 项目配置文件。
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,
    /// .env 文件；缺省读取工作目录或目标项目的 .env。
    #[arg(long, global = true)]
    pub env_file: Option<PathBuf>,
    #[command(subcommand)]
    pub action: AgentAction,
}

/// `agent` 子动作。
#[derive(Debug, Subcommand)]
pub enum AgentAction {
    /// 配置模型与预算。
    Configure {
        #[arg(long)]
        provider: Option<String>,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        base_url: Option<String>,
        #[arg(long)]
        api_key_env: Option<String>,
        #[arg(long)]
        max_turns: Option<u32>,
        #[arg(long)]
        max_tool_calls: Option<u32>,
        #[arg(long)]
        timeout_seconds: Option<u64>,
        #[arg(long)]
        max_input_chars: Option<u64>,
        #[arg(long)]
        max_output_tokens: Option<u64>,
        #[arg(long, action=clap::ArgAction::Set)]
        auto_activate: Option<bool>,
        /// 显式控制模型思考模式（由 Rig 映射到所选协议）。
        #[arg(long, action=clap::ArgAction::Set)]
        thinking: Option<bool>,
        /// 实际调用一次模型查询工具，核验配置端点的工具能力。
        #[arg(long)]
        check: bool,
    },
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
        #[arg(long)]
        with_skill: bool,
        #[arg(long)]
        goal: Option<String>,
    },
    /// 从中断处恢复。
    Resume {
        /// 运行 ID。
        #[arg(long)]
        run: String,
        /// 补充恢复指引；不重置预算、截止时间或已有验收样例。
        #[arg(long)]
        goal: Option<String>,
    },
}

/// 执行 `agent`。
pub fn run(args: AgentArgs) -> Result<Exit, CoreError> {
    #[cfg(not(feature = "agent"))]
    {
        let _ = args;
        Err(CoreError::new(
            project_knowledge_core::contract::codes::NOT_IMPLEMENTED,
            "agent feature disabled; rebuild with default features",
        ))
    }
    #[cfg(feature = "agent")]
    {
        run_enabled(args)
    }
}

#[cfg(feature = "agent")]
fn run_enabled(args: AgentArgs) -> Result<Exit, CoreError> {
    use project_knowledge_agent::{
        config::{self, Environment},
        runner::{self, RunRequest},
    };
    use project_knowledge_core::{contract::error::codes, project::config::LoadedProject};
    let config_path = args.config.as_ref().ok_or_else(|| {
        CoreError::new(
            codes::INVALID_REQUEST,
            "agent requires --config <project.json>",
        )
    })?;
    let mut project = LoadedProject::load(config_path)?;
    let env = Environment::load(&project, args.env_file.as_deref())?;
    let request=match args.action {
        AgentAction::Configure {provider,model,base_url,api_key_env,max_turns,max_tool_calls,timeout_seconds,max_input_chars,max_output_tokens,auto_activate,thinking,check}=>{
            let mut value=env.configuration(project.config.agent.as_ref())?;
            if let Some(v)=provider{value.provider=v;}
            if let Some(v)=model{value.model=v;}
            if let Some(v)=base_url{value.base_url=Some(v);}
            if let Some(v)=api_key_env{value.api_key_env=v;}
            if let Some(v)=max_turns{value.max_turns=v;}
            if let Some(v)=max_tool_calls{value.max_tool_calls=v;}
            if let Some(v)=timeout_seconds{value.timeout_seconds=v;}
            if let Some(v)=max_input_chars{value.max_input_chars=v;}
            if let Some(v)=max_output_tokens{value.max_output_tokens=v;}
            if let Some(v)=auto_activate{value.auto_activate=Some(v);}
            if let Some(v)=thinking{value.thinking=Some(v);}
            config::validate(&value)?;
            let key_configured=env.credential(&value).is_ok();
            config::save(&project,value.clone())?;
            if !check {
                crate::commands::print_json(&serde_json::json!({"status":"configured","agent":value,"keyConfigured":key_configured,"endpointVerified":false,"envFile":env.path}));
                return Ok(Exit::ok());
            }
            project=LoadedProject::load(config_path)?;
            RunRequest{goal:"Verify a Core status tool round trip".into(),focus:None,with_skill:false,resume:None,repair_plugin:None,smoke:true}
        }
        AgentAction::Build{focus,goal,with_skill}=>RunRequest{goal:goal.unwrap_or_else(||"Investigate the project and generate a source-driven collector plugin with evidence and acceptance cases".into()),focus,with_skill,resume:None,repair_plugin:None,smoke:false},
        AgentAction::Repair{plugin,with_skill,goal}=>RunRequest{goal:goal.unwrap_or_else(||format!("Repair active plugin {plugin}; inspect current source and get-capabilities schema repair. Preserve plugin ID and cases; create a new version.")),focus:None,with_skill,resume:None,repair_plugin:Some(plugin),smoke:false},
        AgentAction::Resume{run,goal}=>RunRequest{goal:goal.unwrap_or_default(),focus:None,with_skill:false,resume:Some(run),repair_plugin:None,smoke:false},
    };
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|_| CoreError::new(codes::INTERNAL_ERROR, "cannot start agent runtime"))?;
    let report = runtime.block_on(runner::run(
        project,
        env,
        request,
        tokio_util::sync::CancellationToken::new(),
    ))?;
    let status = report["status"].as_str().unwrap_or("failed");
    let code = match status {
        "complete" => 0,
        "partial" | "pending_activation" => 2,
        "cancelled" => 130,
        _ => 1,
    };
    crate::commands::print_json(&report);
    Ok(Exit { code })
}
