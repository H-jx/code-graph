//! 证据读取与核验（计划 §3.7）。
//!
//! 证据区间是原文件的 UTF-8 原始字节 `[startByte, endByte)`；行号从 1 开始。
//! 读取时核验磁盘内容：一致返回 `current`，变化返回 `expired`，文件缺失返回
//! `missing`；显式允许时返回保存的历史版本。

use std::path::Path;

use crate::contract::error::{CoreError, codes};
use crate::contract::evidence::{SourceReadStatus, SourceReference};
use crate::contract::identity::content_hash;
use crate::project::snapshot::LineMap;

/// 读取结果。
#[derive(Debug, Clone)]
pub struct SourceRead {
    pub reference: SourceReference,
    pub status: SourceReadStatus,
    /// 当前磁盘内容下的片段；过期或缺失时为空。
    pub snippet: String,
    /// 保存的历史片段（过期时提供）。
    pub stored_snippet: Option<String>,
    /// 显式允许过期读取时返回的历史版本标识（内容哈希）。
    pub stored_version: Option<String>,
    pub line_map: Option<LineMap>,
    pub checked_at: String,
}

/// 读取一段来源。
///
/// `allow_stale` 为 `true` 时，过期来源返回保存的历史片段并标注 `expired`；
/// 默认不把历史片段当作当前内容。
pub fn read_source(
    reference: &SourceReference,
    repository_root: &Path,
    relative_path: &str,
    stored_content: Option<&[u8]>,
    allow_stale: bool,
    checked_at: &str,
) -> Result<SourceRead, CoreError> {
    let absolute = repository_root.join(relative_path);
    let current = std::fs::read(&absolute).ok();

    let (status, snippet, stored_snippet, line_map) = match current {
        Some(bytes) => {
            let disk_hash = content_hash(&bytes);
            if disk_hash == reference.content_hash {
                let line_map = LineMap::build(&bytes);
                (
                    SourceReadStatus::Current,
                    slice_text(&bytes, reference.start_byte, reference.end_byte),
                    None,
                    Some(line_map),
                )
            } else {
                let stored = stored_content
                    .map(|content| slice_text(content, reference.start_byte, reference.end_byte));
                (
                    SourceReadStatus::Expired,
                    String::new(),
                    stored,
                    stored_content.map(LineMap::build),
                )
            }
        }
        None => {
            let stored = stored_content
                .map(|content| slice_text(content, reference.start_byte, reference.end_byte));
            (
                SourceReadStatus::Missing,
                String::new(),
                stored,
                stored_content.map(LineMap::build),
            )
        }
    };

    Ok(SourceRead {
        reference: reference.clone(),
        status,
        snippet,
        stored_snippet,
        stored_version: if allow_stale && status != SourceReadStatus::Current {
            Some(reference.content_hash.clone())
        } else {
            None
        },
        line_map,
        checked_at: checked_at.to_string(),
    })
}

/// 截取字节区间（按 UTF-8 字符边界裁剪）。
pub fn slice_text(bytes: &[u8], start_byte: u64, end_byte: u64) -> String {
    let start = (start_byte as usize).min(bytes.len());
    let end = (end_byte as usize).min(bytes.len());
    if end <= start {
        return String::new();
    }
    let mut slice = &bytes[start..end];
    while let Err(error) = std::str::from_utf8(slice) {
        // 去掉无法解码的边界字节，保证输出是合法 UTF-8。
        match error.error_len() {
            Some(len) => slice = &slice[..slice.len() - len],
            None => {
                let valid = error.valid_up_to();
                slice = &slice[..valid];
                break;
            }
        }
    }
    String::from_utf8_lossy(slice).to_string()
}

/// 校验证据位置：区间非空、在文件内、行号一致。
pub fn validate_position(
    content: &[u8],
    start_byte: u64,
    end_byte: u64,
    start_line: u64,
    end_line: u64,
) -> Result<(), CoreError> {
    crate::project::snapshot::validate_range(content, start_byte, end_byte)?;
    let line_map = LineMap::build(content);
    let expected_start = line_map.line_of(start_byte);
    let expected_end = line_map.end_line_of(start_byte, end_byte);
    if expected_start != start_line || expected_end != end_line {
        return Err(CoreError::new(
            codes::EVIDENCE_INVALID,
            format!(
                "evidence lines {start_line}-{end_line} do not match the byte range (expected {expected_start}-{expected_end})"
            ),
        ));
    }
    Ok(())
}
