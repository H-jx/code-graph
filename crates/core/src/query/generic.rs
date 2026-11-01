//! 通用查询组合；搜索分页与直接邻接分页独立于多层路径。
use super::{
    Filters, QueryContext, envelope,
    runtime::{self, Runtime},
    snapshot_ref,
};
use crate::contract::error::{CoreError, codes};
use crate::contract::evidence::SourceReference;
use crate::contract::query::*;
use crate::graph::neighbors;
use crate::search::rank::{exact_rank, fts_query, match_reason};
use crate::storage;
use rusqlite::params;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub fn used(entities: usize, relations: usize) -> BudgetUsed {
    BudgetUsed {
        entities: entities as u64,
        relations: relations as u64,
        visited: 0,
        elapsed_ms: 0,
        response_chars: 0,
    }
}

type SearchKey = (u32, f64, String, String);
fn compare(a: &SearchKey, b: &SearchKey) -> std::cmp::Ordering {
    b.0.cmp(&a.0)
        .then(a.1.total_cmp(&b.1))
        .then(a.2.cmp(&b.2))
        .then(a.3.cmp(&b.3))
}
pub fn search(
    context: &QueryContext<'_>,
    request: &SearchRequest,
    id: String,
    rt: &mut Runtime,
) -> Result<QueryResponse<Value>, CoreError> {
    if request.query.trim().is_empty() || request.query.len() > 4096 || request.page_size == Some(0)
    {
        return Err(CoreError::new(
            codes::INVALID_REQUEST,
            "search needs a nonempty query and positive pageSize",
        ));
    }
    let filters = Filters::from_request(request.filters.as_ref());
    let snapshot = snapshot_ref(context.database)?;
    let hash = runtime::request_hash(request)?;
    let after: Option<SearchKey> =
        runtime::decode_cursor(request.continuation.as_deref(), &snapshot, &hash)?
            .map(serde_json::from_value)
            .transpose()
            .map_err(|_| CoreError::new(codes::INVALID_REQUEST, "invalid search key"))?;
    if after
        .as_ref()
        .is_some_and(|k| k.0 > 4 || !k.1.is_finite() || k.2.len() > 128 || k.3.len() > 128)
    {
        return Err(CoreError::new(codes::INVALID_REQUEST, "invalid search key"));
    }
    let connection = context.database.connection();
    let expression = fts_query(&request.query).unwrap_or_else(|| "\"__pk_empty_token__\"".into());
    // The route branch uses the scope/route index; FTS scores and exact hits are merged before ranking.
    let sql="WITH hits AS MATERIALIZED (
        SELECT d.rowid,bm25(search_fts) score FROM search_fts JOIN search_documents d ON d.rowid=search_fts.rowid WHERE search_fts MATCH ?1
        UNION ALL SELECT d.rowid,0.0 FROM search_documents d WHERE d.scope_id IN (SELECT id FROM scopes) AND d.route_exact=?2
        UNION ALL SELECT d.rowid,0.0 FROM search_documents d WHERE d.entity_id=?2
        UNION ALL SELECT d.rowid,0.0 FROM search_documents d JOIN entities e ON e.id=d.entity_id WHERE e.identity_json=?2)
        SELECT d.assertion_id,d.entity_id,d.text,d.route_exact,d.projection_kind,min(h.score)
        FROM hits h JOIN search_documents d ON d.rowid=h.rowid JOIN assertions a ON a.id=d.assertion_id
        JOIN current_batches cb ON cb.batch_id=a.batch_id JOIN batches b ON b.id=cb.batch_id
        WHERE (?3 OR b.state='fresh') GROUP BY d.rowid ORDER BY d.entity_id,d.assertion_id,d.projection_kind LIMIT ?4";
    let mut stmt = connection.prepare(sql).map_err(storage::database_error)?;
    let rows = stmt
        .query_map(
            params![
                expression,
                request.query,
                filters.freshness_policy == FreshnessPolicy::AllowStale,
                (rt.budget.max_visited + 1) as i64
            ],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, f64>(5)?,
                ))
            },
        )
        .map_err(storage::database_error)?;
    let mut selected: BTreeMap<(String, String), (SearchKey, Value)> = BTreeMap::new();
    let mut incomplete = false;
    for row in rows {
        if !rt.visit() {
            incomplete = true;
            break;
        }
        let (assertion, entity, text, route, projection, score) =
            row.map_err(storage::database_error)?;
        if !runtime::assertion_visible(connection, &assertion, &filters)? {
            continue;
        }
        let Some(mut value) = runtime::entity_projection(connection, &entity, &filters, &snapshot)?
        else {
            continue;
        };
        let Some(matched) = value["assertions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["assertionId"] == assertion)
            .cloned()
        else {
            continue;
        };
        let field_text = match &request.fields {
            None => text.clone(),
            Some(fields) => fields
                .iter()
                .filter_map(|field| match field {
                    SearchField::Source if projection == "source" => Some(text.clone()),
                    SearchField::Comment => matched["attributes"]
                        .get("comment")
                        .or_else(|| matched["attributes"].get("docComment"))
                        .map(Value::to_string)
                        .or_else(|| {
                            if projection == "source" {
                                source_comments(&text)
                            } else {
                                None
                            }
                        }),
                    SearchField::Title => matched["attributes"].get("title").map(Value::to_string),
                    SearchField::Route => route.clone(),
                    SearchField::Path => matched["attributes"].get("path").map(Value::to_string),
                    SearchField::Symbol
                        if matches!(
                            value["kind"].as_str(),
                            Some(
                                "symbol"
                                    | "function"
                                    | "method"
                                    | "class"
                                    | "component"
                                    | "operation"
                            )
                        ) =>
                    {
                        Some(matched["name"].to_string())
                    }
                    SearchField::Module
                        if value["kind"] == "module"
                            || matched["attributes"]["symbolKind"] == "module" =>
                    {
                        Some(matched["name"].to_string())
                    }
                    SearchField::BusinessTerm => {
                        Some(format!("{} {}", matched["name"], matched["attributes"]))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n"),
        };
        let identity: String = connection
            .query_row(
                "SELECT identity_json FROM entities WHERE id=?1",
                [&entity],
                |r| r.get(0),
            )
            .map_err(storage::database_error)?;
        let rank = if request.query == entity || identity == request.query {
            3
        } else {
            exact_rank(
                &request.query,
                &field_text,
                if request
                    .fields
                    .as_ref()
                    .is_none_or(|f| f.contains(&SearchField::Route))
                {
                    route.as_deref()
                } else {
                    None
                },
            )
        };
        if rank == 0 {
            continue;
        }
        let key = (rank, score, entity.clone(), assertion.clone());
        value["matchReason"] = json!(match_reason(rank));
        value["exactRank"] = json!(rank);
        value["bm25"] = json!(score);
        value["projection"] = json!(projection);
        // Return only the matched, permitted assertion; other producers cannot leak through its FTS hit.
        value["assertions"] = json!([matched.clone()]);
        value["name"] = value["assertions"][0]["name"].clone();
        let slot = selected
            .entry((entity, assertion))
            .or_insert_with(|| (key.clone(), value.clone()));
        if compare(&key, &slot.0).is_lt() {
            *slot = (key, value);
        }
    }
    let mut selected = selected.into_values().collect::<Vec<_>>();
    selected.sort_by(|a, b| compare(&a.0, &b.0));
    if let Some(after) = &after {
        selected.retain(|(key, _)| compare(key, after).is_gt());
    }
    let page_size = request
        .page_size
        .unwrap_or(default_budget::PAGE_SIZE_DEFAULT)
        .min(default_budget::PAGE_SIZE_MAX) as usize;
    let size = page_size.min(rt.budget.max_entities as usize);
    let more = selected.len() > size;
    selected.truncate(size);
    let continuation = if more && !incomplete {
        selected
            .last()
            .map(|(key, _)| runtime::encode_cursor(&snapshot, &hash, json!(key)))
    } else {
        None
    };
    let mut refs = Vec::new();
    for (key, _) in &selected {
        refs.extend(runtime::references(connection, &key.3, &snapshot)?);
    }
    dedup_refs(&mut refs);
    let count = selected.len();
    let completeness = rt.completeness(context, &filters, incomplete || more)?;
    let mut response = envelope(
        context,
        id,
        ResponseStatus::Complete,
        json!({"results":selected.into_iter().map(|(_,v)|v).collect::<Vec<_>>(),"fields":request.fields,"candidateSetComplete":!incomplete}),
        Vec::new(),
        refs,
        completeness,
        used(count, 0),
        Vec::new(),
    )?;
    response.continuation = continuation;
    if !incomplete && count > 0 {
        rt.pagination = Some(runtime::Pagination {
            snapshot,
            hash,
            neighbors: false,
        });
    }
    Ok(response)
}

#[cfg(feature = "syntax")]
fn source_comments(projection: &str) -> Option<String> {
    let (path, text) = projection.split_once('\n')?;
    let language = crate::syntax::parser::detect_language(std::path::Path::new(path));
    let tree = crate::syntax::parser::parse_source(language, text)?;
    let mut nodes = vec![tree.root_node()];
    let mut comments = Vec::new();
    while let Some(node) = nodes.pop() {
        if matches!(node.kind(), "comment" | "line_comment" | "block_comment") {
            comments.push(node.utf8_text(text.as_bytes()).ok()?.to_string());
        } else {
            nodes.extend(node.children(&mut node.walk()));
        }
    }
    Some(comments.join("\n"))
}
#[cfg(not(feature = "syntax"))]
fn source_comments(_projection: &str) -> Option<String> {
    None
}

pub fn trace(
    context: &QueryContext<'_>,
    request: &TraceRequest,
    id: String,
    rt: &mut Runtime,
) -> Result<QueryResponse<Value>, CoreError> {
    if request.roots.len() > 1000 {
        return Err(CoreError::new(codes::INVALID_REQUEST, "too many roots"));
    }
    let walk = crate::graph::paths::walk(context, request, rt)?;
    let snapshot = snapshot_ref(context.database)?;
    let frontier = walk
        .frontier
        .iter()
        .map(|id| json!({"snapshot":snapshot,"entityId":id}))
        .collect::<Vec<_>>();
    let next = if walk.truncated {
        vec![NextQuery {operation:"trace".into(),arguments:json!({"roots":frontier,"direction":request.direction,"maxDepth":1,"filters":request.filters}).as_object().unwrap().clone(),reason:"narrow the traversal from a returned frontier".into()}]
    } else {
        Vec::new()
    };
    let completeness = rt.completeness(
        context,
        &Filters::from_request(request.filters.as_ref()),
        walk.truncated,
    )?;
    envelope(
        context,
        id,
        ResponseStatus::Complete,
        json!({"roots":request.roots,"hops":walk.hops,"entities":walk.entities,"frontier":frontier}),
        Vec::new(),
        walk.references,
        completeness,
        used(walk.entities.len(), walk.hops.len()),
        next,
    )
}

pub fn adjacent(
    context: &QueryContext<'_>,
    request: &NeighborsRequest,
    id: String,
    rt: &mut Runtime,
) -> Result<QueryResponse<Value>, CoreError> {
    if request.page_size == Some(0) {
        return Err(CoreError::new(
            codes::INVALID_REQUEST,
            "pageSize must be positive",
        ));
    }
    let snapshot = snapshot_ref(context.database)?;
    runtime::validate_entity_reference(context.database.connection(), &request.root, &snapshot)?;
    let filters = Filters::from_request(request.filters.as_ref());
    let hash = runtime::request_hash(request)?;
    let after = runtime::decode_cursor(request.continuation.as_deref(), &snapshot, &hash)?
        .map(neighbors::decode_key)
        .transpose()?;
    let connection = context.database.connection();
    let size = request
        .page_size
        .unwrap_or(default_budget::PAGE_SIZE_DEFAULT)
        .min(default_budget::PAGE_SIZE_MAX) as usize;
    let size = size.min(rt.budget.max_relations as usize);
    let mut selected = Vec::new();
    let mut refs = Vec::new();
    let mut last = None;
    let mut more = false;
    let mut incomplete = false;
    if runtime::entity_allowed(connection, &request.root.entity_id, &filters)? {
        let rows = neighbors::load(
            connection,
            &request.root.entity_id,
            request.direction,
            request.relation_kinds.as_deref().unwrap_or_default(),
            &filters,
            after.as_ref(),
            rt.budget.max_visited as usize + 1,
        )?;
        for mut row in rows {
            if !rt.visit() {
                incomplete = true;
                break;
            }
            if !runtime::entity_allowed(connection, &row.other, &filters)? {
                continue;
            }
            if selected.len() >= size {
                more = true;
                break;
            }
            refs.extend(neighbors::attach_evidence(
                connection,
                &mut row.value,
                &snapshot,
            )?);
            last = Some(row.key);
            selected.push(row.value);
        }
    }
    dedup_refs(&mut refs);
    let count = selected.len();
    let completeness = rt.completeness(context, &filters, incomplete || more)?;
    let mut response = envelope(
        context,
        id,
        ResponseStatus::Complete,
        json!({"root":request.root,"hops":selected}),
        Vec::new(),
        refs,
        completeness,
        used(0, count),
        Vec::new(),
    )?;
    if more && !incomplete {
        response.continuation =
            last.map(|key| runtime::encode_cursor(&snapshot, &hash, json!(key)));
    }
    if !incomplete && count > 0 {
        rt.pagination = Some(runtime::Pagination {
            snapshot,
            hash,
            neighbors: true,
        });
    }
    Ok(response)
}

pub fn dedup_refs(refs: &mut Vec<SourceReference>) {
    refs.sort_by(|a, b| a.evidence_id.cmp(&b.evidence_id));
    refs.dedup_by(|a, b| a.evidence_id == b.evidence_id);
}

pub fn code_map(
    context: &QueryContext<'_>,
    request: &CodeMapRequest,
    id: String,
    rt: &mut Runtime,
) -> Result<QueryResponse<Value>, CoreError> {
    let mut roots = BTreeSet::new();
    let mut roots_truncated = false;
    let snapshot = snapshot_ref(context.database)?;
    for root in request.roots.as_deref().unwrap_or_default() {
        runtime::validate_entity_reference(context.database.connection(), root, &snapshot)?;
        roots.insert(root.entity_id.clone());
    }
    if request.query.is_none() && request.roots.is_none() {
        let kinds = request
            .filters
            .as_ref()
            .and_then(|f| f.kinds.clone())
            .unwrap_or_else(|| vec![crate::contract::entity::EntityKind::File]);
        let mut statement=context.database.connection().prepare("SELECT DISTINCT e.id FROM entities e JOIN entity_assertions ea ON ea.entity_id=e.id JOIN current_batches cb ON cb.batch_id=ea.batch_id WHERE e.kind IN (SELECT value FROM json_each(?1)) ORDER BY e.id LIMIT ?2").map_err(storage::database_error)?;
        let rows = statement
            .query_map(
                rusqlite::params![
                    serde_json::to_string(&kinds).unwrap(),
                    (rt.budget.max_visited + 1) as i64
                ],
                |r| r.get::<_, String>(0),
            )
            .map_err(storage::database_error)?;
        for row in rows {
            if !rt.visit() {
                roots_truncated = true;
                break;
            }
            let id = row.map_err(storage::database_error)?;
            if runtime::entity_allowed(
                context.database.connection(),
                &id,
                &Filters::from_request(request.filters.as_ref()),
            )? {
                if roots.len() as u64 >= rt.budget.max_entities {
                    roots_truncated = true;
                    break;
                }
                roots.insert(id);
            }
        }
    }
    if let Some(query) = &request.query {
        let response = search(
            context,
            &SearchRequest {
                query: query.clone(),
                fields: None,
                filters: request.filters.clone(),
                page_size: Some(100),
                continuation: None,
                budget: request.budget.clone(),
            },
            id.clone(),
            rt,
        )?;
        roots_truncated = response.completeness.truncated;
        for v in response.data["results"].as_array().unwrap() {
            if let Some(id) = v["entityRef"]["entityId"].as_str() {
                roots.insert(id.into());
            }
        }
    }
    let mut ranked = crate::codemap::rank::select(
        context,
        &roots,
        &Filters::from_request(request.filters.as_ref()),
        rt,
    )?;
    ranked.truncated |= roots_truncated;
    rt.pagination = None;
    let rendered = crate::codemap::render::render(context, ranked, request, rt)?;
    let mut completeness = rt.completeness(
        context,
        &Filters::from_request(request.filters.as_ref()),
        rendered.truncated,
    )?;
    if !rendered.uncovered.is_empty() {
        completeness.coverage = CoverageLevel::Partial;
        completeness
            .reasons
            .push("some selected files have no syntax outline".into());
    }
    envelope(
        context,
        id,
        ResponseStatus::Complete,
        json!({"files":rendered.files,"symbols":rendered.symbols,"uncoveredLanguages":rendered.uncovered}),
        Vec::new(),
        rendered.references,
        completeness,
        used(rendered.symbols.len(), 0),
        Vec::new(),
    )
}
