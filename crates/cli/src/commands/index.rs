//! `index`：建立或更新索引。
//!
//! `--base` 只刷新基础层，并把业务/知识层标记为过期；
//! `--incremental` 重算来源选择集合并重跑受影响阶段。两者互斥。

use std::path::PathBuf;

use clap::Args;
use project_knowledge_core::contract::error::CoreError;
use project_knowledge_core::index::{run_base_only, run_incremental};
use project_knowledge_core::project::config::LoadedProject;
use project_knowledge_core::storage::database::Database;
use serde_json::{Value, json};

use crate::commands::{Exit, database_path, now_timestamp, print_json};

/// `index` 参数。
#[derive(Debug, Args)]
pub struct IndexArgs {
    /// 项目配置文件。
    #[arg(long)]
    pub config: PathBuf,
    /// 只刷新基础层。
    #[arg(long)]
    pub base: bool,
    /// 增量更新。
    #[arg(long)]
    pub incremental: bool,
}

/// 执行 `index`。
pub fn run(args: IndexArgs) -> Result<Exit, CoreError> {
    if args.base == args.incremental {
        return Err(CoreError::new(
            project_knowledge_core::contract::error::codes::INVALID_REQUEST,
            "exactly one of `--base` or `--incremental` is required",
        ));
    }
    let project = LoadedProject::load(&args.config)?;
    let now = now_timestamp();
    let database_path = database_path(&args.config);
    let mut database = Database::open_maintenance(
        &database_path,
        &project.config.project_id,
        &project.config_hash()?,
        &now,
    )?;

    let (result, mode) = if args.base {
        (run_base_only(&project, &mut database, &now)?, "base")
    } else {
        (
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .map_err(|e| {
                    CoreError::new(
                        project_knowledge_core::contract::error::codes::INTERNAL_ERROR,
                        e.to_string(),
                    )
                })?
                .block_on(run_incremental(
                    &project,
                    &mut database,
                    &tokio_util::sync::CancellationToken::new(),
                    &now,
                ))?,
            "incremental",
        )
    };

    let base = &result.base;
    let mut payload = json!({        "mode": mode,
        "configPath": args.config.to_string_lossy(),
        "indexDatabase": database_path.to_string_lossy(),
        "indexId": base.index_id,
        "generation": result.generation.to_string(),
        "sourceSnapshotId": base.snapshot.id,
        "sourceSetHash": base.snapshot.source_set_hash,
        "entities": base.entities,
        "relations": base.relations,
        "searchDocuments": base.documents,
        "gaps": base.gaps.len(),
        "invalidated": result.invalidated,
        "executed": result.executed,
        "completedAt": now,
    });
    merge_coverage(&mut payload, &base.coverage);
    print_json(&payload);
    Ok(Exit::ok())
}

fn merge_coverage(
    payload: &mut Value,
    coverage: &[project_knowledge_core::contract::collector::Coverage],
) {
    let mut map = serde_json::Map::new();
    for entry in coverage {
        let key = format!("{}@{}", entry.capability, entry.scope_id);
        map.insert(
            key,
            json!({
                "capability": entry.capability,
                "scopeId": entry.scope_id,
                "sourceSetId": entry.source_set_id,
                "status": entry.status.as_str(),
                "gapIds": entry.gap_ids,
            }),
        );
    }
    if let Some(object) = payload.as_object_mut() {
        object.insert("coverage".to_string(), Value::Object(map));
    }
}
