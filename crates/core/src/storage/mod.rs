//! SQLite 持久化与事务（计划 §3.4）。
//!
//! `storage` 是数据库写入的唯一入口；其他模块通过本模块接口使用事务、一致读
//! 快照与查询，不直接打开连接。

pub mod database;
pub mod entities;
pub mod plugins;
pub mod relations;
pub mod runs;
pub mod search;
pub mod sources;

pub use database::{Database, SCHEMA_VERSION, SqliteCapabilities, backup_to};
pub use entities::{
    EntityAssertionRow, EntityRow, StoredEntityAssertion, assertion_id, entity_payload_hash,
    insert_entity_assertion, insert_evidence, link_assertion_evidence, load_entity_assertions,
    upsert_entity,
};
pub use plugins::{
    activation_status, active_version_digest, active_version_map, record_activation,
    record_validation, register_plugin_version, set_active_plugin,
};
pub use relations::{
    NeighborRow, RelationAssertionRow, StoredRelationAssertion, insert_relation_assertion,
    load_neighbors, load_relation_assertions, upsert_relation,
};
pub use runs::{
    BatchState, RunStatus, base_replacement_key, batch_id, batch_invalidated_generation,
    batch_state, clear_batch_coverage, create_batch, create_run, current_batch, current_batches,
    finish_run, insert_coverage, insert_dependency, insert_gap, invalidate_batches, load_coverage,
    load_dependencies, load_gaps, plugin_replacement_key, set_current_batch,
};
pub use sources::{
    read_current_source, read_source_version, register_snapshot, snapshot_ref, source_path,
    store_source_version, upsert_source,
};
