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

#[path = "support/collection.rs"]
mod collection;

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
        evidence: Vec::new(),
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
    for cancel_run in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let marker = temp.path().join("owned-pids.json");
        let directory = repository_root().join("fixtures/plugins/staging");
        let mut request = node_request(&directory.join("collect.mjs"), &directory, 1500);
        request.args = vec!["--hold".into(), marker.to_string_lossy().to_string()];
        let cancel = CancellationToken::new();
        if cancel_run {
            let token = cancel.clone();
            let path = marker.clone();
            tokio::spawn(async move {
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
                while !path.exists() && std::time::Instant::now() < deadline {
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
                token.cancel();
            });
        }
        let started = std::time::Instant::now();
        let error = PluginRunner::default()
            .run(&request, &cancel)
            .await
            .unwrap_err();
        assert_eq!(
            error.code,
            if cancel_run {
                codes::CANCELLED
            } else {
                codes::PLUGIN_TIMEOUT
            }
        );
        assert!(started.elapsed().as_secs() < 20);
        let owned: Value = serde_json::from_slice(&std::fs::read(&marker).unwrap()).unwrap();
        for key in ["parent", "descendant"] {
            let pid = owned[key].as_u64().unwrap().to_string();
            let probe = "import fs from 'node:fs';const p=Number(process.argv[1]);try {process.kill(p,0);}catch(e){if(e.code==='ESRCH')process.exit(0);throw e;}if(process.platform==='linux'&&fs.readFileSync(`/proc/${p}/stat`,'utf8').split(') ')[1].startsWith('Z'))process.exit(0);process.exit(1);";
            let mut stopped = false;
            for _ in 0..30 {
                stopped = std::process::Command::new("node")
                    .args(["--input-type=module", "-e", probe, &pid])
                    .output()
                    .unwrap()
                    .status
                    .success();
                if stopped {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(30)).await;
            }
            if !stopped {
                let _ = std::process::Command::new("node")
                    .args(["-e", "process.kill(Number(process.argv[1]));", &pid])
                    .output();
            }
            assert!(
                stopped,
                "owned {key} PID {pid} survived timeout/cancellation"
            );
        }
    }
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

#[tokio::test]
async fn runner_bounds_blocked_input_and_simultaneous_output_floods() {
    let temp = tempfile::tempdir().unwrap();
    let entry = temp.path().join("blocked.mjs");
    for stream in [None, Some(1), Some(2)] {
        let script = if let Some(stream) = stream {
            format!(
                "import fs from 'node:fs'; fs.writeSync({stream},Buffer.alloc(4*1024*1024,120)); setInterval(()=>{{}},1000);"
            )
        } else {
            "setInterval(()=>{},1000);".into()
        };
        std::fs::write(&entry, script).unwrap();
        let mut request = node_request(&entry, temp.path(), 1500);
        request.input.config_subset = json!({"padding":"x".repeat(2*1024*1024)});
        request.limits.max_input_bytes = 4 * 1024 * 1024;
        request.limits.max_output_bytes = 1024;
        request.limits.max_stderr_bytes = 1024;
        let started = std::time::Instant::now();
        let error = PluginRunner::default()
            .run(&request, &CancellationToken::new())
            .await
            .unwrap_err();
        assert_eq!(
            error.code,
            if stream.is_some() {
                codes::OUTPUT_LIMIT
            } else {
                codes::PLUGIN_TIMEOUT
            },
            "{error:?}"
        );
        assert!(started.elapsed().as_secs() < 10);
    }
}

#[tokio::test]
async fn runner_python_template_faults_and_unicode_paths() {
    let configured = std::env::var_os("PROJECT_KNOWLEDGE_PYTHON").map(PathBuf::from);
    let python = configured
        .or_else(|| {
            ["python", "python3"].iter().find_map(|candidate| {
                let output = std::process::Command::new(candidate)
                    .arg("--version")
                    .output()
                    .ok()?;
                (output.status.success()
                    && String::from_utf8_lossy(&output.stdout).starts_with("Python 3.12."))
                .then(|| PathBuf::from(candidate))
            })
        })
        .or_else(|| {
            let output = std::process::Command::new("uv")
                .args(["python", "find", "3.12"])
                .output()
                .ok()?;
            output
                .status
                .success()
                .then(|| PathBuf::from(String::from_utf8_lossy(&output.stdout).trim()))
        })
        .expect("T07 requires configured Python 3.12 or uv python find 3.12");
    let temp = tempfile::Builder::new()
        .prefix("采集 空格-")
        .tempdir()
        .unwrap();
    for (relative, expected) in [
        ("src/collect.py", None),
        ("faults/nonzero_exit.py", Some(codes::PLUGIN_EXIT)),
        ("faults/bad_json.py", Some(codes::SCHEMA_INVALID)),
        ("faults/timeout.py", Some(codes::PLUGIN_TIMEOUT)),
    ] {
        let entry = temp.path().join("collect.py");
        std::fs::write(
            &entry,
            std::fs::read(
                repository_root()
                    .join("collectors/templates/python")
                    .join(relative),
            )
            .unwrap(),
        )
        .unwrap();
        let mut request = node_request(&entry, temp.path(), 1500);
        request.runtime.executable = python.clone();
        request.runtime.version = "Python 3.12".into();
        let result = PluginRunner::default()
            .run(&request, &CancellationToken::new())
            .await;
        if let Some(code) = expected {
            assert_eq!(result.unwrap_err().code, code);
        } else {
            assert!(result.unwrap().output.entities.is_empty());
        }
    }
}

#[test]
fn deps_plan_requires_lock_file_and_reports_missing_runtime() {
    let temp = tempfile::tempdir().expect("tempdir");
    let manifest_path = temp.path().join("package.json");
    let lock_path = temp.path().join("package-lock.json");
    std::fs::write(&manifest_path, "{}").expect("write");
    std::fs::write(&lock_path, "{}").expect("write");

    let plan = DependencyPlan {
        artifact: None,
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
    std::fs::remove_dir(&directory).unwrap();
    std::fs::write(
        temp.path().join("package.json"),
        r#"{"name":"empty-deps","version":"1.0.0"}"#,
    )
    .unwrap();
    std::fs::write(temp.path().join("package-lock.json"),r#"{"name":"empty-deps","version":"1.0.0","lockfileVersion":3,"requires":true,"packages":{"":{"name":"empty-deps","version":"1.0.0"}}}"#).unwrap();

    let plan = DependencyPlan {
        artifact: None,
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
    let again = prepare_dependencies(&plan, &inputs).await.unwrap();
    assert_eq!(again.integrity, prepared.integrity);
    std::fs::write(
        prepared.directory.join("node_modules/tampered.js"),
        "export default true",
    )
    .unwrap();
    assert_eq!(
        verify_cache(&prepared).unwrap_err().code,
        codes::DEPENDENCY_CHANGED
    );
    assert_eq!(
        prepare_dependencies(&plan, &inputs).await.unwrap_err().code,
        codes::DEPENDENCY_CHANGED
    );
    std::fs::remove_file(prepared.directory.join("node_modules/tampered.js")).unwrap();
    verify_cache(&prepared).unwrap();
    let package = prepared.directory.join("package.json");
    let bytes = std::fs::read(&package).unwrap();
    std::fs::remove_file(&package).unwrap();
    assert_eq!(
        verify_cache(&prepared).unwrap_err().code,
        codes::DEPENDENCY_CHANGED
    );
    std::fs::write(package, bytes).unwrap();
    verify_cache(&prepared).unwrap();
}

#[tokio::test]
async fn runner_staging_never_resolves_business_node_modules() {
    let f = collection::Fixture::new();
    let package = f.root.join("node_modules/forbidden-business-package");
    std::fs::create_dir_all(&package).unwrap();
    std::fs::write(
        package.join("package.json"),
        r#"{"type":"module","exports":"./index.mjs"}"#,
    )
    .unwrap();
    std::fs::write(package.join("index.mjs"), "export default 1;").unwrap();
    let entry = f.manifest.parent().unwrap().join("collect.mjs");
    let original = std::fs::read_to_string(&entry).unwrap();
    std::fs::write(
        entry,
        format!("import 'forbidden-business-package';\n{original}"),
    )
    .unwrap();
    let error = f.stage(None).await.err().unwrap();
    assert_eq!(error.code, codes::PLUGIN_EXIT, "{error:?}");
    assert!(error.message.contains("forbidden-business-package"));
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
                relation_id: project_knowledge_core::contract::identity::compute_relation_id(
                    "erp",
                    project_knowledge_core::contract::relation::RelationKind::Contains,
                    &entity_id,
                    &entity_id,
                    &Map::new(),
                )
                .expect("relation identity"),
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
        registry: None,
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
    let mut relation_only = output.clone();
    let entity = relation_only.entities.pop().unwrap();
    input
        .entities
        .push(project_knowledge_core::contract::collector::EntityInput {
            entity_id: entity.entity_id,
            kind: entity.kind,
            scope_id: entity.scope_id,
            identity: entity.identity,
            assertions: vec![],
        });
    let error = project_knowledge_core::collect::validate::validate_output(
        &validation_context(&input, &manifest, stage),
        &relation_only,
    )
    .unwrap_err();
    assert_eq!(error.code, codes::EVIDENCE_INVALID);
    assert_eq!(error.issues[0].instance_path, "/evidence/0");
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

#[tokio::test]
async fn validation_staging_reexports_extract_and_commits_group_once() {
    let mut f = collection::Fixture::new();
    let generation = f.database.generation().unwrap();
    let staged = f.stage(None).await.unwrap();
    let report = staged.report();
    assert_eq!(report["stages"].as_array().unwrap().len(), 2);
    assert_eq!(f.database.generation().unwrap(), generation);
    assert_eq!(
        f.count("SELECT count(*) FROM entities WHERE kind = 'route'"),
        0
    );
    let next = staged
        .commit(&f.project, &mut f.database, "now")
        .await
        .unwrap();
    assert_eq!(next, generation + 1);
    assert_eq!(f.count("SELECT count(*) FROM current_batches"), 3);
    assert_eq!(f.count("SELECT count(DISTINCT kind) FROM dependencies WHERE batch_id IN (SELECT batch_id FROM current_batches)"), 7);
    assert_eq!(
        f.count("SELECT count(*) FROM search_documents WHERE text LIKE '%Smoke route%'"),
        1
    );
    assert_eq!(
        f.count("SELECT count(*) FROM assertions WHERE producer_key = 'untrusted-output-owner'"),
        0
    );
    // 再跑相同插件时，extract 不能读到自己旧批次；新 link 只引用新 extract。
    let staged = f.stage(None).await.unwrap();
    staged
        .commit(&f.project, &mut f.database, "again")
        .await
        .unwrap();
    assert_eq!(f.database.generation().unwrap(), generation + 2);
}

#[tokio::test]
async fn validation_bad_link_and_forged_dependency_leave_formal_database_unchanged() {
    let f = collection::Fixture::new();
    let before = std::fs::read(f.database.path()).unwrap();
    let generation = f.database.generation().unwrap();
    f.edit(|m| m["stages"][1]["args"] = json!(["--bad"]));
    let error = f.stage(None).await.err().unwrap();
    assert_eq!(error.code, codes::EVIDENCE_INVALID);
    assert!(
        error
            .issues
            .iter()
            .any(|i| i.instance_path == "/evidence/0")
    );
    assert_eq!(f.database.generation().unwrap(), generation);
    assert_eq!(std::fs::read(f.database.path()).unwrap(), before);
    assert_eq!(
        f.count("SELECT count(*) FROM entities WHERE kind = 'route'"),
        0
    );
    f.edit(|m| {
        m["stages"][1]["args"] = json!([]);
        m["stages"][0]["args"] = json!(["--forge-dep"]);
    });
    assert_eq!(
        f.stage(None).await.err().unwrap().code,
        codes::DEPENDENCY_CHANGED
    );
}

#[tokio::test]
async fn validation_acceptance_replays_fixture_and_failure_blocks_commit() {
    use project_knowledge_core::contract::entity::EntityKind;
    use project_knowledge_core::contract::identity::{IdentitySlot, compute_entity_id};
    let f = collection::Fixture::new();
    let id = compute_entity_id(
        "smoke",
        EntityKind::Route,
        &[
            IdentitySlot::required("scopeId", "repo.r"),
            IdentitySlot::required("routerId", "router"),
            IdentitySlot::required("mode", "history"),
            IdentitySlot::required("declaredPath", "/smoke"),
        ],
    )
    .unwrap();
    f.add_case(&id);
    assert_eq!(
        f.stage(None).await.unwrap().report()["cases"][0]["passed"],
        true
    );
    let generation = f.database.generation().unwrap();
    f.add_case("ent_does_not_exist");
    let error = f.stage(None).await.err().unwrap();
    assert_eq!(error.issues[0].instance_path, "/expected/entities/0");
    assert_eq!(f.database.generation().unwrap(), generation);
    assert_eq!(
        f.count("SELECT count(*) FROM runs WHERE kind = 'collect'"),
        0
    );
}

#[tokio::test]
async fn validation_empty_extract_invalidates_downstream_and_preserves_base() {
    let mut f = collection::Fixture::new();
    let group = f.stage(None).await.unwrap();
    group
        .commit(&f.project, &mut f.database, "now")
        .await
        .unwrap();
    let link = project_knowledge_core::storage::current_batch(
        f.database.connection(),
        "(smoke.plugin,link,smoke)",
    )
    .unwrap()
    .unwrap();
    f.edit(|m| m["stages"][0]["args"] = json!(["--empty"]));
    let group = f.stage(Some(StagePhase::Extract)).await.unwrap();
    group
        .commit(&f.project, &mut f.database, "now")
        .await
        .unwrap();
    assert_eq!(
        project_knowledge_core::storage::batch_state(f.database.connection(), &link).unwrap(),
        Some(project_knowledge_core::storage::BatchState::Stale)
    );
    assert_eq!(f.count("SELECT count(*) FROM entity_assertions ea JOIN current_batches cb ON cb.batch_id = ea.batch_id JOIN batches b ON b.id = cb.batch_id JOIN entities e ON e.id = ea.entity_id WHERE e.kind = 'route' AND b.state = 'fresh'"), 0);
    assert!(f.count("SELECT count(*) FROM entity_assertions ea JOIN current_batches cb ON cb.batch_id = ea.batch_id WHERE ea.name = 'a.ts'") > 0);
}

#[tokio::test]
async fn validation_commit_rechecks_source_artifact_and_generation() {
    let mut f = collection::Fixture::new();
    let first = f.stage(None).await.unwrap();
    let second = f.stage(None).await.unwrap();
    first
        .commit(&f.project, &mut f.database, "now")
        .await
        .unwrap();
    assert_eq!(
        second
            .commit(&f.project, &mut f.database, "now")
            .await
            .unwrap_err()
            .code,
        codes::ACTIVATION_CONFLICT
    );
    let group = f.stage(None).await.unwrap();
    std::fs::write(f.root.join("repo/new.ts"), "export const added = true;").unwrap();
    assert_eq!(
        group
            .commit(&f.project, &mut f.database, "now")
            .await
            .unwrap_err()
            .code,
        codes::SOURCE_CHANGED
    );
    std::fs::remove_file(f.root.join("repo/new.ts")).unwrap();
    let group = f.stage(None).await.unwrap();
    f.edit(|m| m["version"] = json!("2.0.0"));
    assert_eq!(
        group
            .commit(&f.project, &mut f.database, "now")
            .await
            .unwrap_err()
            .code,
        codes::ARTIFACT_CHANGED
    );
}

#[test]
fn validation_rejects_truncated_candidates_and_relation_identity() {
    let manifest =
        load_manifest(&repository_root().join("collectors/templates/node/manifest.json")).unwrap();
    let mut input = sample_input();
    let mut output = valid_output(&mut input);
    input
        .candidate_sets
        .push(project_knowledge_core::contract::collector::CandidateSet {
            selector_id: "targets".into(),
            digest: "digest".into(),
            entity_ids: vec![],
            complete: false,
        });
    let error = project_knowledge_core::collect::validate::validate_output(
        &validation_context(&input, &manifest, &manifest.stages[0]),
        &output,
    )
    .unwrap_err();
    assert_eq!(error.code, codes::CANDIDATE_SET_INCOMPLETE);
    input.candidate_sets.clear();
    output.relations[0].relation_id = "forged".into();
    assert_eq!(
        project_knowledge_core::collect::validate::validate_output(
            &validation_context(&input, &manifest, &manifest.stages[0]),
            &output
        )
        .unwrap_err()
        .code,
        codes::IDENTITY_CONFLICT
    );
}

#[test]
fn validation_schema_invalid_fixtures_match_serde_and_exact_pointers() {
    let vectors: Vec<Value> = serde_json::from_slice(
        &std::fs::read(repository_root().join("fixtures/protocol/invalid-output-vectors.json"))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(vectors.len(), 4);
    for vector in vectors {
        let value = &vector["value"];
        assert!(
            serde_json::from_value::<CollectorOutput>(value.clone()).is_err(),
            "{}",
            vector["id"]
        );
        let error =
            project_knowledge_core::contract::schema::validate::<CollectorOutput>(value, "")
                .unwrap_err();
        assert_eq!(error.code, vector["expected"]["code"].as_str().unwrap());
        assert!(
            error
                .issues
                .iter()
                .any(|i| i.instance_path == vector["expected"]["instancePath"].as_str().unwrap()),
            "{error:?}"
        );
    }
}

#[test]
fn manifest_extensions_reject_remote_refs_namespace_hijacks_and_public_overrides() {
    use project_knowledge_core::contract::extensions::ExtensionRegistry;
    let original: Value = serde_json::from_slice(
        &std::fs::read(repository_root().join("fixtures/plugins/extensions/manifest.json"))
            .unwrap(),
    )
    .unwrap();
    let manifest: PluginManifest = serde_json::from_value(original.clone()).unwrap();
    assert_eq!(
        ExtensionRegistry::from_manifests([&manifest])
            .unwrap()
            .names()
            .len(),
        2
    );
    let cases = [
        (
            "/extensions/entities/0/name",
            json!("x.other.invoice"),
            "/extensions/entities/0/name",
        ),
        (
            "/extensions/entities/0/identityComponents/0/name",
            json!("name"),
            "/extensions/entities/0/identityComponents",
        ),
        (
            "/extensions/entities/0/attributesSchema/properties/total/$ref",
            json!("https://example.invalid/schema"),
            "/extensions/entities/0/attributesSchema/properties/total/$ref",
        ),
        (
            "/extensions/entities/0/attributesSchema/properties/total/$ref",
            json!("file:///C:/schema.json"),
            "/extensions/entities/0/attributesSchema/properties/total/$ref",
        ),
        (
            "/extensions/entities/0/attributesSchema/properties/total/$ref",
            json!("#/$defs/Missing"),
            "/extensions/entities/0/attributesSchema",
        ),
        (
            "/extensions/relations/0/allowedFrom",
            json!([]),
            "/extensions/relations/0",
        ),
    ];
    for (pointer, value, expected) in cases {
        let mut bad = original.clone();
        *bad.pointer_mut(pointer).unwrap() = value;
        let manifest: PluginManifest = serde_json::from_value(bad).unwrap();
        let error = ExtensionRegistry::from_manifests([&manifest])
            .err()
            .unwrap();
        assert_eq!(error.code, codes::SCHEMA_INVALID);
        assert!(
            error.issues.iter().any(|i| i.instance_path == expected),
            "{error:?}"
        );
    }
    for (field, value) in [
        ("$id", json!("https://example.invalid/")),
        ("$dynamicRef", json!("#/$defs/Amount")),
        ("$schema", json!("http://json-schema.org/draft-07/schema#")),
    ] {
        let mut bad = original.clone();
        bad["extensions"]["entities"][0]["attributesSchema"][field] = value;
        let manifest: PluginManifest = serde_json::from_value(bad).unwrap();
        assert!(ExtensionRegistry::from_manifests([&manifest]).is_err());
    }
    let mut bad = original.clone();
    bad["extensions"]["entities"][0]["attributesSchema"]["$defs"]["Entity"] =
        json!({"type":"string"});
    let manifest: PluginManifest = serde_json::from_value(bad).unwrap();
    assert!(ExtensionRegistry::from_manifests([&manifest]).is_err());
    let valid: PluginManifest = serde_json::from_value(original.clone()).unwrap();
    assert!(ExtensionRegistry::from_manifests([&valid, &valid]).is_err());
    // 声明不能覆盖公共字段；载荷仍受各自 schema 约束。
    let mut bad = original;
    bad["extensions"]["entities"][0]["basis"] = json!("observed");
    assert!(serde_json::from_value::<PluginManifest>(bad.clone()).is_err());
    assert!(
        project_knowledge_core::contract::schema::validate::<PluginManifest>(&bad, "").is_err()
    );
}

#[test]
fn validation_unregistered_extension_names_are_semantic_errors() {
    use project_knowledge_core::contract::{
        EntityKind,
        identity::{IdentitySlot, compute_entity_id},
    };
    let manifest =
        load_manifest(&repository_root().join("collectors/templates/node/manifest.json")).unwrap();
    let mut input = sample_input();
    let mut output = valid_output(&mut input);
    output.relations.clear();
    let entity = &mut output.entities[0];
    entity.kind = EntityKind::parse("x.unregistered.invoice").unwrap();
    entity.identity = vec![
        IdentitySlot::required("scopeId", "app.web"),
        IdentitySlot::required("semanticKey", "invoice"),
    ];
    entity.entity_id = compute_entity_id("erp", entity.kind.clone(), &entity.identity).unwrap();
    project_knowledge_core::contract::schema::validate::<CollectorOutput>(
        &serde_json::to_value(&output).unwrap(),
        "",
    )
    .unwrap();
    let error = project_knowledge_core::collect::validate::validate_output(
        &validation_context(&input, &manifest, &manifest.stages[0]),
        &output,
    )
    .unwrap_err();
    assert_eq!(error.code, codes::SCHEMA_INVALID);
    assert_eq!(error.issues[0].instance_path, "/entities/0/kind");
    output.entities[0].identity.clear();
    let error = project_knowledge_core::collect::validate::validate_output(
        &validation_context(&input, &manifest, &manifest.stages[0]),
        &output,
    )
    .unwrap_err();
    assert_eq!(error.code, codes::SCHEMA_INVALID);
    assert_eq!(error.issues[0].instance_path, "/entities/0/kind");
}

#[tokio::test]
async fn validation_registered_extensions_stage_commit_search_and_reexport() {
    use project_knowledge_core::{
        collect::registry,
        contract::{EntityKind, QueryRequest, query::SearchRequest},
        query::{QueryContext, execute, load_entity},
    };
    let mut f = collection::Fixture::new();
    f.use_extensions();
    let kind = EntityKind::parse("x.smoke.plugin.invoice").unwrap();
    let slots = vec![
        project_knowledge_core::contract::identity::IdentitySlot::required("scopeId", "repo.r"),
        project_knowledge_core::contract::identity::IdentitySlot::required(
            "semanticKey",
            "invoice😀",
        ),
        project_knowledge_core::contract::identity::IdentitySlot::optional(
            "catalog",
            None::<String>,
        ),
    ];
    let id = project_knowledge_core::contract::identity::compute_entity_id(
        "smoke",
        kind.clone(),
        &slots,
    )
    .unwrap();
    f.add_case(&id);
    let candidate = registry::submit(&f.project, &mut f.database, &f.manifest, "now").unwrap();
    registry::validate_many(
        &f.project,
        &mut f.database,
        std::slice::from_ref(&candidate.id),
        &tokio_util::sync::CancellationToken::new(),
        "now",
    )
    .await
    .unwrap();
    registry::activate(&f.project, &mut f.database, &candidate.id, "now")
        .await
        .unwrap();
    let stored = load_entity(f.database.connection(), &id).unwrap().unwrap();
    assert_eq!(stored.kind, kind);
    assert_eq!(stored.identity, slots);
    assert_eq!(
        f.count("SELECT count(*) FROM relations WHERE kind='x.smoke.plugin.binds'"),
        1
    );
    assert_eq!(
        project_knowledge_core::storage::plugins::active_manifests(f.database.connection())
            .unwrap()
            .len(),
        1
    );
    let manifest = load_manifest(&f.manifest).unwrap();
    let input = project_knowledge_core::collect::import::export_input(
        f.database.connection(),
        &f.project,
        &manifest.stages[1],
        &f.database.current_source_snapshot_id().unwrap().unwrap(),
        "export",
        "smoke.plugin",
        "1.0.0",
    )
    .unwrap();
    assert!(
        input
            .entities
            .iter()
            .any(|e| e.entity_id == id && e.kind == kind && e.identity == slots)
    );
    assert!(
        input
            .relations
            .iter()
            .any(|r| r.kind.as_str() == "x.smoke.plugin.binds")
    );
    let response = execute(
        &mut QueryContext {
            database: &mut f.database,
            project: Some(&f.project),
            now: "now".into(),
        },
        &QueryRequest::Search(SearchRequest {
            query: "Invoice".into(),
            fields: None,
            filters: None,
            page_size: Some(20),
            continuation: None,
            budget: None,
        }),
    )
    .unwrap();
    assert!(
        response.data["results"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["entityRef"]["entityId"] == id),
        "{:?}",
        response.data
    );
    // 活动 manifest 也参与登记：新 consumer 无需重新声明别人的命名空间。
    f.edit(|m| {
        m["pluginId"] = json!("consumer");
        m["extensions"] = json!({});
        m["acceptanceCases"] = json!([]);
        m["stages"] = json!([m["stages"][1].clone()]);
    });
    f.stage(None).await.unwrap();
}

#[tokio::test]
async fn validation_extensions_keep_payload_endpoint_and_evidence_guards() {
    for (stage, flag, path, code) in [
        (
            0,
            "--invalid-attributes",
            "/entities/0/assertion/attributes/total",
            codes::SCHEMA_INVALID,
        ),
        (
            1,
            "--invalid-qualifiers",
            "/relations/0/qualifiers/role",
            codes::SCHEMA_INVALID,
        ),
        (1, "--bad-endpoint", "/relations/0", codes::SCHEMA_INVALID),
        (0, "--no-evidence", "/entities/0", codes::EVIDENCE_INVALID),
    ] {
        let f = collection::Fixture::new();
        f.use_extensions();
        f.edit(|m| m["stages"][stage]["args"] = json!([flag]));
        let generation = f.database.generation().unwrap();
        let error = f.stage(None).await.err().unwrap();
        assert_eq!(error.code, code, "{error:?}");
        assert!(
            error.issues.iter().any(|i| i.instance_path == path),
            "{error:?}"
        );
        assert_eq!(f.database.generation().unwrap(), generation);
        assert_eq!(
            f.count("SELECT count(*) FROM entities WHERE kind LIKE 'x.%'"),
            0
        );
    }
}

#[tokio::test]
async fn validation_source_and_candidate_digests_track_content_attributes_and_skips() {
    use project_knowledge_core::collect::import::export_input;
    let mut f = collection::Fixture::new();
    let stage = load_manifest(&f.manifest).unwrap().stages[0].clone();
    let export = |f: &collection::Fixture| {
        export_input(
            f.database.connection(),
            &f.project,
            &stage,
            &f.database.current_source_snapshot_id().unwrap().unwrap(),
            "run",
            "smoke.plugin",
            "1.0.0",
        )
        .unwrap()
    };
    let before = export(&f);
    assert!(before.candidate_sets[0].complete);
    let group = f.stage(None).await.unwrap();
    group
        .commit(&f.project, &mut f.database, "now")
        .await
        .unwrap();
    let after = export(&f);
    assert_ne!(
        before.candidate_sets[0].digest,
        after.candidate_sets[0].digest
    );
    let mut route_stage = stage.clone();
    route_stage.entity_selectors[0].kinds =
        vec![project_knowledge_core::contract::EntityKind::Route];
    let route_before = export_input(
        f.database.connection(),
        &f.project,
        &route_stage,
        &f.database.current_source_snapshot_id().unwrap().unwrap(),
        "run",
        "smoke.plugin",
        "1.0.0",
    )
    .unwrap();
    // 原生产者属性变化、ID 不变；完整集合摘要也必须变化。
    f.database
        .connection()
        .execute(
            "UPDATE entity_assertions SET name = 'Changed' WHERE name = 'Smoke route'",
            [],
        )
        .unwrap();
    let route_after = export_input(
        f.database.connection(),
        &f.project,
        &route_stage,
        &f.database.current_source_snapshot_id().unwrap().unwrap(),
        "run",
        "smoke.plugin",
        "1.0.0",
    )
    .unwrap();
    assert_eq!(
        route_before.candidate_sets[0].entity_ids,
        route_after.candidate_sets[0].entity_ids
    );
    assert_ne!(
        route_before.candidate_sets[0].digest,
        route_after.candidate_sets[0].digest
    );
    std::fs::write(f.root.join("repo/a.ts"), "export const smoke = 2;\n").unwrap();
    std::fs::write(f.root.join("repo/binary.ts"), [0, 1, 2]).unwrap();
    project_knowledge_core::index::build_base_index(&f.project, &mut f.database, "now").unwrap();
    let changed = export(&f);
    assert_ne!(before.source_sets[0].digest, changed.source_sets[0].digest);
    assert!(!changed.source_sets[0].complete);
    assert_eq!(changed.sources.len(), 1);
}
