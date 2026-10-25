//! 查询入口（计划 §3.7）。
//!
//! Core 暴露统一的 `QueryRequest` tagged union；CLI、SDK 与 MCP 使用同一 DTO。
//! 查询入口只读：不建库、不迁移、不运行插件、不调用模型。

use std::collections::BTreeSet;

use rusqlite::Connection;
use serde_json::{Map, Value};

use crate::contract::PROTOCOL_VERSION;
use crate::contract::entity::{Basis, EntityKind};
use crate::contract::error::{CoreError, codes, gap_codes};
use crate::contract::evidence::{SnapshotRef, SourceReference};
use crate::contract::query::{
    Budget, BudgetUsed, CodeMapRequest, Completeness, CoverageLevel, Direction, Freshness,
    FreshnessPolicy, Gap, LocateFeatureRequest, NextQuery, PageRequest, QueryRequest,
    QueryResponse, ReadSourceRequest, ReportRequest, ResponseStatus, SearchRequest, StatusRequest,
    TraceRequest, WorkflowRequest, default_budget,
};
use crate::project::config::LoadedProject;
use crate::search::rank::{exact_rank, fts_query, match_reason};
use crate::storage;
use crate::storage::database::Database;

pub mod feature;
pub mod page;
pub mod report;
pub mod workflow;

/// 查询上下文。
pub struct QueryContext<'a> {
    pub database: &'a mut Database,
    /// 用于只读新鲜度核验；缺省时只报告库内状态。
    pub project: Option<&'a LoadedProject>,
    pub now: String,
}

/// 执行查询。
pub fn execute(
    context: &mut QueryContext<'_>,
    request: &QueryRequest,
) -> Result<QueryResponse<Value>, CoreError> {
    let request_id = format!("req_{}", uuid::Uuid::new_v4());
    match request {
        QueryRequest::Status(inner) => execute_status(context, inner, request_id),
        QueryRequest::Search(inner) => execute_search(context, inner, request_id),
        QueryRequest::ReadSource(inner) => execute_read_source(context, inner, request_id),
        QueryRequest::Trace(inner) => execute_trace(context, inner, request_id),
        QueryRequest::CodeMap(inner) => execute_code_map(context, inner, request_id),
        QueryRequest::LocateFeature(inner) => feature::execute(context, inner, request_id),
        QueryRequest::Page(inner) => page::execute(context, inner, request_id),
        QueryRequest::Workflow(inner) => workflow::execute(context, inner, request_id),
        QueryRequest::Report(inner) => report::execute(context, inner, request_id),
    }
}

/// 当前快照引用。
pub fn snapshot_ref(database: &Database) -> Result<SnapshotRef, CoreError> {
    Ok(SnapshotRef {
        index_id: database.index_id()?,
        generation: database.generation_string()?,
        source_snapshot_id: database.current_source_snapshot_id()?,
    })
}

/// 预算解析：请求值与默认值的较小者，且不超过服务端硬上限。
#[derive(Debug, Clone, Copy)]
pub struct ResolvedBudget {
    pub max_entities: u64,
    pub max_relations: u64,
    pub max_visited: u64,
    pub max_depth: u32,
    pub max_response_chars: u64,
    pub timeout_ms: u64,
}

pub fn resolve_budget(budget: Option<&Budget>) -> ResolvedBudget {
    let take = |requested: Option<u64>, default: u64, hard: u64| -> u64 {
        requested.unwrap_or(default).min(hard)
    };
    ResolvedBudget {
        max_entities: take(
            budget.and_then(|b| b.max_entities),
            default_budget::MAX_ENTITIES,
            default_budget::HARD_MAX_ENTITIES,
        ),
        max_relations: take(
            budget.and_then(|b| b.max_relations),
            default_budget::MAX_RELATIONS,
            default_budget::HARD_MAX_RELATIONS,
        ),
        max_visited: take(
            budget.and_then(|b| b.max_visited),
            default_budget::MAX_VISITED,
            default_budget::HARD_MAX_VISITED,
        ),
        max_depth: budget
            .and_then(|b| b.max_depth)
            .unwrap_or(default_budget::MAX_DEPTH)
            .min(default_budget::HARD_MAX_DEPTH),
        max_response_chars: take(
            budget.and_then(|b| b.max_response_chars),
            default_budget::MAX_RESPONSE_CHARS,
            default_budget::HARD_MAX_RESPONSE_CHARS,
        ),
        timeout_ms: take(
            budget.and_then(|b| b.timeout_ms),
            default_budget::TIMEOUT_MS,
            default_budget::HARD_TIMEOUT_MS,
        ),
    }
}

/// 通用过滤器。
#[derive(Debug, Clone, Default)]
pub struct Filters {
    pub scope_ids: Option<Vec<String>>,
    pub repository_ids: Option<Vec<String>>,
    pub kinds: Option<Vec<EntityKind>>,
    pub basis_filter: Option<Vec<Basis>>,
    pub include_candidates: bool,
    pub freshness_policy: FreshnessPolicy,
}

impl Filters {
    pub fn from_request(filters: Option<&crate::contract::query::CommonFilters>) -> Filters {
        let mut result = Filters {
            scope_ids: None,
            repository_ids: None,
            kinds: None,
            basis_filter: None,
            include_candidates: false,
            freshness_policy: FreshnessPolicy::Verify,
        };
        if let Some(filters) = filters {
            result.scope_ids = filters.scope_ids.clone();
            result.repository_ids = filters.repository_ids.clone();
            result.kinds = filters.kinds.clone();
            result.basis_filter = filters.basis_filter.clone();
            result.include_candidates = filters.include_candidates.unwrap_or(false);
            result.freshness_policy = filters.freshness_policy.unwrap_or(FreshnessPolicy::Verify);
        }
        result
    }

    /// 依据过滤：默认只接受 `observed` 与 `declared`。
    pub fn allows_basis(&self, basis: Basis) -> bool {
        match &self.basis_filter {
            Some(list) => list.contains(&basis),
            None => matches!(basis, Basis::Observed | Basis::Declared),
        }
    }

    pub fn allows_kind(&self, kind: EntityKind) -> bool {
        match &self.kinds {
            Some(list) => list.contains(&kind),
            None => true,
        }
    }

    pub fn allows_scope(&self, scope_id: &str) -> bool {
        match &self.scope_ids {
            Some(list) => list.iter().any(|scope| scope == scope_id),
            None => true,
        }
    }
}

/// 构造查询响应外壳。
#[allow(clippy::too_many_arguments)]
pub fn envelope(
    context: &QueryContext<'_>,
    request_id: String,
    status: ResponseStatus,
    data: Value,
    gaps: Vec<Gap>,
    source_references: Vec<SourceReference>,
    completeness: Completeness,
    budget_used: BudgetUsed,
    next_queries: Vec<NextQuery>,
) -> Result<QueryResponse<Value>, CoreError> {
    Ok(QueryResponse {
        protocol_version: PROTOCOL_VERSION.to_string(),
        request_id,
        snapshot: snapshot_ref(context.database)?,
        status,
        data,
        source_references,
        gaps,
        completeness,
        budget_used,
        continuation: None,
        next_queries,
    })
}

/// 状态查询。
fn execute_status(
    context: &mut QueryContext<'_>,
    request: &StatusRequest,
    request_id: String,
) -> Result<QueryResponse<Value>, CoreError> {
    let connection = context.database.connection();
    let mut data = Map::new();
    data.insert("base".to_string(), Value::Null);
    let mut coverage = Map::new();
    let mut gaps: Vec<Gap> = Vec::new();
    let mut freshest = Freshness::Unknown;
    let mut disk_checked = false;

    let batches = storage::current_batches(connection)?;
    let mut base_state = "not_configured".to_string();
    let mut collector_states: Vec<Value> = Vec::new();
    for (replacement_key, batch_id) in &batches {
        let state = storage::batch_state(connection, batch_id)?;
        let state_text = state
            .map(|state| state.as_str().to_string())
            .unwrap_or_default();
        let invalidated = storage::batch_invalidated_generation(connection, batch_id)?;
        let mut entry = Map::new();
        entry.insert(
            "replacementKey".to_string(),
            Value::from(replacement_key.clone()),
        );
        entry.insert("batchId".to_string(), Value::from(batch_id.clone()));
        entry.insert("state".to_string(), Value::from(state_text.clone()));
        entry.insert(
            "invalidatedGeneration".to_string(),
            match invalidated {
                Some(value) => Value::from(value),
                None => Value::Null,
            },
        );
        if replacement_key.starts_with("(core.base,") {
            base_state = state_text.clone();
            for (capability, scope_id, source_set_id, status) in
                storage::load_coverage(connection, batch_id)?
            {
                let key = format!("{capability}@{scope_id}");
                let mut entry = Map::new();
                entry.insert("capability".to_string(), Value::from(capability));
                entry.insert("scopeId".to_string(), Value::from(scope_id));
                entry.insert("sourceSetId".to_string(), Value::from(source_set_id));
                entry.insert("status".to_string(), Value::from(status));
                coverage.insert(key, Value::Object(entry));
            }
            for (_, code, payload) in storage::load_gaps(connection, batch_id)? {
                gaps.push(gap_from_payload(&code, &payload));
            }
        } else {
            collector_states.push(Value::Object(entry));
        }
    }

    // 只读新鲜度核验：重新扫描并比较集合摘要，不写库。
    if request.check_freshness
        && let Some(project) = context.project
    {
        match verify_freshness(project, context.database) {
            Ok((freshness, checked)) => {
                freshest = freshness;
                disk_checked = checked;
            }
            Err(_) => {
                freshest = Freshness::Unknown;
                disk_checked = false;
            }
        }
    }

    let coverage_level = if coverage.is_empty() {
        CoverageLevel::NotConfigured
    } else if coverage
        .values()
        .all(|value| value.get("status").and_then(Value::as_str) == Some("complete"))
    {
        CoverageLevel::Complete
    } else {
        CoverageLevel::Partial
    };

    let mut base = Map::new();
    base.insert("state".to_string(), Value::from(base_state));
    base.insert("coverage".to_string(), Value::Object(coverage));
    data.insert("base".to_string(), Value::Object(base));
    data.insert("collectors".to_string(), Value::Array(collector_states));
    data.insert("knowledge".to_string(), Value::Null);
    data.insert(
        "activeVersions".to_string(),
        Value::Object(active_versions(connection)?),
    );

    let mut reasons: Vec<String> = Vec::new();
    if !disk_checked {
        reasons.push("disk was not checked".to_string());
    }
    let completeness = Completeness {
        freshness: freshest,
        disk_checked,
        checked_at: context.now.clone(),
        coverage: coverage_level,
        candidate_sets_complete: true,
        truncated: false,
        reasons,
    };
    envelope(
        context,
        request_id,
        ResponseStatus::Complete,
        Value::Object(data),
        gaps,
        Vec::new(),
        completeness,
        BudgetUsed {
            entities: 0,
            relations: 0,
            visited: 0,
            elapsed_ms: 0,
            response_chars: 0,
        },
        Vec::new(),
    )
}

fn active_versions(connection: &Connection) -> Result<Map<String, Value>, CoreError> {
    let mut result = Map::new();
    for (plugin_id, version) in storage::active_version_map(connection)? {
        result.insert(plugin_id, Value::from(version));
    }
    Ok(result)
}

/// 只读新鲜度核验：重新扫描并比较来源集合摘要。
pub fn verify_freshness(
    project: &LoadedProject,
    database: &Database,
) -> Result<(Freshness, bool), CoreError> {
    let stored = database
        .current_source_snapshot_id()?
        .ok_or_else(|| CoreError::new(codes::INDEX_STALE, "index has no source snapshot"))?;
    let config_hash = project.config_hash()?;
    let options = crate::project::scan::ScanOptions {
        respect_git_ignore: project.config.selection.respect_git_ignore,
        max_file_bytes: project.config.selection.max_file_bytes,
        include: project.config.selection.include.clone(),
        exclude: project.config.selection.exclude.clone(),
    };
    let mut results = Vec::new();
    for (repository_id, root) in &project.repositories {
        results.push(
            crate::project::scan::scan_repository(repository_id, root, &options)
                .map_err(|error| CoreError::new(codes::CONFIG_INVALID, error))?,
        );
    }
    let digest = crate::project::scan::source_set_digest(&results);
    let current = crate::project::snapshot::snapshot_id(&config_hash, &digest);
    if current == stored {
        Ok((Freshness::Fresh, true))
    } else {
        Ok((Freshness::Stale, true))
    }
}

fn gap_from_payload(code: &str, payload: &Value) -> Gap {
    Gap {
        id: payload
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        code: code.to_string(),
        capability: payload
            .get("capability")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        scope_id: payload
            .get("scopeId")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        source_refs: Vec::new(),
        candidate_ids: Vec::new(),
        reason: payload
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        required_evidence: Vec::new(),
    }
}

/// 搜索。
fn execute_search(
    context: &mut QueryContext<'_>,
    request: &SearchRequest,
    request_id: String,
) -> Result<QueryResponse<Value>, CoreError> {
    let filters = Filters::from_request(request.filters.as_ref());
    let page_size = request
        .page_size
        .unwrap_or(default_budget::PAGE_SIZE_DEFAULT)
        .min(default_budget::PAGE_SIZE_MAX) as usize;
    let connection = context.database.connection();

    let expression = fts_query(&request.query).ok_or_else(|| {
        CoreError::new(
            codes::INVALID_REQUEST,
            "search query does not contain any searchable token",
        )
    })?;
    let candidate_limit = (page_size * 8).clamp(64, 2_000);
    let hits = storage::search::search_documents(connection, &expression, candidate_limit)?;

    let mut scored: Vec<ScoredHit> = Vec::new();
    for hit in hits {
        if !filters.allows_scope(&hit.scope_id) {
            continue;
        }
        let rank = exact_rank(&request.query, &hit.text, hit.route_exact.as_deref());
        if rank == crate::search::rank::EXACT_NONE {
            continue;
        }
        let Some(entity) = load_entity(connection, &hit.entity_id)? else {
            continue;
        };
        if !filters.allows_kind(entity.kind) {
            continue;
        }
        if let Some(repository_ids) = &filters.repository_ids
            && !entity_in_repositories(connection, &hit.entity_id, repository_ids)?
        {
            continue;
        }
        let assertions =
            storage::entities::load_entity_assertions(connection, &hit.entity_id, true)?;
        if !assertions
            .iter()
            .any(|assertion| filters.allows_basis(assertion.basis))
        {
            continue;
        }
        scored.push(ScoredHit {
            hit,
            rank,
            entity,
            assertions,
        });
    }
    scored.sort_by(|left, right| {
        right
            .rank
            .cmp(&left.rank)
            .then(
                left.hit
                    .bm25
                    .partial_cmp(&right.hit.bm25)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
            .then(left.hit.entity_id.cmp(&right.hit.entity_id))
            .then(left.hit.assertion_id.cmp(&right.hit.assertion_id))
    });

    let total = scored.len();
    let truncated = total > page_size;
    let selected: Vec<ScoredHit> = scored.into_iter().take(page_size).collect();
    let source_references: Vec<SourceReference> = Vec::new();
    let mut results: Vec<Value> = Vec::new();
    for item in &selected {
        let mut entry = Map::new();
        entry.insert(
            "entityRef".to_string(),
            entity_ref_json(context.database, &item.hit.entity_id)?,
        );
        entry.insert("kind".to_string(), Value::from(item.entity.kind.as_str()));
        entry.insert(
            "scopeId".to_string(),
            Value::from(item.hit.scope_id.clone()),
        );
        entry.insert(
            "name".to_string(),
            item.assertions
                .iter()
                .find_map(|assertion| assertion.name.clone())
                .map(Value::from)
                .unwrap_or(Value::Null),
        );
        entry.insert(
            "matchReason".to_string(),
            Value::from(match_reason(item.rank)),
        );
        entry.insert("exactRank".to_string(), Value::from(item.rank));
        entry.insert("bm25".to_string(), Value::from(item.hit.bm25));
        entry.insert(
            "projection".to_string(),
            Value::from(item.hit.projection_kind.clone()),
        );
        let mut assertion_values: Vec<Value> = Vec::new();
        for assertion in &item.assertions {
            let mut value = Map::new();
            value.insert(
                "assertionId".to_string(),
                Value::from(assertion.assertion_id.clone()),
            );
            value.insert(
                "producerKey".to_string(),
                Value::from(assertion.producer_key.clone()),
            );
            value.insert(
                "batchId".to_string(),
                Value::from(assertion.batch_id.clone()),
            );
            value.insert("basis".to_string(), Value::from(assertion.basis.as_str()));
            value.insert("origin".to_string(), Value::from(assertion.origin.as_str()));
            value.insert(
                "evidenceRefs".to_string(),
                Value::Array(
                    storage::entities::load_assertion_evidence(
                        connection,
                        &assertion.assertion_id,
                    )?
                    .into_iter()
                    .map(Value::from)
                    .collect(),
                ),
            );
            assertion_values.push(Value::Object(value));
        }
        entry.insert("assertions".to_string(), Value::Array(assertion_values));
        results.push(Value::Object(entry));
    }

    let mut data = Map::new();
    data.insert("results".to_string(), Value::Array(results));
    data.insert("total".to_string(), Value::from(total as u64));
    data.insert(
        "fields".to_string(),
        Value::Array(
            request
                .fields
                .clone()
                .unwrap_or_default()
                .iter()
                .map(|field| Value::from(format!("{field:?}")))
                .collect(),
        ),
    );

    let mut reasons: Vec<String> = Vec::new();
    if truncated {
        reasons.push(format!("结果超过 pageSize，仅返回前 {page_size} 条"));
    }
    let completeness = Completeness {
        freshness: Freshness::Unknown,
        disk_checked: false,
        checked_at: context.now.clone(),
        coverage: CoverageLevel::Partial,
        candidate_sets_complete: true,
        truncated,
        reasons,
    };
    let mut gaps = Vec::new();
    if total == 0 {
        gaps.push(Gap {
            id: "gap_search_empty".to_string(),
            code: gap_codes::NOT_CONFIGURED.to_string(),
            capability: "search".to_string(),
            scope_id: String::new(),
            source_refs: Vec::new(),
            candidate_ids: Vec::new(),
            reason: "没有命中任何当前断言；可能是业务能力尚未采集".to_string(),
            required_evidence: vec!["采集 routes/frontend/api/database 等能力后重试".to_string()],
        });
    }
    let status = if total == 0 {
        ResponseStatus::Partial
    } else {
        ResponseStatus::Complete
    };
    envelope(
        context,
        request_id,
        status,
        Value::Object(data),
        gaps,
        source_references,
        completeness,
        BudgetUsed {
            entities: selected.len() as u64,
            relations: 0,
            visited: candidate_limit as u64,
            elapsed_ms: 0,
            response_chars: 0,
        },
        Vec::new(),
    )
}

struct ScoredHit {
    hit: storage::search::SearchHit,
    rank: u32,
    entity: crate::contract::entity::Entity,
    assertions: Vec<storage::entities::StoredEntityAssertion>,
}

/// 读取实体（含身份槽位）。
pub fn load_entity(
    connection: &Connection,
    entity_id: &str,
) -> Result<Option<crate::contract::entity::Entity>, CoreError> {
    let row: Option<(String, String, String)> = connection
        .query_row(
            "SELECT kind, scope_id, identity_json FROM entities WHERE id = ?1",
            rusqlite::params![entity_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .ok();
    let Some((kind_text, scope_id, identity_json)) = row else {
        return Ok(None);
    };
    let kind = EntityKind::parse(&kind_text).unwrap_or(EntityKind::File);
    let identity = crate::contract::identity::identity_slots_from_canonical(kind, &identity_json)
        .unwrap_or_default();
    Ok(Some(crate::contract::entity::Entity {
        id: entity_id.to_string(),
        kind,
        scope_id,
        identity,
        attributes: Map::new(),
        basis: Basis::Observed,
        origin: crate::contract::entity::Origin::Static,
        evidence_refs: Vec::new(),
        extensions: Map::new(),
    }))
}

fn entity_in_repositories(
    connection: &Connection,
    entity_id: &str,
    repository_ids: &[String],
) -> Result<bool, CoreError> {
    let sql = "SELECT DISTINCT s.repository_id
                 FROM relations r
                 JOIN entities fe ON fe.id = r.to_id AND fe.kind = 'file'
                 JOIN sources s ON s.id = fe.id
                WHERE r.kind = 'contains' AND r.from_id = ?1";
    let mut statement = connection.prepare(sql).map_err(|error| {
        CoreError::new(
            codes::INTERNAL_ERROR,
            format!("cannot prepare repository filter: {error}"),
        )
    })?;
    let rows = statement
        .query_map(rusqlite::params![entity_id], |row| row.get::<_, String>(0))
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot query repository filter: {error}"),
            )
        })?;
    for row in rows {
        let repository_id = row.map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot read repository filter row: {error}"),
            )
        })?;
        if repository_ids.iter().any(|id| id == &repository_id) {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn entity_ref_json(database: &Database, entity_id: &str) -> Result<Value, CoreError> {
    let snapshot = snapshot_ref(database)?;
    let mut value = Map::new();
    value.insert(
        "snapshot".to_string(),
        serde_json::to_value(&snapshot).unwrap_or(Value::Null),
    );
    value.insert("entityId".to_string(), Value::from(entity_id));
    Ok(Value::Object(value))
}

/// 读取来源。
fn execute_read_source(
    context: &mut QueryContext<'_>,
    request: &ReadSourceRequest,
    request_id: String,
) -> Result<QueryResponse<Value>, CoreError> {
    let max_chars = request
        .max_response_chars
        .unwrap_or(default_budget::MAX_RESPONSE_CHARS) as usize;
    let connection = context.database.connection();
    let mut reads: Vec<Value> = Vec::new();
    let mut used = 0usize;
    let mut gaps: Vec<Gap> = Vec::new();
    let mut source_references: Vec<SourceReference> = Vec::new();

    for reference in &request.references {
        let Some((repository_id, path)) =
            storage::sources::source_path(connection, &reference.source_id)?
        else {
            gaps.push(Gap {
                id: format!("gap_missing_source_{}", reference.source_id),
                code: gap_codes::SOURCE_SKIPPED.to_string(),
                capability: "readSource".to_string(),
                scope_id: String::new(),
                source_refs: vec![reference.evidence_id.clone()],
                candidate_ids: Vec::new(),
                reason: format!("来源 `{}` 不在索引中", reference.source_id),
                required_evidence: Vec::new(),
            });
            continue;
        };
        let root = context
            .project
            .and_then(|project| project.repositories.get(&repository_id).cloned())
            .unwrap_or_else(|| repository_id.clone().into());
        let stored = storage::sources::read_source_version(
            connection,
            &reference.source_id,
            &reference.content_hash,
        )?
        .map(|(bytes, _)| bytes);
        let read = crate::evidence::read::read_source(
            reference,
            &root,
            &path,
            stored.as_deref(),
            false,
            &context.now,
        )?;
        let mut entry = Map::new();
        entry.insert(
            "status".to_string(),
            Value::from(format!("{:?}", read.status).to_lowercase()),
        );
        entry.insert(
            "sourceId".to_string(),
            Value::from(reference.source_id.clone()),
        );
        entry.insert("path".to_string(), Value::from(path));
        let snippet = if read.snippet.is_empty() {
            read.stored_snippet.clone().unwrap_or_default()
        } else {
            read.snippet.clone()
        };
        let remaining = max_chars.saturating_sub(used);
        let snippet: String = snippet.chars().take(remaining).collect();
        used += snippet.chars().count();
        entry.insert("snippet".to_string(), Value::from(snippet));
        entry.insert(
            "lineMap".to_string(),
            read.line_map
                .as_ref()
                .map(|line_map| serde_json::to_value(line_map).unwrap_or(Value::Null))
                .unwrap_or(Value::Null),
        );
        entry.insert(
            "checkedAt".to_string(),
            Value::from(read.checked_at.clone()),
        );
        reads.push(Value::Object(entry));
        source_references.push(reference.clone());
        if used >= max_chars {
            break;
        }
    }

    let mut data = Map::new();
    data.insert("reads".to_string(), Value::Array(reads));
    let completeness = Completeness {
        freshness: Freshness::Unknown,
        disk_checked: true,
        checked_at: context.now.clone(),
        coverage: CoverageLevel::Complete,
        candidate_sets_complete: true,
        truncated: used >= max_chars,
        reasons: Vec::new(),
    };
    envelope(
        context,
        request_id,
        ResponseStatus::Complete,
        Value::Object(data),
        gaps,
        source_references,
        completeness,
        BudgetUsed {
            entities: 0,
            relations: 0,
            visited: request.references.len() as u64,
            elapsed_ms: 0,
            response_chars: used as u64,
        },
        Vec::new(),
    )
}

/// 链路追踪（有界 BFS）。
fn execute_trace(
    context: &mut QueryContext<'_>,
    request: &TraceRequest,
    request_id: String,
) -> Result<QueryResponse<Value>, CoreError> {
    let filters = Filters::from_request(request.filters.as_ref());
    let budget = resolve_budget(request.budget.as_ref());
    let connection = context.database.connection();
    let relation_kinds = request.relation_kinds.clone().unwrap_or_default();

    let mut visited: BTreeSet<String> = BTreeSet::new();
    let mut frontier: Vec<String> = request
        .roots
        .iter()
        .map(|root| root.entity_id.clone())
        .collect();
    let mut hops: Vec<Value> = Vec::new();
    let mut truncated = false;
    let mut depth = 0u32;

    while !frontier.is_empty() && depth < budget.max_depth {
        let mut next_frontier: Vec<String> = Vec::new();
        for entity_id in &frontier {
            if visited.len() as u64 >= budget.max_visited {
                truncated = true;
                break;
            }
            visited.insert(entity_id.clone());
            for direction in directions(request.direction) {
                let neighbors = storage::relations::load_neighbors(
                    connection,
                    entity_id,
                    direction,
                    &relation_kinds,
                    filters.include_candidates,
                    true,
                )?;
                for neighbor in neighbors {
                    if !filters.allows_basis(neighbor.basis) {
                        continue;
                    }
                    let other = if neighbor.from_id == *entity_id {
                        neighbor.to_id.clone()
                    } else {
                        neighbor.from_id.clone()
                    };
                    let mut hop = Map::new();
                    hop.insert("fromId".to_string(), Value::from(neighbor.from_id.clone()));
                    hop.insert("toId".to_string(), Value::from(neighbor.to_id.clone()));
                    hop.insert("kind".to_string(), Value::from(neighbor.kind.as_str()));
                    hop.insert("direction".to_string(), Value::from(direction));
                    hop.insert("depth".to_string(), Value::from(depth + 1));
                    hop.insert(
                        "resolution".to_string(),
                        Value::from(neighbor.resolution.as_str()),
                    );
                    hop.insert("basis".to_string(), Value::from(neighbor.basis.as_str()));
                    hop.insert(
                        "assertionId".to_string(),
                        Value::from(neighbor.assertion_id.clone()),
                    );
                    hops.push(Value::Object(hop));
                    if !visited.contains(&other) {
                        next_frontier.push(other);
                    }
                }
            }
        }
        frontier = next_frontier;
        depth += 1;
        if hops.len() as u64 >= budget.max_relations {
            truncated = true;
            break;
        }
    }

    let mut data = Map::new();
    data.insert("hops".to_string(), Value::Array(hops.clone()));
    data.insert(
        "roots".to_string(),
        Value::Array(
            request
                .roots
                .iter()
                .map(|root| serde_json::to_value(root).unwrap_or(Value::Null))
                .collect(),
        ),
    );
    data.insert("visited".to_string(), Value::from(visited.len() as u64));

    let mut reasons: Vec<String> = Vec::new();
    if truncated {
        reasons.push("预算截断：返回可达前沿，未提供完整路径".to_string());
    }
    let completeness = Completeness {
        freshness: Freshness::Unknown,
        disk_checked: false,
        checked_at: context.now.clone(),
        coverage: CoverageLevel::Partial,
        candidate_sets_complete: true,
        truncated,
        reasons,
    };
    envelope(
        context,
        request_id,
        if truncated {
            ResponseStatus::Partial
        } else {
            ResponseStatus::Complete
        },
        Value::Object(data),
        Vec::new(),
        Vec::new(),
        completeness,
        BudgetUsed {
            entities: visited.len() as u64,
            relations: hops.len() as u64,
            visited: visited.len() as u64,
            elapsed_ms: 0,
            response_chars: 0,
        },
        Vec::new(),
    )
}

fn directions(direction: Direction) -> Vec<&'static str> {
    match direction {
        Direction::In => vec!["in"],
        Direction::Out => vec!["out"],
        Direction::Both => vec!["out", "in"],
    }
}

/// 代码地图。
fn execute_code_map(
    context: &mut QueryContext<'_>,
    request: &CodeMapRequest,
    request_id: String,
) -> Result<QueryResponse<Value>, CoreError> {
    let filters = Filters::from_request(request.filters.as_ref());
    let max_files = request.max_files.unwrap_or(50) as usize;
    let max_symbols = request.max_symbols.unwrap_or(200) as usize;

    let mut entity_ids: Vec<String> = Vec::new();
    if let Some(query) = &request.query {
        let search = SearchRequest {
            query: query.clone(),
            fields: None,
            filters: None,
            page_size: Some(max_symbols.min(default_budget::PAGE_SIZE_MAX as usize) as u32),
            continuation: None,
            budget: None,
        };
        let response = execute_search(context, &search, format!("req_{}", uuid::Uuid::new_v4()))?;
        if let Some(results) = response.data.get("results").and_then(Value::as_array) {
            for item in results {
                if let Some(entity_ref) = item.get("entityRef")
                    && let Some(id) = entity_ref.get("entityId").and_then(Value::as_str)
                {
                    entity_ids.push(id.to_string());
                }
            }
        }
    }
    if let Some(roots) = &request.roots {
        for root in roots {
            entity_ids.push(root.entity_id.clone());
        }
    }

    let connection = context.database.connection();
    let mut files: Vec<Value> = Vec::new();
    let mut symbols: Vec<Value> = Vec::new();
    let mut seen_files: BTreeSet<String> = BTreeSet::new();
    for entity_id in &entity_ids {
        let Some(entity) = load_entity(connection, entity_id)? else {
            continue;
        };
        if !filters.allows_kind(entity.kind) {
            continue;
        }
        let assertions = storage::entities::load_entity_assertions(connection, entity_id, true)?;
        let name = assertions
            .iter()
            .find_map(|assertion| assertion.name.clone())
            .unwrap_or_default();
        let path = assertions
            .iter()
            .find_map(|assertion| assertion.attributes.get("path").and_then(Value::as_str))
            .unwrap_or_default()
            .to_string();
        if !path.is_empty() && !seen_files.contains(&path) && files.len() < max_files {
            seen_files.insert(path.clone());
            let mut file = Map::new();
            file.insert("path".to_string(), Value::from(path.clone()));
            file.insert("entityId".to_string(), Value::from(entity_id.clone()));
            files.push(Value::Object(file));
        }
        if symbols.len() < max_symbols {
            let mut symbol = Map::new();
            symbol.insert("entityId".to_string(), Value::from(entity_id.clone()));
            symbol.insert("kind".to_string(), Value::from(entity.kind.as_str()));
            symbol.insert("name".to_string(), Value::from(name));
            symbol.insert("path".to_string(), Value::from(path));
            symbols.push(Value::Object(symbol));
        }
    }

    let mut data = Map::new();
    data.insert("files".to_string(), Value::Array(files));
    data.insert("symbols".to_string(), Value::Array(symbols));
    data.insert("uncoveredLanguages".to_string(), Value::Array(Vec::new()));
    let completeness = Completeness {
        freshness: Freshness::Unknown,
        disk_checked: false,
        checked_at: context.now.clone(),
        coverage: CoverageLevel::Partial,
        candidate_sets_complete: true,
        truncated: false,
        reasons: Vec::new(),
    };
    envelope(
        context,
        request_id,
        ResponseStatus::Complete,
        Value::Object(data),
        Vec::new(),
        Vec::new(),
        completeness,
        BudgetUsed {
            entities: entity_ids.len() as u64,
            relations: 0,
            visited: entity_ids.len() as u64,
            elapsed_ms: 0,
            response_chars: 0,
        },
        Vec::new(),
    )
}

/// 需求定位（薄封装，事实组合在 `feature` 中实现）。
pub fn locate_feature(
    context: &mut QueryContext<'_>,
    request: &LocateFeatureRequest,
) -> Result<QueryResponse<Value>, CoreError> {
    feature::execute(context, request, format!("req_{}", uuid::Uuid::new_v4()))
}

/// 页面查询。
pub fn page(
    context: &mut QueryContext<'_>,
    request: &PageRequest,
) -> Result<QueryResponse<Value>, CoreError> {
    page::execute(context, request, format!("req_{}", uuid::Uuid::new_v4()))
}

/// 流程查询。
pub fn workflow(
    context: &mut QueryContext<'_>,
    request: &WorkflowRequest,
) -> Result<QueryResponse<Value>, CoreError> {
    workflow::execute(context, request, format!("req_{}", uuid::Uuid::new_v4()))
}

/// 台账查询。
pub fn report(
    context: &mut QueryContext<'_>,
    request: &ReportRequest,
) -> Result<QueryResponse<Value>, CoreError> {
    report::execute(context, request, format!("req_{}", uuid::Uuid::new_v4()))
}

/// 读取当前覆盖情况：能力 → 状态。
pub fn current_coverage(
    connection: &Connection,
) -> Result<std::collections::BTreeMap<String, String>, CoreError> {
    let mut result = std::collections::BTreeMap::new();
    for (replacement_key, batch_id) in storage::current_batches(connection)? {
        if !replacement_key.starts_with("(core.base,") {
            continue;
        }
        for (capability, _scope_id, _source_set_id, status) in
            storage::load_coverage(connection, &batch_id)?
        {
            result.insert(capability, status);
        }
    }
    Ok(result)
}

/// 尚未采集的业务能力 → 缺口。
///
/// 场景查询必须显式报告缺口，不能用“全部未知”冒充已支持。
pub fn missing_capability_gaps(context: &QueryContext<'_>, capability: &str) -> Vec<Gap> {
    let connection = context.database.connection();
    let coverage = match current_coverage(connection) {
        Ok(coverage) => coverage,
        Err(_) => return Vec::new(),
    };
    let mut gaps = Vec::new();
    let status = coverage.get(capability);
    match status {
        Some(status) if status == "complete" => {}
        Some(status) => gaps.push(Gap {
            id: format!("gap_capability_{capability}"),
            code: gap_codes::NOT_CONFIGURED.to_string(),
            capability: capability.to_string(),
            scope_id: String::new(),
            source_refs: Vec::new(),
            candidate_ids: Vec::new(),
            reason: format!("能力 `{capability}` 当前覆盖为 {status}"),
            required_evidence: vec![format!("采集 `{capability}` 能力并激活对应插件")],
        }),
        None => gaps.push(Gap {
            id: format!("gap_capability_{capability}"),
            code: gap_codes::NOT_CONFIGURED.to_string(),
            capability: capability.to_string(),
            scope_id: String::new(),
            source_refs: Vec::new(),
            candidate_ids: Vec::new(),
            reason: format!("能力 `{capability}` 尚未采集"),
            required_evidence: vec![format!("采集 `{capability}` 能力并激活对应插件")],
        }),
    }
    gaps
}

/// 场景查询的通用骨架：搜索 + 缺口 + 覆盖说明。
pub fn scenario_skeleton(
    context: &mut QueryContext<'_>,
    query_text: &str,
    capabilities: &[&str],
    extra: Map<String, Value>,
    request_id: String,
) -> Result<QueryResponse<Value>, CoreError> {
    let filters = Filters::from_request(None);
    let budget = resolve_budget(None);
    let connection = context.database.connection();
    let mut gaps: Vec<Gap> = Vec::new();
    let mut results: Vec<Value> = Vec::new();

    if let Some(expression) = fts_query(query_text) {
        let hits = storage::search::search_documents(connection, &expression, 200)?;
        let mut seen: BTreeSet<String> = BTreeSet::new();
        for hit in hits {
            if !seen.insert(hit.entity_id.clone()) {
                continue;
            }
            let Some(entity) = load_entity(connection, &hit.entity_id)? else {
                continue;
            };
            if !filters.allows_kind(entity.kind) {
                continue;
            }
            let assertions =
                storage::entities::load_entity_assertions(connection, &hit.entity_id, true)?;
            let mut entry = Map::new();
            entry.insert(
                "entityRef".to_string(),
                entity_ref_json(context.database, &hit.entity_id)?,
            );
            entry.insert("kind".to_string(), Value::from(entity.kind.as_str()));
            entry.insert("scopeId".to_string(), Value::from(entity.scope_id.clone()));
            entry.insert(
                "name".to_string(),
                assertions
                    .iter()
                    .find_map(|assertion| assertion.name.clone())
                    .map(Value::from)
                    .unwrap_or(Value::Null),
            );
            entry.insert(
                "path".to_string(),
                assertions
                    .iter()
                    .find_map(|assertion| assertion.attributes.get("path").and_then(Value::as_str))
                    .map(Value::from)
                    .unwrap_or(Value::Null),
            );
            results.push(Value::Object(entry));
            if results.len() as u64 >= budget.max_entities {
                break;
            }
        }
    }

    for capability in capabilities {
        gaps.extend(missing_capability_gaps(context, capability));
    }

    let mut data = extra;
    data.insert("matches".to_string(), Value::Array(results.clone()));
    data.insert("matchCount".to_string(), Value::from(results.len() as u64));
    let coverage = current_coverage(connection)?;
    let mut coverage_value = Map::new();
    for (capability, status) in coverage {
        coverage_value.insert(capability, Value::from(status));
    }
    data.insert("coverage".to_string(), Value::Object(coverage_value));

    let status = if gaps.is_empty() {
        ResponseStatus::Complete
    } else {
        ResponseStatus::Partial
    };
    let completeness = Completeness {
        freshness: Freshness::Unknown,
        disk_checked: false,
        checked_at: context.now.clone(),
        coverage: if gaps.is_empty() {
            CoverageLevel::Complete
        } else {
            CoverageLevel::Partial
        },
        candidate_sets_complete: true,
        truncated: false,
        reasons: gaps.iter().map(|gap| gap.reason.clone()).collect(),
    };
    envelope(
        context,
        request_id,
        status,
        Value::Object(data),
        gaps,
        Vec::new(),
        completeness,
        BudgetUsed {
            entities: results.len() as u64,
            relations: 0,
            visited: results.len() as u64,
            elapsed_ms: 0,
            response_chars: 0,
        },
        Vec::new(),
    )
}
