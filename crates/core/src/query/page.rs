//! 路由 → 页面 → 操作 → 请求 → 后端 → 表字段（计划 §3.7）。
//!
//! 路由按已登记的 application/router mode 解析：hash 模式取 `#` 之后的路径，
//! history 模式取 URL path；查询参数不参与身份，百分号编码只规范化一次。

use serde_json::{Value, json};
use std::collections::BTreeSet;

use crate::contract::error::{CoreError, codes, gap_codes};
use crate::contract::query::*;
use crate::contract::relation::RelationKind;
use crate::query::{
    Filters, QueryContext,
    runtime::{self, Runtime},
    scenario,
};
use crate::storage;

/// 规范化后的路由。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedRoute {
    /// 参与身份匹配的路径（不含查询参数）。
    pub path: String,
    /// 原始输入。
    pub raw: String,
    /// 查询参数（作为请求上下文保留，不参与身份）。
    pub query: String,
    /// 使用的模式：`hash` 或 `history`。
    pub mode: RouteMode,
}

/// 路由模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteMode {
    Hash,
    History,
}

/// 解析路由。
///
/// `mode` 来自项目已登记的 application/router 配置；调用方必须显式给出，
/// Core 不根据输入形状猜测应用。
pub fn normalize_route(raw: &str, mode: RouteMode) -> Result<NormalizedRoute, CoreError> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err(CoreError::new(
            codes::INVALID_REQUEST,
            "route must not be empty",
        ));
    }
    let absolute = if raw.contains("://") {
        Some(
            url::Url::parse(raw)
                .map_err(|_| CoreError::new(codes::INVALID_REQUEST, "invalid absolute URL"))?,
        )
    } else {
        None
    };
    if absolute
        .as_ref()
        .is_some_and(|url| !matches!(url.scheme(), "http" | "https"))
    {
        return Err(CoreError::new(
            codes::INVALID_REQUEST,
            "unsupported URL scheme",
        ));
    }
    let relative_path = raw.split('#').next().unwrap().split('?').next().unwrap();
    let path_part = absolute.as_ref().map_or(relative_path, |url| url.path());
    let fragment = absolute
        .as_ref()
        .and_then(|url| url.fragment())
        .or_else(|| raw.split_once('#').map(|(_, fragment)| fragment));
    let (path, query) = if mode == RouteMode::Hash
        && let Some(fragment) = fragment
    {
        let (path, query) = fragment
            .split_once('?')
            .map_or((fragment, None), |(path, query)| {
                (path, Some(query.to_string()))
            });
        (path.to_string(), query)
    } else {
        (
            path_part.to_string(),
            absolute
                .as_ref()
                .and_then(|url| url.query())
                .or_else(|| {
                    raw.split('#')
                        .next()
                        .unwrap()
                        .split_once('?')
                        .map(|(_, query)| query)
                })
                .map(str::to_string),
        )
    };
    if path.is_empty() {
        return Err(CoreError::new(
            codes::INVALID_REQUEST,
            "route does not contain a path",
        ));
    }
    let decoded = percent_decode_once(&path)?;
    Ok(NormalizedRoute {
        path: decoded,
        raw: raw.to_string(),
        query: query.unwrap_or_default(),
        mode,
    })
}

/// 百分号编码只解码一次；编码的 `/` 不是路径分隔符。
pub fn percent_decode_once(text: &str) -> Result<String, CoreError> {
    let bytes = text.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len() {
                return Err(CoreError::new(
                    codes::INVALID_REQUEST,
                    format!("invalid percent encoding in `{text}`"),
                ));
            }
            let hex = std::str::from_utf8(&bytes[index + 1..index + 3])
                .map_err(|_| CoreError::new(codes::INVALID_REQUEST, "invalid percent encoding"))?;
            let value = u8::from_str_radix(hex, 16)
                .map_err(|_| CoreError::new(codes::INVALID_REQUEST, "invalid percent encoding"))?;
            if matches!(value, b'/' | b'\\') {
                out.extend_from_slice(format!("%{value:02X}").as_bytes());
            } else {
                out.push(value);
            }
            index += 3;
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(out).map_err(|_| {
        CoreError::new(
            codes::INVALID_REQUEST,
            format!("percent decoding produced invalid UTF-8 in `{text}`"),
        )
    })
}

/// 静态路由和 alias 精确匹配；参数模式只提供候选，不推测框架优先级。
fn pattern_matches(pattern: &str, path: &str) -> bool {
    let pattern = pattern.split('/').collect::<Vec<_>>();
    let path = path.split('/').collect::<Vec<_>>();
    pattern.iter().enumerate().all(|(n, part)| {
        if *part == "*" {
            return true;
        }
        path.get(n)
            .is_some_and(|actual| part.starts_with(':') || part == actual)
    }) && (pattern.last() == Some(&"*") || pattern.len() == path.len())
}

pub(super) fn select_routes(
    context: &QueryContext<'_>,
    raw: &str,
    filters: &CommonFilters,
    rt: &mut Runtime,
) -> Result<(Vec<EntityRef>, Vec<Gap>, bool), CoreError> {
    if raw.len() > 4096 {
        return Err(CoreError::new(
            codes::INVALID_REQUEST,
            "route input is too long",
        ));
    }
    let connection = context.database.connection();
    let snapshot = super::snapshot_ref(context.database)?;
    let filters = Filters::from_request(Some(filters));
    let mut statement=connection.prepare("SELECT DISTINCT e.id FROM entities e JOIN entity_assertions ea ON ea.entity_id=e.id JOIN current_batches cb ON cb.batch_id=ea.batch_id WHERE e.kind='route' ORDER BY e.id LIMIT ?1").map_err(storage::database_error)?;
    let rows = statement
        .query_map([(rt.budget.max_visited + 1) as i64], |r| {
            r.get::<_, String>(0)
        })
        .map_err(storage::database_error)?;
    let mut roots = Vec::new();
    let mut gaps = Vec::new();
    let mut truncated = false;
    for id in rows {
        if !rt.visit() {
            truncated = true;
            break;
        }
        let id = id.map_err(storage::database_error)?;
        let Some(entity) = runtime::entity_projection(connection, &id, &filters, &snapshot)? else {
            continue;
        };
        let mode = match scenario::value_slot(&entity, "mode") {
            Some("hash") => RouteMode::Hash,
            Some("history") => RouteMode::History,
            _ => {
                gaps.push(scenario::gap(
                    ROUTE_MODE_UNKNOWN,
                    "routes",
                    "route has no original registered router mode",
                    vec![id],
                ));
                continue;
            }
        };
        let input = normalize_route(raw, mode)?;
        let mut paths = vec![
            scenario::value_slot(&entity, "declaredPath")
                .unwrap_or_default()
                .to_string(),
        ];
        for attrs in scenario::attributes(&entity) {
            if let Some(alias) = attrs["alias"].as_str() {
                paths.push(alias.into())
            }
            if let Some(aliases) = attrs["alias"].as_array() {
                paths.extend(aliases.iter().filter_map(Value::as_str).map(str::to_string));
            }
        }
        let mut matched = false;
        for path in paths {
            let declared = normalize_route(&path, mode)?.path;
            if declared == input.path {
                matched = true;
            } else if (declared.contains(':') || declared.contains('*'))
                && pattern_matches(&declared, &input.path)
            {
                matched = true;
                gaps.push(scenario::gap(
                    gap_codes::AMBIGUOUS_BINDING,
                    "routes",
                    "dynamic route pattern is a possible match; no framework priority was proven",
                    vec![id.clone()],
                ));
            }
        }
        if matched {
            roots.push(EntityRef {
                snapshot: snapshot.clone(),
                entity_id: id,
            });
        }
    }
    roots.sort_by(|a, b| a.entity_id.cmp(&b.entity_id));
    roots.dedup_by(|a, b| a.entity_id == b.entity_id);
    if roots.len() > 1 {
        gaps.push(scenario::gap(
            gap_codes::AMBIGUOUS_BINDING,
            "routes",
            "multiple registered routes match; specify application scope",
            roots.iter().map(|r| r.entity_id.clone()).collect(),
        ));
    }
    Ok((roots, gaps, truncated))
}

pub fn execute(
    context: &mut QueryContext<'_>,
    request: &PageRequest,
    id: String,
    rt: &mut Runtime,
) -> Result<QueryResponse<Value>, CoreError> {
    let filters =
        scenario::roots_filters(request.filters.as_ref(), request.scope_ids.as_ref(), None);
    let (roots, mut gaps, truncated) = select_routes(context, &request.route, &filters, rt)?;
    let trace = TraceRequest {
        roots: roots.clone(),
        direction: Direction::Both,
        relation_kinds: Some(vec![
            RelationKind::Renders,
            RelationKind::Contains,
            RelationKind::Describes,
            RelationKind::Triggers,
            RelationKind::Handles,
            RelationKind::Calls,
            RelationKind::Wraps,
            RelationKind::Requests,
            RelationKind::References,
            RelationKind::Resolves,
            RelationKind::Reads,
            RelationKind::Writes,
            RelationKind::MapsTo,
            RelationKind::Serves,
        ]),
        max_depth: None,
        filters: request.filters.clone(),
        budget: request.budget.clone(),
    };
    let mut walk = crate::graph::paths::walk_selected(
        context,
        &trace,
        rt,
        &Filters::from_request(request.filters.as_ref()),
        |neighbor, current_kind| {
            if neighbor.key.0 == "in" {
                return matches!(neighbor.key.1.as_str(), "describes" | "triggers" | "serves");
            }
            if matches!(neighbor.key.1.as_str(), "describes" | "serves") {
                return false;
            }
            !(neighbor.key.1 == "contains" && matches!(current_kind, "dataSource" | "dataTable"))
        },
    )?;
    walk.truncated |= truncated;
    let root_ids = roots
        .iter()
        .map(|r| r.entity_id.as_str())
        .collect::<BTreeSet<_>>();
    let title_ids = walk
        .hops
        .iter()
        .filter(|h| {
            h["kind"] == "describes" && h["toId"].as_str().is_some_and(|id| root_ids.contains(id))
        })
        .filter_map(|h| h["fromId"].as_str())
        .collect::<BTreeSet<_>>();
    let titles = walk
        .entities
        .iter()
        .filter(|e| {
            e["kind"] == "title"
                && e["entityRef"]["entityId"]
                    .as_str()
                    .is_some_and(|id| title_ids.contains(id))
        })
        .cloned()
        .collect::<Vec<_>>();
    let backend_ids = walk
        .hops
        .iter()
        .filter(|h| matches!(h["kind"].as_str(), Some("handles" | "resolves")))
        .filter_map(|h| h["toId"].as_str())
        .collect::<BTreeSet<_>>();
    let backend = walk
        .entities
        .iter()
        .filter(|e| {
            matches!(
                e["kind"].as_str(),
                Some("endpoint" | "httpEndpoint" | "graphqlField")
            ) || e["entityRef"]["entityId"]
                .as_str()
                .is_some_and(|id| backend_ids.contains(id))
        })
        .map(|e| e["entityRef"].clone())
        .collect::<Vec<_>>();
    for (missing, code, reason) in [
        (
            titles.is_empty(),
            "PAGE_TITLE_MISSING",
            "no original title/i18n assertion describes the matched route",
        ),
        (
            !walk.entities.iter().any(|e| e["kind"] == "component"),
            "PAGE_COMPONENT_MISSING",
            "route has no supported component binding",
        ),
        (
            !walk.entities.iter().any(|e| e["kind"] == "event"),
            "PAGE_EVENT_MISSING",
            "no supported UI event binding was collected",
        ),
        (
            !walk.entities.iter().any(|e| e["kind"] == "request"),
            "PAGE_REQUEST_MISSING",
            "no proven request wrapper chain was collected",
        ),
        (
            backend.is_empty(),
            "PAGE_BACKEND_BINDING_MISSING",
            "no proven backend target was collected",
        ),
        (
            !walk.entities.iter().any(|e| e["kind"] == "dataColumn"),
            "PAGE_DATA_SOURCE_MISSING",
            "no proven table/column access was collected",
        ),
    ] {
        if missing {
            gaps.push(scenario::gap(code, "page", reason, Vec::new()));
        }
    }
    let data = json!({"route":request.route,"title":titles,"components":scenario::ids(&walk.entities,&["component"]),"operations":scenario::ids(&walk.entities,&["operation"]),"events":scenario::ids(&walk.entities,&["event"]),"requests":scenario::ids(&walk.entities,&["request","graphqlOperation"]),"backend":backend,"tableColumns":scenario::ids(&walk.entities,&["dataColumn"])});
    scenario::finish(
        context,
        id,
        roots,
        walk,
        data,
        &["routes", "frontend", "api", "database", "link"],
        request.filters.as_ref(),
        gaps,
        rt,
    )
}

pub const ROUTE_MODE_UNKNOWN: &str = gap_codes::NOT_CONFIGURED;
