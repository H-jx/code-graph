//! `query`：执行统一查询请求。
//!
//! `--request <json-file>` 接受完整的 `QueryRequest`；
//! `--operation search --text ...` 是便捷形式，转换为同一个 DTO。

use std::io::Read;
use std::path::PathBuf;

use clap::Args;
use project_knowledge_core::contract::error::CoreError;
use project_knowledge_core::contract::query::{
    CodeMapRequest, PageRequest, QueryRequest, ReadSourceRequest, ReportRequest, SearchRequest,
    StatusRequest, TraceRequest, WorkflowRequest,
};
use project_knowledge_core::project::config::LoadedProject;
use project_knowledge_core::query::QueryContext;
use project_knowledge_core::storage::database::Database;
use serde_json::Value;

use crate::commands::{Exit, database_path, now_timestamp, print_json};

/// `query` 参数。
#[derive(Debug, Args)]
pub struct QueryArgs {
    /// 项目配置文件。
    #[arg(long)]
    pub config: PathBuf,
    /// 完整查询请求 JSON 文件。
    #[arg(long)]
    pub request: Option<PathBuf>,
    /// 便捷操作名（search/status/trace/codeMap/readSource/page/workflow/report）。
    #[arg(long)]
    pub operation: Option<String>,
    /// 便捷查询文本。
    #[arg(long)]
    pub text: Option<String>,
    /// 便捷路由（page 操作）。
    #[arg(long)]
    pub route: Option<String>,
}

/// 执行 `query`。
pub fn run(args: QueryArgs) -> Result<Exit, CoreError> {
    let project = LoadedProject::load(&args.config)?;
    let request = build_request(&args)?;
    let database_path = database_path(&args.config);
    let mut database = Database::open_readonly(&database_path)?;
    let now = now_timestamp();
    let mut context = QueryContext {
        database: &mut database,
        project: Some(&project),
        now,
    };
    let response = project_knowledge_core::query::execute(&mut context, &request)?;
    let payload = serde_json::to_value(&response).map_err(|error| {
        CoreError::new(
            project_knowledge_core::contract::error::codes::INTERNAL_ERROR,
            format!("cannot serialize query response: {error}"),
        )
    })?;
    print_json(&payload);
    let code = match response.status {
        project_knowledge_core::contract::query::ResponseStatus::Complete => {
            crate::commands::EXIT_OK
        }
        project_knowledge_core::contract::query::ResponseStatus::Partial => {
            crate::commands::EXIT_PARTIAL
        }
    };
    Ok(Exit { code })
}

/// 从请求文件或便捷参数构造统一 DTO。
fn build_request(args: &QueryArgs) -> Result<QueryRequest, CoreError> {
    if let Some(path) = &args.request {
        let mut raw = Vec::new();
        std::fs::File::open(path)
            .and_then(|file| file.take(1_048_577).read_to_end(&mut raw))
            .map_err(|error| {
                CoreError::new(
                    project_knowledge_core::contract::error::codes::INVALID_REQUEST,
                    format!("cannot read request file `{}`: {error}", path.display()),
                )
            })?;
        if raw.len() > 1_048_576 {
            return Err(CoreError::new(
                project_knowledge_core::contract::error::codes::INVALID_REQUEST,
                "query request exceeds 1 MiB",
            ));
        }
        let text = String::from_utf8(raw).map_err(|error| {
            CoreError::new(
                project_knowledge_core::contract::error::codes::INVALID_REQUEST,
                format!(
                    "request file `{}` is not valid UTF-8: {error}",
                    path.display()
                ),
            )
        })?;
        let value: Value = serde_json::from_str(
            project_knowledge_core::project::config::strip_utf8_bom(&text),
        )
        .map_err(|error| {
            CoreError::new(
                project_knowledge_core::contract::error::codes::INVALID_REQUEST,
                format!(
                    "request file `{}` is not valid JSON: {error}",
                    path.display()
                ),
            )
        })?;
        return serde_json::from_value(value).map_err(|error| {
            CoreError::new(
                project_knowledge_core::contract::error::codes::INVALID_REQUEST,
                format!(
                    "request file `{}` does not match QueryRequest: {error}",
                    path.display()
                ),
            )
        });
    }

    let operation = args.operation.clone().ok_or_else(|| {
        CoreError::new(
            project_knowledge_core::contract::error::codes::INVALID_REQUEST,
            "either `--request` or `--operation` is required",
        )
    })?;
    let text = args.text.clone().unwrap_or_default();
    match operation.as_str() {
        "status" => Ok(QueryRequest::Status(StatusRequest {
            check_freshness: true,
        })),
        "search" => Ok(QueryRequest::Search(SearchRequest {
            query: text,
            fields: None,
            filters: None,
            page_size: None,
            continuation: None,
            budget: None,
        })),
        "trace" => Ok(QueryRequest::Trace(TraceRequest {
            roots: Vec::new(),
            direction: project_knowledge_core::contract::query::Direction::Both,
            relation_kinds: None,
            max_depth: None,
            filters: None,
            budget: None,
        })),
        "codeMap" => Ok(QueryRequest::CodeMap(CodeMapRequest {
            query: if text.is_empty() { None } else { Some(text) },
            roots: None,
            max_files: None,
            max_symbols: None,
            filters: None,
            budget: None,
        })),
        "readSource" => Ok(QueryRequest::ReadSource(ReadSourceRequest {
            references: Vec::new(),
            max_response_chars: None,
            filters: None,
        })),
        "page" => Ok(QueryRequest::Page(PageRequest {
            route: args
                .route
                .clone()
                .or_else(|| Some(text.clone()))
                .unwrap_or_default(),
            scope_ids: None,
            filters: None,
            budget: None,
        })),
        "workflow" => Ok(QueryRequest::Workflow(WorkflowRequest {
            query: text,
            roots: None,
            scope_ids: None,
            filters: None,
            budget: None,
        })),
        "report" => Ok(QueryRequest::Report(ReportRequest {
            query: text,
            roots: None,
            scope_ids: None,
            filters: None,
            budget: None,
        })),
        other => Err(CoreError::new(
            project_knowledge_core::contract::error::codes::INVALID_REQUEST,
            format!("unknown operation `{other}`"),
        )),
    }
}
