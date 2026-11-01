//! 显式准备锁定依赖；采集与激活只读缓存并校验完整文件树，不触发安装。
use super::manifest::ValidatedManifest;
use super::runner::ResolvedRuntime;
use crate::contract::collector::DependencyManagerKind;
use crate::contract::error::{CoreError, codes};
use crate::contract::identity::content_hash;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;

pub const PREPARE_TIMEOUT_MS: u64 = 120_000;
pub const PREPARE_LOG_LIMIT: u64 = 1_048_576;

#[derive(Debug, Clone)]
pub struct DependencyPlan {
    pub kind: DependencyManagerKind,
    pub manifest_path: PathBuf,
    pub lock_path: PathBuf,
    /// Node 或 Python 本身，绝不是任意 shell/npm.cmd。
    pub runtime_executable: PathBuf,
    pub cache_root: PathBuf,
    pub artifact_hash: String,
    pub artifact: Option<ValidatedManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreparedDependencies {
    pub directory: PathBuf,
    pub digest: String,
    pub integrity: String,
    pub env: BTreeMap<String, String>,
    pub executable: PathBuf,
}

#[derive(Debug, Clone)]
pub struct CacheInputs {
    pub os: String,
    pub arch: String,
    pub runtime_version: String,
    pub artifact_hash: String,
    pub lock_digest: String,
}
impl CacheInputs {
    pub fn digest(&self) -> String {
        content_hash(
            serde_json::to_vec(&(
                &self.os,
                &self.arch,
                &self.runtime_version,
                &self.artifact_hash,
                &self.lock_digest,
            ))
            .expect("cache key")
            .as_slice(),
        )
    }
}

#[derive(Serialize, Deserialize)]
struct Seal {
    digest: String,
    integrity: String,
}
struct PreparationLock(std::fs::File);
impl Drop for PreparationLock {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}
fn failed(message: impl Into<String>) -> CoreError {
    CoreError::new(codes::DEPENDENCY_PREPARE_FAILED, message)
}
fn changed(message: impl Into<String>) -> CoreError {
    CoreError::new(codes::DEPENDENCY_CHANGED, message)
}

/// 安装缓存位于宿主用户缓存目录，不在业务项目的 node_modules 祖先树内。
pub fn plugin_plan(
    manifest: &ValidatedManifest,
    runtime: &ResolvedRuntime,
) -> Result<(DependencyPlan, CacheInputs), CoreError> {
    let spec = manifest
        .manifest
        .dependencies
        .as_ref()
        .ok_or_else(|| failed("plugin declares no dependencies"))?;
    let plan = DependencyPlan {
        kind: spec.kind,
        manifest_path: manifest.directory.join(&spec.manifest),
        lock_path: manifest.directory.join(&spec.lock_file),
        runtime_executable: runtime.executable.clone(),
        cache_root: dirs::cache_dir()
            .ok_or_else(|| failed("host cache directory unavailable"))?
            .join("project-knowledge/dependencies"),
        artifact_hash: manifest.artifact_hash.clone(),
        artifact: Some(manifest.clone()),
    };
    let inputs = CacheInputs {
        os: std::env::consts::OS.into(),
        arch: std::env::consts::ARCH.into(),
        runtime_version: super::import::digest(&(spec.kind, &runtime.executable, &runtime.version)),
        artifact_hash: plan.artifact_hash.clone(),
        lock_digest: content_hash(
            &std::fs::read(&plan.lock_path).map_err(|e| failed(e.to_string()))?,
        ),
    };
    Ok((plan, inputs))
}

/// 不联网；缺失或破损都要求显式 prepare。
pub fn prepared_dependencies(
    plan: &DependencyPlan,
    inputs: &CacheInputs,
) -> Result<PreparedDependencies, CoreError> {
    let digest = inputs.digest();
    let directory = super::staging::portable_path(
        std::fs::canonicalize(&plan.cache_root)
            .map_err(|_| failed("dependency cache missing; run plugin prepare explicitly"))?,
    )
    .join(&digest);
    let seal: Seal = serde_json::from_slice(
        &std::fs::read(directory.join(".prepared"))
            .map_err(|_| failed("dependency cache missing; run plugin prepare explicitly"))?,
    )
    .map_err(|_| changed("dependency cache seal is invalid"))?;
    let mut env = BTreeMap::from([
        ("PYTHONUTF8".into(), "1".into()),
        ("PYTHONDONTWRITEBYTECODE".into(), "1".into()),
    ]);
    let executable = match plan.kind {
        DependencyManagerKind::Npm => plan.runtime_executable.clone(),
        DependencyManagerKind::Pip => {
            env.insert(
                "VIRTUAL_ENV".into(),
                directory.join("venv").to_string_lossy().to_string(),
            );
            #[cfg(windows)]
            {
                // Rust/Windows 的 argv0 可能去掉 extended prefix；给导入器一个已验证的长路径。
                let site = std::fs::canonicalize(directory.join("venv/Lib/site-packages"))
                    .map_err(|e| changed(e.to_string()))?;
                env.insert("PYTHONPATH".into(), site.to_string_lossy().to_string());
            }
            venv_python(&directory.join("venv"))
        }
    };
    let prepared = PreparedDependencies {
        directory,
        digest,
        integrity: seal.integrity,
        env,
        executable,
    };
    if seal.digest != prepared.digest {
        return Err(changed("dependency seal has the wrong cache key"));
    }
    verify_cache(&prepared)?;
    Ok(prepared)
}

pub async fn prepare_dependencies(
    plan: &DependencyPlan,
    inputs: &CacheInputs,
) -> Result<PreparedDependencies, CoreError> {
    if plan.artifact_hash != inputs.artifact_hash {
        return Err(failed("artifact hash differs from cache inputs"));
    }
    let digest = inputs.digest();
    std::fs::create_dir_all(&plan.cache_root).map_err(|e| failed(e.to_string()))?;
    // Windows 的 AppData 可能是跨卷 junction；发布前统一到真实父目录。
    let cache_root = super::staging::portable_path(
        std::fs::canonicalize(&plan.cache_root).map_err(|e| failed(e.to_string()))?,
    );
    // OS 锁跨进程互斥。失败安装只留下无 seal 的目录；绝不复用半成品。
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(cache_root.join(format!("{digest}.lock")))
        .map_err(|e| failed(e.to_string()))?;
    lock.try_lock()
        .map_err(|_| CoreError::new(codes::BUSY, "dependency preparation already running"))?;
    let _lock = PreparationLock(lock);
    let directory = cache_root.join(&digest);
    if directory.join(".prepared").exists() {
        return prepared_dependencies(plan, inputs);
    }
    // 不删除计算出的目标。独立临时安装成功后才 rename 到最终摘要目录。
    if directory.exists() {
        return Err(changed(
            "unsealed dependency target exists; inspect it before preparing",
        ));
    }
    let temporary = tempfile::Builder::new()
        .prefix("prepare_")
        .tempdir_in(&cache_root)
        .map_err(|e| failed(e.to_string()))?;
    let work = temporary.path();
    let install = async {
        match plan.kind {
            DependencyManagerKind::Npm => {
                std::fs::copy(&plan.manifest_path, work.join("package.json"))
                    .map_err(|e| failed(e.to_string()))?;
                std::fs::copy(&plan.lock_path, work.join("package-lock.json"))
                    .map_err(|e| failed(e.to_string()))?;
                let cli = npm_cli(&plan.runtime_executable)?;
                run_prepare(
                    &plan.runtime_executable,
                    &[
                        cli.to_string_lossy().into(),
                        "ci".into(),
                        "--ignore-scripts".into(),
                        "--no-audit".into(),
                        "--no-fund".into(),
                        "--registry=https://registry.npmjs.org".into(),
                    ],
                    work,
                )
                .await?;
                // npm 无依赖时可能不建 node_modules；结构保持一致。
                std::fs::create_dir_all(work.join("node_modules"))
                    .map_err(|e| failed(e.to_string()))?;
            }
            DependencyManagerKind::Pip => {
                let lock =
                    std::fs::read_to_string(&plan.lock_path).map_err(|e| failed(e.to_string()))?;
                if !lock.contains("--hash=sha256:") {
                    return Err(failed("Python lock requires pinned SHA-256 hashes"));
                }
                std::fs::write(work.join("requirements.lock"), lock)
                    .map_err(|e| failed(e.to_string()))?;
                run_prepare(
                    &plan.runtime_executable,
                    &[
                        "-m".into(),
                        "venv".into(),
                        work.join("venv").to_string_lossy().into(),
                    ],
                    work,
                )
                .await?;
                run_prepare(
                    &venv_python(&work.join("venv")),
                    &[
                        "-m".into(),
                        "pip".into(),
                        "install".into(),
                        "--require-hashes".into(),
                        "--only-binary=:all:".into(),
                        "--no-input".into(),
                        "--disable-pip-version-check".into(),
                        "--index-url=https://pypi.org/simple".into(),
                        "-r".into(),
                        work.join("requirements.lock").to_string_lossy().into(),
                    ],
                    work,
                )
                .await?;
            }
        }
        if let Some(artifact) = &plan.artifact {
            copy_artifact(artifact, &work.join("artifact"))?;
        }
        Ok::<_, CoreError>(())
    };
    tokio::time::timeout(
        std::time::Duration::from_millis(PREPARE_TIMEOUT_MS),
        install,
    )
    .await
    .map_err(|_| failed("dependency preparation exceeded 120000 ms"))??;
    // venv 在 Windows 可重定位；Unix bin/python 为指向宿主的链接，清单允许该唯一运行时链接。
    let integrity = tree_digest(work)?;
    let seal = Seal { digest, integrity };
    std::fs::write(
        work.join(".prepared"),
        serde_json::to_vec(&seal).expect("seal"),
    )
    .map_err(|e| failed(e.to_string()))?;
    std::fs::rename(work, &directory).map_err(|e| {
        failed(format!(
            "cannot publish dependency cache {} -> {}: {e}",
            work.display(),
            directory.display()
        ))
    })?;
    prepared_dependencies(plan, inputs)
}

pub fn verify_cache(prepared: &PreparedDependencies) -> Result<(), CoreError> {
    let seal: Seal = serde_json::from_slice(
        &std::fs::read(prepared.directory.join(".prepared")).map_err(|e| changed(e.to_string()))?,
    )
    .map_err(|e| changed(e.to_string()))?;
    if seal.digest != prepared.digest
        || seal.integrity != prepared.integrity
        || tree_digest(&prepared.directory)? != prepared.integrity
    {
        return Err(changed(
            "dependency files were changed, removed or added after preparation",
        ));
    }
    Ok(())
}

pub(crate) fn copy_artifact(
    manifest: &ValidatedManifest,
    destination: &Path,
) -> Result<(), CoreError> {
    for file in &manifest.files {
        let bytes = std::fs::read(manifest.directory.join(&file.relative_path))
            .map_err(|e| failed(e.to_string()))?;
        if content_hash(&bytes) != file.content_hash {
            return Err(CoreError::new(
                codes::ARTIFACT_CHANGED,
                "artifact changed while staging runtime",
            ));
        }
        let path = destination.join(&file.relative_path);
        std::fs::create_dir_all(path.parent().expect("artifact parent"))
            .map_err(|e| failed(e.to_string()))?;
        std::fs::write(path, bytes).map_err(|e| failed(e.to_string()))?;
    }
    Ok(())
}

fn tree_digest(root: &Path) -> Result<String, CoreError> {
    fn visit(
        root: &Path,
        directory: &Path,
        rows: &mut BTreeMap<String, String>,
    ) -> Result<(), CoreError> {
        for entry in std::fs::read_dir(directory).map_err(|e| changed(e.to_string()))? {
            let entry = entry.map_err(|e| changed(e.to_string()))?;
            let path = entry.path();
            let relative = path
                .strip_prefix(root)
                .expect("cache child")
                .to_string_lossy()
                .replace('\\', "/");
            if relative == ".prepared" {
                continue;
            }
            let metadata = std::fs::symlink_metadata(&path).map_err(|e| changed(e.to_string()))?;
            if metadata.file_type().is_symlink() {
                let target = std::fs::read_link(&path).map_err(|e| changed(e.to_string()))?;
                let resolved = std::fs::canonicalize(&path).map_err(|e| changed(e.to_string()))?;
                let base = std::fs::canonicalize(root).map_err(|e| changed(e.to_string()))?;
                let runtime_link = matches!(
                    relative.as_str(),
                    "venv/bin/python" | "venv/bin/python3" | "venv/bin/python3.12"
                );
                if !resolved.starts_with(base) && !runtime_link {
                    return Err(changed("dependency link leaves its cache"));
                }
                rows.insert(relative, format!("link:{}", target.to_string_lossy()));
            } else if metadata.is_dir() {
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    if metadata.file_attributes() & 0x400 != 0 {
                        return Err(changed("dependency cache contains a reparse directory"));
                    }
                }
                rows.insert(relative, "directory".into());
                visit(root, &path, rows)?;
            } else if metadata.is_file() {
                rows.insert(
                    relative,
                    content_hash(&std::fs::read(&path).map_err(|e| changed(e.to_string()))?),
                );
            } else {
                return Err(changed("unsupported dependency cache file"));
            }
        }
        Ok(())
    }
    let metadata = std::fs::symlink_metadata(root).map_err(|e| changed(e.to_string()))?;
    if metadata.file_type().is_symlink() {
        return Err(changed("cache root must not be a link"));
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err(changed("cache root must not be a reparse directory"));
        }
    }
    let mut rows = BTreeMap::new();
    visit(root, root, &mut rows)?;
    Ok(super::import::digest(&rows))
}

pub fn venv_python(venv: &Path) -> PathBuf {
    let path = venv.join(if cfg!(windows) {
        "Scripts/python.exe"
    } else {
        "bin/python"
    });
    // Python 以 executable 推导 sys.prefix；保留 Windows extended prefix 才能导入长路径包。
    #[cfg(windows)]
    {
        std::fs::canonicalize(&path).unwrap_or(path)
    }
    #[cfg(not(windows))]
    {
        path
    }
}

fn npm_cli(node: &Path) -> Result<PathBuf, CoreError> {
    let node = if node.is_absolute() {
        node.to_path_buf()
    } else {
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
            .map(|p| p.join(if cfg!(windows) { "node.exe" } else { "node" }))
            .find(|p| p.is_file())
            .ok_or_else(|| CoreError::new(codes::RUNTIME_MISSING, "Node runtime missing"))?
    };
    let parent = node.parent().expect("node parent");
    let mut candidates = vec![
        parent.join("node_modules/npm/bin/npm-cli.js"),
        parent.join("../lib/node_modules/npm/bin/npm-cli.js"),
        parent.join("../share/nodejs/npm/bin/npm-cli.js"),
    ];
    for directory in std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()) {
        if let Ok(path) = std::fs::canonicalize(directory.join("npm")) {
            candidates.push(path);
        }
    }
    candidates
        .into_iter()
        .find(|p| p.is_file() && p.file_name().is_some_and(|n| n == "npm-cli.js"))
        .ok_or_else(|| {
            CoreError::new(
                codes::RUNTIME_MISSING,
                "npm-cli.js for the configured Node runtime is missing",
            )
        })
}

/// 实时限制两个输出流；超限/超时/宿主退出通过 owned Job/process group 清理后代。
async fn run_prepare(
    executable: &Path,
    args: &[String],
    directory: &Path,
) -> Result<(), CoreError> {
    async fn read(
        mut stream: impl AsyncRead + Unpin,
        total: &std::sync::atomic::AtomicU64,
    ) -> Result<Vec<u8>, CoreError> {
        let mut bytes = Vec::new();
        let mut buffer = [0; 8192];
        loop {
            let n = stream
                .read(&mut buffer)
                .await
                .map_err(|e| failed(e.to_string()))?;
            if n == 0 {
                return Ok(bytes);
            }
            if total.fetch_add(n as u64, std::sync::atomic::Ordering::Relaxed) + n as u64
                > PREPARE_LOG_LIMIT
            {
                return Err(failed(
                    "dependency installation output exceeded 1048576 bytes",
                ));
            }
            bytes.extend_from_slice(&buffer[..n]);
        }
    }
    let mut command = Command::new(executable);
    command
        .args(args)
        .current_dir(directory)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .env_clear();
    for key in super::runner::ENV_ALLOWLIST.iter().copied().filter(|k| {
        !matches!(
            *k,
            "NODE_PATH" | "PYTHONPATH" | "PYTHONHOME" | "VIRTUAL_ENV"
        )
    }) {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    command
        .env("PYTHONUTF8", "1")
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .env(
            "PIP_CONFIG_FILE",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .env(
            "NPM_CONFIG_USERCONFIG",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        );
    super::process::configure_process_tree(&mut command);
    let mut child = command
        .spawn()
        .map_err(|e| CoreError::new(codes::RUNTIME_MISSING, e.to_string()))?;
    let mut guard = super::process::ProcessTreeGuard::attach(&mut child);
    let stdout = child.stdout.take().expect("stdout");
    let stderr = child.stderr.take().expect("stderr");
    let total = std::sync::atomic::AtomicU64::new(0);
    let outcome = tokio::try_join!(
        async { child.wait().await.map_err(|e| failed(e.to_string())) },
        read(stdout, &total),
        read(stderr, &total)
    );
    guard.terminate();
    let (status, stdout, stderr) = outcome?;
    let log = format!(
        "{}{}",
        String::from_utf8_lossy(&stdout),
        String::from_utf8_lossy(&stderr)
    );
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(directory.join("prepare.log"))
        .map_err(|e| failed(e.to_string()))?;
    if file.metadata().map_err(|e| failed(e.to_string()))?.len() + log.len() as u64
        > PREPARE_LOG_LIMIT
    {
        return Err(failed("combined dependency log exceeded 1048576 bytes"));
    }
    use std::io::Write;
    writeln!(
        file,
        "executable={} args={} exit={status}\n{log}",
        executable.display(),
        serde_json::to_string(args).expect("args")
    )
    .map_err(|e| failed(e.to_string()))?;
    if !status.success() {
        return Err(failed(format!(
            "locked dependency installation failed: {}",
            super::runner::excerpt(&log, 2000)
        )));
    }
    Ok(())
}
