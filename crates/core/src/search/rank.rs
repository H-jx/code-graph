//! 精确匹配、全文结果与入口排序（计划 §3.7）。
//!
//! FTS5 只做候选召回；真正的精确度由 Rust 计算，避免“全文近似”冒充业务命中。
//! 稳定排序键为 `(exactRank, bm25, entityId, assertionId)`。

use crate::search::tokenize::tokenize;

/// 精确度等级。
pub const EXACT_NONE: u32 = 0;
pub const EXACT_PARTIAL: u32 = 1;
pub const EXACT_SUBSTRING: u32 = 2;
pub const EXACT_IDENTIFIER: u32 = 3;
pub const EXACT_ROUTE: u32 = 4;

/// 计算精确度等级。
///
/// - `route`：查询与已登记路由完全一致；
/// - `identifier`：查询与标识符（大小写敏感）完全一致；
/// - `substring`：查询是文档文本的子串（大小写不敏感）；
/// - `partial`：查询的全部 token 都出现在文档中。
pub fn exact_rank(query: &str, text: &str, route_exact: Option<&str>) -> u32 {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return EXACT_NONE;
    }
    if let Some(route) = route_exact
        && route == trimmed
    {
        return EXACT_ROUTE;
    }
    if text
        .split(|c: char| c.is_whitespace())
        .any(|word| word == trimmed)
    {
        return EXACT_IDENTIFIER;
    }
    let lowered_text = text.to_lowercase();
    if lowered_text.contains(&trimmed.to_lowercase()) {
        return EXACT_SUBSTRING;
    }
    let tokens = tokenize(trimmed);
    if tokens.is_empty() {
        return EXACT_NONE;
    }
    let lowered_tokens: Vec<String> = tokens.iter().map(|token| token.to_lowercase()).collect();
    let haystack = lowered_text;
    if lowered_tokens
        .iter()
        .all(|token| haystack.contains(token.as_str()))
    {
        return EXACT_PARTIAL;
    }
    EXACT_NONE
}

/// 命中理由（用于返回给调用方）。
pub fn match_reason(rank: u32) -> &'static str {
    match rank {
        EXACT_ROUTE => "route",
        EXACT_IDENTIFIER => "identifier",
        EXACT_SUBSTRING => "substring",
        EXACT_PARTIAL => "tokens",
        _ => "fulltext",
    }
}

/// 构造 FTS5 MATCH 表达式：所有查询 token 以 OR 连接（召回），精确度在 Rust 侧计算。
pub fn fts_query(query: &str) -> Option<String> {
    let tokens = tokenize(query);
    if tokens.is_empty() {
        return None;
    }
    let mut parts: Vec<String> = Vec::new();
    for token in tokens {
        let cleaned: String = token
            .chars()
            .filter(|c| !matches!(c, '"' | '*' | '(' | ')' | ':' | '^' | '-' | '+'))
            .collect();
        if cleaned.is_empty() {
            continue;
        }
        parts.push(format!("\"{}\"", cleaned));
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" OR "))
    }
}
