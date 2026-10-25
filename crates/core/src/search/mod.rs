//! 统一文本与事实搜索（计划 §3.7、D10）。
//!
//! 检索由两部分组成：FTS5 负责候选召回，Rust 负责精确度判定与稳定排序。

pub mod documents;
pub mod rank;
pub mod tokenize;

pub use documents::{
    PROJECTION_ENTITY, PROJECTION_SOURCE, SearchDocument, entity_projection, relation_projection,
    source_projection,
};
pub use rank::{
    EXACT_IDENTIFIER, EXACT_NONE, EXACT_PARTIAL, EXACT_ROUTE, EXACT_SUBSTRING, exact_rank,
    fts_query, match_reason,
};
pub use tokenize::{split_identifier, token_string, tokenize};
