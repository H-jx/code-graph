//! 提取记录、替换范围、状态与覆盖缺口（计划 §3.4）。
//!
//! 批次 freshness 与 run status 独立：`fresh|stale|unknown`；run 可以 succeeded
//! 但 coverage partial。历史批次一旦失效不再自动恢复 fresh。

use rusqlite::{Connection, OptionalExtension, params};

use crate::contract::error::{CoreError, codes};
use crate::contract::identity::content_hash;

/// 运行状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    Pending,
    Running,
    Succeeded,
    Partial,
    Failed,
    Cancelled,
    Interrupted,
}

impl RunStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            RunStatus::Pending => "pending",
            RunStatus::Running => "running",
            RunStatus::Succeeded => "succeeded",
            RunStatus::Partial => "partial",
            RunStatus::Failed => "failed",
            RunStatus::Cancelled => "cancelled",
            RunStatus::Interrupted => "interrupted",
        }
    }

    pub fn parse(value: &str) -> RunStatus {
        match value {
            "pending" => RunStatus::Pending,
            "running" => RunStatus::Running,
            "succeeded" => RunStatus::Succeeded,
            "partial" => RunStatus::Partial,
            "failed" => RunStatus::Failed,
            "cancelled" => RunStatus::Cancelled,
            "interrupted" => RunStatus::Interrupted,
            _ => RunStatus::Failed,
        }
    }
}

/// 批次状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchState {
    Fresh,
    Stale,
    Unknown,
}

impl BatchState {
    pub fn as_str(&self) -> &'static str {
        match self {
            BatchState::Fresh => "fresh",
            BatchState::Stale => "stale",
            BatchState::Unknown => "unknown",
        }
    }

    pub fn parse(value: &str) -> BatchState {
        match value {
            "fresh" => BatchState::Fresh,
            "stale" => BatchState::Stale,
            _ => BatchState::Unknown,
        }
    }
}

/// 创建运行记录。
pub fn create_run(
    connection: &Connection,
    id: &str,
    kind: &str,
    stage: &str,
    input_snapshot_id: Option<&str>,
    now: &str,
) -> Result<(), CoreError> {
    connection
        .execute(
            "INSERT INTO runs (id, kind, stage, status, input_snapshot_id, started_at)
             VALUES (?1, ?2, ?3, 'running', ?4, ?5)",
            params![id, kind, stage, input_snapshot_id, now],
        )
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot create run `{id}`: {error}"),
            )
        })?;
    Ok(())
}

/// 结束运行记录（诊断事务，不改变当前事实）。
pub fn finish_run(
    connection: &Connection,
    id: &str,
    status: RunStatus,
    error: Option<&serde_json::Value>,
    now: &str,
) -> Result<(), CoreError> {
    let error_json = match error {
        Some(value) => Some(serde_json::to_string(value).map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot serialize run error: {error}"),
            )
        })?),
        None => None,
    };
    connection
        .execute(
            "UPDATE runs SET status = ?2, finished_at = ?3, error_json = ?4 WHERE id = ?1",
            params![id, status.as_str(), now, error_json],
        )
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot finish run `{id}`: {error}"),
            )
        })?;
    Ok(())
}

/// 批次 ID：由运行与替换范围派生，重复提交保持幂等。
pub fn batch_id(run_id: &str, replacement_key: &str) -> String {
    format!(
        "bat_{}",
        content_hash(format!("{run_id}\u{1f}{replacement_key}").as_bytes())
    )
}

/// 基础层的替换范围键。
pub fn base_replacement_key(repository_id: &str) -> String {
    format!("(core.base,{repository_id})")
}

/// 插件阶段的替换范围键。
pub fn plugin_replacement_key(plugin_id: &str, stage_id: &str, project_id: &str) -> String {
    format!("({plugin_id},{stage_id},{project_id})")
}

/// 创建批次。
pub fn create_batch(
    connection: &Connection,
    id: &str,
    run_id: &str,
    producer_key: &str,
    replacement_key: &str,
    artifact_hash: &str,
    input_digest: &str,
    now: &str,
) -> Result<(), CoreError> {
    connection
        .execute(
            "INSERT INTO batches (id, run_id, producer_key, replacement_key, artifact_hash, input_digest, state, invalidated_generation, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'fresh', NULL, ?7)",
            params![
                id,
                run_id,
                producer_key,
                replacement_key,
                artifact_hash,
                input_digest,
                now
            ],
        )
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot create batch `{id}`: {error}"),
            )
        })?;
    Ok(())
}

/// 指向当前批次（替换旧批次的可见性）。
pub fn set_current_batch(
    connection: &Connection,
    replacement_key: &str,
    batch_id: &str,
) -> Result<(), CoreError> {
    connection
        .execute(
            "INSERT INTO current_batches (replacement_key, batch_id) VALUES (?1, ?2)
             ON CONFLICT(replacement_key) DO UPDATE SET batch_id = excluded.batch_id",
            params![replacement_key, batch_id],
        )
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot set current batch for `{replacement_key}`: {error}"),
            )
        })?;
    Ok(())
}

/// 当前批次。
pub fn current_batch(
    connection: &Connection,
    replacement_key: &str,
) -> Result<Option<String>, CoreError> {
    connection
        .query_row(
            "SELECT batch_id FROM current_batches WHERE replacement_key = ?1",
            params![replacement_key],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot read current batch: {error}"),
            )
        })
}

/// 全部当前批次。
pub fn current_batches(connection: &Connection) -> Result<Vec<(String, String)>, CoreError> {
    let mut statement = connection
        .prepare("SELECT replacement_key, batch_id FROM current_batches ORDER BY replacement_key")
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot prepare current batch query: {error}"),
            )
        })?;
    let rows = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot query current batches: {error}"),
            )
        })?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot read current batch row: {error}"),
            )
        })?);
    }
    Ok(result)
}

/// 失效事务 A 的核心：把受影响批次标记为 stale，并记录失效 generation。
///
/// 失效是单向的：`invalidated_generation` 一旦写入不再清空。
pub fn invalidate_batches(
    connection: &Connection,
    replacement_keys: &[String],
    generation: u64,
) -> Result<usize, CoreError> {
    let mut affected = 0usize;
    for key in replacement_keys {
        let changed = connection
            .execute(
                "UPDATE batches
                    SET state = 'stale',
                        invalidated_generation = COALESCE(invalidated_generation, ?2)
                  WHERE id IN (SELECT batch_id FROM current_batches WHERE replacement_key = ?1)
                    AND state <> 'stale'",
                params![key, generation as i64],
            )
            .map_err(|error| {
                CoreError::new(
                    codes::INTERNAL_ERROR,
                    format!("cannot invalidate batch for `{key}`: {error}"),
                )
            })?;
        affected += changed;
    }
    super::knowledge::invalidate_batches(connection, replacement_keys)?;
    Ok(affected)
}

/// 读取批次的失效 generation（用于验证单向失效不变量）。
pub fn batch_invalidated_generation(
    connection: &Connection,
    batch_id: &str,
) -> Result<Option<u64>, CoreError> {
    let value: Option<i64> = connection
        .query_row(
            "SELECT invalidated_generation FROM batches WHERE id = ?1",
            params![batch_id],
            |row| row.get::<_, Option<i64>>(0),
        )
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot read invalidated generation: {error}"),
            )
        })?;
    Ok(value.map(|value| value as u64))
}

/// 批次状态。
pub fn batch_state(
    connection: &Connection,
    batch_id: &str,
) -> Result<Option<BatchState>, CoreError> {
    let value: Option<String> = connection
        .query_row(
            "SELECT state FROM batches WHERE id = ?1",
            params![batch_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot read batch state: {error}"),
            )
        })?;
    Ok(value.map(|value| BatchState::parse(&value)))
}

/// 写入依赖记录。
pub fn insert_dependency(
    connection: &Connection,
    batch_id: &str,
    kind: &str,
    key: &str,
    digest: &str,
) -> Result<(), CoreError> {
    connection
        .execute(
            "INSERT INTO dependencies (batch_id, kind, key, digest) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(batch_id, kind, key) DO UPDATE SET digest = excluded.digest",
            params![batch_id, kind, key, digest],
        )
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot insert dependency: {error}"),
            )
        })?;
    Ok(())
}

/// 读取批次依赖。
pub fn load_dependencies(
    connection: &Connection,
    batch_id: &str,
) -> Result<Vec<(String, String, String)>, CoreError> {
    let mut statement = connection
        .prepare(
            "SELECT kind, key, digest FROM dependencies WHERE batch_id = ?1 ORDER BY kind, key",
        )
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot prepare dependency query: {error}"),
            )
        })?;
    let rows = statement
        .query_map(params![batch_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot query dependencies: {error}"),
            )
        })?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot read dependency: {error}"),
            )
        })?);
    }
    Ok(result)
}

/// 删除批次覆盖记录（替换前清理）。
pub fn clear_batch_coverage(connection: &Connection, batch_id: &str) -> Result<(), CoreError> {
    connection
        .execute(
            "DELETE FROM coverage WHERE batch_id = ?1",
            params![batch_id],
        )
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot clear coverage: {error}"),
            )
        })?;
    Ok(())
}

/// 写入覆盖记录。
pub fn insert_coverage(
    connection: &Connection,
    batch_id: &str,
    capability: &str,
    scope_id: &str,
    source_set_id: &str,
    status: &str,
    counts: &serde_json::Map<String, serde_json::Value>,
    gap_ids: &[String],
) -> Result<(), CoreError> {
    let counts_json = serde_json::to_string(counts).map_err(|error| {
        CoreError::new(
            codes::INTERNAL_ERROR,
            format!("cannot serialize coverage counts: {error}"),
        )
    })?;
    let gap_ids_json = serde_json::to_string(gap_ids).map_err(|error| {
        CoreError::new(
            codes::INTERNAL_ERROR,
            format!("cannot serialize gap ids: {error}"),
        )
    })?;
    connection
        .execute(
            "INSERT INTO coverage (batch_id, capability, scope_id, source_set_id, status, counts_json, gap_ids_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(batch_id, capability, scope_id, source_set_id) DO UPDATE SET status = excluded.status,
                                                                              counts_json = excluded.counts_json,
                                                                              gap_ids_json = excluded.gap_ids_json",
            params![
                batch_id,
                capability,
                scope_id,
                source_set_id,
                status,
                counts_json,
                gap_ids_json
            ],
        )
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot insert coverage: {error}"),
            )
        })?;
    Ok(())
}

/// 写入缺口。
pub fn insert_gap(
    connection: &Connection,
    id: &str,
    batch_id: &str,
    code: &str,
    payload: &serde_json::Value,
) -> Result<(), CoreError> {
    let payload_json = serde_json::to_string(payload).map_err(|error| {
        CoreError::new(
            codes::INTERNAL_ERROR,
            format!("cannot serialize gap: {error}"),
        )
    })?;
    connection
        .execute(
            "INSERT INTO gaps (id, batch_id, code, payload_json) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET batch_id = excluded.batch_id,
                                           code = excluded.code,
                                           payload_json = excluded.payload_json",
            params![id, batch_id, code, payload_json],
        )
        .map_err(|error| {
            CoreError::new(codes::INTERNAL_ERROR, format!("cannot insert gap: {error}"))
        })?;
    Ok(())
}

/// 读取批次的覆盖记录。
pub fn load_coverage(
    connection: &Connection,
    batch_id: &str,
) -> Result<Vec<(String, String, String, String)>, CoreError> {
    let mut statement = connection
        .prepare(
            "SELECT capability, scope_id, source_set_id, status FROM coverage WHERE batch_id = ?1
              ORDER BY capability, scope_id, source_set_id",
        )
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot prepare coverage query: {error}"),
            )
        })?;
    let rows = statement
        .query_map(params![batch_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot query coverage: {error}"),
            )
        })?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot read coverage: {error}"),
            )
        })?);
    }
    Ok(result)
}

/// 读取批次的缺口。
pub fn load_gaps(
    connection: &Connection,
    batch_id: &str,
) -> Result<Vec<(String, String, serde_json::Value)>, CoreError> {
    let mut statement = connection
        .prepare("SELECT id, code, payload_json FROM gaps WHERE batch_id = ?1 ORDER BY id")
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot prepare gap query: {error}"),
            )
        })?;
    let rows = statement
        .query_map(params![batch_id], |row| {
            let payload: String = row.get(2)?;
            Ok((row.get(0)?, row.get(1)?, payload))
        })
        .map_err(|error| {
            CoreError::new(codes::INTERNAL_ERROR, format!("cannot query gaps: {error}"))
        })?;
    let mut result = Vec::new();
    for row in rows {
        let (id, code, payload) = row.map_err(|error| {
            CoreError::new(codes::INTERNAL_ERROR, format!("cannot read gap: {error}"))
        })?;
        let value = serde_json::from_str(&payload).unwrap_or(serde_json::Value::Null);
        result.push((id, code, value));
    }
    Ok(result)
}
