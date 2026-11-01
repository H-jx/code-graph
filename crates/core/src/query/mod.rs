//! 查询入口（计划 §3.7）。
//!
//! Core 暴露统一的 `QueryRequest` tagged union；CLI、SDK 与 MCP 使用同一 DTO。
//! 查询入口只读：不建库、不迁移、不运行插件、不调用模型。

use rusqlite::{Connection, OptionalExtension};
use serde_json::{Map, Value};

use crate::contract::PROTOCOL_VERSION;
use crate::contract::entity::{Basis, EntityKind};
use crate::contract::error::{CoreError, codes, gap_codes};
use crate::contract::evidence::{SnapshotRef, SourceReference};
use crate::contract::query::{
    Budget, BudgetUsed, Completeness, CoverageLevel, Freshness, FreshnessPolicy, Gap,
    LocateFeatureRequest, NextQuery, PageRequest, QueryRequest, QueryResponse, ReadSourceRequest,
    ReportRequest, ResponseStatus, StatusRequest, WorkflowRequest, default_budget,
};
use crate::project::config::LoadedProject;
use crate::storage;
use crate::storage::database::Database;

pub mod feature;
pub(crate) mod generic;
pub mod page;
pub mod report;
pub mod runtime;
mod scenario;
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
    execute_with_cancellation(context, request, tokio_util::sync::CancellationToken::new())
}

/// Core 计算和 SQLite VM 共用取消令牌；调用方持有令牌即可停止运行中请求。
pub fn execute_with_cancellation(
    context: &mut QueryContext<'_>,
    request: &QueryRequest,
    cancel: tokio_util::sync::CancellationToken,
) -> Result<QueryResponse<Value>, CoreError> {
    execute_with_request_id(
        context,
        request,
        cancel,
        format!("req_{}", uuid::Uuid::new_v4()),
    )
}

/// Transports register this ID before scheduling work and use it for cancellation.
pub fn execute_with_request_id(
    context: &mut QueryContext<'_>,
    request: &QueryRequest,
    cancel: tokio_util::sync::CancellationToken,
    request_id: String,
) -> Result<QueryResponse<Value>, CoreError> {
    if request_id.is_empty() || request_id.len() > 128 {
        return Err(CoreError::new(codes::INVALID_REQUEST, "invalid request ID"));
    }
    if cancel.is_cancelled() {
        return Err(CoreError::new(codes::CANCELLED, "query cancelled"));
    }
    if serde_json::to_vec(request)
        .map_err(|_| CoreError::new(codes::INVALID_REQUEST, "invalid request"))?
        .len()
        > 1_048_576
    {
        return Err(CoreError::new(
            codes::INVALID_REQUEST,
            "query request exceeds 1 MiB",
        ));
    }
    let requested = match request {
        QueryRequest::Search(r) => r.budget.as_ref(),
        QueryRequest::Trace(r) => r.budget.as_ref(),
        QueryRequest::Neighbors(r) => r.budget.as_ref(),
        QueryRequest::CodeMap(r) => r.budget.as_ref(),
        QueryRequest::LocateFeature(r) => r.budget.as_ref(),
        QueryRequest::Page(r) => r.budget.as_ref(),
        QueryRequest::Workflow(r) => r.budget.as_ref(),
        QueryRequest::Report(r) => r.budget.as_ref(),
        _ => None,
    };
    let mut budget = resolve_budget(requested);
    if let QueryRequest::ReadSource(r) = request {
        budget.max_response_chars = r
            .max_response_chars
            .unwrap_or(budget.max_response_chars)
            .min(default_budget::HARD_MAX_RESPONSE_CHARS);
    }
    if budget.max_response_chars < 1024 {
        return Err(CoreError::new(
            codes::BUDGET_EXHAUSTED,
            "response budget is too small for the query envelope",
        ));
    }
    let mut rt = runtime::Runtime::new(budget, cancel);
    let deadline = rt.deadline;
    let cancellation = rt.cancel.clone();
    let previous_busy_ms: u64 = context
        .database
        .connection()
        .query_row("PRAGMA busy_timeout", [], |r| r.get::<_, i64>(0))
        .map_err(storage::database_error)? as u64;
    let previous_query_only: bool = context
        .database
        .connection()
        .query_row("PRAGMA query_only", [], |r| r.get(0))
        .map_err(storage::database_error)?;
    if !context.database.connection().is_autocommit() {
        return Err(CoreError::new(
            codes::INVALID_REQUEST,
            "query requires its own read transaction",
        ));
    }
    context
        .database
        .connection()
        .busy_timeout(std::time::Duration::from_millis(budget.timeout_ms))
        .map_err(storage::database_error)?;
    context
        .database
        .connection()
        .execute_batch("PRAGMA query_only=ON; BEGIN DEFERRED")
        .map_err(storage::database_error)?;
    context
        .database
        .query_connection_mut()
        .progress_handler(
            500,
            Some(move || cancellation.is_cancelled() || std::time::Instant::now() >= deadline),
        )
        .map_err(storage::database_error)?;
    let fallback_request_id = request_id.clone();
    let result = (|| {
        // Pin the generation before any projection is read.
        snapshot_ref(context.database)?;
        match request {
            QueryRequest::Status(inner) => execute_status(context, inner, request_id, &mut rt),
            QueryRequest::Search(inner) => generic::search(context, inner, request_id, &mut rt),
            QueryRequest::ReadSource(inner) => {
                execute_read_source(context, inner, request_id, &mut rt)
            }
            QueryRequest::Trace(inner) => generic::trace(context, inner, request_id, &mut rt),
            QueryRequest::Neighbors(inner) => {
                generic::adjacent(context, inner, request_id, &mut rt)
            }
            QueryRequest::CodeMap(inner) => generic::code_map(context, inner, request_id, &mut rt),
            QueryRequest::LocateFeature(inner) => {
                feature::execute(context, inner, request_id, &mut rt)
            }
            QueryRequest::Page(inner) => page::execute(context, inner, request_id, &mut rt),
            QueryRequest::Workflow(inner) => workflow::execute(context, inner, request_id, &mut rt),
            QueryRequest::Report(inner) => report::execute(context, inner, request_id, &mut rt),
        }
    })();
    context
        .database
        .query_connection_mut()
        .progress_handler(0, None::<fn() -> bool>)
        .map_err(storage::database_error)?;
    context
        .database
        .connection()
        .execute_batch("ROLLBACK")
        .map_err(storage::database_error)?;
    if !previous_query_only {
        context
            .database
            .connection()
            .execute_batch("PRAGMA query_only=OFF")
            .map_err(storage::database_error)?;
    }
    context
        .database
        .connection()
        .busy_timeout(std::time::Duration::from_millis(previous_busy_ms))
        .map_err(storage::database_error)?;
    match result {
        Ok(mut response) => {
            if response.completeness.freshness == Freshness::Stale
                && !matches!(
                    request,
                    QueryRequest::Status(_) | QueryRequest::ReadSource(_)
                )
            {
                if let Some(data) = response.data.as_object_mut() {
                    for (key, value) in data.iter_mut() {
                        if value.is_array() {
                            *value = serde_json::json!([]);
                        } else if !matches!(key.as_str(), "query" | "route" | "routeHint") {
                            *value = Value::Null;
                        }
                    }
                }
                response.source_references.clear();
                response.next_queries.clear();
                response.continuation = None;
                response.budget_used.entities = 0;
                response.budget_used.relations = 0;
            }
            rt.finish(response)
        }
        Err(_) if rt.cancel.is_cancelled() => {
            Err(CoreError::new(codes::CANCELLED, "query cancelled"))
        }
        Err(_) if rt.stopped() => {
            let response = envelope(
                context,
                fallback_request_id,
                ResponseStatus::Partial,
                serde_json::json!({"frontier":[]}),
                Vec::new(),
                Vec::new(),
                Completeness {
                    freshness: Freshness::Unknown,
                    disk_checked: false,
                    checked_at: context.now.clone(),
                    coverage: CoverageLevel::Partial,
                    candidate_sets_complete: false,
                    truncated: true,
                    reasons: vec!["query deadline reached".into()],
                },
                generic::used(0, 0),
                Vec::new(),
            )?;
            rt.finish(response)
        }
        Err(error) => Err(error),
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
    rt: &mut runtime::Runtime,
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
    if request.check_freshness {
        let check = rt.completeness(context, &Filters::from_request(None), false)?;
        freshest = check.freshness;
        disk_checked = check.disk_checked;
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
    data.insert(
        "knowledge".to_string(),
        crate::knowledge::documents::status_with_budget(
            connection,
            context.project,
            request.check_freshness,
            &mut || rt.visit(),
        )?,
    );
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
        .optional()
        .map_err(storage::database_error)?;
    let Some((kind_text, scope_id, identity_json)) = row else {
        return Ok(None);
    };
    let kind = EntityKind::parse(&kind_text)
        .ok_or_else(|| CoreError::new(codes::INTERNAL_ERROR, "invalid stored entity kind"))?;
    let identity =
        crate::contract::identity::identity_slots_from_canonical(kind.clone(), &identity_json)
            .map_err(|_| CoreError::new(codes::INTERNAL_ERROR, "invalid stored entity identity"))?;
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
    rt: &mut runtime::Runtime,
) -> Result<QueryResponse<Value>, CoreError> {
    let max_chars = request
        .max_response_chars
        .unwrap_or(default_budget::MAX_RESPONSE_CHARS) as usize;
    let connection = context.database.connection();
    let mut reads: Vec<Value> = Vec::new();
    let mut used = 0usize;
    let snippet_budget = max_chars.saturating_sub(2048);
    let mut gaps: Vec<Gap> = Vec::new();
    let mut source_references: Vec<SourceReference> = Vec::new();
    let snapshot = snapshot_ref(context.database)?;
    let filters = Filters::from_request(request.filters.as_ref());
    let allow_stale = filters.freshness_policy == FreshnessPolicy::AllowStale;
    let mut freshness = Freshness::Fresh;
    let mut truncated = false;

    for reference in &request.references {
        runtime::validate_snapshot(&reference.snapshot, &snapshot)?;
        storage::sources::validate_original_reference(connection, reference)?;
        if !rt.visit() {
            truncated = true;
            break;
        }
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
        if filters
            .repository_ids
            .as_ref()
            .is_some_and(|ids| !ids.contains(&repository_id))
        {
            continue;
        }
        let root = context
            .project
            .and_then(|project| project.repositories.get(&repository_id).cloned())
            .ok_or_else(|| {
                CoreError::new(
                    codes::CONFIG_INVALID,
                    "source repository root is unavailable",
                )
            })?;
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
            allow_stale,
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
        if read.status != crate::contract::evidence::SourceReadStatus::Current {
            freshness = Freshness::Stale;
        }
        let snippet = if allow_stale && read.snippet.is_empty() {
            read.stored_snippet.clone().unwrap_or_default()
        } else {
            read.snippet.clone()
        };
        let remaining = snippet_budget.saturating_sub(used);
        let snippet_truncated = snippet.chars().count() > remaining;
        if snippet.chars().count() > remaining {
            truncated = true;
        }
        let snippet: String = snippet.chars().take(remaining).collect();
        used += snippet.chars().count();
        entry.insert(
            "evidenceId".into(),
            Value::from(reference.evidence_id.clone()),
        );
        entry.insert("startByte".into(), Value::from(reference.start_byte));
        entry.insert(
            "returnedEndByte".into(),
            Value::from(reference.start_byte + snippet.len() as u64),
        );
        entry.insert("snippetTruncated".into(), Value::from(snippet_truncated));
        entry.insert("snippet".to_string(), Value::from(snippet));
        entry.insert("lineMap".to_string(),read.line_map.as_ref().map(|m|serde_json::json!({"startLine":m.line_of(reference.start_byte),"endLine":m.end_line_of(reference.start_byte,reference.end_byte)})).unwrap_or(Value::Null));
        entry.insert(
            "storedVersion".into(),
            serde_json::json!(read.stored_version),
        );
        entry.insert(
            "checkedAt".to_string(),
            Value::from(read.checked_at.clone()),
        );
        reads.push(Value::Object(entry));
        source_references.push(reference.clone());
        if used >= snippet_budget {
            break;
        }
    }

    let mut data = Map::new();
    data.insert("reads".to_string(), Value::Array(reads));
    let completeness = Completeness {
        freshness,
        disk_checked: true,
        checked_at: context.now.clone(),
        coverage: CoverageLevel::Complete,
        candidate_sets_complete: true,
        truncated: truncated || source_references.len() < request.references.len(),
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

/// 需求定位（薄封装，事实组合在 `feature` 中实现）。
pub fn locate_feature(
    context: &mut QueryContext<'_>,
    request: &LocateFeatureRequest,
) -> Result<QueryResponse<Value>, CoreError> {
    execute(context, &QueryRequest::LocateFeature(request.clone()))
}

/// 页面查询。
pub fn page(
    context: &mut QueryContext<'_>,
    request: &PageRequest,
) -> Result<QueryResponse<Value>, CoreError> {
    execute(context, &QueryRequest::Page(request.clone()))
}

/// 流程查询。
pub fn workflow(
    context: &mut QueryContext<'_>,
    request: &WorkflowRequest,
) -> Result<QueryResponse<Value>, CoreError> {
    execute(context, &QueryRequest::Workflow(request.clone()))
}

/// 台账查询。
pub fn report(
    context: &mut QueryContext<'_>,
    request: &ReportRequest,
) -> Result<QueryResponse<Value>, CoreError> {
    execute(context, &QueryRequest::Report(request.clone()))
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
