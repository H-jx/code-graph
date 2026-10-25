//! `status`：新鲜度、覆盖、错误与缺口（只读）。
//!
//! 只读路径不建库、不迁移、不写诊断，也不执行插件或模型。

use std::path::PathBuf;

use clap::Args;
use project_knowledge_core::contract::error::CoreError;
use project_knowledge_core::contract::query::{QueryRequest, StatusRequest};
use project_knowledge_core::project::config::LoadedProject;
use project_knowledge_core::query::QueryContext;
use project_knowledge_core::storage::database::Database;

use crate::commands::{Exit, database_path, now_timestamp, print_json};

/// `status` 参数。
#[derive(Debug, Args)]
pub struct StatusArgs {
    /// 项目配置文件。
    #[arg(long)]
    pub config: PathBuf,
    /// 只读核验新鲜度（默认开启）。
    #[arg(long)]
    pub check_freshness: bool,
}

/// 执行 `status`。
pub fn run(args: StatusArgs) -> Result<Exit, CoreError> {
    let project = LoadedProject::load(&args.config)?;
    let database_path = database_path(&args.config);
    let mut database = Database::open_readonly(&database_path)?;
    let now = now_timestamp();
    let request = QueryRequest::Status(StatusRequest {
        check_freshness: args.check_freshness,
    });
    let mut context = QueryContext {
        database: &mut database,
        project: Some(&project),
        now,
    };
    let response = project_knowledge_core::query::execute(&mut context, &request)?;
    let payload = serde_json::to_value(&response).map_err(|error| {
        CoreError::new(
            project_knowledge_core::contract::error::codes::INTERNAL_ERROR,
            format!("cannot serialize status response: {error}"),
        )
    })?;
    print_json(&payload);
    let freshness = response.completeness.freshness;
    let code = if matches!(
        freshness,
        project_knowledge_core::contract::query::Freshness::Fresh
    ) {
        crate::commands::EXIT_OK
    } else {
        crate::commands::EXIT_PARTIAL
    };
    Ok(Exit { code })
}
