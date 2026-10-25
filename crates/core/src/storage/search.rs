//! 搜索投影的持久化（计划 §3.4）。
//!
//! `search_documents` 与 FTS5 表在同一事务内更新；失效文档保留在库中但不属于
//! 当前结果（可见性由 `current_batches` 决定）。

use rusqlite::{Connection, params};

use crate::contract::error::{CoreError, codes};
use crate::search::documents::SearchDocument;

/// 写入一行搜索投影并同步 FTS5。
pub fn insert_document(
    connection: &Connection,
    document: &SearchDocument,
) -> Result<(), CoreError> {
    connection
        .execute(
            "INSERT INTO search_documents
               (assertion_id, entity_id, scope_id, route_exact, text, tokens, projection_kind)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(assertion_id, projection_kind) DO UPDATE SET
               entity_id = excluded.entity_id,
               scope_id = excluded.scope_id,
               route_exact = excluded.route_exact,
               text = excluded.text,
               tokens = excluded.tokens",
            params![
                document.assertion_id,
                document.entity_id,
                document.scope_id,
                document.route_exact,
                document.text,
                document.tokens,
                document.projection_kind
            ],
        )
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot insert search document: {error}"),
            )
        })?;
    let rowid = connection
        .query_row(
            "SELECT rowid FROM search_documents WHERE assertion_id = ?1 AND projection_kind = ?2",
            params![document.assertion_id, document.projection_kind],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot read search document row id: {error}"),
            )
        })?;
    // FTS5 虚拟表不支持 UPSERT：先删除再插入，保持 rowid 一致。
    connection
        .execute("DELETE FROM search_fts WHERE rowid = ?1", params![rowid])
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot clear search index row: {error}"),
            )
        })?;
    connection
        .execute(
            "INSERT INTO search_fts (rowid, tokens) VALUES (?1, ?2)",
            params![rowid, document.tokens],
        )
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot update search index: {error}"),
            )
        })?;
    Ok(())
}

/// 搜索命中行。
pub struct SearchHit {
    pub rowid: i64,
    pub assertion_id: String,
    pub entity_id: String,
    pub scope_id: String,
    pub route_exact: Option<String>,
    pub text: String,
    pub projection_kind: String,
    pub bm25: f64,
}

/// FTS5 候选召回 + 当前批次过滤。
///
/// 只有属于当前批次断言的投影才是当前结果；失效批次的历史投影不参与。
pub fn search_documents(
    connection: &Connection,
    match_expression: &str,
    limit: usize,
) -> Result<Vec<SearchHit>, CoreError> {
    let sql = "SELECT d.rowid, d.assertion_id, d.entity_id, d.scope_id, d.route_exact,
                      d.text, d.projection_kind, bm25(search_fts) AS score
                 FROM search_fts
                 JOIN search_documents d ON d.rowid = search_fts.rowid
                WHERE search_fts MATCH ?1
                  AND EXISTS (
                        SELECT 1 FROM assertions a
                          JOIN current_batches cb ON cb.batch_id = a.batch_id
                        JOIN batches b ON b.id = cb.batch_id
                       WHERE a.id = d.assertion_id AND b.state = 'fresh')
                ORDER BY score, d.entity_id, d.assertion_id
                LIMIT ?2";
    let mut statement = connection.prepare(sql).map_err(|error| {
        CoreError::new(
            codes::INTERNAL_ERROR,
            format!("cannot prepare search query: {error}"),
        )
    })?;
    let rows = statement
        .query_map(params![match_expression, limit as i64], |row| {
            Ok(SearchHit {
                rowid: row.get(0)?,
                assertion_id: row.get(1)?,
                entity_id: row.get(2)?,
                scope_id: row.get(3)?,
                route_exact: row.get(4)?,
                text: row.get(5)?,
                projection_kind: row.get(6)?,
                bm25: row.get(7)?,
            })
        })
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot execute search query: {error}"),
            )
        })?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot read search row: {error}"),
            )
        })?);
    }
    Ok(result)
}

/// 精确路由查找（不走全文索引）。
pub fn find_by_route_exact(
    connection: &Connection,
    route: &str,
    scope_ids: Option<&[String]>,
) -> Result<Vec<SearchHit>, CoreError> {
    let sql = "SELECT d.rowid, d.assertion_id, d.entity_id, d.scope_id, d.route_exact,
                      d.text, d.projection_kind, 0.0 AS score
                 FROM search_documents d
                WHERE d.route_exact = ?1
                  AND EXISTS (
                        SELECT 1 FROM assertions a
                          JOIN current_batches cb ON cb.batch_id = a.batch_id
                          JOIN batches b ON b.id = cb.batch_id
                       WHERE a.id = d.assertion_id AND b.state = 'fresh')
                ORDER BY d.entity_id, d.assertion_id";
    let mut statement = connection.prepare(sql).map_err(|error| {
        CoreError::new(
            codes::INTERNAL_ERROR,
            format!("cannot prepare route query: {error}"),
        )
    })?;
    let scope_filter: String = scope_ids
        .map(|scopes| scopes.join("\u{1f}"))
        .unwrap_or_default();
    let rows = statement
        .query_map(params![route, scope_filter], |row| {
            Ok(SearchHit {
                rowid: row.get(0)?,
                assertion_id: row.get(1)?,
                entity_id: row.get(2)?,
                scope_id: row.get(3)?,
                route_exact: row.get(4)?,
                text: row.get(5)?,
                projection_kind: row.get(6)?,
                bm25: row.get(7)?,
            })
        })
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot execute route query: {error}"),
            )
        })?;
    let mut result = Vec::new();
    for row in rows {
        let hit = row.map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot read route row: {error}"),
            )
        })?;
        if !scope_filter.is_empty()
            && !scope_filter
                .split('\u{1f}')
                .any(|scope| scope == hit.scope_id.as_str())
        {
            continue;
        }
        result.push(hit);
    }
    Ok(result)
}
