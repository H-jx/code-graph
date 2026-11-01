//! 共用数据与接口契约（计划 §3.3）。
//!
//! 这里定义 Core / CLI / SDK / MCP / 插件共同遵守的协议来源：
//! 公共字段与类型在本模块定义，JSON Schema 由它导出，Node 侧类型再由 Schema 生成。
//!
//! 规则（计划 §3.3）：
//! - 所有 JSON 字段使用 camelCase；Rust 内部可用 snake_case 并统一 serde 映射。
//! - `protocolVersion = "1.0"`；`schemaVersion = 1` 只用于项目配置，二者不得混用。
//! - 主版本不匹配直接拒绝；v1 只接受已支持 minor；未知公共字段拒绝。

pub mod client;
pub mod collector;
pub mod entity;
pub mod error;
pub mod evidence;
pub mod extensions;
pub mod identity;
pub mod knowledge;
pub mod query;
pub mod relation;
pub mod schema;

/// 采集协议版本。
pub const PROTOCOL_VERSION: &str = "1.0";

/// 项目配置 Schema 版本。
pub const CONFIG_SCHEMA_VERSION: u32 = 1;

/// 已支持的协议 minor 版本。
pub const SUPPORTED_PROTOCOL_MINORS: &[u32] = &[0];

/// 检查协议版本是否受支持。
///
/// 主版本不匹配返回 `PROTOCOL_UNSUPPORTED`；已支持的 minor 直接通过。
pub fn check_protocol_version(version: &str) -> Result<(), error::CoreError> {
    let (major, minor) = parse_protocol_version(version).ok_or_else(|| {
        error::CoreError::new(
            error::codes::PROTOCOL_UNSUPPORTED,
            format!("protocol version `{version}` is not a valid `major.minor` string"),
        )
    })?;
    if major != 1 {
        return Err(error::CoreError::new(
            error::codes::PROTOCOL_UNSUPPORTED,
            format!("protocol major version {major} is not supported (expected 1.x)"),
        ));
    }
    if !SUPPORTED_PROTOCOL_MINORS.contains(&minor) {
        return Err(error::CoreError::new(
            error::codes::PROTOCOL_UNSUPPORTED,
            format!("protocol minor version {minor} is not supported"),
        ));
    }
    Ok(())
}

/// 解析 `major.minor` 形式的协议版本。
pub fn parse_protocol_version(version: &str) -> Option<(u32, u32)> {
    let (major, minor) = version.split_once('.')?;
    let major: u32 = major.parse().ok()?;
    let minor: u32 = minor.parse().ok()?;
    Some((major, minor))
}

pub use collector::{
    AcceptanceCase, CollectorInput, CollectorOutput, Coverage, CoverageStatus, Dependency,
    DependencyKind, DependencyManagerKind, DependencySpec, EntityOutput, EntitySelector, Gap,
    Limits, PluginManifest, RelationOutput, RuntimeKind, RuntimeSpec, SourceSelector, StagePhase,
    StageSpec, ValidationReport, ValidationStatus,
};
pub use entity::{Attributes, Basis, Entity, EntityKind, MaybeKnown, Origin, UnknownValue};
pub use error::{CoreError, ErrorBody, ErrorEnvelope, Issue, codes, gap_codes};
pub use evidence::{Evidence, SnapshotRef, Source, SourceClass, SourceReadStatus, SourceReference};
pub use identity::{
    IdentitySlot, IdentityValue, compute_entity_id, compute_evidence_id, compute_id,
    compute_relation_id, compute_source_id, content_hash,
};
pub use query::{
    Budget, BudgetUsed, CodeMapRequest, CommonFilters, Completeness, CoverageLevel, Direction,
    EntityAssertionProjection, EntityProjection, EntityRef, Freshness, FreshnessPolicy,
    Gap as QueryGap, LocateFeatureRequest, NextQuery, PageRequest, QueryRequest, QueryResponse,
    ReadSourceRequest, RelationAssertionProjection, RelationProjection, RelationRef, ReportRequest,
    ResponseStatus, RootFilter, SearchField, SearchRequest, StatusRequest, TraceRequest, Traversal,
    TraversalFilter, WorkflowRequest,
};
pub use relation::{
    ALL_RELATION_KINDS, AggregateSpec, Cardinality, JoinOperator, JoinQualifiers, JoinType,
    KeyPair, QueryImplementationAttributes, Relation, RelationKind, ReportFieldAttributes,
    Resolution, StatementRange, TimeFieldSpec, TransitionQualifiers, UnitSpec, attributes_from,
    attributes_into, endpoint_allowed, endpoint_kinds,
};
