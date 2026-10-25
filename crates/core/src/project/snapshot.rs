//! 来源快照、内容哈希与行映射（计划 §3.2）。
//!
//! 快照成员是排序后的 `(repositoryId, path, hash, selectionStatus)`；新增、删除、
//! 被忽略与零匹配变化都会改变集合摘要。读取期间元信息或哈希不一致时最多重读
//! 两次，仍不稳定则返回 `SOURCE_CHANGED`，不提交 fresh 快照。

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::contract::error::{CoreError, codes};
use crate::contract::identity::{compute_source_id, content_hash};
use crate::project::scan::{ScanResult, SelectionStatus};

/// 快照成员。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotMember {
    pub source_id: String,
    pub repository_id: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
    pub selection_status: SelectionStatus,
}

/// 来源快照。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceSnapshot {
    pub id: String,
    pub config_hash: String,
    pub source_set_hash: String,
    pub created_at: String,
    #[serde(default)]
    pub members: Vec<SnapshotMember>,
}

impl SourceSnapshot {
    /// 按来源 ID 查找成员。
    pub fn member(&self, source_id: &str) -> Option<&SnapshotMember> {
        self.members
            .iter()
            .find(|member| member.source_id == source_id)
    }

    /// 选中的成员。
    pub fn selected_members(&self) -> impl Iterator<Item = &SnapshotMember> {
        self.members
            .iter()
            .filter(|member| member.selection_status == SelectionStatus::Selected)
    }
}

/// 来源快照 ID：由配置哈希与集合摘要决定，因此同样的输入得到同样的 ID。
pub fn snapshot_id(config_hash: &str, source_set_hash: &str) -> String {
    format!(
        "snap_{}",
        content_hash(format!("{config_hash}\u{1f}{source_set_hash}").as_bytes())
    )
}

/// 从扫描结果构建来源快照。
pub fn build_snapshot(
    project_id: &str,
    config_hash: &str,
    results: &[ScanResult],
    created_at: &str,
) -> SourceSnapshot {
    let source_set_hash = crate::project::scan::source_set_digest(results);
    let mut members: Vec<SnapshotMember> = Vec::new();
    for result in results {
        for source in &result.sources {
            members.push(SnapshotMember {
                source_id: compute_source_id(project_id, &source.repository_id, &source.path),
                repository_id: source.repository_id.clone(),
                path: source.path.clone(),
                content_hash: if source.selection_status == SelectionStatus::Selected {
                    Some(source.content_hash.clone())
                } else {
                    None
                },
                selection_status: source.selection_status,
            });
        }
    }
    members.sort_by(|left, right| {
        left.repository_id
            .cmp(&right.repository_id)
            .then(left.path.cmp(&right.path))
    });
    SourceSnapshot {
        id: snapshot_id(config_hash, &source_set_hash),
        config_hash: config_hash.to_string(),
        source_set_hash,
        created_at: created_at.to_string(),
        members,
    }
}

/// 稳定读取文件内容。
///
/// 读取前后比较元信息（长度与修改时间）；若期间变化则重读，最多两次；仍不稳定
/// 返回 `SOURCE_CHANGED`。
pub fn read_stable(path: &Path) -> Result<StableContent, CoreError> {
    let mut last_error: Option<CoreError> = None;
    for _ in 0..3 {
        let before = match std::fs::metadata(path) {
            Ok(metadata) => metadata,
            Err(error) => {
                return Err(CoreError::new(
                    codes::SOURCE_CHANGED,
                    format!("cannot stat {}: {error}", path.display()),
                ));
            }
        };
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) => {
                return Err(CoreError::new(
                    codes::SOURCE_CHANGED,
                    format!("cannot read {}: {error}", path.display()),
                ));
            }
        };
        let after = match std::fs::metadata(path) {
            Ok(metadata) => metadata,
            Err(error) => {
                return Err(CoreError::new(
                    codes::SOURCE_CHANGED,
                    format!("cannot stat {}: {error}", path.display()),
                ));
            }
        };
        if before.len() == after.len() && before.len() == bytes.len() as u64 {
            return Ok(StableContent {
                content_hash: content_hash(&bytes),
                bytes,
            });
        }
        last_error = Some(CoreError::new(
            codes::SOURCE_CHANGED,
            format!("{} changed while being read", path.display()),
        ));
    }
    Err(last_error.unwrap_or_else(|| {
        CoreError::new(
            codes::SOURCE_CHANGED,
            format!("{} could not be read consistently", path.display()),
        )
    }))
}

/// 稳定读取的结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StableContent {
    pub bytes: Vec<u8>,
    pub content_hash: String,
}

/// 行映射：字节偏移 → 行号（从 1 开始）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LineMap {
    /// 每行起始字节偏移；第 0 项恒为 0。
    pub line_starts: Vec<u64>,
    pub byte_length: u64,
}

impl LineMap {
    /// 从原始字节构建行映射。CRLF 不改变行计数：CRLF 视为一个换行。
    pub fn build(bytes: &[u8]) -> LineMap {
        let mut line_starts = vec![0u64];
        let mut index = 0usize;
        while index < bytes.len() {
            match bytes[index] {
                b'\n' => {
                    line_starts.push((index + 1) as u64);
                    index += 1;
                }
                b'\r' => {
                    if index + 1 < bytes.len() && bytes[index + 1] == b'\n' {
                        line_starts.push((index + 2) as u64);
                        index += 2;
                    } else {
                        line_starts.push((index + 1) as u64);
                        index += 1;
                    }
                }
                _ => index += 1,
            }
        }
        LineMap {
            line_starts,
            byte_length: bytes.len() as u64,
        }
    }

    /// 字节偏移所在行号（从 1 开始）。
    pub fn line_of(&self, offset: u64) -> u64 {
        match self.line_starts.binary_search(&offset) {
            Ok(index) => index as u64 + 1,
            Err(index) => index as u64,
        }
    }

    /// 字节偏移所在列号（从 1 开始，按 UTF-8 字节计数）。
    pub fn column_of(&self, offset: u64) -> u64 {
        let index = self.line_starts.partition_point(|start| *start <= offset);
        let line_start = if index == 0 {
            0
        } else {
            self.line_starts[index - 1]
        };
        offset.saturating_sub(line_start) + 1
    }

    /// 区间覆盖的最后一行号。
    pub fn end_line_of(&self, start_byte: u64, end_byte: u64) -> u64 {
        if end_byte == 0 {
            return self.line_of(start_byte);
        }
        let last = end_byte.saturating_sub(1);
        self.line_of(last).max(self.line_of(start_byte))
    }
}

/// 校验字节区间在给定内容内有效且非空。
pub fn validate_range(content: &[u8], start_byte: u64, end_byte: u64) -> Result<(), CoreError> {
    if end_byte <= start_byte {
        return Err(CoreError::new(
            codes::EVIDENCE_INVALID,
            format!("evidence range [{start_byte}, {end_byte}) must be non-empty"),
        ));
    }
    if end_byte > content.len() as u64 {
        return Err(CoreError::new(
            codes::EVIDENCE_INVALID,
            format!(
                "evidence range [{start_byte}, {end_byte}) exceeds the source length {}",
                content.len()
            ),
        ));
    }
    Ok(())
}
