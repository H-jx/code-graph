//! 以原始证据定位文件，返回声明轮廓和签名；不读取或拼接函数体。
use super::rank::Ranked;
use crate::contract::{error::CoreError, evidence::SourceReference, query::CodeMapRequest};
use crate::query::{
    Filters, QueryContext,
    runtime::{self, Runtime},
};
use crate::storage;
use serde_json::{Value, json};
use std::collections::BTreeSet;

pub struct Rendered {
    pub files: Vec<Value>,
    pub symbols: Vec<Value>,
    pub uncovered: Vec<Value>,
    pub references: Vec<SourceReference>,
    pub truncated: bool,
}
pub fn render(
    context: &QueryContext<'_>,
    ranked: Ranked,
    request: &CodeMapRequest,
    rt: &mut Runtime,
) -> Result<Rendered, CoreError> {
    let mut result = Rendered {
        files: Vec::new(),
        symbols: Vec::new(),
        uncovered: Vec::new(),
        references: Vec::new(),
        truncated: ranked.truncated,
    };
    let filters = Filters::from_request(request.filters.as_ref());
    let snapshot = crate::query::snapshot_ref(context.database)?;
    let connection = context.database.connection();
    let mut files = BTreeSet::new();
    let mut languages = BTreeSet::new();
    let max_files = request
        .max_files
        .unwrap_or(50)
        .min(rt.budget.max_entities as u32) as usize;
    let max_symbols = request
        .max_symbols
        .unwrap_or(200)
        .min(rt.budget.max_entities as u32) as usize;
    for (id, rank) in ranked.entities {
        if !rt.visit() {
            result.truncated = true;
            break;
        }
        let Some(entity) = runtime::entity_projection(connection, &id, &filters, &snapshot)? else {
            continue;
        };
        for a in entity["assertions"].as_array().unwrap() {
            let assertion = a["assertionId"].as_str().unwrap();
            let refs = runtime::references(connection, assertion, &snapshot)?;
            let mut paths = Vec::new();
            for reference in &refs {
                if let Some((repository, path)) =
                    storage::sources::source_path(connection, &reference.source_id)?
                {
                    let key = (repository.clone(), path.clone());
                    if !files.contains(&key) {
                        if result.files.len() >= max_files {
                            result.truncated = true;
                            continue;
                        }
                        files.insert(key);
                        result.files.push(json!({"repositoryId":repository,"path":path,"sourceId":reference.source_id}));
                    }
                    let extension = std::path::Path::new(&path)
                        .extension()
                        .and_then(|x| x.to_str())
                        .unwrap_or_default();
                    if !matches!(
                        extension,
                        "ts" | "mts" | "cts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "py" | "pyi"
                    ) && languages.insert(extension.to_string())
                    {
                        result.uncovered.push(json!({"language":extension,"reason":"text/evidence available; no built-in syntax outline"}));
                    }
                    paths.push(json!({"repositoryId":repository,"path":path,"startByte":reference.start_byte,"endByte":reference.end_byte}));
                }
            }
            if !matches!(entity["kind"].as_str(), Some("file" | "module")) {
                if result.symbols.len() >= max_symbols {
                    result.truncated = true;
                    continue;
                }
                result.symbols.push(json!({"entityRef":entity["entityRef"],"kind":entity["kind"],"scopeId":entity["scopeId"],"name":a["name"],"signature":a["attributes"].get("signature").or_else(||a["attributes"].get("normalizedSignature")),"assertionId":assertion,"basis":a["basis"],"origin":a["origin"],"rank":rank,"locations":paths,"evidenceRefs":a["evidenceRefs"]}));
            }
            result.references.extend(refs);
        }
    }
    crate::query::generic::dedup_refs(&mut result.references);
    Ok(result)
}
