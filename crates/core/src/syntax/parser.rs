//! Tree-sitter 调度（计划 D18）。
//!
//! 首版内置 TS/TSX/JS/Python 的基础声明与保守引用；没有解析器的语言仍可被
//! 文本检索，符号、导入与调用事实保持未知。

use std::path::Path;

use tree_sitter::{Language, Parser, Tree};

/// 支持的语言。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageKind {
    TypeScript,
    Tsx,
    JavaScript,
    Python,
    Unknown,
}

impl LanguageKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            LanguageKind::TypeScript => "typescript",
            LanguageKind::Tsx => "tsx",
            LanguageKind::JavaScript => "javascript",
            LanguageKind::Python => "python",
            LanguageKind::Unknown => "unknown",
        }
    }

    /// 是否为受支持语言。
    pub fn is_supported(&self) -> bool {
        !matches!(self, LanguageKind::Unknown)
    }
}

/// 按扩展名判定语言。
pub fn detect_language(path: &Path) -> LanguageKind {
    let extension = path
        .extension()
        .map(|value| value.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    match extension.as_str() {
        "ts" | "mts" | "cts" => LanguageKind::TypeScript,
        "tsx" => LanguageKind::Tsx,
        "js" | "mjs" | "cjs" | "jsx" => LanguageKind::JavaScript,
        "py" | "pyi" => LanguageKind::Python,
        _ => LanguageKind::Unknown,
    }
}

/// 载入语言。
pub fn language(kind: LanguageKind) -> Option<Language> {
    match kind {
        LanguageKind::TypeScript => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
        LanguageKind::Tsx => Some(tree_sitter_typescript::LANGUAGE_TSX.into()),
        LanguageKind::JavaScript => Some(tree_sitter_javascript::LANGUAGE.into()),
        LanguageKind::Python => Some(tree_sitter_python::LANGUAGE.into()),
        LanguageKind::Unknown => None,
    }
}

/// 解析源码。语法不支持或解析失败返回 `None`。
pub fn parse_source(kind: LanguageKind, source: &str) -> Option<Tree> {
    let language = language(kind)?;
    let mut parser = Parser::new();
    parser.set_language(&language).ok()?;
    parser.parse(source, None)
}

/// 语法树的 ABI 版本（用于能力声明）。
pub fn abi_version(kind: LanguageKind) -> Option<usize> {
    language(kind).map(|language| language.abi_version())
}
