//! CLI 子命令实现。
//!
//! 每个子命令只做参数解析与结果呈现：算法与持久化全部在 Core。

pub mod agent;
pub mod collect;
pub mod generate;
pub mod index;
pub mod init;
pub mod plugin;
pub mod query;
pub mod status;

use std::path::{Path, PathBuf};

use project_knowledge_core::contract::error::CoreError;

/// 成功完整。
pub const EXIT_OK: u8 = 0;
/// 请求或执行失败。
pub const EXIT_FAILURE: u8 = 1;
/// 结果 partial 或 stale。
pub const EXIT_PARTIAL: u8 = 2;
/// 已取消。
pub const EXIT_CANCELLED: u8 = 130;

/// 命令退出结果。
#[derive(Debug, Clone)]
pub struct Exit {
    pub code: u8,
}

impl Exit {
    pub fn ok() -> Exit {
        Exit { code: EXIT_OK }
    }
}

/// 输出 JSON 到 stdout。
pub fn print_json(value: &serde_json::Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).unwrap_or_else(|error| {
            format!("{{\"error\":{{\"code\":\"INTERNAL_ERROR\",\"message\":\"{error}\"}}}}")
        })
    );
}

/// 索引数据库路径：与配置文件同目录的 `index.sqlite`。
pub fn database_path(config_path: &Path) -> PathBuf {
    config_path
        .parent()
        .map(|parent| parent.join("index.sqlite"))
        .unwrap_or_else(|| PathBuf::from("index.sqlite"))
}

/// 当前时间戳（秒级，UTC）。
pub fn now_timestamp() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// 尚未实现的能力：明确失败，不返回假成功。
pub fn not_implemented(feature: &str) -> CoreError {
    CoreError::new(
        project_knowledge_core::contract::error::codes::NOT_IMPLEMENTED,
        format!("`{feature}` 尚未实现（见 plan/NEW-PROJECT-LONG-TASK.md 的对应任务）"),
    )
}
