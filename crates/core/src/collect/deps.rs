//! 依赖准备（计划 §3.5、D16）。
//!
//! 采集阶段只用**已准备好的依赖**，不在 extract/link 期间隐式安装，也不继承业务
//! 项目的 node_modules。准备过程有独立的 120 秒超时与日志上限；缓存摘要包含
//! OS/arch、运行时、制品与锁文件；运行前校验缓存完整性。
//!
//! - Node：`npm ci --ignore-scripts`（锁文件必须是 package-lock.json）。
//! - Python：独立 venv + `pip install --require-hashes`，只允许带哈希的 wheel，
//!   缺少 wheel 直接报错，不回退到源码构建。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use tokio::process::Command;

use crate::contract::collector::DependencyManagerKind;
use crate::contract::error::{CoreError, Issue, codes};
use crate::contract::identity::content_hash;

/// 依赖准备的超时。
pub const PREPARE_TIMEOUT_MS: u64 = 120_000;
/// 依赖准备的日志上限。
pub const PREPARE_LOG_LIMIT: u64 = 1_048_576;

/// 依赖准备请求。
#[derive(Debug, Clone)]
pub struct DependencyPlan {
    pub kind: DependencyManagerKind,
    /// 依赖声明文件（package.json / requirements.in）。
    pub manifest_path: PathBuf,
    /// 锁文件（package-lock.json / requirements.lock）。
    pub lock_path: PathBuf,
    /// 运行时可执行文件（node / python）。
    pub runtime_executable: PathBuf,
    /// 依赖缓存根目录（与制品目录分离）。
    pub cache_root: PathBuf,
    /// 参与缓存摘要的制品哈希。
    pub artifact_hash: String,
}

/// 已准备好的依赖。
#[derive(Debug, Clone)]
pub struct PreparedDependencies {
    /// 依赖目录（缓存中的 `<digest>` 目录）。
    pub directory: PathBuf,
    /// 缓存摘要。
    pub digest: String,
    /// 需要透传给插件的环境变量。
    pub env: BTreeMap<String, String>,
}

/// 缓存摘要输入。
#[derive(Debug, Clone)]
pub struct CacheInputs {
    pub os: String,
    pub arch: String,
    pub runtime_version: String,
    pub artifact_hash: String,
    pub lock_digest: String,
}

impl CacheInputs {
    /// 计算缓存摘要。
    pub fn digest(&self) -> String {
        let canonical = format!(
            "{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}",
            self.os, self.arch, self.runtime_version, self.artifact_hash, self.lock_digest
        );
        content_hash(canonical.as_bytes())
    }
}

/// 准备依赖。
pub async fn prepare_dependencies(
    plan: &DependencyPlan,
    inputs: &CacheInputs,
) -> Result<PreparedDependencies, CoreError> {
    let digest = inputs.digest();
    let directory = plan.cache_root.join(&digest);
    let marker = directory.join(".prepared");
    if marker.exists() {
        // 缓存完整：直接复用，不联网。
        return Ok(PreparedDependencies {
            directory: directory.clone(),
            digest,
            env: environment_for(plan, &directory),
        });
    }

    std::fs::create_dir_all(&directory).map_err(|error| {
        CoreError::new(
            codes::DEPENDENCY_PREPARE_FAILED,
            format!("cannot create dependency cache: {error}"),
        )
    })?;

    match plan.kind {
        DependencyManagerKind::Npm => prepare_npm(plan, &directory).await?,
        DependencyManagerKind::Pip => prepare_pip(plan, &directory).await?,
    }

    std::fs::write(&marker, format!("{digest}\n")).map_err(|error| {
        CoreError::new(
            codes::DEPENDENCY_PREPARE_FAILED,
            format!("cannot mark dependency cache: {error}"),
        )
    })?;

    Ok(PreparedDependencies {
        directory: directory.clone(),
        digest,
        env: environment_for(plan, &directory),
    })
}

/// 校验缓存完整性。
pub fn verify_cache(prepared: &PreparedDependencies) -> Result<(), CoreError> {
    let marker = prepared.directory.join(".prepared");
    let content = std::fs::read_to_string(&marker).map_err(|error| {
        CoreError::new(
            codes::DEPENDENCY_CHANGED,
            format!("dependency cache is incomplete: {error}"),
        )
    })?;
    if content.trim() != prepared.digest {
        return Err(CoreError::new(
            codes::DEPENDENCY_CHANGED,
            "dependency cache digest does not match the prepared digest",
        ));
    }
    Ok(())
}

/// Node 依赖准备：`npm ci --ignore-scripts`。
async fn prepare_npm(plan: &DependencyPlan, directory: &Path) -> Result<(), CoreError> {
    let manifest_name = plan
        .manifest_path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "package.json".to_string());
    let lock_name = plan
        .lock_path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "package-lock.json".to_string());
    let manifest_bytes = std::fs::read(&plan.manifest_path).map_err(|error| {
        CoreError::new(
            codes::DEPENDENCY_PREPARE_FAILED,
            format!("cannot read dependency manifest: {error}"),
        )
    })?;
    let lock_bytes = std::fs::read(&plan.lock_path).map_err(|error| {
        CoreError::new(
            codes::DEPENDENCY_PREPARE_FAILED,
            format!("cannot read dependency lock file: {error}"),
        )
    })?;
    std::fs::write(directory.join(&manifest_name), manifest_bytes).map_err(|error| {
        CoreError::new(
            codes::DEPENDENCY_PREPARE_FAILED,
            format!("cannot stage dependency manifest: {error}"),
        )
    })?;
    std::fs::write(directory.join(&lock_name), lock_bytes).map_err(|error| {
        CoreError::new(
            codes::DEPENDENCY_PREPARE_FAILED,
            format!("cannot stage dependency lock file: {error}"),
        )
    })?;

    let output = run_prepare(
        &plan.runtime_executable,
        &[
            "ci".to_string(),
            "--ignore-scripts".to_string(),
            "--no-audit".to_string(),
            "--no-fund".to_string(),
            "--prefix".to_string(),
            directory.to_string_lossy().to_string(),
        ],
        directory,
    )
    .await?;
    if !output.success {
        return Err(CoreError::new(
            codes::DEPENDENCY_PREPARE_FAILED,
            format!(
                "npm ci failed: {}",
                crate::collect::runner::excerpt(&output.log, 2_000)
            ),
        )
        .with_issues(vec![
            Issue::new(codes::DEPENDENCY_PREPARE_FAILED, "/dependencies")
                .with_actual(crate::collect::runner::excerpt(&output.log, 400))
                .with_hint("检查锁文件与依赖声明是否一致"),
        ]));
    }
    Ok(())
}

/// Python 依赖准备：独立 venv + 带哈希的 wheel 安装。
async fn prepare_pip(plan: &DependencyPlan, directory: &Path) -> Result<(), CoreError> {
    let venv = directory.join("venv");
    let output = run_prepare(
        &plan.runtime_executable,
        &[
            "-m".to_string(),
            "venv".to_string(),
            venv.to_string_lossy().to_string(),
        ],
        directory,
    )
    .await?;
    if !output.success {
        return Err(CoreError::new(
            codes::DEPENDENCY_PREPARE_FAILED,
            format!(
                "python venv creation failed: {}",
                crate::collect::runner::excerpt(&output.log, 2_000)
            ),
        ));
    }
    let python = venv_python(&venv);
    let lock_text = std::fs::read_to_string(&plan.lock_path).map_err(|error| {
        CoreError::new(
            codes::DEPENDENCY_PREPARE_FAILED,
            format!("cannot read dependency lock file: {error}"),
        )
    })?;
    if !lock_text.contains("--hash=") {
        return Err(CoreError::new(
            codes::DEPENDENCY_PREPARE_FAILED,
            "python dependency lock file must pin hashes (`--hash=sha256:...`)",
        )
        .with_issues(vec![
            Issue::new(codes::DEPENDENCY_PREPARE_FAILED, "/dependencies/lockFile").with_hint(
                "使用 `pip download --no-binary :none: --dest wheels` 生成带哈希的锁定文件",
            ),
        ]));
    }
    std::fs::write(directory.join("requirements.lock"), &lock_text).map_err(|error| {
        CoreError::new(
            codes::DEPENDENCY_PREPARE_FAILED,
            format!("cannot stage dependency lock file: {error}"),
        )
    })?;

    let output = run_prepare(
        &python,
        &[
            "-m".to_string(),
            "pip".to_string(),
            "install".to_string(),
            "--require-hashes".to_string(),
            "--only-binary=:all:".to_string(),
            "--no-input".to_string(),
            "-r".to_string(),
            directory
                .join("requirements.lock")
                .to_string_lossy()
                .to_string(),
        ],
        directory,
    )
    .await?;
    if !output.success {
        return Err(CoreError::new(
            codes::DEPENDENCY_PREPARE_FAILED,
            format!(
                "pip install failed: {}",
                crate::collect::runner::excerpt(&output.log, 2_000)
            ),
        )
        .with_issues(vec![
            Issue::new(codes::DEPENDENCY_PREPARE_FAILED, "/dependencies/lockFile")
                .with_actual(crate::collect::runner::excerpt(&output.log, 400))
                .with_hint("只允许带哈希的可用 wheel；缺少 wheel 时报错，不回退源码构建"),
        ]));
    }
    Ok(())
}

/// venv 内的 Python 可执行文件。
pub fn venv_python(venv: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        venv.join("Scripts").join("python.exe")
    }
    #[cfg(not(windows))]
    {
        venv.join("bin").join("python3")
    }
}

/// 依赖准备返回的结果。
struct PrepareOutput {
    success: bool,
    log: String,
}

/// 执行一条准备命令（有独立超时与日志上限）。
async fn run_prepare(
    executable: &Path,
    args: &[String],
    working_dir: &Path,
) -> Result<PrepareOutput, CoreError> {
    let mut command = Command::new(executable);
    command
        .args(args)
        .current_dir(working_dir)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    let child = command.spawn().map_err(|error| {
        CoreError::new(
            codes::RUNTIME_MISSING,
            format!(
                "cannot start dependency preparation `{}`: {error}",
                executable.display()
            ),
        )
    })?;
    let output = tokio::time::timeout(
        std::time::Duration::from_millis(PREPARE_TIMEOUT_MS),
        child.wait_with_output(),
    )
    .await
    .map_err(|_| {
        CoreError::new(
            codes::DEPENDENCY_PREPARE_FAILED,
            format!("dependency preparation exceeded the {PREPARE_TIMEOUT_MS} ms timeout"),
        )
    })?
    .map_err(|error| {
        CoreError::new(
            codes::DEPENDENCY_PREPARE_FAILED,
            format!("dependency preparation failed: {error}"),
        )
    })?;
    let mut log = String::from_utf8_lossy(&output.stdout).to_string();
    log.push_str(&String::from_utf8_lossy(&output.stderr));
    if log.len() as u64 > PREPARE_LOG_LIMIT {
        log = crate::collect::runner::excerpt(&log, PREPARE_LOG_LIMIT as usize);
    }
    Ok(PrepareOutput {
        success: output.status.success(),
        log,
    })
}

/// 依赖环境变量。
fn environment_for(plan: &DependencyPlan, directory: &Path) -> BTreeMap<String, String> {
    let mut env = BTreeMap::new();
    match plan.kind {
        DependencyManagerKind::Npm => {
            env.insert(
                "NODE_PATH".to_string(),
                directory.join("node_modules").to_string_lossy().to_string(),
            );
        }
        DependencyManagerKind::Pip => {
            let venv = directory.join("venv");
            let bin = if cfg!(windows) {
                venv.join("Scripts")
            } else {
                venv.join("bin")
            };
            env.insert(
                "VIRTUAL_ENV".to_string(),
                venv.to_string_lossy().to_string(),
            );
            env.insert(
                "PATH".to_string(),
                format!(
                    "{};{}",
                    bin.to_string_lossy(),
                    std::env::var("PATH").unwrap_or_default()
                ),
            );
        }
    }
    env
}
