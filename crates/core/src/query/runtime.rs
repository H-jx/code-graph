//! 查询生命周期、严格游标、预算与只读来源集合核验。
use std::time::{Duration, Instant};

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use super::{Filters, QueryContext, ResolvedBudget};
use crate::contract::error::{CoreError, codes};
use crate::contract::evidence::{SnapshotRef, SourceReference};
use crate::contract::identity::content_hash;
use crate::contract::query::{
    Completeness, CoverageLevel, Freshness, FreshnessPolicy, QueryResponse, ResponseStatus,
};
use crate::storage;

pub struct Runtime {
    pub budget: ResolvedBudget,
    pub start: Instant,
    pub deadline: Instant,
    pub cancel: CancellationToken,
    pub visited: u64,
    pub pagination: Option<Pagination>,
}

pub struct Pagination {
    pub snapshot: SnapshotRef,
    pub hash: String,
    pub neighbors: bool,
}

impl Runtime {
    pub fn new(budget: ResolvedBudget, cancel: CancellationToken) -> Self {
        let start = Instant::now();
        Self {
            budget,
            start,
            deadline: start + Duration::from_millis(budget.timeout_ms),
            cancel,
            visited: 0,
            pagination: None,
        }
    }
    pub fn stopped(&self) -> bool {
        self.cancel.is_cancelled() || Instant::now() >= self.deadline
    }
    pub fn visit(&mut self) -> bool {
        if self.stopped() || self.visited >= self.budget.max_visited {
            return false;
        }
        self.visited += 1;
        true
    }
    pub fn completeness(
        &mut self,
        context: &QueryContext<'_>,
        filters: &Filters,
        truncated: bool,
    ) -> Result<Completeness, CoreError> {
        let mut freshness = Freshness::Unknown;
        let mut disk_checked = false;
        let mut reasons = Vec::new();
        if filters.freshness_policy == FreshnessPolicy::Verify
            && let Some(project) = context.project
        {
            let options = crate::project::scan::ScanOptions {
                respect_git_ignore: project.config.selection.respect_git_ignore,
                max_file_bytes: project.config.selection.max_file_bytes,
                include: project.config.selection.include.clone(),
                exclude: project.config.selection.exclude.clone(),
            };
            let mut results = Vec::new();
            let mut complete = true;
            for (id, root) in &project.repositories {
                let result =
                    crate::project::scan::scan_repository_bounded(id, root, &options, &mut || {
                        self.visit()
                    })
                    .map_err(|e| CoreError::new(codes::CONFIG_INVALID, e))?;
                if let Some(result) = result {
                    results.push(result);
                } else {
                    complete = false;
                    break;
                }
            }
            if complete {
                let digest = crate::project::scan::source_set_digest(&results);
                let id = crate::project::snapshot::snapshot_id(&project.config_hash()?, &digest);
                disk_checked = true;
                freshness =
                    if context.database.current_source_snapshot_id()?.as_deref() == Some(&id) {
                        Freshness::Fresh
                    } else {
                        Freshness::Stale
                    };
            }
        }
        if freshness != Freshness::Fresh {
            reasons.push(format!(
                "source freshness is {freshness:?}; no index refresh was performed"
            ));
        }
        if filters.freshness_policy == FreshnessPolicy::AllowStale {
            reasons.push("historical assertions explicitly requested".into());
        }
        if truncated {
            reasons.push("query budget or page boundary reached".into());
        }
        // A complete empty result describes the requested indexed scope, not all business capabilities.
        let count: i64 = context.database.connection().query_row("SELECT count(*) FROM current_batches cb JOIN batches b ON b.id=cb.batch_id WHERE b.state='fresh'", [], |r|r.get(0)).map_err(storage::database_error)?;
        Ok(Completeness {
            freshness,
            disk_checked,
            checked_at: context.now.clone(),
            coverage: if count > 0 {
                CoverageLevel::Complete
            } else {
                CoverageLevel::NotConfigured
            },
            candidate_sets_complete: true,
            truncated,
            reasons,
        })
    }
    pub fn finish(
        &self,
        mut response: QueryResponse<Value>,
    ) -> Result<QueryResponse<Value>, CoreError> {
        if self.cancel.is_cancelled() {
            return Err(CoreError::new(codes::CANCELLED, "query cancelled"));
        }
        response.budget_used.visited = self.visited;
        response.budget_used.elapsed_ms = self.start.elapsed().as_millis() as u64;
        if response.completeness.freshness != Freshness::Fresh
            || response.completeness.coverage != CoverageLevel::Complete
            || response.completeness.truncated
            || !response.gaps.is_empty()
        {
            response.status = ResponseStatus::Partial;
        }
        let mut trimmed = false;
        let mut serialization_deadline = false;
        loop {
            if self.cancel.is_cancelled() {
                return Err(CoreError::new(codes::CANCELLED, "query cancelled"));
            }
            response.budget_used.elapsed_ms = self.start.elapsed().as_millis() as u64;
            if !serialization_deadline && self.stopped() {
                let root = response
                    .data
                    .get("frontier")
                    .or_else(|| response.data.get("roots"))
                    .and_then(Value::as_array)
                    .and_then(|roots| roots.first())
                    .cloned();
                response.data = json!({"frontier":root.clone().into_iter().collect::<Vec<_>>()});
                response.source_references.clear();
                response.gaps.clear();
                response.continuation = None;
                response.next_queries.clear();
                if let Some(root) = root {
                    response
                        .next_queries
                        .push(crate::contract::query::NextQuery {
                            operation: "trace".into(),
                            arguments: json!({"roots":[root],"direction":"both","maxDepth":1})
                                .as_object()
                                .unwrap()
                                .clone(),
                            reason: "serialization deadline reached; narrow the query".into(),
                        });
                }
                response.status = ResponseStatus::Partial;
                response.completeness.truncated = true;
                response
                    .completeness
                    .reasons
                    .push("query deadline reached during response serialization".into());
                response.budget_used.entities = 0;
                response.budget_used.relations = 0;
                serialization_deadline = true;
            }
            // Setting the counter changes its own serialized width; converge before measuring.
            let chars = serde_json::to_string(&response)
                .map_err(|_| {
                    CoreError::new(codes::INTERNAL_ERROR, "response serialization failed")
                })?
                .chars()
                .count() as u64;
            if response.budget_used.response_chars != chars {
                response.budget_used.response_chars = chars;
                continue;
            }
            // Reserve fixed metadata widths so clock/counter digit changes cannot
            // remove a different graph row on another transport. Report actual chars.
            let reserved = chars
                + 64_u64.saturating_sub(response.completeness.checked_at.chars().count() as u64)
                + 20_u64.saturating_sub(response.budget_used.elapsed_ms.to_string().len() as u64)
                + 20_u64
                    .saturating_sub(response.budget_used.response_chars.to_string().len() as u64);
            if reserved <= self.budget.max_response_chars {
                break;
            }
            if !trimmed {
                response.status = ResponseStatus::Partial;
                response.completeness.truncated = true;
                response
                    .completeness
                    .reasons
                    .push("response character budget reached; narrow the query".into());
                if self.pagination.is_none() {
                    response.continuation = None;
                    if response.next_queries.is_empty()
                        && let Some(roots) = response.data.get("roots").and_then(Value::as_array)
                        && let Some(root) = roots.first().cloned()
                    {
                        response
                            .next_queries
                            .push(crate::contract::query::NextQuery {
                            operation: "trace".into(),
                            arguments: json!({"roots":[root],"direction":"both","maxDepth":1})
                                .as_object()
                                .unwrap()
                                .clone(),
                            reason:
                                "response budget reached; narrow the query from the original entry"
                                    .into(),
                        });
                        if let Some(frontier) = response
                            .data
                            .get_mut("frontier")
                            .and_then(Value::as_array_mut)
                            && frontier.is_empty()
                        {
                            frontier.push(root.clone());
                        }
                    }
                }
                trimmed = true;
            }
            let paged_removal = self.pagination.as_ref().is_some_and(|page| {
                let key = if page.neighbors { "hops" } else { "results" };
                let Some(rows) = response.data.get_mut(key).and_then(Value::as_array_mut) else {
                    return false;
                };
                if rows.pop().is_none() {
                    return false;
                }
                response.continuation = rows.last().map(|row| {
                    let last = if page.neighbors {
                        json!([
                            row["direction"],
                            row["kind"],
                            row["relationId"],
                            row["assertionId"]
                        ])
                    } else {
                        json!([
                            row["exactRank"],
                            row["bm25"],
                            row["entityRef"]["entityId"],
                            row["assertions"][0]["assertionId"]
                        ])
                    };
                    encode_cursor(&page.snapshot, &page.hash, last)
                });
                let evidence = rows
                    .iter()
                    .flat_map(|row| {
                        if page.neighbors {
                            row["evidenceRefs"]
                                .as_array()
                                .into_iter()
                                .flatten()
                                .filter_map(Value::as_str)
                                .map(str::to_string)
                                .collect::<Vec<_>>()
                        } else {
                            row["assertions"]
                                .as_array()
                                .into_iter()
                                .flatten()
                                .flat_map(|a| a["evidenceRefs"].as_array().into_iter().flatten())
                                .filter_map(Value::as_str)
                                .map(str::to_string)
                                .collect()
                        }
                    })
                    .collect::<std::collections::BTreeSet<_>>();
                response
                    .source_references
                    .retain(|r| evidence.contains(&r.evidence_id));
                true
            });
            let removed = paged_removal
                || response.data.as_object_mut().is_some_and(|data| {
                    let key = data
                        .iter()
                        .filter_map(|(k, v)| {
                            v.as_array()
                                .filter(|a| !a.is_empty())
                                .map(|a| (k.clone(), a.len()))
                        })
                        .max_by(|(left_key, left_len), (right_key, right_len)| {
                            left_len.cmp(right_len).then(left_key.cmp(right_key))
                        })
                        .map(|(k, _)| k);
                    key.is_some_and(|key| {
                        data.get_mut(&key)
                            .and_then(Value::as_array_mut)
                            .unwrap()
                            .pop();
                        true
                    })
                });
            if !removed {
                if !response.source_references.is_empty() {
                    response.source_references.pop();
                } else if !response.next_queries.is_empty() {
                    response.next_queries.pop();
                } else if !response.gaps.is_empty() {
                    response.gaps.pop();
                    if !response
                        .completeness
                        .reasons
                        .iter()
                        .any(|r| r == "some gap details were omitted to fit the response budget")
                    {
                        response.completeness.reasons.push(
                            "some gap details were omitted to fit the response budget".into(),
                        );
                    }
                } else {
                    return Err(CoreError::new(
                        codes::BUDGET_EXHAUSTED,
                        "response budget cannot contain the complete envelope",
                    ));
                }
            }
            // These counters describe returned projections. Visited still includes discarded work.
            response.budget_used.entities = response
                .data
                .get("results")
                .or_else(|| response.data.get("entities"))
                .or_else(|| response.data.get("symbols"))
                .and_then(Value::as_array)
                .map_or(0, |a| a.len() as u64);
            response.budget_used.relations = response
                .data
                .get("hops")
                .and_then(Value::as_array)
                .map_or(0, |a| a.len() as u64);
        }
        Ok(response)
    }
}

pub fn validate_snapshot(expected: &SnapshotRef, actual: &SnapshotRef) -> Result<(), CoreError> {
    if expected.index_id != actual.index_id
        || expected.generation != actual.generation
        || expected
            .source_snapshot_id
            .as_ref()
            .is_some_and(|id| Some(id) != actual.source_snapshot_id.as_ref())
    {
        return Err(CoreError::new(
            codes::SNAPSHOT_EXPIRED,
            "reference belongs to another index or generation",
        ));
    }
    Ok(())
}

pub fn validate_entity_reference(
    connection: &Connection,
    reference: &crate::contract::query::EntityRef,
    snapshot: &SnapshotRef,
) -> Result<(), CoreError> {
    validate_snapshot(&reference.snapshot, snapshot)?;
    if super::load_entity(connection, &reference.entity_id)?.is_none() {
        return Err(CoreError::new(
            codes::REFERENCE_MISSING,
            "entity reference is absent from this index",
        ));
    }
    Ok(())
}

pub fn assertion_visible(
    connection: &Connection,
    assertion: &str,
    filters: &Filters,
) -> Result<bool, CoreError> {
    connection.query_row("SELECT EXISTS(SELECT 1 FROM assertions a JOIN current_batches cb ON cb.batch_id=a.batch_id JOIN batches b ON b.id=cb.batch_id WHERE a.id=?1 AND (?2 OR b.state='fresh'))", rusqlite::params![assertion, filters.freshness_policy==FreshnessPolicy::AllowStale], |r|r.get(0)).map_err(storage::database_error)
}

pub fn references(
    connection: &Connection,
    assertion: &str,
    snapshot: &SnapshotRef,
) -> Result<Vec<SourceReference>, CoreError> {
    let mut stmt=connection.prepare("SELECT e.id,e.source_id,e.content_hash,e.start_byte,e.end_byte FROM assertion_evidence ae JOIN evidence e ON e.id=ae.evidence_id WHERE ae.assertion_id=?1 ORDER BY e.id").map_err(storage::database_error)?;
    stmt.query_map([assertion], |r| {
        Ok(SourceReference {
            snapshot: snapshot.clone(),
            evidence_id: r.get(0)?,
            source_id: r.get(1)?,
            content_hash: r.get(2)?,
            start_byte: r.get::<_, i64>(3)? as u64,
            end_byte: r.get::<_, i64>(4)? as u64,
        })
    })
    .map_err(storage::database_error)?
    .collect::<Result<Vec<_>, _>>()
    .map_err(storage::database_error)
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Cursor {
    version: u32,
    snapshot: SnapshotRef,
    request_hash: String,
    last: Value,
}
pub fn request_hash<T: Serialize>(request: &T) -> Result<String, CoreError> {
    let mut value = serde_json::to_value(request)
        .map_err(|_| CoreError::new(codes::INVALID_REQUEST, "invalid request"))?;
    if let Some(map) = value.as_object_mut() {
        map.remove("continuation");
    }
    let canonical = crate::contract::identity::canonical_json(&value).map_err(|_| {
        CoreError::new(
            codes::INVALID_REQUEST,
            "request contains a non-canonical value",
        )
    })?;
    Ok(content_hash(canonical.as_bytes()))
}
pub fn decode_cursor(
    token: Option<&str>,
    snapshot: &SnapshotRef,
    hash: &str,
) -> Result<Option<Value>, CoreError> {
    let Some(token) = token else {
        return Ok(None);
    };
    let invalid = || {
        CoreError::new(
            codes::INVALID_REQUEST,
            "invalid continuation or request mismatch",
        )
    };
    if token.len() > 8192
        || !token
            .bytes()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    {
        return Err(invalid());
    }
    let cursor: Cursor = serde_json::from_slice(&hex::decode(token).map_err(|_| invalid())?)
        .map_err(|_| invalid())?;
    validate_snapshot(&cursor.snapshot, snapshot)?;
    if cursor.version != 1 || cursor.request_hash != hash {
        return Err(invalid());
    }
    Ok(Some(cursor.last))
}
pub fn encode_cursor(snapshot: &SnapshotRef, hash: &str, last: Value) -> String {
    hex::encode(
        serde_json::to_vec(&Cursor {
            version: 1,
            snapshot: snapshot.clone(),
            request_hash: hash.into(),
            last,
        })
        .unwrap(),
    )
}

pub fn entity_allowed(
    connection: &Connection,
    id: &str,
    filters: &Filters,
) -> Result<bool, CoreError> {
    let Some(entity) = super::load_entity(connection, id)? else {
        return Ok(false);
    };
    if !filters.allows_scope(&entity.scope_id) || !filters.allows_kind(entity.kind) {
        return Ok(false);
    }
    let assertions = storage::entities::load_entity_assertions(connection, id, true)?;
    if filters.freshness_policy != FreshnessPolicy::AllowStale
        && !assertions.iter().any(|a| filters.allows_basis(a.basis))
    {
        return Ok(false);
    }
    if filters.freshness_policy == FreshnessPolicy::AllowStale {
        let assertions = storage::entities::load_entity_assertions(connection, id, false)?;
        let mut found = false;
        for a in assertions {
            if filters.allows_basis(a.basis)
                && assertion_visible(connection, &a.assertion_id, filters)?
            {
                found = true;
                break;
            }
        }
        if !found {
            return Ok(false);
        }
    }
    if let Some(repos) = &filters.repository_ids {
        let slot = entity
            .identity
            .iter()
            .find(|s| s.name == "repositoryId")
            .and_then(|s| s.value.as_ref())
            .and_then(|v| v.as_text());
        if slot.is_some_and(|id| repos.iter().any(|r| r == id)) {
            return Ok(true);
        }
        let mut stmt=connection.prepare("SELECT DISTINCT s.repository_id FROM entity_assertions ea JOIN assertion_evidence ae ON ae.assertion_id=ea.id JOIN evidence e ON e.id=ae.evidence_id JOIN sources s ON s.id=e.source_id JOIN current_batches cb ON cb.batch_id=ea.batch_id JOIN batches b ON b.id=cb.batch_id WHERE ea.entity_id=?1 AND (?2 OR b.state='fresh')").map_err(storage::database_error)?;
        let rows = stmt
            .query_map(
                rusqlite::params![id, filters.freshness_policy == FreshnessPolicy::AllowStale],
                |r| r.get::<_, String>(0),
            )
            .map_err(storage::database_error)?;
        for row in rows {
            if repos.contains(&row.map_err(storage::database_error)?) {
                return Ok(true);
            }
        }
        return Ok(false);
    }
    Ok(true)
}

pub fn entity_projection(
    connection: &Connection,
    id: &str,
    filters: &Filters,
    snapshot: &SnapshotRef,
) -> Result<Option<Value>, CoreError> {
    if !entity_allowed(connection, id, filters)? {
        return Ok(None);
    }
    let entity = super::load_entity(connection, id)?.unwrap();
    let mut assertions = Vec::new();
    for a in storage::entities::load_entity_assertions(connection, id, false)? {
        if !filters.allows_basis(a.basis)
            || !assertion_visible(connection, &a.assertion_id, filters)?
        {
            continue;
        }
        assertions.push(json!({"assertionId":a.assertion_id,"producerKey":a.producer_key,"batchId":a.batch_id,"basis":a.basis,"origin":a.origin,"name":a.name,"attributes":a.attributes,"evidenceRefs":storage::entities::load_assertion_evidence(connection,&a.assertion_id)?}));
    }
    assertions.sort_by(|a, b| a["assertionId"].as_str().cmp(&b["assertionId"].as_str()));
    Ok(Some(
        json!({"entityRef":{"snapshot":snapshot,"entityId":id},"kind":entity.kind,"scopeId":entity.scope_id,"identity":entity.identity,"name":assertions.iter().find_map(|a|a["name"].as_str()),"assertions":assertions}),
    ))
}
