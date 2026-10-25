//! 项目配置（计划 §3.2）。
//!
//! `project.json` 的 `schemaVersion = 1`。配置里的路径都相对配置文件所在目录
//! 解析；`include`/`exclude` 使用 `/` 分隔、大小写敏感的 glob。
//!
//! 配置校验分两类：*结构错误*（重复 ID、根重叠、未知引用）直接返回
//! `CONFIG_INVALID`；*身份增量*（`identityDelta`）通过 `expectedConfigHash` 做
//! CAS，写入前不覆盖既有 ID。

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::contract::error::{CoreError, Issue, codes};
use crate::contract::evidence::SourceClass;
use crate::contract::identity::{canonical_config_json, content_hash};

/// 默认 `maxFileBytes`。
pub const DEFAULT_MAX_FILE_BYTES: u64 = 2_097_152;

/// 项目配置。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ProjectConfig {
    pub schema_version: u32,
    pub project_id: String,
    pub repositories: Vec<RepositoryConfig>,
    pub selection: SelectionConfig,
    #[serde(default)]
    pub scopes: Vec<ScopeConfig>,
    #[serde(default)]
    pub services: Vec<ServiceConfig>,
    #[serde(default)]
    pub data_sources: Vec<DataSourceConfig>,
    #[serde(default)]
    pub bindings: Vec<BindingConfig>,
    #[serde(default)]
    pub knowledge_inputs: Vec<KnowledgeInputConfig>,
    #[serde(default)]
    pub plugins: PluginDiscoveryConfig,
    #[serde(default)]
    pub collectors: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<AgentConfig>,
    #[serde(default)]
    pub runtimes: Vec<RuntimeConfig>,
}

/// 仓库登记。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RepositoryConfig {
    pub id: String,
    /// 相对配置文件所在目录。
    pub root: String,
}

/// 文件选择规则。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SelectionConfig {
    #[serde(default = "default_include")]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default = "default_true")]
    pub respect_git_ignore: bool,
    #[serde(default = "default_max_file_bytes")]
    pub max_file_bytes: u64,
}

fn default_include() -> Vec<String> {
    vec!["**/*".to_string()]
}

fn default_true() -> bool {
    true
}

fn default_max_file_bytes() -> u64 {
    DEFAULT_MAX_FILE_BYTES
}

/// 作用域。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ScopeConfig {
    pub id: String,
    #[serde(default)]
    pub repository_ids: Vec<String>,
    #[serde(default)]
    pub roots: Vec<ScopeRoot>,
    pub kind: ScopeKind,
}

/// 作用域种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ScopeKind {
    Application,
    Module,
    Service,
}

/// 作用域根。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ScopeRoot {
    pub repository_id: String,
    pub path: String,
}

/// 服务登记。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ServiceConfig {
    pub id: String,
    pub scope_id: String,
    pub protocol: String,
    #[serde(default)]
    pub base_urls: Vec<String>,
}

/// 数据源登记。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DataSourceConfig {
    pub id: String,
    pub scope_id: String,
    pub dialect: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalog: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
}

/// 绑定登记：客户端或代理 → service，或 ORM instance → dataSource。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BindingConfig {
    pub id: String,
    pub kind: BindingKind,
    pub scope_id: String,
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

/// 绑定种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum BindingKind {
    ClientToService,
    ProxyToService,
    OrmInstanceToDataSource,
}

/// 原始知识资料登记。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct KnowledgeInputConfig {
    pub repository_id: String,
    pub path: String,
    pub source_class: SourceClass,
}

/// 插件发现配置。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PluginDiscoveryConfig {
    #[serde(default)]
    pub directories: Vec<String>,
    #[serde(default = "default_true")]
    pub auto_load: bool,
    #[serde(default = "default_true")]
    pub auto_activate: bool,
}

impl Default for PluginDiscoveryConfig {
    fn default() -> Self {
        PluginDiscoveryConfig {
            directories: Vec::new(),
            auto_load: true,
            auto_activate: true,
        }
    }
}

/// 内置 Agent 配置（产品格式，不是 Rig 的直接配置 API）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AgentConfig {
    pub enabled: bool,
    pub provider: String,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(default = "default_api_key_env")]
    pub api_key_env: String,
    #[serde(default = "default_max_turns")]
    pub max_turns: u32,
    #[serde(default = "default_max_tool_calls")]
    pub max_tool_calls: u32,
    #[serde(default = "default_max_repair_attempts")]
    pub max_repair_attempts: u32,
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,
    #[serde(default = "default_max_input_chars")]
    pub max_input_chars: u64,
    #[serde(default = "default_max_output_tokens")]
    pub max_output_tokens: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_activate: Option<bool>,
}

fn default_api_key_env() -> String {
    "PROJECT_KNOWLEDGE_LLM_KEY".to_string()
}

fn default_max_turns() -> u32 {
    24
}

fn default_max_tool_calls() -> u32 {
    80
}

fn default_max_repair_attempts() -> u32 {
    2
}

fn default_timeout_seconds() -> u64 {
    600
}

fn default_max_input_chars() -> u64 {
    80_000
}

fn default_max_output_tokens() -> u64 {
    8_192
}

/// 项目显式配置的运行时。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RuntimeConfig {
    pub id: String,
    pub kind: String,
    /// 绝对可执行文件路径。
    pub executable: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// 加载后的项目：配置 + 解析后的仓库根。
#[derive(Debug, Clone)]
pub struct LoadedProject {
    pub config: ProjectConfig,
    /// 配置文件绝对路径。
    pub config_path: PathBuf,
    /// 配置文件所在目录（相对路径的基准）。
    pub base_dir: PathBuf,
    /// 解析后的仓库（ID → 绝对根路径）。
    pub repositories: BTreeMap<String, PathBuf>,
}

impl LoadedProject {
    /// 读取并校验配置。
    pub fn load(config_path: &Path) -> Result<Self, CoreError> {
        let raw = std::fs::read(config_path).map_err(|error| {
            CoreError::new(
                codes::CONFIG_INVALID,
                format!("cannot read config {}: {error}", config_path.display()),
            )
        })?;
        let text = String::from_utf8(raw).map_err(|_| {
            CoreError::new(
                codes::CONFIG_INVALID,
                format!("config {} is not valid UTF-8", config_path.display()),
            )
        })?;
        let text = strip_utf8_bom(&text);
        let config: ProjectConfig = serde_json::from_str(text).map_err(|error| {
            CoreError::new(
                codes::CONFIG_INVALID,
                format!("config {} is invalid: {error}", config_path.display()),
            )
            .with_issues(vec![
                Issue::new(codes::CONFIG_INVALID, "").with_actual(error.to_string()),
            ])
        })?;
        let base_dir = config_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let repositories = resolve_repositories(&config, &base_dir)?;
        let project = LoadedProject {
            config,
            config_path: config_path.to_path_buf(),
            base_dir,
            repositories,
        };
        project.validate()?;
        Ok(project)
    }

    /// 结构校验：重复 ID、根重叠、未知引用。
    pub fn validate(&self) -> Result<(), CoreError> {
        let mut issues: Vec<Issue> = Vec::new();
        let config = &self.config;

        if config.schema_version != crate::contract::CONFIG_SCHEMA_VERSION {
            issues.push(
                Issue::new(codes::CONFIG_INVALID, "/schemaVersion")
                    .with_expected(crate::contract::CONFIG_SCHEMA_VERSION.to_string())
                    .with_actual(config.schema_version.to_string()),
            );
        }
        if config.project_id.trim().is_empty() {
            issues.push(Issue::new(codes::CONFIG_INVALID, "/projectId"));
        }

        // 仓库 ID 唯一 + 根不重叠。
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        for (index, repository) in config.repositories.iter().enumerate() {
            if !seen.insert(repository.id.as_str()) {
                issues.push(
                    Issue::new(codes::CONFIG_INVALID, &format!("/repositories/{index}/id"))
                        .with_actual(repository.id.clone())
                        .with_hint("repository ids must be unique"),
                );
            }
        }
        let mut roots: Vec<(&str, &PathBuf)> = Vec::new();
        for (index, repository) in config.repositories.iter().enumerate() {
            let Some(root) = self.repositories.get(&repository.id) else {
                continue;
            };
            for (other_id, other_root) in &roots {
                if roots_overlap(root, other_root) {
                    issues.push(
                        Issue::new(codes::CONFIG_INVALID, &format!("/repositories/{index}/root"))
                            .with_actual(repository.id.clone())
                            .with_hint(format!(
                                "repository root overlaps with `{other_id}`; the same physical directory must not be registered twice"
                            )),
                    );
                }
            }
            roots.push((repository.id.as_str(), root));
        }

        // 作用域。
        let mut scope_ids: BTreeSet<&str> = BTreeSet::new();
        for (index, scope) in config.scopes.iter().enumerate() {
            if !scope_ids.insert(scope.id.as_str()) {
                issues.push(
                    Issue::new(codes::CONFIG_INVALID, &format!("/scopes/{index}/id"))
                        .with_actual(scope.id.clone()),
                );
            }
            for (root_index, root) in scope.roots.iter().enumerate() {
                if !self.repositories.contains_key(&root.repository_id) {
                    issues.push(
                        Issue::new(
                            codes::CONFIG_INVALID,
                            &format!("/scopes/{index}/roots/{root_index}/repositoryId"),
                        )
                        .with_actual(root.repository_id.clone())
                        .with_hint("unknown repository id"),
                    );
                }
                if let Err(reason) = validate_relative_path(&root.path) {
                    issues.push(
                        Issue::new(
                            codes::CONFIG_INVALID,
                            &format!("/scopes/{index}/roots/{root_index}/path"),
                        )
                        .with_actual(root.path.clone())
                        .with_hint(reason),
                    );
                }
            }
            for (repo_index, repository_id) in scope.repository_ids.iter().enumerate() {
                if !self.repositories.contains_key(repository_id) {
                    issues.push(
                        Issue::new(
                            codes::CONFIG_INVALID,
                            &format!("/scopes/{index}/repositoryIds/{repo_index}"),
                        )
                        .with_actual(repository_id.clone())
                        .with_hint("unknown repository id"),
                    );
                }
            }
        }

        // 服务与数据源。
        for (index, service) in config.services.iter().enumerate() {
            if !scope_ids.contains(service.scope_id.as_str()) {
                issues.push(
                    Issue::new(codes::CONFIG_INVALID, &format!("/services/{index}/scopeId"))
                        .with_actual(service.scope_id.clone())
                        .with_hint("unknown scope id"),
                );
            }
        }
        for (index, data_source) in config.data_sources.iter().enumerate() {
            if !scope_ids.contains(data_source.scope_id.as_str()) {
                issues.push(
                    Issue::new(
                        codes::CONFIG_INVALID,
                        &format!("/dataSources/{index}/scopeId"),
                    )
                    .with_actual(data_source.scope_id.clone())
                    .with_hint("unknown scope id"),
                );
            }
        }
        for (index, binding) in config.bindings.iter().enumerate() {
            if !scope_ids.contains(binding.scope_id.as_str()) {
                issues.push(
                    Issue::new(codes::CONFIG_INVALID, &format!("/bindings/{index}/scopeId"))
                        .with_actual(binding.scope_id.clone())
                        .with_hint("unknown scope id"),
                );
            }
            let target_ok = match binding.kind {
                BindingKind::OrmInstanceToDataSource => config
                    .data_sources
                    .iter()
                    .any(|source| source.id == binding.to),
                _ => config
                    .services
                    .iter()
                    .any(|service| service.id == binding.to),
            };
            if !target_ok {
                issues.push(
                    Issue::new(codes::CONFIG_INVALID, &format!("/bindings/{index}/to"))
                        .with_actual(binding.to.clone())
                        .with_hint("binding target does not exist"),
                );
            }
        }
        for (index, input) in config.knowledge_inputs.iter().enumerate() {
            if !self.repositories.contains_key(&input.repository_id) {
                issues.push(
                    Issue::new(
                        codes::CONFIG_INVALID,
                        &format!("/knowledgeInputs/{index}/repositoryId"),
                    )
                    .with_actual(input.repository_id.clone())
                    .with_hint("unknown repository id"),
                );
            }
            if let Err(reason) = validate_relative_path(&input.path) {
                issues.push(
                    Issue::new(
                        codes::CONFIG_INVALID,
                        &format!("/knowledgeInputs/{index}/path"),
                    )
                    .with_actual(input.path.clone())
                    .with_hint(reason),
                );
            }
        }
        for (index, runtime) in config.runtimes.iter().enumerate() {
            if runtime.executable.is_empty() {
                issues.push(Issue::new(
                    codes::CONFIG_INVALID,
                    &format!("/runtimes/{index}/executable"),
                ));
            }
        }

        if issues.is_empty() {
            Ok(())
        } else {
            Err(CoreError::new(
                codes::CONFIG_INVALID,
                format!("project config has {} invalid field(s)", issues.len()),
            )
            .with_issues(issues))
        }
    }

    /// 配置哈希：规范 JSON 的 SHA-256，用于 CAS 与依赖记录。
    pub fn config_hash(&self) -> Result<String, CoreError> {
        let value = serde_json::to_value(&self.config).map_err(|error| {
            CoreError::new(
                codes::CONFIG_INVALID,
                format!("cannot serialize config: {error}"),
            )
        })?;
        let canonical = canonical_config_json(&value).map_err(|error| {
            CoreError::new(
                codes::CONFIG_INVALID,
                format!("cannot canonicalize config: {error}"),
            )
        })?;
        Ok(content_hash(canonical.as_bytes()))
    }

    /// 收集器 manifest 的绝对路径。
    pub fn collector_manifest_paths(&self) -> Vec<PathBuf> {
        self.config
            .collectors
            .iter()
            .map(|path| self.base_dir.join(path))
            .collect()
    }

    /// 插件目录的绝对路径。
    pub fn plugin_directories(&self) -> Vec<PathBuf> {
        self.config
            .plugins
            .directories
            .iter()
            .map(|path| self.base_dir.join(path))
            .collect()
    }
}

/// 解析仓库根路径并检测重叠。
fn resolve_repositories(
    config: &ProjectConfig,
    base_dir: &Path,
) -> Result<BTreeMap<String, PathBuf>, CoreError> {
    let mut repositories = BTreeMap::new();
    for repository in &config.repositories {
        if let Err(reason) = validate_root_path(&repository.root) {
            return Err(CoreError::new(
                codes::CONFIG_INVALID,
                format!("repository `{}` root is invalid: {reason}", repository.id),
            ));
        }
        let root = normalize_path(&base_dir.join(&repository.root));
        if !root.is_dir() {
            return Err(CoreError::new(
                codes::CONFIG_INVALID,
                format!(
                    "repository `{}` root `{}` does not exist",
                    repository.id,
                    root.display()
                ),
            ));
        }
        repositories.insert(repository.id.clone(), root);
    }
    Ok(repositories)
}

/// 规范化路径（不要求存在）。
pub fn normalize_path(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            other => result.push(other.as_os_str()),
        }
    }
    result
}

/// 校验仓库根路径：禁止绝对路径与 NUL；允许 `..`（设计 §6.1 的 `root: ".."`）。
///
/// 仓库根相对配置文件所在目录解析，`..` 指向目标仓库根目录，属于正常用法；
/// 越界解析只针对仓库内部的相对路径（见 [`validate_relative_path`]）。
pub fn validate_root_path(path: &str) -> Result<(), String> {
    if path.is_empty() {
        return Err("path must not be empty".to_string());
    }
    if path.contains('\0') {
        return Err("path must not contain NUL".to_string());
    }
    let candidate = Path::new(path);
    if candidate.is_absolute() {
        return Err("path must be relative".to_string());
    }
    for component in candidate.components() {
        if let Component::Prefix(_) = component {
            return Err("path must be relative".to_string());
        }
    }
    Ok(())
}

/// 校验相对路径：禁止绝对路径、`..`、NUL 与越界。
pub fn validate_relative_path(path: &str) -> Result<(), String> {
    if path.is_empty() {
        return Err("path must not be empty".to_string());
    }
    if path.contains('\0') {
        return Err("path must not contain NUL".to_string());
    }
    let candidate = Path::new(path);
    if candidate.is_absolute() {
        return Err("path must be relative".to_string());
    }
    for component in candidate.components() {
        match component {
            Component::ParentDir => return Err("path must not contain `..`".to_string()),
            Component::RootDir | Component::Prefix(_) => {
                return Err("path must be relative".to_string());
            }
            _ => {}
        }
    }
    Ok(())
}

/// 两个根是否重叠（含相等）。
fn roots_overlap(left: &Path, right: &Path) -> bool {
    if left == right {
        return true;
    }
    left.starts_with(right) || right.starts_with(left)
}

/// 身份增量：只允许新增或显式修改，不允许按名字覆盖既有 ID。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct IdentityDelta {
    #[serde(default)]
    pub scopes: Vec<ScopeConfig>,
    #[serde(default)]
    pub services: Vec<ServiceConfig>,
    #[serde(default)]
    pub data_sources: Vec<DataSourceConfig>,
    #[serde(default)]
    pub bindings: Vec<BindingConfig>,
}

/// 身份增量的应用结果。
#[derive(Debug, Clone, PartialEq)]
pub struct IdentityDeltaResult {
    pub config: ProjectConfig,
    pub previous_config_hash: String,
    pub config_hash: String,
}

/// 用 `expectedConfigHash` 做 CAS 地应用身份增量。
pub fn apply_identity_delta(
    config: &ProjectConfig,
    delta: &IdentityDelta,
    expected_config_hash: &str,
) -> Result<IdentityDeltaResult, CoreError> {
    let current_hash = {
        let value = serde_json::to_value(config).map_err(|error| {
            CoreError::new(
                codes::CONFIG_INVALID,
                format!("cannot serialize config: {error}"),
            )
        })?;
        let canonical = canonical_config_json(&value).map_err(|error| {
            CoreError::new(
                codes::CONFIG_INVALID,
                format!("cannot canonicalize config: {error}"),
            )
        })?;
        content_hash(canonical.as_bytes())
    };
    if current_hash != expected_config_hash {
        return Err(CoreError::new(
            codes::ACTIVATION_CONFLICT,
            format!(
                "config hash mismatch: expected {expected_config_hash}, current {current_hash}"
            ),
        ));
    }

    let mut issues: Vec<Issue> = Vec::new();
    let mut next = config.clone();

    for (index, scope) in delta.scopes.iter().enumerate() {
        match next
            .scopes
            .iter()
            .position(|existing| existing.id == scope.id)
        {
            Some(position) => {
                // 显式修改：保留既有 ID，只更新明确给出的字段。
                next.scopes[position].kind = scope.kind;
                next.scopes[position].repository_ids = scope.repository_ids.clone();
                next.scopes[position].roots = scope.roots.clone();
            }
            None => {
                if scope.id.trim().is_empty() {
                    issues.push(Issue::new(
                        codes::CONFIG_INVALID,
                        &format!("/identityDelta/scopes/{index}/id"),
                    ));
                    continue;
                }
                next.scopes.push(scope.clone());
            }
        }
    }
    for (index, service) in delta.services.iter().enumerate() {
        match next
            .services
            .iter()
            .position(|existing| existing.id == service.id)
        {
            Some(position) => {
                next.services[position].scope_id = service.scope_id.clone();
                next.services[position].protocol = service.protocol.clone();
                next.services[position].base_urls = service.base_urls.clone();
            }
            None => {
                if service.id.trim().is_empty() {
                    issues.push(Issue::new(
                        codes::CONFIG_INVALID,
                        &format!("/identityDelta/services/{index}/id"),
                    ));
                    continue;
                }
                next.services.push(service.clone());
            }
        }
    }
    for (index, data_source) in delta.data_sources.iter().enumerate() {
        match next
            .data_sources
            .iter()
            .position(|existing| existing.id == data_source.id)
        {
            Some(position) => {
                next.data_sources[position].scope_id = data_source.scope_id.clone();
                next.data_sources[position].dialect = data_source.dialect.clone();
                next.data_sources[position].catalog = data_source.catalog.clone();
                next.data_sources[position].schema = data_source.schema.clone();
            }
            None => {
                if data_source.id.trim().is_empty() {
                    issues.push(Issue::new(
                        codes::CONFIG_INVALID,
                        &format!("/identityDelta/dataSources/{index}/id"),
                    ));
                    continue;
                }
                next.data_sources.push(data_source.clone());
            }
        }
    }
    for (index, binding) in delta.bindings.iter().enumerate() {
        if next
            .bindings
            .iter()
            .any(|existing| existing.id == binding.id)
        {
            next.bindings.retain(|existing| existing.id != binding.id);
        }
        if binding.id.trim().is_empty() {
            issues.push(Issue::new(
                codes::CONFIG_INVALID,
                &format!("/identityDelta/bindings/{index}/id"),
            ));
            continue;
        }
        next.bindings.push(binding.clone());
    }

    if !issues.is_empty() {
        return Err(
            CoreError::new(codes::CONFIG_INVALID, "identity delta is invalid").with_issues(issues),
        );
    }

    let next_hash = {
        let value = serde_json::to_value(&next).map_err(|error| {
            CoreError::new(
                codes::CONFIG_INVALID,
                format!("cannot serialize config: {error}"),
            )
        })?;
        let canonical = canonical_config_json(&value).map_err(|error| {
            CoreError::new(
                codes::CONFIG_INVALID,
                format!("cannot canonicalize config: {error}"),
            )
        })?;
        content_hash(canonical.as_bytes())
    };

    Ok(IdentityDeltaResult {
        config: next,
        previous_config_hash: current_hash,
        config_hash: next_hash,
    })
}

/// 去掉 UTF-8 BOM（Windows 工具写出的 JSON 常带 BOM，解析前必须处理）。
pub fn strip_utf8_bom(text: &str) -> &str {
    text.strip_prefix('\u{feff}').unwrap_or(text)
}

/// 便于从 JSON 值读取配置（CLI 使用）。
pub fn config_from_value(value: &Value) -> Result<ProjectConfig, CoreError> {
    serde_json::from_value(value.clone()).map_err(|error| {
        CoreError::new(
            codes::CONFIG_INVALID,
            format!("invalid project config: {error}"),
        )
    })
}
