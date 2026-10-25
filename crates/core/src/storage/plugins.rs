//! 插件版本、活动版本与原子激活事务（计划 §3.4、§3.6）。

use rusqlite::{Connection, OptionalExtension, params};

use crate::contract::error::{CoreError, codes};
use crate::contract::identity::content_hash;

/// 登记插件版本。同版本不同哈希必须拒绝。
#[allow(clippy::too_many_arguments)]
pub fn register_plugin_version(
    connection: &Connection,
    plugin_id: &str,
    version: &str,
    artifact_hash: &str,
    manifest_json: &str,
    runtime_digest: &str,
    artifact_path: &str,
    validation_id: Option<&str>,
    now: &str,
) -> Result<(), CoreError> {
    let existing: Option<String> = connection
        .query_row(
            "SELECT artifact_hash FROM plugin_versions WHERE plugin_id = ?1 AND version = ?2",
            params![plugin_id, version],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot read plugin version: {error}"),
            )
        })?;
    if let Some(existing_hash) = existing {
        if existing_hash != artifact_hash {
            return Err(CoreError::new(
                codes::ARTIFACT_CHANGED,
                format!(
                    "plugin `{plugin_id}` version `{version}` is already published with a different artifact hash"
                ),
            ));
        }
        return Ok(());
    }
    connection
        .execute(
            "INSERT INTO plugin_versions
               (plugin_id, version, artifact_hash, manifest_json, runtime_digest, artifact_path, validation_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                plugin_id,
                version,
                artifact_hash,
                manifest_json,
                runtime_digest,
                artifact_path,
                validation_id,
                now
            ],
        )
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot register plugin version: {error}"),
            )
        })?;
    Ok(())
}

/// 活动版本映射。
pub fn active_version_map(connection: &Connection) -> Result<Vec<(String, String)>, CoreError> {
    let mut statement = connection
        .prepare("SELECT plugin_id, version FROM active_plugins ORDER BY plugin_id")
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot prepare active plugin query: {error}"),
            )
        })?;
    let rows = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot query active plugins: {error}"),
            )
        })?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot read active plugin: {error}"),
            )
        })?);
    }
    Ok(result)
}

/// 活动版本映射的摘要（用于验证绑定）。
pub fn active_version_digest(connection: &Connection) -> Result<String, CoreError> {
    let map = active_version_map(connection)?;
    let mut canonical = String::new();
    for (plugin_id, version) in map {
        canonical.push_str(&plugin_id);
        canonical.push('\u{1f}');
        canonical.push_str(&version);
        canonical.push('\n');
    }
    Ok(content_hash(canonical.as_bytes()))
}

/// 切换活动版本（CAS 由调用方在同一事务内检查 generation）。
pub fn set_active_plugin(
    connection: &Connection,
    plugin_id: &str,
    version: &str,
) -> Result<(), CoreError> {
    let revision: i64 = connection
        .query_row(
            "SELECT COALESCE(MAX(activation_revision), 0) + 1 FROM active_plugins",
            [],
            |row| row.get(0),
        )
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot compute activation revision: {error}"),
            )
        })?;
    connection
        .execute(
            "INSERT INTO active_plugins (plugin_id, version, activation_revision) VALUES (?1, ?2, ?3)
             ON CONFLICT(plugin_id) DO UPDATE SET version = excluded.version,
                                                   activation_revision = excluded.activation_revision",
            params![plugin_id, version, revision],
        )
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot set active plugin: {error}"),
            )
        })?;
    Ok(())
}

/// 记录激活尝试（用于中断恢复）。
pub fn record_activation(
    connection: &Connection,
    id: &str,
    expected_generation: u64,
    group: &serde_json::Value,
    status: &str,
    run_id: Option<&str>,
    now: &str,
) -> Result<(), CoreError> {
    let group_json = serde_json::to_string(group).map_err(|error| {
        CoreError::new(
            codes::INTERNAL_ERROR,
            format!("cannot serialize activation group: {error}"),
        )
    })?;
    connection
        .execute(
            "INSERT OR REPLACE INTO activations (id, expected_generation, group_json, status, run_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                id,
                expected_generation as i64,
                group_json,
                status,
                run_id,
                now
            ],
        )
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot record activation: {error}"),
            )
        })?;
    Ok(())
}

/// 激活状态。
pub fn activation_status(connection: &Connection, id: &str) -> Result<Option<String>, CoreError> {
    connection
        .query_row(
            "SELECT status FROM activations WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot read activation: {error}"),
            )
        })
}

/// 登记验证结果。
#[allow(clippy::too_many_arguments)]
pub fn record_validation(
    connection: &Connection,
    id: &str,
    artifact_hash: &str,
    input_digest: &str,
    active_version_map: &serde_json::Value,
    candidate_set_digest: &str,
    truth_hash: &str,
    status: &str,
    staged_result_hash: Option<&str>,
    now: &str,
) -> Result<(), CoreError> {
    let map_json = serde_json::to_string(active_version_map).map_err(|error| {
        CoreError::new(
            codes::INTERNAL_ERROR,
            format!("cannot serialize active version map: {error}"),
        )
    })?;
    connection
        .execute(
            "INSERT OR REPLACE INTO validations
               (id, artifact_hash, input_digest, active_version_map_json, candidate_set_digest, truth_hash, status, staged_result_hash, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                id,
                artifact_hash,
                input_digest,
                map_json,
                candidate_set_digest,
                truth_hash,
                status,
                staged_result_hash,
                now
            ],
        )
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot record validation: {error}"),
            )
        })?;
    Ok(())
}
