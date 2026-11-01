//! 插件 manifest 校验与制品哈希（计划 §3.5）。
//!
//! manifest 声明协议版本、插件 ID、版本、运行时、入口、制品、依赖、阶段 DAG 与替换范围；
//! Core 负责校验路径边界、DAG 与替换范围，并计算制品哈希（manifest 不能自报哈希）。

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use crate::contract::collector::{
    DependencyManagerKind, PluginManifest, RuntimeKind, StagePhase, StageSpec,
};
use crate::contract::error::{CoreError, Issue, codes};
use crate::contract::identity::content_hash;
use crate::project::config::RuntimeConfig;

/// 制品中的一个文件。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ManifestFile {
    pub relative_path: String,
    pub content_hash: String,
    pub byte_length: u64,
}

/// 校验后的 manifest。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ValidatedManifest {
    pub manifest: PluginManifest,
    /// manifest 所在目录（候选或已发布制品目录）。
    pub directory: PathBuf,
    /// 参与哈希的文件（排序后）。
    pub files: Vec<ManifestFile>,
    /// 制品哈希。
    pub artifact_hash: String,
    /// 依赖锁摘要（无依赖时为空串）。
    pub dependency_digest: String,
}

/// 读取并解析 manifest。
pub fn load_manifest(path: &Path) -> Result<PluginManifest, CoreError> {
    let raw = std::fs::read(path).map_err(|error| {
        CoreError::new(
            codes::CONFIG_INVALID,
            format!("cannot read manifest `{}`: {error}", path.display()),
        )
    })?;
    let text = String::from_utf8(raw).map_err(|_| {
        CoreError::new(
            codes::CONFIG_INVALID,
            format!("manifest `{}` is not valid UTF-8", path.display()),
        )
    })?;
    let text = crate::project::config::strip_utf8_bom(&text);
    serde_json::from_str(text).map_err(|error| {
        CoreError::new(
            codes::SCHEMA_INVALID,
            format!("manifest `{}` is invalid: {error}", path.display()),
        )
    })
}

/// 校验 manifest 并计算制品哈希。
///
/// `directory` 是 manifest 所在目录；入口、制品与锁文件必须位于该目录内，
/// 且不得是符号链接。
pub fn validate_manifest(
    manifest: &PluginManifest,
    directory: &Path,
    runtimes: &[RuntimeConfig],
) -> Result<ValidatedManifest, CoreError> {
    let mut issues: Vec<Issue> = Vec::new();

    crate::contract::check_protocol_version(&manifest.protocol_version).map_err(|error| {
        error.with_issues(vec![Issue::new(
            codes::PROTOCOL_UNSUPPORTED,
            "/protocolVersion",
        )])
    })?;

    if manifest.plugin_id.trim().is_empty() {
        issues.push(Issue::new(codes::CONFIG_INVALID, "/pluginId"));
    }
    if manifest.version.trim().is_empty() {
        issues.push(Issue::new(codes::CONFIG_INVALID, "/version"));
    }

    // 运行时：`executable` 只能引用项目显式配置的 runtime ID。
    match manifest.runtime.kind {
        RuntimeKind::Executable => {
            let id = manifest.runtime.executable_id.as_deref();
            match id {
                None => issues.push(Issue::new(codes::CONFIG_INVALID, "/runtime/executableId")),
                Some(id) => {
                    if !runtimes.iter().any(|runtime| runtime.id == id) {
                        issues.push(
                            Issue::new(codes::CONFIG_INVALID, "/runtime/executableId")
                                .with_actual(id.to_string())
                                .with_hint("executable 运行时必须引用项目显式配置的 runtime ID"),
                        );
                    }
                }
            }
        }
        _ => {
            if manifest.runtime.executable_id.is_some() {
                issues.push(
                    Issue::new(codes::CONFIG_INVALID, "/runtime/executableId")
                        .with_hint("只有 executable 运行时才使用 executableId"),
                );
            }
        }
    }

    // 制品与入口路径边界。
    let mut files: BTreeMap<String, ManifestFile> = BTreeMap::new();
    if let Err(reason) = collect_file(directory, "manifest.json", &mut files) {
        issues.push(Issue::new(codes::CONFIG_INVALID, "/artifacts").with_hint(reason));
    }
    for artifact in &manifest.artifacts {
        match validate_artifact_path(artifact) {
            Ok(relative) => {
                if let Err(reason) = collect_file(directory, &relative, &mut files) {
                    issues.push(
                        Issue::new(codes::CONFIG_INVALID, "/artifacts")
                            .with_actual(artifact.clone())
                            .with_hint(reason),
                    );
                }
            }
            Err(reason) => issues.push(
                Issue::new(codes::CONFIG_INVALID, "/artifacts")
                    .with_actual(artifact.clone())
                    .with_hint(reason),
            ),
        }
    }
    for stage in &manifest.stages {
        match validate_artifact_path(&stage.entry) {
            Ok(relative) => {
                if let Err(reason) = collect_file(directory, &relative, &mut files) {
                    issues.push(
                        Issue::new(codes::CONFIG_INVALID, "/stages/entry")
                            .with_actual(stage.entry.clone())
                            .with_hint(reason),
                    );
                }
            }
            Err(reason) => issues.push(
                Issue::new(codes::CONFIG_INVALID, "/stages/entry")
                    .with_actual(stage.entry.clone())
                    .with_hint(reason),
            ),
        }
    }

    // 依赖声明。
    let mut dependency_digest = String::new();
    if let Some(dependencies) = &manifest.dependencies {
        for (path, _kind) in [
            (&dependencies.manifest, dependencies.kind),
            (&dependencies.lock_file, dependencies.kind),
        ] {
            match validate_artifact_path(path) {
                Ok(relative) => {
                    if let Err(reason) = collect_file(directory, &relative, &mut files) {
                        issues.push(
                            Issue::new(codes::CONFIG_INVALID, "/dependencies")
                                .with_actual(path.clone())
                                .with_hint(reason),
                        );
                    }
                }
                Err(reason) => issues.push(
                    Issue::new(codes::CONFIG_INVALID, "/dependencies")
                        .with_actual(path.clone())
                        .with_hint(reason),
                ),
            }
        }
        if dependencies.kind == DependencyManagerKind::Npm
            && !dependencies.lock_file.ends_with("package-lock.json")
        {
            issues.push(
                Issue::new(codes::CONFIG_INVALID, "/dependencies/lockFile")
                    .with_actual(dependencies.lock_file.clone())
                    .with_hint("npm 依赖必须使用 package-lock.json"),
            );
        }
        if dependencies.kind == DependencyManagerKind::Pip
            && !dependencies.lock_file.ends_with(".lock")
            && !dependencies.lock_file.ends_with(".txt")
        {
            issues.push(
                Issue::new(codes::CONFIG_INVALID, "/dependencies/lockFile")
                    .with_actual(dependencies.lock_file.clone())
                    .with_hint("pip 依赖必须使用带哈希的锁定文件"),
            );
        }
        let mut lock_rows: Vec<String> = Vec::new();
        for path in [&dependencies.manifest, &dependencies.lock_file] {
            if let Ok(relative) = validate_artifact_path(path)
                && let Some(file) = files.get(&relative)
            {
                lock_rows.push(format!("{relative}={}", file.content_hash));
            }
        }
        dependency_digest = content_hash(lock_rows.join("\n").as_bytes());
    }

    // 验收用例及其输入也属于不可变制品；修改 expected 不能沿用旧验证。
    for case in &manifest.acceptance_cases {
        let result = (|| {
            let relative = validate_artifact_path(case)?;
            collect_file(directory, &relative, &mut files)?;
            let bytes = std::fs::read(directory.join(&relative)).map_err(|e| e.to_string())?;
            let case: crate::contract::collector::AcceptanceCase =
                serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            let fixture = validate_artifact_path(&case.input_fixture)?;
            collect_file(directory, &fixture, &mut files)?;
            if let Some(truth) = &case.truth_ref {
                let truth = validate_artifact_path(truth)?;
                collect_file(directory, &truth, &mut files)?;
                if case.truth_hash.as_deref() != files.get(&truth).map(|f| f.content_hash.as_str())
                {
                    return Err("truthHash does not match truthRef content".to_string());
                }
            } else if case.truth_hash.is_some() {
                return Err("truthHash requires truthRef".to_string());
            }
            Ok::<_, String>(())
        })();
        if let Err(reason) = result {
            issues.push(
                Issue::new(codes::CONFIG_INVALID, "/acceptanceCases")
                    .with_actual(case.clone())
                    .with_hint(reason),
            );
        }
    }

    // 扩展声明在执行插件前编译；只允许自有命名空间和 bundled schema。
    crate::contract::extensions::ExtensionRegistry::from_manifests([manifest])?;

    // 阶段 DAG。
    if let Err(error) = validate_stage_dag(std::slice::from_ref(manifest)) {
        issues.push(Issue::new(codes::CONFIG_INVALID, "/stages").with_actual(error.message));
    }

    if !issues.is_empty() {
        return Err(
            CoreError::new(codes::CONFIG_INVALID, "plugin manifest is invalid").with_issues(issues),
        );
    }

    let mut file_list: Vec<ManifestFile> = files.into_values().collect();
    file_list.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    let artifact_hash = compute_artifact_hash(&file_list, &dependency_digest);

    Ok(ValidatedManifest {
        manifest: manifest.clone(),
        directory: directory.to_path_buf(),
        files: file_list,
        artifact_hash,
        dependency_digest,
    })
}

/// 制品哈希：排序后的规范路径与逐文件内容哈希，另绑定依赖锁摘要。
pub fn compute_artifact_hash(files: &[ManifestFile], dependency_digest: &str) -> String {
    let mut canonical = String::new();
    for file in files {
        canonical.push_str(&file.relative_path);
        canonical.push('\u{1f}');
        canonical.push_str(&file.content_hash);
        canonical.push('\n');
    }
    canonical.push_str("dependencies=");
    canonical.push_str(dependency_digest);
    content_hash(canonical.as_bytes())
}

/// 校验制品路径：必须是相对路径、不得含 `..`、不得是绝对路径。
pub fn validate_artifact_path(path: &str) -> Result<String, String> {
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
    Ok(path.replace('\\', "/"))
}

/// 读取文件并登记到制品清单（拒绝符号链接）。
fn collect_file(
    directory: &Path,
    relative: &str,
    files: &mut BTreeMap<String, ManifestFile>,
) -> Result<(), String> {
    let absolute = directory.join(relative);
    // 检查每一级目录，不能仅检查最终文件而放过目录链接。
    let mut cursor = directory.to_path_buf();
    for component in Path::new(relative).components() {
        cursor.push(component);
        let metadata = std::fs::symlink_metadata(&cursor)
            .map_err(|e| format!("cannot stat artifact `{relative}`: {e}"))?;
        if metadata.file_type().is_symlink() {
            return Err(format!("`{relative}` contains a symbolic link"));
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if metadata.file_attributes() & 0x400 != 0 {
                return Err(format!("`{relative}` contains a reparse point"));
            }
        }
    }
    let metadata = std::fs::symlink_metadata(&absolute)
        .map_err(|error| format!("cannot stat `{relative}`: {error}"))?;
    if metadata.file_type().is_symlink() {
        return Err(format!("`{relative}` must not be a symbolic link"));
    }
    if !metadata.is_file() {
        return Err(format!("`{relative}` must be a regular file"));
    }
    let bytes =
        std::fs::read(&absolute).map_err(|error| format!("cannot read `{relative}`: {error}"))?;
    files.insert(
        relative.to_string(),
        ManifestFile {
            relative_path: relative.to_string(),
            content_hash: content_hash(&bytes),
            byte_length: bytes.len() as u64,
        },
    );
    Ok(())
}

/// 校验阶段 DAG。
///
/// - 阶段 ID 唯一；
/// - `dependsOn` 形如 `pluginId:stageId`；
/// - 不允许自依赖、环与依赖任意活动版本；
/// - `extract` 只能依赖基础层或 extract 阶段；`link` 可以依赖 extract 与更早的 link。
pub fn validate_stage_dag(manifests: &[PluginManifest]) -> Result<(), CoreError> {
    let mut stage_index: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut phases: BTreeMap<String, StagePhase> = BTreeMap::new();
    let mut plugin_ids: BTreeSet<String> = BTreeSet::new();

    for manifest in manifests {
        plugin_ids.insert(manifest.plugin_id.clone());
        let mut ids: BTreeSet<String> = BTreeSet::new();
        for stage in &manifest.stages {
            if !ids.insert(stage.id.clone()) {
                return Err(CoreError::new(
                    codes::CONFIG_INVALID,
                    format!(
                        "plugin `{}` declares stage `{}` more than once",
                        manifest.plugin_id, stage.id
                    ),
                ));
            }
            phases.insert(format!("{}:{}", manifest.plugin_id, stage.id), stage.phase);
        }
        for stage in &manifest.stages {
            let mut deps: BTreeSet<String> = BTreeSet::new();
            for dependency in &stage.depends_on {
                let Some((plugin_id, stage_id)) = dependency.split_once(':') else {
                    return Err(CoreError::new(
                        codes::CONFIG_INVALID,
                        format!(
                            "plugin `{}` stage `{}` has malformed dependency `{dependency}` (expected `pluginId:stageId`)",
                            manifest.plugin_id, stage.id
                        ),
                    ));
                };
                if plugin_id == manifest.plugin_id && stage_id == stage.id {
                    return Err(CoreError::new(
                        codes::CONFIG_INVALID,
                        format!(
                            "plugin `{}` stage `{}` depends on itself",
                            manifest.plugin_id, stage.id
                        ),
                    ));
                }
                deps.insert(format!("{plugin_id}:{stage_id}"));
            }
            stage_index.insert(format!("{}:{}", manifest.plugin_id, stage.id), deps);
        }
    }

    // 环检测。
    let mut visiting: BTreeSet<String> = BTreeSet::new();
    let mut visited: BTreeSet<String> = BTreeSet::new();
    let keys: Vec<String> = stage_index.keys().cloned().collect();
    for key in keys {
        detect_cycle(&key, &stage_index, &mut visiting, &mut visited)?;
    }

    // 阶段语义：extract 不得依赖 link。
    for (key, deps) in &stage_index {
        let Some(phase) = phases.get(key) else {
            continue;
        };
        if *phase != StagePhase::Extract {
            continue;
        }
        for dependency in deps {
            let Some(dependency_phase) = phases.get(dependency) else {
                continue;
            };
            if *dependency_phase == StagePhase::Link {
                return Err(CoreError::new(
                    codes::CONFIG_INVALID,
                    format!("extract stage `{key}` must not depend on link stage `{dependency}`"),
                ));
            }
        }
    }
    Ok(())
}

fn detect_cycle(
    key: &str,
    index: &BTreeMap<String, BTreeSet<String>>,
    visiting: &mut BTreeSet<String>,
    visited: &mut BTreeSet<String>,
) -> Result<(), CoreError> {
    if visiting.contains(key) {
        return Err(CoreError::new(
            codes::CONFIG_INVALID,
            format!("stage dependency cycle detected at `{key}`"),
        ));
    }
    if visited.contains(key) {
        return Ok(());
    }
    visiting.insert(key.to_string());
    if let Some(deps) = index.get(key) {
        for dependency in deps {
            if index.contains_key(dependency) {
                detect_cycle(dependency, index, visiting, visited)?;
            }
        }
    }
    visiting.remove(key);
    visited.insert(key.to_string());
    Ok(())
}

/// 阶段选择：按 ID 在 manifest 中查找。
pub fn find_stage<'a>(manifest: &'a PluginManifest, stage_id: &str) -> Option<&'a StageSpec> {
    manifest.stages.iter().find(|stage| stage.id == stage_id)
}

/// 按阶段筛选。
pub fn stages_of_phase(manifest: &PluginManifest, phase: StagePhase) -> Vec<&StageSpec> {
    manifest
        .stages
        .iter()
        .filter(|stage| stage.phase == phase)
        .collect()
}
