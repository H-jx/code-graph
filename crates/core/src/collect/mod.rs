//! 外部提取器宿主：manifest、注册、运行、校验与导入。
//!
//! 脚本使用统一的 JSON stdin/stdout 协议接入；Core 负责 manifest 校验、运行时检查、
//! 依赖准备、子进程限额与结果校验。

pub mod candidate;
pub mod commit;
pub mod deps;
pub mod import;
pub mod manifest;
pub mod process;
pub mod registry;
pub mod runner;
pub mod staging;
pub mod validate;
pub(crate) mod workspace;

pub use commit::{StagedBatch, StagedEntity, StagedRelation, commit_batch};
pub use deps::{
    CacheInputs, DependencyPlan, PreparedDependencies, prepare_dependencies, verify_cache,
};
pub use manifest::{
    ManifestFile, ValidatedManifest, compute_artifact_hash, find_stage, load_manifest,
    stages_of_phase, validate_artifact_path, validate_manifest, validate_stage_dag,
};
pub use runner::{
    DEFAULT_MAX_CONCURRENCY, ENV_ALLOWLIST, PluginRunner, ResolvedRuntime, RunDiagnostics,
    RunOutcome, RunRequest, stage_command,
};
