//! 有界子进程运行（计划 §3.5）。
//!
//! stdin 一个 UTF-8 JSON 对象，stdout 只能一个 JSON 对象，调试日志写 stderr。
//! 每次执行使用任务临时 cwd；输出大小、stderr、耗时与并发有硬上限。
//!
//! 进程组 / Job Object 在取消、超时和宿主恢复时清理后代进程。
//! 工作目录与环境变量控制**不等于**操作系统沙箱：本版只运行本机受信任插件，
//! 不声称限制其一切系统调用或强制硬内存上限。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

use crate::contract::collector::{CollectorInput, CollectorOutput, Limits, StageSpec};
use crate::contract::error::{CoreError, Issue, codes};

/// 默认并发上限。
pub const DEFAULT_MAX_CONCURRENCY: usize = 2;

/// 允许透传给子进程的环境变量。
pub const ENV_ALLOWLIST: &[&str] = &[
    "PATH",
    "SystemRoot",
    "SYSTEMROOT",
    "TEMP",
    "TMP",
    "TMPDIR",
    "HOME",
    "USERPROFILE",
    "APPDATA",
    "LOCALAPPDATA",
    "ProgramFiles",
    "PROGRAMFILES",
    "ProgramData",
    "COMSPEC",
    "PATHEXT",
    "WINDIR",
    "LANG",
    "LC_ALL",
    "TZ",
    "NODE_PATH",
    "PYTHONPATH",
    "PYTHONHOME",
    "VIRTUAL_ENV",
];

/// 解析后的运行时。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRuntime {
    /// 可执行文件绝对路径。
    pub executable: PathBuf,
    /// 运行时版本字符串（用于制品哈希绑定）。
    pub version: String,
}

/// 一次运行的请求。
#[derive(Debug, Clone)]
pub struct RunRequest {
    pub runtime: ResolvedRuntime,
    pub entry: PathBuf,
    pub args: Vec<String>,
    pub directory: PathBuf,
    pub input: CollectorInput,
    pub limits: Limits,
    /// 额外允许透传的环境变量（项目显式配置）。
    pub extra_env: BTreeMap<String, String>,
}

/// 一次运行的结果。
#[derive(Debug, Clone)]
pub struct RunOutcome {
    pub output: CollectorOutput,
    pub stdout_bytes: u64,
    pub stderr_bytes: u64,
    pub duration_ms: u64,
}

/// 运行诊断（失败时保留有界信息）。
#[derive(Debug, Clone)]
pub struct RunDiagnostics {
    pub exit_code: Option<i32>,
    pub stdout_bytes: u64,
    pub stderr_bytes: u64,
    pub duration_ms: u64,
    /// 脱敏后的 stderr 片段。
    pub stderr_excerpt: String,
}

/// 子进程运行器：负责并发上限与进程树清理。
#[derive(Debug, Clone)]
pub struct PluginRunner {
    semaphore: Arc<Semaphore>,
}

impl Default for PluginRunner {
    fn default() -> Self {
        PluginRunner::new(DEFAULT_MAX_CONCURRENCY)
    }
}

impl PluginRunner {
    pub fn new(max_concurrency: usize) -> Self {
        PluginRunner {
            semaphore: Arc::new(Semaphore::new(max_concurrency.max(1))),
        }
    }

    /// 运行一个阶段。
    pub async fn run(
        &self,
        request: &RunRequest,
        cancel: &CancellationToken,
    ) -> Result<RunOutcome, CoreError> {
        let _permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|error| {
                CoreError::new(codes::BUSY, format!("cannot acquire runner slot: {error}"))
            })?;

        let input = serde_json::to_vec(&request.input).map_err(|error| {
            CoreError::new(
                codes::SCHEMA_INVALID,
                format!("cannot serialize collector input: {error}"),
            )
        })?;
        if input.len() as u64 > request.limits.max_input_bytes {
            return Err(CoreError::new(
                codes::OUTPUT_LIMIT,
                format!(
                    "collector input is {} bytes, exceeding the limit of {} bytes",
                    input.len(),
                    request.limits.max_input_bytes
                ),
            )
            .with_issues(vec![
                Issue::new(codes::OUTPUT_LIMIT, "/input")
                    .with_actual(input.len().to_string())
                    .with_expected(request.limits.max_input_bytes.to_string())
                    .with_hint("缩小插件作用域或显式提高有上限的配置"),
            ]));
        }

        let started = std::time::Instant::now();
        let mut command = Command::new(&request.runtime.executable);
        command
            // 入口脚本作为第一个参数；manifest 的 args 紧随其后。
            .arg(&request.entry)
            .args(&request.args)
            .current_dir(&request.directory)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .env_clear();
        for (key, value) in environment_for(&request.extra_env) {
            command.env(key, value);
        }
        crate::collect::process::configure_process_tree(&mut command);

        let mut child = command.spawn().map_err(|error| {
            CoreError::new(
                codes::RUNTIME_MISSING,
                format!(
                    "cannot start collector `{}`: {error}",
                    request.runtime.executable.display()
                ),
            )
        })?;
        let mut guard = crate::collect::process::ProcessTreeGuard::attach(&mut child);

        if let Some(mut stdin) = child.stdin.take() {
            // 写入输入并关闭 stdin，插件据此判断输入结束。
            let write_result = stdin.write_all(&input).await;
            let _ = stdin.shutdown().await;
            write_result.map_err(|error| {
                CoreError::new(
                    codes::PLUGIN_EXIT,
                    format!("cannot write collector input: {error}"),
                )
            })?;
        }

        let stdout_limit = request.limits.max_output_bytes;
        let stderr_limit = request.limits.max_stderr_bytes;
        let stdout_task = child
            .stdout
            .take()
            .map(|stdout| tokio::spawn(async move { read_bounded(stdout, stdout_limit).await }));
        let stderr_task = child
            .stderr
            .take()
            .map(|stderr| tokio::spawn(async move { read_bounded(stderr, stderr_limit).await }));

        let status = tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                guard.terminate();
                return Err(CoreError::new(
                    codes::CANCELLED,
                    "collector run was cancelled",
                ));
            }
            status = tokio::time::timeout(
                std::time::Duration::from_millis(request.limits.timeout_ms),
                child.wait(),
            ) => {
                match status {
                    Ok(Ok(status)) => status,
                    Ok(Err(error)) => {
                        guard.terminate();
                        return Err(CoreError::new(
                            codes::PLUGIN_EXIT,
                            format!("collector process failed: {error}"),
                        ));
                    }
                    Err(_) => {
                        guard.terminate();
                        return Err(CoreError::new(
                            codes::PLUGIN_TIMEOUT,
                            format!(
                                "collector exceeded the {} ms timeout",
                                request.limits.timeout_ms
                            ),
                        ));
                    }
                }
            }
        };

        let stdout_bytes = match stdout_task {
            Some(handle) => handle.await.map_err(|error| {
                CoreError::new(
                    codes::PLUGIN_EXIT,
                    format!("collector stdout task failed: {error}"),
                )
            })??,
            None => Vec::new(),
        };
        let stderr_bytes = match stderr_task {
            Some(handle) => handle.await.map_err(|error| {
                CoreError::new(
                    codes::PLUGIN_EXIT,
                    format!("collector stderr task failed: {error}"),
                )
            })??,
            None => Vec::new(),
        };

        let duration_ms = started.elapsed().as_millis() as u64;
        let stderr_excerpt = String::from_utf8_lossy(&stderr_bytes).to_string();

        if !status.success() {
            return Err(CoreError::new(
                codes::PLUGIN_EXIT,
                format!(
                    "collector exited with code {}: {}",
                    status.code().unwrap_or(-1),
                    excerpt(&stderr_excerpt, 2_000)
                ),
            )
            .with_issues(vec![
                Issue::new(codes::PLUGIN_EXIT, "/exitCode")
                    .with_actual(status.code().unwrap_or(-1).to_string())
                    .with_hint("检查插件 stderr 日志"),
            ]));
        }

        let text = String::from_utf8(stdout_bytes.clone()).map_err(|_| {
            CoreError::new(codes::SCHEMA_INVALID, "collector stdout is not valid UTF-8")
        })?;
        let output: CollectorOutput = serde_json::from_str(&text).map_err(|error| {
            CoreError::new(
                codes::SCHEMA_INVALID,
                format!("collector stdout is not a valid CollectorOutput: {error}"),
            )
            .with_issues(vec![
                Issue::new(codes::SCHEMA_INVALID, "/")
                    .with_actual(excerpt(&text, 400))
                    .with_hint("stdout 必须只有一个 CollectorOutput JSON 对象"),
            ])
        })?;

        Ok(RunOutcome {
            output,
            stdout_bytes: stdout_bytes.len() as u64,
            stderr_bytes: stderr_bytes.len() as u64,
            duration_ms,
        })
    }
}

/// 构造允许透传的环境变量。
fn environment_for(extra: &BTreeMap<String, String>) -> Vec<(String, String)> {
    let mut result: Vec<(String, String)> = Vec::new();
    for key in ENV_ALLOWLIST {
        if let Ok(value) = std::env::var(key) {
            result.push(((*key).to_string(), value));
        }
    }
    for (key, value) in extra {
        if !result.iter().any(|(existing, _)| existing == key) {
            result.push((key.clone(), value.clone()));
        }
    }
    result
}

/// 有界读取：超出上限立即返回错误。
async fn read_bounded(
    mut reader: impl AsyncReadExt + Unpin,
    limit: u64,
) -> Result<Vec<u8>, CoreError> {
    let mut buffer: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 8 * 1024];
    loop {
        let read = reader.read(&mut chunk).await.map_err(|error| {
            CoreError::new(
                codes::PLUGIN_EXIT,
                format!("cannot read collector output: {error}"),
            )
        })?;
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);
        if buffer.len() as u64 > limit {
            return Err(CoreError::new(
                codes::OUTPUT_LIMIT,
                format!("collector output exceeded the limit of {limit} bytes"),
            )
            .with_issues(vec![
                Issue::new(codes::OUTPUT_LIMIT, "/")
                    .with_expected(limit.to_string())
                    .with_hint("缩小插件输出或显式提高有上限的配置"),
            ]));
        }
    }
    Ok(buffer)
}

/// 有界文本片段。
pub fn excerpt(text: &str, limit: usize) -> String {
    if text.len() <= limit {
        return text.to_string();
    }
    let mut end = limit;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &text[..end])
}

/// 阶段参数：入口 + manifest 声明的参数。
pub fn stage_command(entry: &Path, stage: &StageSpec) -> (PathBuf, Vec<String>) {
    (entry.to_path_buf(), stage.args.clone())
}
