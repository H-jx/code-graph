//! `generate`：导出上下文、导入主题与渲染 Skill。
//!
//! 主题与 Skill 校验、持久化和渲染统一由 Core 完成。

use std::path::PathBuf;

use clap::{Args, Subcommand};
use project_knowledge_core::contract::error::CoreError;

use crate::commands::{EXIT_PARTIAL, Exit, database_path, now_timestamp, print_json};
use project_knowledge_core::{
    contract::{
        error::codes,
        knowledge::{ConventionsImport, TopicArtifact, TopicContextRequest},
        query::{QueryRequest, SearchRequest},
    },
    knowledge::{context, documents, skill},
    project::config::LoadedProject,
    storage::Database,
};

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
        /// 完整 QueryRequest JSON；缺省按 topic 搜索。
        #[arg(long)]
        request: Option<PathBuf>,
        #[arg(long, default_value_t = 80_000)]
        max_chars: u64,
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
    /// 导入有原始依据的调查约定及身份增量（CAS）。
    Conventions {
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
    let project = LoadedProject::load(&args.config)?;
    let now = now_timestamp();
    let mut database = Database::open_maintenance(
        &database_path(&args.config),
        &project.config.project_id,
        &project.config_hash()?,
        &now,
    )?;
    let value = match args.action {
        GenerateAction::Context {
            topic,
            request,
            max_chars,
            output,
        } => {
            let query = if let Some(path) = request {
                read::<QueryRequest>(&path)?
            } else {
                QueryRequest::Search(SearchRequest {
                    query: topic.clone(),
                    fields: None,
                    filters: None,
                    page_size: Some(12),
                    continuation: None,
                    budget: None,
                })
            };
            let value = serde_json::to_value(context::export(
                &mut database,
                &project,
                &TopicContextRequest {
                    topic_id: topic,
                    query,
                    max_chars,
                },
                &now,
            )?)
            .unwrap();
            if let Some(path) = output {
                std::fs::write(path, serde_json::to_vec_pretty(&value).unwrap()).map_err(|_| {
                    CoreError::new(codes::INTERNAL_ERROR, "cannot write context output")
                })?;
            }
            value
        }
        GenerateAction::Docs { input } => documents::import_topic(
            &mut database,
            &project,
            &read::<TopicArtifact>(&input)?,
            &now,
        )?,
        GenerateAction::Conventions { input } => documents::import_conventions(
            &mut database,
            &project,
            &read::<ConventionsImport>(&input)?,
            &now,
        )?,
        GenerateAction::Skill { name, output } => {
            let output = output.unwrap_or_else(|| {
                project
                    .base_dir
                    .join(".project-knowledge/skills")
                    .join(&name)
            });
            skill::generate(&mut database, &project, &name, &output, &now)?
        }
    };
    let partial = value["status"] == "partial"
        || value["freshness"] == "stale"
        || value
            .get("gaps")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|g| !g.is_empty());
    print_json(&value);
    Ok(Exit {
        code: if partial { EXIT_PARTIAL } else { 0 },
    })
}
fn read<T: serde::de::DeserializeOwned + schemars::JsonSchema>(
    path: &std::path::Path,
) -> Result<T, CoreError> {
    let bytes = std::fs::read_to_string(path)
        .map_err(|_| CoreError::new(codes::INVALID_REQUEST, "cannot read generate input"))?;
    let value: serde_json::Value = serde_json::from_str(
        project_knowledge_core::project::config::strip_utf8_bom(&bytes),
    )
    .map_err(|_| CoreError::new(codes::INVALID_REQUEST, "generate input is not valid JSON"))?;
    project_knowledge_core::contract::schema::validate::<T>(&value, "")?;
    serde_json::from_value(value)
        .map_err(|_| CoreError::new(codes::INVALID_REQUEST, "invalid generate input"))
}
