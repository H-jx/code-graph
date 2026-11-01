//! 插件版本、活动版本与原子激活事务（计划 §3.4、§3.6）。

use rusqlite::{Connection, OptionalExtension, params};

use crate::contract::error::{CoreError, codes};
use crate::contract::identity::content_hash;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub id: String,
    pub plugin_id: String,
    pub version: String,
    pub artifact_hash: String,
    pub directory: String,
    pub state: String,
    pub validation_id: Option<String>,
    #[serde(skip_serializing)]
    pub(crate) staged_json: Option<String>,
}

pub fn candidate(connection: &Connection, id: &str) -> Result<Candidate, CoreError> {
    connection.query_row("SELECT id, plugin_id, version, artifact_hash, directory, state, validation_id, staged_json FROM plugin_candidates WHERE id = ?1",
        [id], |r| Ok(Candidate { id:r.get(0)?,plugin_id:r.get(1)?,version:r.get(2)?,artifact_hash:r.get(3)?,directory:r.get(4)?,
            state:r.get(5)?,validation_id:r.get(6)?,staged_json:r.get(7)? }))
        .optional().map_err(super::database_error)?
        .ok_or_else(|| CoreError::new(codes::REFERENCE_MISSING, "candidate not found"))
}

pub fn validated_candidate_id(
    connection: &Connection,
    plugin: &str,
    version: &str,
) -> Result<String, CoreError> {
    let mut statement = connection.prepare("SELECT id FROM plugin_candidates WHERE plugin_id = ?1 AND version = ?2 AND state IN ('validated','published') ORDER BY id")
        .map_err(super::database_error)?;
    let ids = statement
        .query_map(params![plugin, version], |r| r.get(0))
        .map_err(super::database_error)?
        .collect::<Result<Vec<String>, _>>()
        .map_err(super::database_error)?;
    if ids.len() != 1 {
        return Err(CoreError::new(
            codes::INVALID_REQUEST,
            "select one validated candidate by --candidate; plugin/version is missing or ambiguous",
        ));
    }
    Ok(ids[0].clone())
}

pub fn insert_candidate(
    connection: &Connection,
    candidate: &Candidate,
    now: &str,
) -> Result<(), CoreError> {
    connection.execute("INSERT INTO plugin_candidates (id,plugin_id,version,artifact_hash,directory,state,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![candidate.id,candidate.plugin_id,candidate.version,candidate.artifact_hash,candidate.directory,candidate.state,now])
        .map_err(super::database_error)?;
    Ok(())
}

pub fn update_candidate(
    connection: &Connection,
    id: &str,
    state: &str,
    validation_id: Option<&str>,
    staged_json: Option<&str>,
    error: Option<&str>,
) -> Result<(), CoreError> {
    connection.execute("UPDATE plugin_candidates SET state = ?2, validation_id = COALESCE(?3, validation_id), staged_json = COALESCE(?4, staged_json), error_json = ?5 WHERE id = ?1",
        params![id,state,validation_id,staged_json,error]).map_err(super::database_error)?;
    Ok(())
}

pub fn published_manifest(
    connection: &Connection,
    plugin: &str,
    version: &str,
) -> Result<std::path::PathBuf, CoreError> {
    let path: String = connection
        .query_row(
            "SELECT artifact_path FROM plugin_versions WHERE plugin_id = ?1 AND version = ?2",
            params![plugin, version],
            |r| r.get(0),
        )
        .optional()
        .map_err(super::database_error)?
        .ok_or_else(|| {
            CoreError::new(codes::REFERENCE_MISSING, "plugin version is not published")
        })?;
    Ok(std::path::PathBuf::from(path).join("manifest.json"))
}

/// 只读活动版本的已登记 manifest，不扫描插件目录或运行插件。
pub fn active_manifests(
    connection: &Connection,
) -> Result<Vec<crate::contract::collector::PluginManifest>, CoreError> {
    let mut statement = connection.prepare("SELECT v.manifest_json FROM active_plugins a JOIN plugin_versions v ON v.plugin_id = a.plugin_id AND v.version = a.version ORDER BY a.plugin_id").map_err(super::database_error)?;
    let rows = statement
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(super::database_error)?;
    rows.map(|row| {
        let json = row.map_err(super::database_error)?;
        serde_json::from_str(&json).map_err(|e| {
            CoreError::new(
                codes::SCHEMA_INVALID,
                format!("stored active manifest: {e}"),
            )
        })
    })
    .collect()
}

pub fn recover_candidates(connection: &Connection, now: &str) -> Result<usize, CoreError> {
    connection
        .execute(
            "UPDATE runs SET status = 'interrupted', finished_at = ?1 WHERE status = 'running'",
            [now],
        )
        .map_err(super::database_error)?;
    connection
        .execute(
            "UPDATE activations SET status = 'failed' WHERE status = 'pending'",
            [],
        )
        .map_err(super::database_error)?;
    connection.execute("UPDATE plugin_candidates SET state = 'failed', error_json = '{\"code\":\"INTERRUPTED\"}' WHERE state = 'prepared'", [])
        .map_err(super::database_error)
}

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
