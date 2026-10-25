//! 原文件 UTF-8 字节与行范围校验（计划 §3.3）。
//!
//! 行列不参与身份计算；JS 的 UTF-16 offset 不能直接写入，必须先映射到 UTF-8
//! 字节位置。

use crate::contract::error::CoreError;
use crate::project::snapshot::LineMap;

/// 位置转换结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Position {
    pub start_byte: u64,
    pub end_byte: u64,
    pub start_line: u64,
    pub end_line: u64,
}

/// 由 UTF-16 偏移转换为 UTF-8 字节偏移。
///
/// JS/TS 工具链使用 UTF-16 code unit 偏移；协议中的字节位置必须是 UTF-8 字节。
pub fn utf16_offset_to_utf8(text: &str, utf16_offset: usize) -> u64 {
    let mut utf16_units = 0usize;
    for (byte_index, ch) in text.char_indices() {
        if utf16_units >= utf16_offset {
            return byte_index as u64;
        }
        utf16_units += ch.len_utf16();
    }
    text.len() as u64
}

/// 由 UTF-8 字节偏移转换为 UTF-16 偏移。
pub fn utf8_offset_to_utf16(text: &str, utf8_offset: usize) -> usize {
    let mut utf16_units = 0usize;
    for (byte_index, ch) in text.char_indices() {
        if byte_index >= utf8_offset {
            return utf16_units;
        }
        utf16_units += ch.len_utf16();
    }
    utf16_units
}

/// 计算字节区间的行列。
pub fn position_of(content: &[u8], start_byte: u64, end_byte: u64) -> Position {
    let line_map = LineMap::build(content);
    Position {
        start_byte,
        end_byte,
        start_line: line_map.line_of(start_byte),
        end_line: line_map.end_line_of(start_byte, end_byte),
    }
}

/// 拒绝越界或空区间。
pub fn ensure_valid(content: &[u8], start_byte: u64, end_byte: u64) -> Result<(), CoreError> {
    crate::project::snapshot::validate_range(content, start_byte, end_byte)
}
