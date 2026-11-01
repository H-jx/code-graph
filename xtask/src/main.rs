//! xtask：Rust 侧开发辅助命令。
//!
//! 计划 §4.1 指定的目标入口：
//! - `cargo run -p xtask -- export-contracts`：用 `schemars` 从 `contract` 导出 JSON Schema
//!   与类型注册表到 `docs/schema/`，再由 `scripts/generate-types.mjs` 生成 SDK 类型；
//! - `cargo run -p xtask -- package --target <triple>`：构建 CLI 二进制、嵌入资源与 native
//!   发布候选，核对版本清单与制品哈希（T23）。
//!
//! `export-contracts --check` 在内存中重新生成并与磁盘内容比较，不一致时返回非零，
//! 避免“生成后再声称一致”。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process;

use anyhow::{Context, bail};
use clap::{Parser, Subcommand};
use project_knowledge_core::contract::entity::{ALL_ENTITY_KINDS, EntityKind};
use project_knowledge_core::contract::error::{codes, gap_codes};
use project_knowledge_core::contract::identity::entity_components;
use project_knowledge_core::contract::relation::{
    ALL_RELATION_KINDS, RelationKind, endpoint_kinds,
};
use serde_json::{Map, Value, json};

#[derive(Parser)]
#[command(name = "xtask", version, about = "Project Knowledge 开发辅助命令")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// 导出契约 JSON Schema、类型注册表与协议向量到 `docs/schema/`。
    ExportContracts {
        /// 只比较，不写文件；不一致时返回非零。
        #[arg(long)]
        check: bool,
        /// 输出目录（缺省 `docs/schema`）。
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// 构建当前主机的可安装发布候选，不执行公开发布。
    Package {
        /// 目标三元组，例如 `x86_64-pc-windows-msvc`。
        #[arg(long)]
        target: String,
    },
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::ExportContracts { check, out } => export_contracts(check, out),
        Command::Package { target } => package(&target),
    };
    if let Err(error) = result {
        eprintln!("xtask error: {error:#}");
        process::exit(1);
    }
}

/// 导出目录（相对仓库根）。
fn schema_dir(out: Option<PathBuf>) -> PathBuf {
    match out {
        Some(path) => path,
        None => repository_root().join("docs/schema"),
    }
}

/// 仓库根目录。
fn repository_root() -> PathBuf {
    // xtask 的 CARGO_MANIFEST_DIR 是 `<repo>/xtask`。
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// 生成契约文件。
fn export_contracts(check: bool, out: Option<PathBuf>) -> anyhow::Result<()> {
    let files = build_contract_files()?;
    let directory = schema_dir(out);

    if check {
        let mut differences = Vec::new();
        for (name, content) in &files {
            let path = directory.join(name);
            let existing = std::fs::read_to_string(&path).unwrap_or_default();
            if existing != *content {
                differences.push(name.clone());
            }
        }
        if differences.is_empty() {
            println!("契约文件与生成结果一致（{} 个文件）", files.len());
            return Ok(());
        }
        bail!("以下契约文件与生成结果不一致：{}", differences.join(", "));
    }

    std::fs::create_dir_all(&directory)
        .with_context(|| format!("cannot create {}", directory.display()))?;
    for (name, content) in &files {
        let path = directory.join(name);
        std::fs::write(&path, content)
            .with_context(|| format!("cannot write {}", path.display()))?;
        println!("wrote {}", path.display());
    }
    Ok(())
}

/// 契约文件集合（名称 → 内容）。
fn build_contract_files() -> anyhow::Result<BTreeMap<String, String>> {
    let mut files = BTreeMap::new();
    files.insert(
        "contract.schema.json".to_string(),
        pretty(&contract_schema()?)?,
    );
    files.insert("entity-kinds.json".to_string(), pretty(&entity_registry())?);
    files.insert(
        "relation-kinds.json".to_string(),
        pretty(&relation_registry())?,
    );
    files.insert("error-codes.json".to_string(), pretty(&error_registry())?);
    files.insert("protocol.json".to_string(), pretty(&protocol_registry())?);
    Ok(files)
}

/// 契约 JSON Schema（2020-12）。
fn contract_schema() -> anyhow::Result<Value> {
    Ok(project_knowledge_core::contract::schema::document())
}

/// 实体类型注册表：类型名 + 身份槽位。
fn entity_registry() -> Value {
    let mut kinds = Vec::new();
    for kind in ALL_ENTITY_KINDS {
        let components: Vec<Value> = entity_components(kind.clone())
            .iter()
            .map(|spec| {
                json!({
                    "name": spec.name,
                    "required": spec.required,
                })
            })
            .collect();
        kinds.push(json!({
            "kind": kind.as_str(),
            "identityComponents": components,
        }));
    }
    json!({ "entityKinds": kinds })
}

/// 关系类型注册表：类型名 + 允许的端点类型。
fn relation_registry() -> Value {
    let mut kinds = Vec::new();
    for kind in ALL_RELATION_KINDS {
        let (from, to) = endpoint_kinds(kind.clone());
        kinds.push(json!({
            "kind": kind.as_str(),
            "allowedFrom": from,
            "allowedTo": to,
        }));
    }
    json!({ "relationKinds": kinds })
}

/// 错误码与缺口码注册表。
fn error_registry() -> Value {
    let error_codes = vec![
        codes::CONFIG_INVALID,
        codes::PROTOCOL_UNSUPPORTED,
        codes::SCHEMA_INVALID,
        codes::IDENTITY_CONFLICT,
        codes::REFERENCE_MISSING,
        codes::EVIDENCE_INVALID,
        codes::SOURCE_CHANGED,
        codes::SNAPSHOT_EXPIRED,
        codes::DEPENDENCY_CHANGED,
        codes::CANDIDATE_SET_INCOMPLETE,
        codes::RUNTIME_MISSING,
        codes::DEPENDENCY_PREPARE_FAILED,
        codes::PLUGIN_EXIT,
        codes::PLUGIN_TIMEOUT,
        codes::OUTPUT_LIMIT,
        codes::ARTIFACT_CHANGED,
        codes::ACTIVATION_CONFLICT,
        codes::INDEX_MISSING,
        codes::INDEX_STALE,
        codes::CANCELLED,
        codes::BUDGET_EXHAUSTED,
        codes::MODEL_CONFIG_INVALID,
        codes::BUSY,
        codes::CLOSED,
        codes::INTERNAL_ERROR,
        codes::NOT_IMPLEMENTED,
        codes::INVALID_REQUEST,
    ];
    let gap_code_list = vec![
        gap_codes::AMBIGUOUS_BINDING,
        gap_codes::DYNAMIC_TARGET,
        gap_codes::NOT_CONFIGURED,
        gap_codes::WORKFLOW_EVIDENCE_MISSING,
        gap_codes::DEFINITION_MISSING,
        gap_codes::CONFLICTING_EVIDENCE,
        gap_codes::KEY_MAPPING_INCOMPLETE,
        gap_codes::SOURCE_SKIPPED,
        gap_codes::PARSE_FAILED,
        gap_codes::PATH_CASE_CONFLICT,
        gap_codes::SYMLINK_SKIPPED,
        gap_codes::FILE_TOO_LARGE,
        gap_codes::NOT_UTF8,
        gap_codes::UNREADABLE,
        gap_codes::DATA_SOURCE_UNRESOLVED,
        gap_codes::NAMESPACE_UNRESOLVED,
    ];
    json!({
        "errorCodes": error_codes,
        "gapCodes": gap_code_list,
    })
}

/// 协议版本信息。
fn protocol_registry() -> Value {
    json!({
        "protocolVersion": project_knowledge_core::contract::PROTOCOL_VERSION,
        "configSchemaVersion": project_knowledge_core::contract::CONFIG_SCHEMA_VERSION,
        "supportedProtocolMinors": project_knowledge_core::contract::SUPPORTED_PROTOCOL_MINORS,
        "entityKinds": ALL_ENTITY_KINDS.iter().map(|kind| kind.as_str()).collect::<Vec<_>>(),
        "relationKinds": ALL_RELATION_KINDS.iter().map(|kind| kind.as_str()).collect::<Vec<_>>(),
        "unknownFields": "reject",
        "extensionsNamespace": "x.<pluginId>.<localName>",
        "extensionNamePattern": project_knowledge_core::contract::extensions::NAME_PATTERN,
        "extensionEntityIdentity": "[projectId,kind,[[slotName,[]|[value]],...]]",
        "extensionDeclarations": "PluginManifest.extensions.entities/relations; own namespace; local bundled $defs only",
    })
}

/// 稳定的 JSON 输出（键排序，便于比较）。
fn pretty(value: &Value) -> anyhow::Result<String> {
    let text = serde_json::to_string_pretty(value)?;
    Ok(format!("{text}\n"))
}

/// 组装发布候选（T23）。
fn package(target: &str) -> anyhow::Result<()> {
    let status = process::Command::new("node")
        .arg(repository_root().join("scripts/package.mjs"))
        .args(["--target", target])
        .current_dir(repository_root())
        .status()
        .context("start release candidate builder (Node 24 required for source builds)")?;
    if !status.success() {
        bail!("release candidate builder failed: {status}");
    }
    Ok(())
}

/// 允许 `Map` 与 `Value` 的互转（保持工具函数编译通过）。
#[allow(dead_code)]
fn into_map(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(map) => map,
        _ => Map::new(),
    }
}

/// 允许引用关系类型枚举（保持注册表编译通过）。
#[allow(dead_code)]
fn relation_kind_name(kind: RelationKind) -> String {
    kind.as_str().to_string()
}

/// 允许引用实体类型枚举（保持注册表编译通过）。
#[allow(dead_code)]
fn entity_kind_name(kind: EntityKind) -> String {
    kind.as_str().to_string()
}
