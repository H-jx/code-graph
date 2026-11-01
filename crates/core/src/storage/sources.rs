//! 来源内容、哈希、位置与依赖（计划 §3.4）。

use rusqlite::{Connection, OptionalExtension, params};

use crate::contract::error::{CoreError, codes};
use crate::contract::evidence::{SnapshotRef, Source, SourceClass};
use crate::project::snapshot::{LineMap, SourceSnapshot};

/// Match an untrusted reference against immutable evidence and original source rows.
pub fn validate_original_reference(
    connection: &Connection,
    reference: &crate::contract::evidence::SourceReference,
) -> Result<(), CoreError> {
    let row:Option<(String,String,i64,i64,String,String)>=connection.query_row(
        "SELECT e.source_id,e.content_hash,e.start_byte,e.end_byte,s.path,sv.source_class FROM evidence e JOIN sources s ON s.id=e.source_id JOIN source_versions sv ON sv.source_id=e.source_id AND sv.content_hash=e.content_hash WHERE e.id=?1",
        [&reference.evidence_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).optional().map_err(super::database_error)?;
    let valid = row.is_some_and(|(id, hash, start, end, path, class)| {
        id == reference.source_id
            && hash == reference.content_hash
            && start as u64 == reference.start_byte
            && end as u64 == reference.end_byte
            && !crate::project::scan::is_hard_excluded(&path)
            && matches!(
                class.as_str(),
                "code" | "config" | "schema" | "documentation"
            )
    });
    if valid {
        Ok(())
    } else {
        Err(CoreError::new(
            codes::EVIDENCE_INVALID,
            "reference must match stored original evidence",
        ))
    }
}

/// 写入来源行（幂等）。
pub fn upsert_source(
    connection: &Connection,
    repository_id: &str,
    source: &Source,
    selection_status: &str,
) -> Result<(), CoreError> {
    connection
        .execute(
            "INSERT INTO sources (id, repository_id, path, current_hash, selection_status)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET
               current_hash = excluded.current_hash,
               selection_status = excluded.selection_status",
            params![
                source.id,
                repository_id,
                source.path,
                source.content_hash,
                selection_status
            ],
        )
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot upsert source `{}`: {error}", source.id),
            )
        })?;
    Ok(())
}

/// 保存来源版本（原始字节 + 行映射）。
pub fn store_source_version(
    connection: &Connection,
    source_id: &str,
    content_hash: &str,
    content: &[u8],
    line_map: &LineMap,
    source_class: SourceClass,
    now: &str,
) -> Result<(), CoreError> {
    let line_map_json = serde_json::to_string(line_map).map_err(|error| {
        CoreError::new(
            codes::INTERNAL_ERROR,
            format!("cannot serialize line map: {error}"),
        )
    })?;
    connection
        .execute(
            "INSERT INTO source_versions
               (source_id, content_hash, content, byte_length, line_map_json, source_class, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(source_id, content_hash) DO NOTHING",
            params![
                source_id,
                content_hash,
                content,
                content.len() as i64,
                line_map_json,
                source_class.as_str(),
                now
            ],
        )
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot store source version: {error}"),
            )
        })?;
    Ok(())
}

/// 登记来源快照及其成员。
pub fn register_snapshot(
    connection: &Connection,
    snapshot: &SourceSnapshot,
) -> Result<(), CoreError> {
    connection
        .execute(
            "INSERT INTO source_snapshots (id, config_hash, source_set_hash, created_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET config_hash = excluded.config_hash,
                                           source_set_hash = excluded.source_set_hash,
                                           created_at = excluded.created_at",
            params![
                snapshot.id,
                snapshot.config_hash,
                snapshot.source_set_hash,
                snapshot.created_at
            ],
        )
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot register source snapshot: {error}"),
            )
        })?;
    for member in &snapshot.members {
        connection
            .execute(
                "INSERT INTO snapshot_sources (snapshot_id, source_id, content_hash, selection_status)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(snapshot_id, source_id) DO UPDATE SET content_hash = excluded.content_hash,
                                                                   selection_status = excluded.selection_status",
                params![
                    snapshot.id,
                    member.source_id,
                    member.content_hash,
                    member.selection_status.as_str()
                ],
            )
            .map_err(|error| {
                CoreError::new(
                    codes::INTERNAL_ERROR,
                    format!("cannot register snapshot member: {error}"),
                )
            })?;
    }
    Ok(())
}

/// 读取来源版本的原始字节。
pub fn read_source_version(
    connection: &Connection,
    source_id: &str,
    content_hash: &str,
) -> Result<Option<(Vec<u8>, String)>, CoreError> {
    let row: Option<(Vec<u8>, String)> = connection
        .query_row(
            "SELECT content, line_map_json FROM source_versions WHERE source_id = ?1 AND content_hash = ?2",
            params![source_id, content_hash],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot read source version: {error}"),
            )
        })?;
    Ok(row)
}

/// 读取来源的当前版本内容。
pub fn read_current_source(
    connection: &Connection,
    source_id: &str,
) -> Result<Option<(String, Vec<u8>)>, CoreError> {
    let row: Option<(String, Vec<u8>)> = connection
        .query_row(
            "SELECT sv.content_hash, sv.content
               FROM sources s
               JOIN source_versions sv
                 ON sv.source_id = s.id AND sv.content_hash = s.current_hash
              WHERE s.id = ?1",
            params![source_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot read current source: {error}"),
            )
        })?;
    Ok(row)
}

/// 来源 ID → 仓库相对路径。
pub fn source_path(
    connection: &Connection,
    source_id: &str,
) -> Result<Option<(String, String)>, CoreError> {
    let row: Option<(String, String)> = connection
        .query_row(
            "SELECT repository_id, path FROM sources WHERE id = ?1",
            params![source_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot read source path: {error}"),
            )
        })?;
    Ok(row)
}

/// 快照引用（API 层使用的来源快照标识）。
pub fn snapshot_ref(
    index_id: &str,
    generation: u64,
    source_snapshot_id: Option<String>,
) -> SnapshotRef {
    SnapshotRef {
        index_id: index_id.to_string(),
        generation: generation.to_string(),
        source_snapshot_id,
    }
}
