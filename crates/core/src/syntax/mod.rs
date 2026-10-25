//! 可选通用语法能力：Tree-sitter 调度、保守语法事实。
//!
//! 整个模块的 Tree-sitter 实现只在 `feature = "syntax"` 下编译。未启用该
//! feature 时，Core 仍然可以索引与检索文本，只是没有符号、导入与调用事实。

#[cfg(feature = "syntax")]
pub mod parser;
#[cfg(feature = "syntax")]
pub mod symbols;

#[cfg(feature = "syntax")]
pub use parser::{LanguageKind, abi_version, detect_language, parse_source};

#[cfg(not(feature = "syntax"))]
pub use fallback::{LanguageKind, abi_version, detect_language};

/// 未启用 `syntax` feature 时的占位实现：所有语言都视为未知。
#[cfg(not(feature = "syntax"))]
pub mod fallback {
    use std::path::Path;

    /// 占位语言类型。
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum LanguageKind {
        Unknown,
    }

    impl LanguageKind {
        pub fn as_str(&self) -> &'static str {
            "unknown"
        }

        pub fn is_supported(&self) -> bool {
            false
        }
    }

    /// 恒返回未知语言。
    pub fn detect_language(_path: &Path) -> LanguageKind {
        LanguageKind::Unknown
    }

    /// 没有可用语法时 ABI 版本为 `None`。
    pub fn abi_version(_kind: LanguageKind) -> Option<usize> {
        None
    }
}
