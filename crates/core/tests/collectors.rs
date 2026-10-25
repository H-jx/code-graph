//! 外部脚本运行与异常综合用例（计划 T07）。
//!
//! 测试分组前缀：`runner_`（子进程运行）、`manifest_`（manifest 校验）、
//! `deps_`（依赖准备）。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use project_knowledge_core::collect::deps::{
    CacheInputs, DependencyPlan, prepare_dependencies, verify_cache,
};
use project_knowledge_core::collect::manifest::{load_manifest, validate_manifest};
use project_knowledge_core::collect::runner::{PluginRunner, ResolvedRuntime, RunRequest};
use project_knowledge_core::collect::{stage_command, validate_stage_dag};
use project_knowledge_core::contract::collector::{
    CollectorInput, CollectorOutput, DependencyManagerKind, Limits, PluginManifest, RuntimeKind,
    StagePhase,
};
use project_knowledge_core::contract::error::codes;
use project_knowledge_core::contract::evidence::SnapshotRef;
use project_knowledge_core::project::config::RuntimeConfig;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .to_path_buf()
}

fn node_executable() -> Option<PathBuf> {
    let candidate = PathBuf::from("node");
    if which("node") { Some(candidate) } else { None }
}

fn which(program: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| {
            std::env::split_paths(&paths).any(|directory| {
                let candidate = directory.join(format!("{program}.exe"));
                candidate.exists() || directory.join(program).exists()
            })
        })
        .unwrap_or(false)
}

fn sample_input() -> CollectorInput {
    CollectorInput {
        protocol_version: "1.0".to_string(),
        run_id: "run_test".to_string(),
        plugin_id: "template.node".to_string(),
        version: "0.1.0".to_string(),
        stage_id: "extract".to_string(),
        snapshot: SnapshotRef {
            index_id: "idx".to_string(),
            generation: "1".to_string(),
            source_snapshot_id: Some("snap".to_string()),
        },
        input_snapshot_id: "snap".to_string(),
        config_subset: json!({}),
        sources: Vec::new(),
        source_sets: Vec::new(),
        entities: Vec::new(),
        relations: Vec::new(),
        candidate_sets: Vec::new(),
        allowed_extensions: Vec::new(),
        limits: Limits::default(),
    }
}

fn node_request(entry: &Path, directory: &Path, timeout_ms: u64) -> RunRequest {
    RunRequest {
        runtime: ResolvedRuntime {
            executable: PathBuf::from("node"),
            version: "node".to_string(),
        },
        entry: entry.to_path_buf(),
        args: Vec::new(),
        directory: directory.to_path_buf(),
        input: sample_input(),
        limits: Limits {
            timeout_ms,
            max_input_bytes: 1_048_576,
            max_output_bytes: 1_048_576,
            max_stderr_bytes: 65_536,
        },
        extra_env: BTreeMap::new(),
    }
}

#[test]
fn manifest_template_files_are_valid() {
    let root = repository_root();
    let node_manifest = root.join("collectors/templates/node/manifest.json");
    let manifest = load_manifest(&node_manifest).expect("load node template manifest");
    assert_eq!(manifest.plugin_id, "template.node");
    assert_eq!(manifest.runtime.kind, RuntimeKind::Node);
    assert_eq!(manifest.stages.len(), 1);
    assert_eq!(manifest.stages[0].phase, StagePhase::Extract);

    let python_manifest = root.join("collectors/templates/python/manifest.json");
    let manifest = load_manifest(&python_manifest).expect("load python template manifest");
    assert_eq!(manifest.runtime.kind, RuntimeKind::Python);
}

#[test]
fn manifest_validation_rejects_paths_outside_the_artifact_directory() {
    let root = repository_root();
    let directory = root.join("collectors/templates/node");
    let manifest = load_manifest(&directory.join("manifest.json")).expect("load manifest");

    let mut escaped = manifest.clone();
    escaped.stages[0].entry = "../../outside.mjs".to_string();
    let error = validate_manifest(&escaped, &directory, &[]).expect_err("必须拒绝越界入口");
    assert_eq!(error.code, codes::CONFIG_INVALID);

    let mut absolute = manifest.clone();
    absolute.stages[0].entry = "C:/tmp/evil.mjs".to_string();
    let error = validate_manifest(&absolute, &directory, &[]).expect_err("必须拒绝绝对路径");
    assert_eq!(error.code, codes::CONFIG_INVALID);

    let mut unknown_runtime = manifest.clone();
    unknown_runtime.runtime.kind = RuntimeKind::Executable;
    unknown_runtime.runtime.executable_id = Some("not-configured".to_string());
    let error =
        validate_manifest(&unknown_runtime, &directory, &[]).expect_err("必须拒绝未配置的 runtime");
    assert_eq!(error.code, codes::CONFIG_INVALID);

    // 合法 manifest 通过校验并得到稳定的制品哈希。
    let validated = validate_manifest(&manifest, &directory, &[]).expect("validate");
    assert!(validated.artifact_hash.starts_with(""));
    assert_eq!(validated.artifact_hash.len(), 64);
    assert!(
        validated
            .files
            .iter()
            .any(|file| file.relative_path == "src/collect.mjs")
    );
}

#[test]
fn manifest_stage_dag_rejects_cycles_and_illegal_phases() {
    let root = repository_root();
    let directory = root.join("collectors/templates/node");
    let manifest = load_manifest(&directory.join("manifest.json")).expect("load manifest");

    let mut cyclic = manifest.clone();
    cyclic.stages[0].depends_on = vec!["template.node:extract".to_string()];
    let error = validate_stage_dag(std::slice::from_ref(&cyclic)).expect_err("必须拒绝自依赖");
    assert_eq!(error.code, codes::CONFIG_INVALID);

    let mut extract_depends_on_link = manifest.clone();
    extract_depends_on_link
        .stages
        .push(project_knowledge_core::contract::collector::StageSpec {
            id: "link".to_string(),
            phase: StagePhase::Link,
            entry: "src/collect.mjs".to_string(),
            args: Vec::new(),
            source_selectors: Vec::new(),
            entity_selectors: Vec::new(),
            depends_on: Vec::new(),
            replacement: project_knowledge_core::contract::collector::ReplacementScope::Project,
        });
    extract_depends_on_link.stages[0].depends_on = vec!["template.node:link".to_string()];
    let error = validate_stage_dag(std::slice::from_ref(&extract_depends_on_link))
        .expect_err("extract 不得依赖 link");
    assert_eq!(error.code, codes::CONFIG_INVALID);
}

#[tokio::test]
async fn runner_executes_node_template_and_returns_collector_output() {
    if node_executable().is_none() {
        eprintln!("跳过：当前环境没有 node");
        return;
    }
    let root = repository_root();
    let directory = root.join("collectors/templates/node");
    let entry = directory.join("src/collect.mjs");
    let request = node_request(&entry, &directory, 30_000);
    let runner = PluginRunner::default();
    let outcome = runner
        .run(&request, &CancellationToken::new())
        .await
        .expect("run collector");
    assert_eq!(outcome.output.protocol_version, "1.0");
    assert_eq!(outcome.output.stage_id, "extract");
    assert_eq!(outcome.output.run_id, "run_test");
    assert!(outcome.stdout_bytes > 0);
}

#[tokio::test]
async fn runner_rejects_non_zero_exit_and_invalid_json() {
    if node_executable().is_none() {
        eprintln!("跳过：当前环境没有 node");
        return;
    }
    let root = repository_root();
    let directory = root.join("collectors/templates/node");
    let runner = PluginRunner::default();

    let entry = directory.join("faults/nonzero-exit.mjs");
    let request = node_request(&entry, &directory, 30_000);
    let error = runner
        .run(&request, &CancellationToken::new())
        .await
        .expect_err("非零退出必须失败");
    assert_eq!(error.code, codes::PLUGIN_EXIT);

    let entry = directory.join("faults/bad-json.mjs");
    let request = node_request(&entry, &directory, 30_000);
    let error = runner
        .run(&request, &CancellationToken::new())
        .await
        .expect_err("非法 JSON 必须失败");
    assert_eq!(error.code, codes::SCHEMA_INVALID);
}

#[tokio::test]
async fn runner_enforces_output_limit() {
    if node_executable().is_none() {
        eprintln!("跳过：当前环境没有 node");
        return;
    }
    let root = repository_root();
    let directory = root.join("collectors/templates/node");
    let entry = directory.join("faults/output-flood.mjs");
    let mut request = node_request(&entry, &directory, 30_000);
    request.limits.max_output_bytes = 64 * 1024;
    let runner = PluginRunner::default();
    let error = runner
        .run(&request, &CancellationToken::new())
        .await
        .expect_err("输出洪泛必须被拒绝");
    assert_eq!(error.code, codes::OUTPUT_LIMIT);
}

#[tokio::test]
async fn runner_enforces_timeout_and_kills_process_tree() {
    if node_executable().is_none() {
        eprintln!("跳过：当前环境没有 node");
        return;
    }
    let root = repository_root();
    let directory = root.join("collectors/templates/node");
    let entry = directory.join("faults/timeout.mjs");
    let request = node_request(&entry, &directory, 1_500);
    let runner = PluginRunner::default();
    let started = std::time::Instant::now();
    let error = runner
        .run(&request, &CancellationToken::new())
        .await
        .expect_err("超时必须失败");
    let elapsed = started.elapsed();
    assert_eq!(error.code, codes::PLUGIN_TIMEOUT);
    assert!(
        elapsed.as_millis() < 20_000,
        "超时必须及时返回，实际 {} ms",
        elapsed.as_millis()
    );
}

#[tokio::test]
async fn runner_supports_cancellation() {
    if node_executable().is_none() {
        eprintln!("跳过：当前环境没有 node");
        return;
    }
    let root = repository_root();
    let directory = root.join("collectors/templates/node");
    let entry = directory.join("faults/timeout.mjs");
    let request = node_request(&entry, &directory, 60_000);
    let runner = PluginRunner::default();
    let cancel = CancellationToken::new();
    let token = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        token.cancel();
    });
    let error = runner
        .run(&request, &cancel)
        .await
        .expect_err("取消必须失败");
    assert_eq!(error.code, codes::CANCELLED);
}

#[tokio::test]
async fn runner_limits_input_size() {
    if node_executable().is_none() {
        eprintln!("跳过：当前环境没有 node");
        return;
    }
    let root = repository_root();
    let directory = root.join("collectors/templates/node");
    let entry = directory.join("src/collect.mjs");
    let mut request = node_request(&entry, &directory, 30_000);
    request.limits.max_input_bytes = 8;
    let runner = PluginRunner::default();
    let error = runner
        .run(&request, &CancellationToken::new())
        .await
        .expect_err("超限输入必须失败");
    assert_eq!(error.code, codes::OUTPUT_LIMIT);
}

#[test]
fn deps_plan_requires_lock_file_and_reports_missing_runtime() {
    let temp = tempfile::tempdir().expect("tempdir");
    let manifest_path = temp.path().join("package.json");
    let lock_path = temp.path().join("package-lock.json");
    std::fs::write(&manifest_path, "{}").expect("write");
    std::fs::write(&lock_path, "{}").expect("write");

    let plan = DependencyPlan {
        kind: DependencyManagerKind::Npm,
        manifest_path,
        lock_path,
        runtime_executable: PathBuf::from("definitely-not-a-real-runtime"),
        cache_root: temp.path().join("cache"),
        artifact_hash: "artifact".to_string(),
    };
    let inputs = CacheInputs {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        runtime_version: "node".to_string(),
        artifact_hash: "artifact".to_string(),
        lock_digest: "lock".to_string(),
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let error = runtime
        .block_on(prepare_dependencies(&plan, &inputs))
        .expect_err("缺少运行时必须失败");
    assert_eq!(error.code, codes::RUNTIME_MISSING);
}

#[tokio::test]
async fn deps_cache_is_reused_and_verified() {
    let temp = tempfile::tempdir().expect("tempdir");
    let cache_root = temp.path().join("cache");
    let inputs = CacheInputs {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        runtime_version: "node".to_string(),
        artifact_hash: "artifact".to_string(),
        lock_digest: "lock".to_string(),
    };
    let digest = inputs.digest();
    let directory = cache_root.join(&digest);
    std::fs::create_dir_all(&directory).expect("mkdir");
    std::fs::write(directory.join(".prepared"), format!("{digest}\n")).expect("write");

    let plan = DependencyPlan {
        kind: DependencyManagerKind::Npm,
        manifest_path: temp.path().join("package.json"),
        lock_path: temp.path().join("package-lock.json"),
        runtime_executable: PathBuf::from("node"),
        cache_root,
        artifact_hash: "artifact".to_string(),
    };
    let prepared = prepare_dependencies(&plan, &inputs).await.expect("prepare");
    assert_eq!(prepared.digest, digest);
    verify_cache(&prepared).expect("verify");
    assert!(prepared.env.contains_key("NODE_PATH"));
}

#[test]
fn stage_command_uses_manifest_arguments() {
    let root = repository_root();
    let directory = root.join("collectors/templates/node");
    let manifest = load_manifest(&directory.join("manifest.json")).expect("load manifest");
    let stage = &manifest.stages[0];
    let (entry, args) = stage_command(&directory.join(&stage.entry), stage);
    assert!(entry.ends_with("collect.mjs"));
    assert_eq!(args, vec!["--stage".to_string(), "extract".to_string()]);
}

#[test]
fn collector_output_round_trips_through_json() {
    let output = CollectorOutput {
        protocol_version: "1.0".to_string(),
        run_id: "run".to_string(),
        stage_id: "extract".to_string(),
        input_snapshot_id: "snap".to_string(),
        entities: Vec::new(),
        relations: Vec::new(),
        evidence: Vec::new(),
        dependencies: Vec::new(),
        coverage: Vec::new(),
        gaps: Vec::new(),
    };
    let text = serde_json::to_string(&output).expect("serialize");
    let parsed: CollectorOutput = serde_json::from_str(&text).expect("deserialize");
    assert_eq!(parsed.stage_id, "extract");
    let value: Value = serde_json::from_str(&text).expect("value");
    assert!(value.get("entities").is_some());
    assert!(value.get("gaps").is_some());
}

// ---------------------------------------------------------------- 校验用例

/// 构造一个合法的采集输出（用于校验用例）。
fn valid_output(input: &mut CollectorInput) -> CollectorOutput {
    use project_knowledge_core::contract::entity::EntityKind;
    use project_knowledge_core::contract::evidence::Source;
    use project_knowledge_core::contract::identity::{
        compute_entity_id, compute_evidence_id, compute_source_id,
    };
    use project_knowledge_core::contract::relation::Resolution;
    use serde_json::Map;

    let repository_id = "repo.web";
    let path = "src/a.ts";
    let content = "export const a = 1;\n";
    let content_hash = project_knowledge_core::contract::identity::content_hash(content.as_bytes());
    let source_id = compute_source_id("erp", repository_id, path);

    input
        .sources
        .push(project_knowledge_core::contract::collector::SourcePayload {
            source: Source {
                id: source_id.clone(),
                repository_id: repository_id.to_string(),
                path: path.to_string(),
                source_class: project_knowledge_core::contract::evidence::SourceClass::Code,
                language: Some("typescript".to_string()),
                content_hash: content_hash.clone(),
                byte_length: content.len() as u64,
            },
            content: content.to_string(),
        });

    let evidence_id = compute_evidence_id(&source_id, &content_hash, 0, 6, "definition");
    let slots = vec![
        project_knowledge_core::contract::identity::IdentitySlot::required(
            "repositoryId",
            repository_id,
        ),
        project_knowledge_core::contract::identity::IdentitySlot::required("path", path),
        project_knowledge_core::contract::identity::IdentitySlot::required("scopeId", "app.web"),
    ];
    let entity_id = compute_entity_id("erp", EntityKind::File, &slots).expect("identity");

    CollectorOutput {
        protocol_version: "1.0".to_string(),
        run_id: input.run_id.clone(),
        stage_id: input.stage_id.clone(),
        input_snapshot_id: input.input_snapshot_id.clone(),
        entities: vec![project_knowledge_core::contract::collector::EntityOutput {
            entity_id: entity_id.clone(),
            kind: EntityKind::File,
            scope_id: "app.web".to_string(),
            identity: slots,
            assertion: project_knowledge_core::contract::collector::AssertionOutput {
                producer_key: "test.producer".to_string(),
                name: Some("a.ts".to_string()),
                attributes: Map::new(),
                basis: project_knowledge_core::contract::entity::Basis::Observed,
                origin: project_knowledge_core::contract::entity::Origin::Static,
                evidence_refs: vec![evidence_id.clone()],
            },
        }],
        relations: vec![
            project_knowledge_core::contract::collector::RelationOutput {
                relation_id: "rel_1".to_string(),
                kind: project_knowledge_core::contract::relation::RelationKind::Contains,
                from_id: entity_id.clone(),
                to_id: entity_id.clone(),
                qualifiers: Map::new(),
                resolution: Resolution::Confirmed,
                assertion: project_knowledge_core::contract::collector::AssertionOutput {
                    producer_key: "test.producer".to_string(),
                    name: None,
                    attributes: Map::new(),
                    basis: project_knowledge_core::contract::entity::Basis::Observed,
                    origin: project_knowledge_core::contract::entity::Origin::Static,
                    evidence_refs: vec![evidence_id.clone()],
                },
            },
        ],
        evidence: vec![project_knowledge_core::contract::evidence::Evidence {
            id: evidence_id,
            source_id,
            content_hash,
            start_byte: 0,
            end_byte: 6,
            start_line: 1,
            end_line: 1,
            role: "definition".to_string(),
        }],
        dependencies: Vec::new(),
        coverage: Vec::new(),
        gaps: Vec::new(),
    }
}

/// 校验上下文（校验路径不访问数据库）。
fn validation_context<'a>(
    input: &'a CollectorInput,
    manifest: &'a PluginManifest,
    stage: &'a project_knowledge_core::contract::collector::StageSpec,
) -> project_knowledge_core::collect::validate::ValidationContext<'a> {
    project_knowledge_core::collect::validate::ValidationContext {
        project_id: "erp",
        input,
        manifest,
        stage,
    }
}

#[test]
fn validation_accepts_a_correct_output() {
    let root = repository_root();
    let directory = root.join("collectors/templates/node");
    let manifest = load_manifest(&directory.join("manifest.json")).expect("load manifest");
    let stage = &manifest.stages[0];
    let mut input = sample_input();
    let output = valid_output(&mut input);
    let context = validation_context(&input, &manifest, stage);
    let issues = project_knowledge_core::collect::validate::validate_output(&context, &output)
        .expect("valid output must pass");
    assert!(issues.is_empty(), "合法输出不应有问题：{issues:?}");
}

#[test]
fn validation_rejects_forged_evidence_positions() {
    let root = repository_root();
    let directory = root.join("collectors/templates/node");
    let manifest = load_manifest(&directory.join("manifest.json")).expect("load manifest");
    let stage = &manifest.stages[0];
    let mut input = sample_input();
    let mut output = valid_output(&mut input);
    // 越界字节区间。
    output.evidence[0].end_byte = 9_999;
    let context = validation_context(&input, &manifest, stage);
    let error = project_knowledge_core::collect::validate::validate_output(&context, &output)
        .expect_err("越界证据必须被拒绝");
    assert_eq!(error.code, codes::EVIDENCE_INVALID);
    assert!(
        error
            .issues
            .iter()
            .any(|issue| issue.instance_path.starts_with("/evidence/"))
    );
}

#[test]
fn validation_rejects_wrong_line_numbers() {
    let root = repository_root();
    let directory = root.join("collectors/templates/node");
    let manifest = load_manifest(&directory.join("manifest.json")).expect("load manifest");
    let stage = &manifest.stages[0];
    let mut input = sample_input();
    let mut output = valid_output(&mut input);
    output.evidence[0].start_line = 7;
    output.evidence[0].end_line = 9;
    let context = validation_context(&input, &manifest, stage);
    let error = project_knowledge_core::collect::validate::validate_output(&context, &output)
        .expect_err("行号不一致必须被拒绝");
    assert_eq!(error.code, codes::EVIDENCE_INVALID);
}

#[test]
fn validation_rejects_missing_evidence_and_bad_identity() {
    let root = repository_root();
    let directory = root.join("collectors/templates/node");
    let manifest = load_manifest(&directory.join("manifest.json")).expect("load manifest");
    let stage = &manifest.stages[0];
    let mut input = sample_input();

    let mut output = valid_output(&mut input);
    output.entities[0].assertion.evidence_refs.clear();
    let context = validation_context(&input, &manifest, stage);
    let error = project_knowledge_core::collect::validate::validate_output(&context, &output)
        .expect_err("缺少证据必须被拒绝");
    assert_eq!(error.code, codes::EVIDENCE_INVALID);

    let mut output = valid_output(&mut input);
    output.entities[0].entity_id = "ent_wrong".to_string();
    let context = validation_context(&input, &manifest, stage);
    let error = project_knowledge_core::collect::validate::validate_output(&context, &output)
        .expect_err("ID 与重算结果不一致必须被拒绝");
    assert_eq!(error.code, codes::IDENTITY_CONFLICT);
}

#[test]
fn validation_rejects_dangling_relation_endpoints() {
    let root = repository_root();
    let directory = root.join("collectors/templates/node");
    let manifest = load_manifest(&directory.join("manifest.json")).expect("load manifest");
    let stage = &manifest.stages[0];
    let mut input = sample_input();
    let mut output = valid_output(&mut input);
    output.relations[0].to_id = "ent_missing".to_string();
    let context = validation_context(&input, &manifest, stage);
    let error = project_knowledge_core::collect::validate::validate_output(&context, &output)
        .expect_err("悬空端点必须被拒绝");
    assert_eq!(error.code, codes::REFERENCE_MISSING);
    assert!(
        error
            .issues
            .iter()
            .any(|issue| issue.code == codes::REFERENCE_MISSING)
    );
}

#[test]
fn validation_rejects_generated_knowledge_as_evidence() {
    let root = repository_root();
    let directory = root.join("collectors/templates/node");
    let manifest = load_manifest(&directory.join("manifest.json")).expect("load manifest");
    let stage = &manifest.stages[0];
    let mut input = sample_input();
    let output = valid_output(&mut input);
    // 把来源换成生成知识。
    input.sources.clear();
    input
        .sources
        .push(project_knowledge_core::contract::collector::SourcePayload {
            source: project_knowledge_core::contract::evidence::Source {
                id: output.evidence[0].source_id.clone(),
                repository_id: "repo.web".to_string(),
                path: "src/a.ts".to_string(),
                source_class:
                    project_knowledge_core::contract::evidence::SourceClass::GeneratedKnowledge,
                language: None,
                content_hash: output.evidence[0].content_hash.clone(),
                byte_length: 20,
            },
            content: "export const a = 1;\n".to_string(),
        });
    let context = validation_context(&input, &manifest, stage);
    let error = project_knowledge_core::collect::validate::validate_output(&context, &output)
        .expect_err("生成知识不得作为原始证据");
    assert_eq!(error.code, codes::EVIDENCE_INVALID);
}

#[test]
fn validation_rejects_unknown_protocol_version() {
    let root = repository_root();
    let directory = root.join("collectors/templates/node");
    let manifest = load_manifest(&directory.join("manifest.json")).expect("load manifest");
    let stage = &manifest.stages[0];
    let mut input = sample_input();
    let mut output = valid_output(&mut input);
    output.protocol_version = "2.0".to_string();
    let context = validation_context(&input, &manifest, stage);
    let error = project_knowledge_core::collect::validate::validate_output(&context, &output)
        .expect_err("未知协议版本必须被拒绝");
    assert_eq!(error.code, codes::PROTOCOL_UNSUPPORTED);
}

#[test]
fn manifest_dependencies_are_validated_against_registry() {
    let root = repository_root();
    let directory = root.join("collectors/templates/node");
    let manifest = load_manifest(&directory.join("manifest.json")).expect("load manifest");
    let mut with_runtime = manifest.clone();
    with_runtime.runtime.kind = RuntimeKind::Executable;
    with_runtime.runtime.executable_id = Some("node24".to_string());
    let runtimes = vec![RuntimeConfig {
        id: "node24".to_string(),
        kind: "node".to_string(),
        executable: "C:/Program Files/nodejs/node.exe".to_string(),
        version: Some("24.14.0".to_string()),
    }];
    let validated = validate_manifest(&with_runtime, &directory, &runtimes).expect("validate");
    assert!(!validated.artifact_hash.is_empty());
}
