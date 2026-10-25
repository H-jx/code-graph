//! 变化检测、失效、受影响任务重跑（计划 §3.6、T10）。
//!
//! v1 策略：全量与增量都**重算来源选择集合**，重跑受影响阶段的整个项目范围，
//! 优先正确性，不做微分补丁。失效事务先提交；失败的采集不撤销失效。
//!
//! 增量入口不调用模型，也不执行任何插件运行时。

use rusqlite::Connection;

use crate::contract::error::{CoreError, codes};
use crate::index::build::{BaseIndexResult, build_base_index};
use crate::project::config::LoadedProject;
use crate::storage;
use crate::storage::database::Database;

/// 增量结果。
#[derive(Debug, Clone)]
pub struct IncrementalResult {
    pub base: BaseIndexResult,
    /// 被标记为 stale 的替换范围。
    pub invalidated: Vec<String>,
}

/// 增量更新。
///
/// 步骤：
/// 1. 收集当前批次及其依赖，计算失效集合；
/// 2. 提交失效事务 A（批次单向失效、generation+1）；
/// 3. 重跑基础层（全范围重算选择集合）。
///
/// 采集插件阶段的真正重跑在 T09/T10 的插件宿主完成后接入；在此之前业务批次会被
/// 标记为 stale，并保留准确的诊断信息。
pub fn run_incremental(
    project: &LoadedProject,
    database: &mut Database,
    now: &str,
) -> Result<IncrementalResult, CoreError> {
    let invalidated = invalidate_stale_batches(database, now)?;
    let base = build_base_index(project, database, now)?;
    Ok(IncrementalResult { base, invalidated })
}

/// 只刷新基础层，并使业务/知识批次 stale。
pub fn run_base_only(
    project: &LoadedProject,
    database: &mut Database,
    now: &str,
) -> Result<IncrementalResult, CoreError> {
    let invalidated = invalidate_stale_batches(database, now)?;
    let base = build_base_index(project, database, now)?;
    Ok(IncrementalResult { base, invalidated })
}

/// 失效事务 A：把非基础层的当前批次标记为 stale。
fn invalidate_stale_batches(database: &mut Database, now: &str) -> Result<Vec<String>, CoreError> {
    let transaction = database.write_transaction()?;
    let connection = &transaction;
    let current_generation: i64 = connection
        .query_row(
            "SELECT generation FROM index_meta WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot read generation: {error}"),
            )
        })?;
    let generation = current_generation as u64 + 1;

    let mut invalidated: Vec<String> = Vec::new();
    for (replacement_key, _) in storage::current_batches(connection)? {
        if replacement_key.starts_with("(core.base,") {
            continue;
        }
        invalidated.push(replacement_key.clone());
    }
    if !invalidated.is_empty() {
        storage::invalidate_batches(connection, &invalidated, generation)?;
    }
    connection
        .execute(
            "UPDATE index_meta SET generation = ?1 WHERE id = 1",
            rusqlite::params![generation as i64],
        )
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot advance generation: {error}"),
            )
        })?;
    let _ = now;
    transaction.commit().map_err(|error| {
        CoreError::new(
            codes::BUSY,
            format!("cannot commit invalidation transaction: {error}"),
        )
    })?;
    Ok(invalidated)
}

/// 读取当前批次的失效状态（只读核验，不写库）。
pub fn current_freshness(
    connection: &Connection,
) -> Result<Vec<(String, String, Option<u64>)>, CoreError> {
    let mut result = Vec::new();
    for (replacement_key, batch_id) in storage::current_batches(connection)? {
        let state = storage::batch_state(connection, &batch_id)?;
        let invalidated = storage::batch_invalidated_generation(connection, &batch_id)?;
        result.push((
            replacement_key,
            state
                .map(|state| state.as_str().to_string())
                .unwrap_or_default(),
            invalidated,
        ));
    }
    Ok(result)
}
