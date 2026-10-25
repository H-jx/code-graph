//! 项目接入与来源管理（计划 §3.2）。
//!
//! - [`config`]：配置解析、校验与身份增量 CAS。
//! - [`scan`]：文件发现、忽略规则与选择集合。
//! - [`snapshot`]：内容哈希、来源快照、行映射与稳定读取。

pub mod config;
pub mod scan;
pub mod snapshot;

pub use config::{
    AgentConfig, BindingConfig, BindingKind, DataSourceConfig, IdentityDelta, IdentityDeltaResult,
    KnowledgeInputConfig, LoadedProject, PluginDiscoveryConfig, ProjectConfig, RepositoryConfig,
    RuntimeConfig, ScopeConfig, ScopeKind, ScopeRoot, SelectionConfig, ServiceConfig,
};
pub use scan::{
    GlobRules, ScanGap, ScanOptions, ScanResult, ScannedSource, SelectionStatus, scan_repository,
    source_set_digest,
};
pub use snapshot::{
    LineMap, SnapshotMember, SourceSnapshot, StableContent, build_snapshot, read_stable,
    snapshot_id, validate_range,
};
