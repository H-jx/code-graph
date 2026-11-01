//! 文件发现与选择（计划 §3.2）。
//!
//! 顺序固定：先硬排除生成/控制输出，再应用仓库 `.gitignore`/`.ignore`，最后
//! include/exclude（exclude 优先，显式 include 不能覆盖排除）。
//!
//! 不跟随符号链接/重解析点；非 UTF-8、二进制、超限、不可读与路径大小写冲突
//! 分别登记 gap，不静默丢弃。

use std::collections::BTreeMap;
use std::path::Path;

use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};

use crate::contract::error::gap_codes;
use crate::contract::identity::content_hash;

/// 硬排除前缀（相对仓库根，`/` 分隔）。控制输出与生成物不参与采集。
pub const HARD_EXCLUDED_PREFIXES: &[&str] = &[
    ".git/",
    ".project-knowledge/",
    ".agents/skills/",
    "node_modules/",
    "target/",
    "dist/",
];

/// 硬排除的精确文件名。
pub const HARD_EXCLUDED_FILES: &[&str] = &["index.sqlite", "index.sqlite-wal", "index.sqlite-shm"];

/// 选择状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SelectionStatus {
    /// 被选中并读取了内容。
    Selected,
    /// 被规则排除（硬排除、ignore 文件或 exclude glob）。
    Excluded,
    /// 被跳过（符号链接、二进制、非 UTF-8、超限、不可读）。
    Skipped,
}

impl SelectionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SelectionStatus::Selected => "selected",
            SelectionStatus::Excluded => "excluded",
            SelectionStatus::Skipped => "skipped",
        }
    }
}

/// 扫描缺口。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanGap {
    pub code: String,
    pub repository_id: String,
    pub path: String,
    pub reason: String,
}

/// 扫描出的来源。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScannedSource {
    pub repository_id: String,
    /// 仓库根相对路径，统一使用 `/`。
    pub path: String,
    pub content_hash: String,
    pub byte_length: u64,
    pub selection_status: SelectionStatus,
}

/// 单个仓库的扫描结果。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub repository_id: String,
    #[serde(default)]
    pub sources: Vec<ScannedSource>,
    #[serde(default)]
    pub gaps: Vec<ScanGap>,
}

/// 扫描参数。
#[derive(Debug, Clone)]
pub struct ScanOptions {
    pub respect_git_ignore: bool,
    pub max_file_bytes: u64,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}

impl Default for ScanOptions {
    fn default() -> Self {
        ScanOptions {
            respect_git_ignore: true,
            max_file_bytes: crate::project::config::DEFAULT_MAX_FILE_BYTES,
            include: vec!["**/*".to_string()],
            exclude: Vec::new(),
        }
    }
}

/// 编译后的 glob 集合。
pub struct GlobRules {
    include: GlobSet,
    exclude: GlobSet,
}

impl GlobRules {
    /// 编译 include/exclude glob。分隔符固定为 `/`，大小写敏感。
    pub fn compile(include: &[String], exclude: &[String]) -> Result<Self, String> {
        let mut include_builder = GlobSetBuilder::new();
        for pattern in include {
            include_builder.add(compile_glob(pattern)?);
        }
        let mut exclude_builder = GlobSetBuilder::new();
        for pattern in exclude {
            exclude_builder.add(compile_glob(pattern)?);
        }
        Ok(GlobRules {
            include: include_builder
                .build()
                .map_err(|error| format!("invalid include glob set: {error}"))?,
            exclude: exclude_builder
                .build()
                .map_err(|error| format!("invalid exclude glob set: {error}"))?,
        })
    }

    /// 判断路径是否被选中（exclude 优先，include 必须命中）。
    pub fn is_selected(&self, relative_path: &str) -> bool {
        !self.exclude.is_match(relative_path) && self.include.is_match(relative_path)
    }
}

fn compile_glob(pattern: &str) -> Result<Glob, String> {
    Glob::new(pattern).map_err(|error| format!("invalid glob `{pattern}`: {error}"))
}

/// 扫描单个仓库。
pub fn scan_repository(
    repository_id: &str,
    root: &Path,
    options: &ScanOptions,
) -> Result<ScanResult, String> {
    scan_repository_bounded(repository_id, root, options, &mut || true)?
        .ok_or_else(|| "unbounded scan unexpectedly stopped".to_string())
}

/// 查询复用维护扫描规则；停止时不把不完整集合当作新鲜度证明。
pub fn scan_repository_bounded(
    repository_id: &str,
    root: &Path,
    options: &ScanOptions,
    keep_going: &mut impl FnMut() -> bool,
) -> Result<Option<ScanResult>, String> {
    let rules = GlobRules::compile(&options.include, &options.exclude)?;
    let mut result = ScanResult {
        repository_id: repository_id.to_string(),
        sources: Vec::new(),
        gaps: Vec::new(),
    };

    let mut walker = WalkBuilder::new(root);
    // 先关掉全部标准过滤，再逐项显式开启，避免标准预设覆盖本项目的规则。
    walker.standard_filters(false);
    walker
        // 隐藏文件不默认忽略。
        .hidden(false)
        // 禁用仓库外父级 ignore 文件。
        .parents(false)
        // 禁用机器 global ignore。
        .git_global(false)
        // 仓库没有 `.git` 时也按配置读取 ignore 文件。
        .require_git(false)
        .git_ignore(options.respect_git_ignore)
        .git_exclude(options.respect_git_ignore)
        // `.ignore` 文件始终生效。
        .ignore(true)
        .follow_links(false)
        .sort_by_file_path(|left, right| left.cmp(right));
    let control_root = root.to_path_buf();
    walker.filter_entry(move |entry| {
        entry.depth() == 0
            || relative_path(&control_root, entry.path())
                .is_ok_and(|path| !is_control_output(&path))
    });

    // 路径大小写冲突检测：同一仓库内仅大小写不同的路径视为冲突。
    let mut case_index: BTreeMap<String, String> = BTreeMap::new();

    for entry in walker.build() {
        if !keep_going() {
            return Ok(None);
        }
        let entry =
            entry.map_err(|error| format!("cannot walk repository `{repository_id}`: {error}"))?;
        let path = entry.path();
        let file_type = match entry.file_type() {
            Some(file_type) => file_type,
            None => continue,
        };
        if file_type.is_dir() {
            continue;
        }

        let relative = match relative_path(root, path) {
            Ok(relative) => relative,
            Err(reason) => {
                result.gaps.push(ScanGap {
                    code: gap_codes::SOURCE_SKIPPED.to_string(),
                    repository_id: repository_id.to_string(),
                    path: path.to_string_lossy().to_string(),
                    reason,
                });
                continue;
            }
        };

        if file_type.is_symlink() || is_reparse_point(path) {
            result.gaps.push(ScanGap {
                code: gap_codes::SYMLINK_SKIPPED.to_string(),
                repository_id: repository_id.to_string(),
                path: relative,
                reason: "symbolic links and reparse points are not followed".to_string(),
            });
            continue;
        }

        if is_hard_excluded(&relative) {
            result.sources.push(ScannedSource {
                repository_id: repository_id.to_string(),
                path: relative,
                content_hash: String::new(),
                byte_length: 0,
                selection_status: SelectionStatus::Excluded,
            });
            continue;
        }

        let lowercase = relative.to_lowercase();
        if let Some(existing) = case_index.get(&lowercase) {
            if existing != &relative {
                result.gaps.push(ScanGap {
                    code: gap_codes::PATH_CASE_CONFLICT.to_string(),
                    repository_id: repository_id.to_string(),
                    path: relative.clone(),
                    reason: format!("conflicts with `{existing}` which differs only by case"),
                });
                continue;
            }
        } else {
            case_index.insert(lowercase, relative.clone());
        }

        if !rules.is_selected(&relative) {
            result.sources.push(ScannedSource {
                repository_id: repository_id.to_string(),
                path: relative,
                content_hash: String::new(),
                byte_length: 0,
                selection_status: SelectionStatus::Excluded,
            });
            continue;
        }

        let metadata = match std::fs::metadata(path) {
            Ok(metadata) => metadata,
            Err(error) => {
                result.gaps.push(ScanGap {
                    code: gap_codes::UNREADABLE.to_string(),
                    repository_id: repository_id.to_string(),
                    path: relative,
                    reason: format!("cannot read metadata: {error}"),
                });
                continue;
            }
        };

        if metadata.len() > options.max_file_bytes {
            result.gaps.push(ScanGap {
                code: gap_codes::FILE_TOO_LARGE.to_string(),
                repository_id: repository_id.to_string(),
                path: relative,
                reason: format!(
                    "{} bytes exceeds the configured limit of {} bytes",
                    metadata.len(),
                    options.max_file_bytes
                ),
            });
            continue;
        }

        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) => {
                result.gaps.push(ScanGap {
                    code: gap_codes::UNREADABLE.to_string(),
                    repository_id: repository_id.to_string(),
                    path: relative,
                    reason: format!("cannot read file: {error}"),
                });
                continue;
            }
        };

        if is_binary(&bytes) {
            result.gaps.push(ScanGap {
                code: gap_codes::SOURCE_SKIPPED.to_string(),
                repository_id: repository_id.to_string(),
                path: relative,
                reason: "binary content is not indexed as text".to_string(),
            });
            continue;
        }
        if std::str::from_utf8(&bytes).is_err() {
            result.gaps.push(ScanGap {
                code: gap_codes::NOT_UTF8.to_string(),
                repository_id: repository_id.to_string(),
                path: relative,
                reason: "file is not valid UTF-8".to_string(),
            });
            continue;
        }

        result.sources.push(ScannedSource {
            repository_id: repository_id.to_string(),
            path: relative,
            content_hash: content_hash(&bytes),
            byte_length: bytes.len() as u64,
            selection_status: SelectionStatus::Selected,
        });
    }

    // 跳过项也必须进入集合，否则新增不可读/二进制文件不会改变输入摘要。
    for gap in &result.gaps {
        if !result.sources.iter().any(|s| s.path == gap.path) {
            result.sources.push(ScannedSource {
                repository_id: repository_id.to_string(),
                path: gap.path.clone(),
                content_hash: String::new(),
                byte_length: 0,
                selection_status: SelectionStatus::Skipped,
            });
        }
    }
    result.sources.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then(left.selection_status.cmp(&right.selection_status))
    });
    result
        .gaps
        .sort_by(|left, right| left.path.cmp(&right.path).then(left.code.cmp(&right.code)));
    Ok(Some(result))
}

/// 仓库根相对路径，统一 `/` 分隔。
pub fn relative_path(root: &Path, path: &Path) -> Result<String, String> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| format!("path `{}` is outside the repository root", path.display()))?;
    let mut parts: Vec<String> = Vec::new();
    for component in relative.components() {
        let text = component.as_os_str().to_string_lossy().to_string();
        if text.contains('\0') {
            return Err("path must not contain NUL".to_string());
        }
        parts.push(text);
    }
    if parts.is_empty() {
        return Err("path resolves to the repository root".to_string());
    }
    Ok(parts.join("/"))
}

/// 是否命中硬排除。
pub fn is_hard_excluded(relative_path: &str) -> bool {
    let name = relative_path.rsplit('/').next().unwrap_or(relative_path);
    // Credentials are local configuration, never model context or collector input.
    if name == ".env" || (name.starts_with(".env.") && name != ".env.example") {
        return true;
    }
    if HARD_EXCLUDED_FILES
        .iter()
        .any(|name| relative_path == *name || relative_path.ends_with(&format!("/{name}")))
    {
        return true;
    }
    HARD_EXCLUDED_PREFIXES
        .iter()
        .any(|prefix| relative_path.starts_with(prefix))
}

/// Product control output must not change source snapshot membership as it grows.
/// User selection/ignore exclusions still remain tracked below this boundary.
pub fn is_control_output(path: &str) -> bool {
    [".project-knowledge", ".agents/skills"]
        .iter()
        .any(|prefix| path == *prefix || path.starts_with(&format!("{prefix}/")))
        || HARD_EXCLUDED_FILES
            .iter()
            .any(|name| path == *name || path.ends_with(&format!("/{name}")))
}

/// 二进制内容判定：前 8 KiB 内出现 NUL 字节。
pub fn is_binary(bytes: &[u8]) -> bool {
    let window = &bytes[..bytes.len().min(8 * 1024)];
    window.contains(&0)
}

/// Windows 重解析点判定；其他平台恒为 `false`。
pub fn is_reparse_point(path: &Path) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        match std::fs::symlink_metadata(path) {
            Ok(metadata) => metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0,
            Err(_) => false,
        }
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        false
    }
}

/// 计算文件集合摘要：排序后的 `(repositoryId, path, hash, selectionStatus)`。
pub fn source_set_digest(results: &[ScanResult]) -> String {
    let mut rows: Vec<(String, String, String, String)> = Vec::new();
    for result in results {
        for source in &result.sources {
            rows.push((
                source.repository_id.clone(),
                source.path.clone(),
                source.content_hash.clone(),
                source.selection_status.as_str().to_string(),
            ));
        }
    }
    rows.sort();
    let mut canonical = String::new();
    for row in rows {
        canonical.push_str(&row.0);
        canonical.push('\u{1f}');
        canonical.push_str(&row.1);
        canonical.push('\u{1f}');
        canonical.push_str(&row.2);
        canonical.push('\u{1f}');
        canonical.push_str(&row.3);
        canonical.push('\n');
    }
    content_hash(canonical.as_bytes())
}
