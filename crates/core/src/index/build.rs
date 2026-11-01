//! 基础索引构建（计划 T05）。
//!
//! 顺序：扫描 → 来源快照 → 语法事实 → 实体/关系 → 搜索投影 → 事务提交。
//!
//! 基础层不调用模型、不连接业务数据库，也不需要 Node/Python 插件运行时；
//! 未采集的业务能力一律登记为 `not_configured`，不冒充已覆盖。

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use rusqlite::OptionalExtension;
use serde_json::{Map, Value};

use crate::contract::collector::{Coverage, CoverageStatus, Gap};
use crate::contract::entity::{Attributes, Basis, Entity, EntityKind, Origin};
use crate::contract::error::{CoreError, Issue, codes, gap_codes};
use crate::contract::evidence::{Evidence, Source, SourceClass};
use crate::contract::identity::{
    IdentitySlot, canonical_entity_identity, compute_entity_id, compute_evidence_id,
    compute_relation_id, compute_source_id,
};
use crate::contract::relation::{Relation, RelationKind, Resolution};
use crate::project::config::{LoadedProject, ScopeKind};
use crate::project::scan::SelectionStatus;
use crate::project::snapshot::{LineMap, SourceSnapshot, build_snapshot};
use crate::search::documents::{entity_projection, source_projection};
use crate::storage;
use crate::storage::database::Database;

/// 基础层覆盖的业务能力（未采集时为 `not_configured`）。
pub const BUSINESS_CAPABILITIES: &[&str] = &[
    "routes", "frontend", "api", "database", "workflow", "report",
];

/// 基础索引结果。
#[derive(Debug, Clone)]
pub struct BaseIndexResult {
    pub index_id: String,
    pub generation: u64,
    pub snapshot: SourceSnapshot,
    pub entities: usize,
    pub relations: usize,
    pub documents: usize,
    pub gaps: Vec<Gap>,
    pub coverage: Vec<Coverage>,
}

/// 作用域信息。
#[derive(Debug, Clone)]
pub struct ScopeInfo {
    pub id: String,
    pub kind: ScopeKind,
    pub repository_ids: Vec<String>,
    pub roots: Vec<(String, String)>,
}

/// 仓库默认作用域 ID。
pub fn default_scope_id(repository_id: &str) -> String {
    format!("repo.{repository_id}")
}

/// 有效作用域：配置作用域 + 每个仓库的稳定默认作用域。
pub fn effective_scopes(project: &LoadedProject) -> Vec<ScopeInfo> {
    let mut scopes: Vec<ScopeInfo> = Vec::new();
    for scope in &project.config.scopes {
        scopes.push(ScopeInfo {
            id: scope.id.clone(),
            kind: scope.kind,
            repository_ids: scope.repository_ids.clone(),
            roots: scope
                .roots
                .iter()
                .map(|root| (root.repository_id.clone(), root.path.clone()))
                .collect(),
        });
    }
    for repository_id in project.repositories.keys() {
        let id = default_scope_id(repository_id);
        if scopes.iter().any(|scope| scope.id == id) {
            continue;
        }
        scopes.push(ScopeInfo {
            id,
            kind: ScopeKind::Module,
            repository_ids: vec![repository_id.clone()],
            roots: Vec::new(),
        });
    }
    scopes
}

/// 基础符号按同仓库最长匹配根选择作用域。
///
/// 同等长度但不同作用域视为配置冲突；没有匹配则使用稳定的仓库默认作用域。
pub fn select_scope(
    scopes: &[ScopeInfo],
    repository_id: &str,
    relative_path: &str,
) -> Result<String, CoreError> {
    let mut best: Option<(usize, String)> = None;
    let mut conflict: Option<String> = None;
    for scope in scopes {
        for (root_repository, root_path) in &scope.roots {
            if root_repository != repository_id {
                continue;
            }
            let root = normalize_root(root_path);
            if root.is_empty() {
                continue;
            }
            if !path_under_root(relative_path, &root) {
                continue;
            }
            let length = root.len();
            match &best {
                Some((best_length, _)) if *best_length > length => continue,
                Some((best_length, best_id)) if *best_length == length => {
                    if best_id != &scope.id {
                        conflict = Some(scope.id.clone());
                    }
                }
                _ => best = Some((length, scope.id.clone())),
            }
        }
    }
    if let Some(conflicting) = conflict {
        return Err(CoreError::new(
            codes::CONFIG_INVALID,
            format!(
                "scope roots of equal length match `{relative_path}` in different scopes (`{conflicting}`)"
            ),
        ));
    }
    Ok(best
        .map(|(_, id)| id)
        .unwrap_or_else(|| default_scope_id(repository_id)))
}

fn normalize_root(path: &str) -> String {
    path.trim_start_matches("./")
        .trim_end_matches('/')
        .to_string()
}

fn path_under_root(path: &str, root: &str) -> bool {
    path == root || path.starts_with(&format!("{root}/"))
}

/// 构建基础索引。
pub fn build_base_index(
    project: &LoadedProject,
    database: &mut Database,
    now: &str,
) -> Result<BaseIndexResult, CoreError> {
    build_base_index_with_cancellation(
        project,
        database,
        now,
        &tokio_util::sync::CancellationToken::new(),
    )
}

pub fn build_base_index_with_cancellation(
    project: &LoadedProject,
    database: &mut Database,
    now: &str,
    cancel: &tokio_util::sync::CancellationToken,
) -> Result<BaseIndexResult, CoreError> {
    crate::client::check_cancel(cancel)?;
    crate::knowledge::documents::refresh(database, project)?;
    let project_id = project.config.project_id.clone();
    let config_hash = project.config_hash()?;
    let scopes = effective_scopes(project);

    // 1. 扫描全部仓库。
    let scan_results = crate::project::snapshot::scan_project_with_cancellation(project, cancel)?;

    // 2. 来源快照。
    let snapshot = build_snapshot(&project_id, &config_hash, &scan_results, now);

    // 3. 写入事务。
    let transaction = database.write_transaction()?;
    let connection = &transaction;

    // 3.1 配置同步。
    for (repository_id, root) in &project.repositories {
        connection
            .execute(
                "INSERT INTO repositories (id, root) VALUES (?1, ?2)
                 ON CONFLICT(id) DO UPDATE SET root = excluded.root",
                rusqlite::params![repository_id, root.to_string_lossy()],
            )
            .map_err(|error| {
                CoreError::new(
                    codes::INTERNAL_ERROR,
                    format!("cannot upsert repository: {error}"),
                )
            })?;
    }
    for scope in &scopes {
        connection
            .execute(
                "INSERT INTO scopes (id, kind) VALUES (?1, ?2)
                 ON CONFLICT(id) DO UPDATE SET kind = excluded.kind",
                rusqlite::params![scope.id, scope_kind_text(scope.kind)],
            )
            .map_err(|error| {
                CoreError::new(
                    codes::INTERNAL_ERROR,
                    format!("cannot upsert scope: {error}"),
                )
            })?;
        for (repository_id, path) in &scope.roots {
            if !project.repositories.contains_key(repository_id) {
                continue;
            }
            connection
                .execute(
                    "INSERT OR IGNORE INTO scope_roots (scope_id, repository_id, path) VALUES (?1, ?2, ?3)",
                    rusqlite::params![scope.id, repository_id, path],
                )
                .map_err(|error| {
                    CoreError::new(
                        codes::INTERNAL_ERROR,
                        format!("cannot upsert scope root: {error}"),
                    )
                })?;
        }
    }
    for service in &project.config.services {
        let base_urls_json = serde_json::to_string(&service.base_urls).map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot serialize service base urls: {error}"),
            )
        })?;
        connection
            .execute(
                "INSERT INTO services (id, scope_id, protocol, base_urls_json) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(id) DO UPDATE SET scope_id = excluded.scope_id,
                                                 protocol = excluded.protocol,
                                                 base_urls_json = excluded.base_urls_json",
                rusqlite::params![service.id, service.scope_id, service.protocol, base_urls_json],
            )
            .map_err(|error| {
                CoreError::new(
                    codes::INTERNAL_ERROR,
                    format!("cannot upsert service: {error}"),
                )
            })?;
    }
    for data_source in &project.config.data_sources {
        connection
            .execute(
                "INSERT INTO data_sources (id, scope_id, dialect, catalog, schema_name) VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(id) DO UPDATE SET scope_id = excluded.scope_id,
                                                 dialect = excluded.dialect,
                                                 catalog = excluded.catalog,
                                                 schema_name = excluded.schema_name",
                rusqlite::params![
                    data_source.id,
                    data_source.scope_id,
                    data_source.dialect,
                    data_source.catalog,
                    data_source.schema
                ],
            )
            .map_err(|error| {
                CoreError::new(
                    codes::INTERNAL_ERROR,
                    format!("cannot upsert data source: {error}"),
                )
            })?;
    }
    for binding in &project.config.bindings {
        let evidence_json = serde_json::to_string(&binding.evidence_refs).map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot serialize binding evidence: {error}"),
            )
        })?;
        connection
            .execute(
                "INSERT INTO project_bindings (id, kind, scope_id, from_key, to_key, evidence_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(id) DO UPDATE SET kind = excluded.kind,
                                                 scope_id = excluded.scope_id,
                                                 from_key = excluded.from_key,
                                                 to_key = excluded.to_key,
                                                 evidence_json = excluded.evidence_json",
                rusqlite::params![
                    binding.id,
                    binding_kind_text(binding.kind),
                    binding.scope_id,
                    binding.from,
                    binding.to,
                    evidence_json
                ],
            )
            .map_err(|error| {
                CoreError::new(
                    codes::INTERNAL_ERROR,
                    format!("cannot upsert binding: {error}"),
                )
            })?;
    }

    // 3.2 运行记录。
    let run_id = format!("run_{}", uuid::Uuid::new_v4());
    storage::create_run(
        connection,
        &run_id,
        "index.base",
        "base",
        Some(&snapshot.id),
        now,
    )?;

    // 3.3 每个仓库一个基础批次。
    let mut entity_count = 0usize;
    let mut relation_count = 0usize;
    let mut document_count = 0usize;
    let mut all_gaps: Vec<Gap> = Vec::new();
    let mut all_coverage: Vec<Coverage> = Vec::new();

    for result in &scan_results {
        crate::client::check_cancel(cancel)?;
        let replacement_key = storage::base_replacement_key(&result.repository_id);
        let batch_id = storage::batch_id(&run_id, &replacement_key);
        // 每仓库独立比较；无关仓库变化不能更换本仓库断言/候选身份。
        let input_digest = crate::collect::import::digest(&(
            &config_hash,
            crate::project::scan::source_set_digest(std::slice::from_ref(result)),
        ));
        if let Some(previous) = storage::current_batch(connection, &replacement_key)? {
            let previous_digest: String = connection
                .query_row(
                    "SELECT input_digest FROM batches WHERE id=?1 AND state='fresh'",
                    [&previous],
                    |row| row.get(0),
                )
                .optional()
                .map_err(crate::storage::database_error)?
                .unwrap_or_default();
            if previous_digest == input_digest {
                for (table, count) in [
                    ("entity_assertions", &mut entity_count),
                    ("relation_assertions", &mut relation_count),
                    ("search_documents", &mut document_count),
                ] {
                    let sql = if table == "search_documents" {
                        "SELECT count(*) FROM search_documents d JOIN assertions a ON a.id=d.assertion_id WHERE a.batch_id=?1".to_string()
                    } else {
                        format!("SELECT count(*) FROM {table} WHERE batch_id=?1")
                    };
                    *count += connection
                        .query_row(&sql, [&previous], |row| row.get::<_, i64>(0))
                        .map_err(crate::storage::database_error)?
                        as usize;
                }
                for (_, _, gap) in storage::load_gaps(connection, &previous)? {
                    all_gaps.push(serde_json::from_value(gap).map_err(|error| {
                        CoreError::new(codes::INTERNAL_ERROR, error.to_string())
                    })?);
                }
                let mut statement = connection.prepare(
                    "SELECT capability,scope_id,source_set_id,status,counts_json,gap_ids_json FROM coverage WHERE batch_id=?1 ORDER BY capability,scope_id"
                ).map_err(crate::storage::database_error)?;
                let rows = statement
                    .query_map([&previous], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                            row.get::<_, String>(4)?,
                            row.get::<_, String>(5)?,
                        ))
                    })
                    .map_err(crate::storage::database_error)?;
                for row in rows {
                    let (capability, scope_id, source_set_id, status, counts, gap_ids) =
                        row.map_err(crate::storage::database_error)?;
                    all_coverage.push(serde_json::from_value(serde_json::json!({
                        "capability":capability,"scopeId":scope_id,"sourceSetId":source_set_id,"status":if status=="not_configured" {"notConfigured"}else{&status},
                        "counts":serde_json::from_str::<Value>(&counts).map_err(|e| CoreError::new(codes::INTERNAL_ERROR,e.to_string()))?,
                        "gapIds":serde_json::from_str::<Value>(&gap_ids).map_err(|e| CoreError::new(codes::INTERNAL_ERROR,e.to_string()))?
                    })).map_err(|e| CoreError::new(codes::INTERNAL_ERROR,e.to_string()))?);
                }
                continue;
            }
            storage::invalidate_batches(
                connection,
                std::slice::from_ref(&replacement_key),
                database_generation(connection)? + 1,
            )?;
        }
        storage::create_batch(
            connection,
            &batch_id,
            &run_id,
            "core.base",
            &replacement_key,
            &input_digest,
            &input_digest,
            now,
        )?;

        let mut builder = RepositoryBuilder {
            project_id: &project_id,
            repository_id: &result.repository_id,
            scopes: &scopes,
            batch_id: &batch_id,
            entity_count: 0,
            relation_count: 0,
            document_count: 0,
            gaps: Vec::new(),
        };

        for source in &result.sources {
            crate::client::check_cancel(cancel)?;
            let root = project
                .repositories
                .get(&result.repository_id)
                .expect("scan results only contain configured repositories");
            if source.selection_status != SelectionStatus::Selected {
                // 被排除的文件也要登记来源行（currentHash 为空），否则快照成员会
                // 出现悬空外键；集合摘要仍然包含它们，保证排除变化可检测。
                let source_id = compute_source_id(&project_id, &result.repository_id, &source.path);
                let excluded = Source {
                    id: source_id.clone(),
                    repository_id: result.repository_id.clone(),
                    path: source.path.clone(),
                    source_class: classify_source(&source.path),
                    language: None,
                    content_hash: String::new(),
                    byte_length: 0,
                };
                storage::upsert_source(
                    connection,
                    &result.repository_id,
                    &excluded,
                    source.selection_status.as_str(),
                )?;
                continue;
            }
            builder.index_file(connection, root, source)?;
        }

        entity_count += builder.entity_count;
        relation_count += builder.relation_count;
        document_count += builder.document_count;

        // 扫描缺口。
        for gap in &result.gaps {
            let payload = Gap {
                id: format!(
                    "gap_{}",
                    crate::contract::identity::content_hash(
                        format!(
                            "{}\u{1f}{}\u{1f}{}",
                            result.repository_id, gap.path, gap.code
                        )
                        .as_bytes()
                    )
                ),
                code: gap.code.clone(),
                capability: "base".to_string(),
                scope_id: default_scope_id(&result.repository_id),
                source_refs: Vec::new(),
                candidate_ids: Vec::new(),
                reason: gap.reason.clone(),
                required_evidence: Vec::new(),
            };
            storage::insert_gap(
                connection,
                &payload.id,
                &batch_id,
                &payload.code,
                &serde_json::to_value(&payload).unwrap_or(Value::Null),
            )?;
            builder.gaps.push(payload);
        }

        // 依赖：来源集合摘要 + 配置哈希。
        storage::insert_dependency(
            connection,
            &batch_id,
            "sourceSet",
            &result.repository_id,
            &crate::project::scan::source_set_digest(std::slice::from_ref(result)),
        )?;
        storage::insert_dependency(connection, &batch_id, "config", &project_id, &config_hash)?;

        // 覆盖：本仓库涉及的作用域。
        let mut counts: Map<String, Value> = Map::new();
        counts.insert(
            "entities".to_string(),
            Value::from(builder.entity_count as u64),
        );
        counts.insert(
            "relations".to_string(),
            Value::from(builder.relation_count as u64),
        );
        counts.insert(
            "files".to_string(),
            Value::from(
                result
                    .sources
                    .iter()
                    .filter(|source| source.selection_status == SelectionStatus::Selected)
                    .count() as u64,
            ),
        );
        let scope_ids = scopes_for_repository(&scopes, &result.repository_id);
        for scope_id in &scope_ids {
            let status = if builder.gaps.is_empty() {
                CoverageStatus::Complete
            } else {
                CoverageStatus::Partial
            };
            let gap_ids: Vec<String> = builder.gaps.iter().map(|gap| gap.id.clone()).collect();
            let coverage = Coverage {
                capability: "base".to_string(),
                scope_id: scope_id.clone(),
                source_set_id: result.repository_id.clone(),
                status,
                counts: counts.clone(),
                gap_ids,
            };
            storage::insert_coverage(
                connection,
                &batch_id,
                &coverage.capability,
                &coverage.scope_id,
                &coverage.source_set_id,
                coverage.status.as_str(),
                &coverage.counts,
                &coverage.gap_ids,
            )?;
            all_coverage.push(coverage);
            for capability in BUSINESS_CAPABILITIES {
                let coverage = Coverage {
                    capability: (*capability).to_string(),
                    scope_id: scope_id.clone(),
                    source_set_id: result.repository_id.clone(),
                    status: CoverageStatus::NotConfigured,
                    counts: Map::new(),
                    gap_ids: Vec::new(),
                };
                storage::insert_coverage(
                    connection,
                    &batch_id,
                    &coverage.capability,
                    &coverage.scope_id,
                    &coverage.source_set_id,
                    coverage.status.as_str(),
                    &coverage.counts,
                    &coverage.gap_ids,
                )?;
                all_coverage.push(coverage);
            }
        }

        storage::set_current_batch(connection, &replacement_key, &batch_id)?;
        all_gaps.extend(builder.gaps);
    }

    // 3.4 来源快照与索引元数据。
    for (key, _) in storage::current_batches(connection)? {
        if key.starts_with("(core.base,")
            && !project
                .repositories
                .keys()
                .any(|id| storage::base_replacement_key(id) == key)
        {
            storage::invalidate_batches(
                connection,
                std::slice::from_ref(&key),
                database_generation(connection)? + 1,
            )?;
            connection
                .execute(
                    "DELETE FROM current_batches WHERE replacement_key=?1",
                    [&key],
                )
                .map_err(crate::storage::database_error)?;
        }
    }
    storage::register_snapshot(connection, &snapshot)?;
    let current_generation: i64 = connection
        .query_row(
            "SELECT generation FROM index_meta WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot read generation: {error}"),
            )
        })?;
    let generation = current_generation as u64 + 1;
    connection
        .execute(
            "UPDATE index_meta SET generation = ?1, current_source_snapshot_id = ?2, config_hash = ?3 WHERE id = 1",
            rusqlite::params![generation as i64, snapshot.id, config_hash],
        )
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot update index metadata: {error}"),
            )
        })?;

    storage::finish_run(
        connection,
        &run_id,
        storage::RunStatus::Succeeded,
        None,
        now,
    )?;
    crate::client::check_cancel(cancel)?;
    transaction.commit().map_err(|error| {
        CoreError::new(
            codes::BUSY,
            format!("cannot commit base index transaction: {error}"),
        )
    })?;

    Ok(BaseIndexResult {
        index_id: database.index_id()?,
        generation,
        snapshot,
        entities: entity_count,
        relations: relation_count,
        documents: document_count,
        gaps: all_gaps,
        coverage: all_coverage,
    })
}

fn database_generation(connection: &rusqlite::Connection) -> Result<u64, CoreError> {
    connection
        .query_row("SELECT generation FROM index_meta WHERE id=1", [], |row| {
            row.get::<_, i64>(0).map(|n| n as u64)
        })
        .map_err(crate::storage::database_error)
}

fn scopes_for_repository(scopes: &[ScopeInfo], repository_id: &str) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();
    for scope in scopes {
        let covers = scope.repository_ids.iter().any(|id| id == repository_id)
            || scope
                .roots
                .iter()
                .any(|(root_repository, _)| root_repository == repository_id);
        if covers || scope.id == default_scope_id(repository_id) {
            result.push(scope.id.clone());
        }
    }
    if result.is_empty() {
        result.push(default_scope_id(repository_id));
    }
    result
}

fn scope_kind_text(kind: ScopeKind) -> &'static str {
    match kind {
        ScopeKind::Application => "application",
        ScopeKind::Module => "module",
        ScopeKind::Service => "service",
    }
}

fn binding_kind_text(kind: crate::project::config::BindingKind) -> &'static str {
    use crate::project::config::BindingKind;
    match kind {
        BindingKind::ClientToService => "clientToService",
        BindingKind::ProxyToService => "proxyToService",
        BindingKind::OrmInstanceToDataSource => "ormInstanceToDataSource",
    }
}

/// 单仓库的实体/关系构建器。
struct RepositoryBuilder<'a> {
    project_id: &'a str,
    repository_id: &'a str,
    scopes: &'a [ScopeInfo],
    batch_id: &'a str,
    entity_count: usize,
    relation_count: usize,
    document_count: usize,
    gaps: Vec<Gap>,
}

impl<'a> RepositoryBuilder<'a> {
    #[allow(clippy::too_many_arguments)]
    fn index_file(
        &mut self,
        connection: &rusqlite::Connection,
        root: &Path,
        source: &crate::project::scan::ScannedSource,
    ) -> Result<(), CoreError> {
        let absolute = root.join(&source.path);
        let stable = crate::project::snapshot::read_stable(&absolute)?;
        if stable.content_hash != source.content_hash {
            return Err(CoreError::new(
                codes::SOURCE_CHANGED,
                format!("{} changed while indexing", source.path),
            ));
        }
        let text = match String::from_utf8(stable.bytes.clone()) {
            Ok(text) => text,
            Err(_) => {
                self.gaps.push(Gap {
                    id: format!(
                        "gap_{}",
                        crate::contract::identity::content_hash(
                            format!("{}\u{1f}{}", self.repository_id, source.path).as_bytes()
                        )
                    ),
                    code: gap_codes::NOT_UTF8.to_string(),
                    capability: "base".to_string(),
                    scope_id: default_scope_id(self.repository_id),
                    source_refs: Vec::new(),
                    candidate_ids: Vec::new(),
                    reason: "file is not valid UTF-8".to_string(),
                    required_evidence: Vec::new(),
                });
                return Ok(());
            }
        };
        if text.is_empty() {
            // 零字节文件没有可引用的非空区间，不建立实体。
            return Ok(());
        }

        let scope_id = select_scope(self.scopes, self.repository_id, &source.path)?;
        let source_id = compute_source_id(self.project_id, self.repository_id, &source.path);
        let source_class = classify_source(&source.path);

        let contract_source = Source {
            id: source_id.clone(),
            repository_id: self.repository_id.to_string(),
            path: source.path.clone(),
            source_class,
            language: Some(
                crate::syntax::parser::detect_language(Path::new(&source.path))
                    .as_str()
                    .to_string(),
            ),
            content_hash: source.content_hash.clone(),
            byte_length: stable.bytes.len() as u64,
        };
        storage::upsert_source(
            connection,
            self.repository_id,
            &contract_source,
            SelectionStatus::Selected.as_str(),
        )?;
        let line_map = LineMap::build(&stable.bytes);
        storage::store_source_version(
            connection,
            &source_id,
            &source.content_hash,
            &stable.bytes,
            &line_map,
            source_class,
            "",
        )?;

        // 文件实体。
        let file_slots = vec![
            IdentitySlot::required("repositoryId", self.repository_id),
            IdentitySlot::required("path", source.path.clone()),
            IdentitySlot::required("scopeId", scope_id.clone()),
        ];
        let file_id = compute_entity_id(self.project_id, EntityKind::File, &file_slots)
            .map_err(|error| CoreError::new(codes::IDENTITY_CONFLICT, error.to_string()))?;
        let file_identity =
            canonical_entity_identity(self.project_id, EntityKind::File, &file_slots)
                .map_err(|error| CoreError::new(codes::IDENTITY_CONFLICT, error.to_string()))?;

        let file_evidence = Evidence {
            id: compute_evidence_id(&source_id, &source.content_hash, 0, 1, "file"),
            source_id: source_id.clone(),
            content_hash: source.content_hash.clone(),
            start_byte: 0,
            end_byte: 1,
            start_line: 1,
            end_line: line_map.end_line_of(0, 1),
            role: "file".to_string(),
        };
        storage::insert_evidence(connection, &file_evidence)?;

        let mut file_attributes: Attributes = Map::new();
        file_attributes.insert("path".to_string(), Value::from(source.path.clone()));
        file_attributes.insert(
            "repositoryId".to_string(),
            Value::from(self.repository_id.to_string()),
        );
        file_attributes.insert(
            "byteLength".to_string(),
            Value::from(stable.bytes.len() as u64),
        );
        if let Some(language) = &contract_source.language {
            file_attributes.insert("language".to_string(), Value::from(language.clone()));
        }

        let file_assertion_id = self.insert_entity_assertion(
            connection,
            &Entity {
                id: file_id.clone(),
                kind: EntityKind::File,
                scope_id: scope_id.clone(),
                identity: file_slots.clone(),
                attributes: file_attributes.clone(),
                basis: Basis::Observed,
                origin: Origin::Static,
                evidence_refs: vec![file_evidence.id.clone()],
                extensions: Map::new(),
            },
            &file_identity,
            Some(&source.path),
            &file_evidence.id,
        )?;

        // 源码投影：整份原始文本都可检索。
        let source_document =
            source_projection(&file_assertion_id, &file_id, &scope_id, &source.path, &text);
        storage::search::insert_document(connection, &source_document)?;
        self.document_count += 1;

        // 语法事实。
        self.index_syntax(
            connection,
            &source_id,
            &source.content_hash,
            &line_map,
            &text,
            &source.path,
            &scope_id,
            &file_id,
            &file_assertion_id,
        )?;

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn index_syntax(
        &mut self,
        connection: &rusqlite::Connection,
        source_id: &str,
        content_hash: &str,
        line_map: &LineMap,
        text: &str,
        path: &str,
        scope_id: &str,
        file_id: &str,
        file_assertion_id: &str,
    ) -> Result<(), CoreError> {
        let language = crate::syntax::parser::detect_language(Path::new(path));
        if !language.is_supported() {
            return Ok(());
        }
        let Some(tree) = crate::syntax::parser::parse_source(language, text) else {
            self.gaps.push(Gap {
                id: format!(
                    "gap_{}",
                    crate::contract::identity::content_hash(
                        format!("{}\u{1f}{}\u{1f}parse", self.repository_id, path).as_bytes()
                    )
                ),
                code: gap_codes::PARSE_FAILED.to_string(),
                capability: "base".to_string(),
                scope_id: scope_id.to_string(),
                source_refs: Vec::new(),
                candidate_ids: Vec::new(),
                reason: format!("{} source could not be parsed", language.as_str()),
                required_evidence: Vec::new(),
            });
            return Ok(());
        };
        let facts = crate::syntax::symbols::extract_facts(&tree, text, language);

        // 符号实体。
        let mut symbol_ids: BTreeMap<String, String> = BTreeMap::new();
        for symbol in &facts.symbols {
            let slots = vec![
                IdentitySlot::required("repositoryId", self.repository_id),
                IdentitySlot::required("path", path),
                IdentitySlot::required("scopeId", scope_id),
                IdentitySlot::required("lexicalPath", symbol.lexical_path.clone()),
                IdentitySlot::required("symbolKind", symbol.symbol_kind.as_str()),
                IdentitySlot::required("normalizedSignature", symbol.normalized_signature.clone()),
            ];
            let symbol_id = compute_entity_id(self.project_id, EntityKind::Symbol, &slots)
                .map_err(|error| CoreError::new(codes::IDENTITY_CONFLICT, error.to_string()))?;
            let identity =
                canonical_entity_identity(self.project_id, EntityKind::Symbol, &slots)
                    .map_err(|error| CoreError::new(codes::IDENTITY_CONFLICT, error.to_string()))?;

            let evidence = Evidence {
                id: compute_evidence_id(
                    source_id,
                    content_hash,
                    symbol.start_byte,
                    symbol.end_byte,
                    "definition",
                ),
                source_id: source_id.to_string(),
                content_hash: content_hash.to_string(),
                start_byte: symbol.start_byte,
                end_byte: symbol.end_byte,
                start_line: symbol.start_line,
                end_line: symbol.end_line,
                role: "definition".to_string(),
            };
            storage::insert_evidence(connection, &evidence)?;

            let mut attributes: Attributes = Map::new();
            attributes.insert("name".to_string(), Value::from(symbol.name.clone()));
            attributes.insert(
                "symbolKind".to_string(),
                Value::from(symbol.symbol_kind.as_str()),
            );
            attributes.insert(
                "lexicalPath".to_string(),
                Value::from(symbol.lexical_path.clone()),
            );
            attributes.insert(
                "normalizedSignature".to_string(),
                Value::from(symbol.normalized_signature.clone()),
            );
            attributes.insert("path".to_string(), Value::from(path));
            if let Some(container) = &symbol.container {
                attributes.insert("container".to_string(), Value::from(container.clone()));
            }

            let assertion_id = self.insert_entity_assertion(
                connection,
                &Entity {
                    id: symbol_id.clone(),
                    kind: EntityKind::Symbol,
                    scope_id: scope_id.to_string(),
                    identity: slots,
                    attributes: attributes.clone(),
                    basis: Basis::Observed,
                    origin: Origin::Static,
                    evidence_refs: vec![evidence.id.clone()],
                    extensions: Map::new(),
                },
                &identity,
                Some(&symbol.name),
                &evidence.id,
            )?;

            let document = entity_projection(
                &assertion_id,
                &symbol_id,
                scope_id,
                &symbol.name,
                symbol.symbol_kind.as_str(),
                path,
                &symbol.normalized_signature,
                &Value::Object(attributes),
                None,
            );
            storage::search::insert_document(connection, &document)?;
            self.document_count += 1;

            // file → symbol
            self.insert_relation(
                connection,
                RelationKind::Contains,
                file_id,
                &symbol_id,
                Map::new(),
                Resolution::Confirmed,
                &evidence.id,
                None,
            )?;

            symbol_ids.insert(symbol.lexical_path.clone(), symbol_id);
        }

        // 文件内调用：被调者在同一文件内定义时才建立 confirmed 关系。
        for call in &facts.calls {
            let Some(from_id) = call
                .container
                .as_ref()
                .and_then(|container| symbol_ids.get(container))
                .cloned()
                .or_else(|| symbol_ids.get(&call.callee).cloned())
            else {
                continue;
            };
            let Some(to_id) = symbol_ids.get(&call.callee).cloned() else {
                continue;
            };
            if from_id == to_id {
                continue;
            }
            let evidence = Evidence {
                id: compute_evidence_id(
                    source_id,
                    content_hash,
                    call.start_byte,
                    call.end_byte,
                    "call",
                ),
                source_id: source_id.to_string(),
                content_hash: content_hash.to_string(),
                start_byte: call.start_byte,
                end_byte: call.end_byte,
                start_line: call.start_line,
                end_line: call.end_line,
                role: "call".to_string(),
            };
            storage::insert_evidence(connection, &evidence)?;
            self.insert_relation(
                connection,
                RelationKind::Calls,
                &from_id,
                &to_id,
                Map::new(),
                Resolution::Confirmed,
                &evidence.id,
                None,
            )?;
        }

        let _ = (file_assertion_id, line_map);
        Ok(())
    }

    fn insert_entity_assertion(
        &mut self,
        connection: &rusqlite::Connection,
        entity: &Entity,
        identity_json: &str,
        name: Option<&str>,
        evidence_id: &str,
    ) -> Result<String, CoreError> {
        storage::upsert_entity(
            connection,
            &storage::entities::EntityRow {
                id: &entity.id,
                kind: entity.kind.clone(),
                scope_id: &entity.scope_id,
                identity_json,
            },
        )?;
        let payload_hash = storage::entities::entity_payload_hash(
            "core.base",
            self.batch_id,
            &entity.id,
            name,
            &entity.attributes,
            entity.basis,
            entity.origin,
        )?;
        let assertion_id = storage::entities::assertion_id("entity", self.batch_id, &payload_hash);
        storage::entities::insert_entity_assertion(
            connection,
            &storage::entities::EntityAssertionRow {
                assertion_id: &assertion_id,
                entity_id: &entity.id,
                batch_id: self.batch_id,
                basis: entity.basis,
                origin: entity.origin,
                name,
                attributes: &entity.attributes,
            },
            &payload_hash,
            "core.base",
        )?;
        storage::entities::link_assertion_evidence(connection, &assertion_id, evidence_id)?;
        self.entity_count += 1;
        Ok(assertion_id)
    }

    #[allow(clippy::too_many_arguments)]
    fn insert_relation(
        &mut self,
        connection: &rusqlite::Connection,
        kind: RelationKind,
        from_id: &str,
        to_id: &str,
        qualifiers: Map<String, Value>,
        resolution: Resolution,
        evidence_id: &str,
        route_exact: Option<&str>,
    ) -> Result<(), CoreError> {
        let relation_id =
            compute_relation_id(self.project_id, kind.clone(), from_id, to_id, &qualifiers)
                .map_err(|error| CoreError::new(codes::IDENTITY_CONFLICT, error.to_string()))?;
        let relation = Relation {
            id: relation_id.clone(),
            kind: kind.clone(),
            from_id: from_id.to_string(),
            to_id: to_id.to_string(),
            qualifiers: qualifiers.clone(),
            basis: Basis::Observed,
            origin: Origin::Static,
            resolution,
            evidence_refs: vec![evidence_id.to_string()],
            extensions: Map::new(),
        };
        storage::upsert_relation(connection, &relation)?;
        let payload_hash = crate::contract::identity::content_hash(
            format!(
                "{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}",
                self.batch_id,
                relation_id,
                kind.as_str(),
                from_id,
                to_id
            )
            .as_bytes(),
        );
        let assertion_id =
            storage::entities::assertion_id("relation", self.batch_id, &payload_hash);
        storage::relations::insert_relation_assertion(
            connection,
            &storage::relations::RelationAssertionRow {
                assertion_id: &assertion_id,
                relation_id: &relation_id,
                batch_id: self.batch_id,
                basis: relation.basis,
                origin: relation.origin,
                resolution,
            },
            &payload_hash,
            "core.base",
        )?;
        storage::entities::link_assertion_evidence(connection, &assertion_id, evidence_id)?;
        self.relation_count += 1;
        let _ = route_exact;
        Ok(())
    }
}

/// 按扩展名判定来源类别。
pub fn classify_source(path: &str) -> SourceClass {
    let extension = Path::new(path)
        .extension()
        .map(|value| value.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    match extension.as_str() {
        "sql" | "ddl" => SourceClass::Schema,
        "json" | "yaml" | "yml" | "toml" | "ini" | "env" | "properties" | "xml" => {
            SourceClass::Config
        }
        "md" | "markdown" | "txt" | "adoc" | "rst" => SourceClass::Documentation,
        _ => SourceClass::Code,
    }
}

/// 校验范围：未采集的能力必须显式登记为 `not_configured`。
pub fn assert_capabilities_declared(coverage: &[Coverage]) -> Result<(), CoreError> {
    let declared: BTreeSet<&str> = coverage
        .iter()
        .map(|entry| entry.capability.as_str())
        .collect();
    let mut issues = Vec::new();
    for capability in BUSINESS_CAPABILITIES {
        if !declared.contains(capability) {
            issues.push(
                Issue::new(codes::CONFIG_INVALID, "/coverage")
                    .with_actual((*capability).to_string())
                    .with_hint("business capabilities must be declared as not_configured"),
            );
        }
    }
    if issues.is_empty() {
        Ok(())
    } else {
        Err(CoreError::new(codes::CONFIG_INVALID, "coverage is incomplete").with_issues(issues))
    }
}
