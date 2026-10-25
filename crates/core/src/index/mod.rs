//! 索引构建与增量更新（计划 §3.6、T05、T10）。
//!
//! - [`build`]：基础索引（扫描 → 语法 → 实体/关系 → 搜索投影）。
//! - [`dependencies`]：来源、集合、配置、制品与候选集合的依赖记录与反向闭包。
//! - [`incremental`]：变化检测、失效与受影响阶段重跑。

pub mod build;
pub mod dependencies;
pub mod incremental;

pub use build::{
    BUSINESS_CAPABILITIES, BaseIndexResult, ScopeInfo, assert_capabilities_declared,
    build_base_index, classify_source, default_scope_id, effective_scopes, select_scope,
};
pub use dependencies::{
    DependencyChange, DependencyKindTag, dependency_closure, record_batch_dependencies,
};
pub use incremental::{IncrementalResult, run_base_only, run_incremental};
