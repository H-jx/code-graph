//! 路由 → 页面 → 操作 → 请求 → 后端 → 表字段（计划 §3.7）。
//!
//! 路由按已登记的 application/router mode 解析：hash 模式取 `#` 之后的路径，
//! history 模式取 URL path；查询参数不参与身份，百分号编码只规范化一次。

use serde_json::{Map, Value};

use crate::contract::error::{CoreError, codes, gap_codes};
use crate::contract::query::{PageRequest, QueryResponse};
use crate::query::{QueryContext, scenario_skeleton};

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
    let (without_fragment, fragment) = match raw.split_once('#') {
        Some((head, tail)) => (head, Some(tail)),
        None => (raw, None),
    };
    let (path_part, query_part) = match without_fragment.split_once('?') {
        Some((head, tail)) => (head, Some(tail)),
        None => (without_fragment, None),
    };
    let (path, query) = match (mode, fragment) {
        (RouteMode::Hash, Some(fragment)) => {
            let (path, query) = match fragment.split_once('?') {
                Some((head, tail)) => (head, Some(tail.to_string())),
                None => (fragment, None),
            };
            (path.to_string(), query)
        }
        (RouteMode::Hash, None) => (
            path_part.to_string(),
            query_part.map(|value| value.to_string()),
        ),
        (RouteMode::History, _) => (
            path_part.to_string(),
            query_part.map(|value| value.to_string()),
        ),
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
            let hex = &text[index + 1..index + 3];
            let value = u8::from_str_radix(hex, 16).map_err(|_| {
                CoreError::new(
                    codes::INVALID_REQUEST,
                    format!("invalid percent encoding in `{text}`"),
                )
            })?;
            out.push(value);
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

/// 执行页面查询。
pub fn execute(
    context: &mut QueryContext<'_>,
    request: &PageRequest,
    request_id: String,
) -> Result<QueryResponse<Value>, CoreError> {
    let mut extra = Map::new();
    extra.insert("route".to_string(), Value::from(request.route.clone()));
    // 页面查询需要已登记的 router mode；缺少时明确返回缺口而不是猜测。
    extra.insert("title".to_string(), Value::Null);
    extra.insert("operations".to_string(), Value::Array(Vec::new()));
    extra.insert("requests".to_string(), Value::Array(Vec::new()));
    extra.insert("backend".to_string(), Value::Array(Vec::new()));
    extra.insert("tableColumns".to_string(), Value::Array(Vec::new()));

    let response = scenario_skeleton(
        context,
        &request.route,
        &["routes", "frontend", "api", "database"],
        extra,
        request_id,
    )?;
    Ok(response)
}

/// 路由缺口代码。
pub const ROUTE_MODE_UNKNOWN: &str = gap_codes::NOT_CONFIGURED;
