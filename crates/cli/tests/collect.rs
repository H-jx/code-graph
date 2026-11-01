#[allow(dead_code)]
#[path = "../../core/tests/support/collection.rs"]
mod collection;

use serde_json::{Value, json};

#[test]
fn incremental_cli_and_failed_collect_preserve_invalidation_and_stage_boundaries() {
    let f = collection::Fixture::new();
    std::fs::copy(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/plugins/incremental/collect.mjs"),
        f.manifest.parent().unwrap().join("collect.mjs"),
    )
    .unwrap();
    f.edit(|m| m["stages"][0]["entitySelectors"] = json!([]));
    let invoke = |command: &[&str]| {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_project-knowledge"))
            .arg(command[0])
            .arg("--config")
            .arg(&f.project.config_path)
            .args(&command[1..])
            .output()
            .unwrap();
        (
            output.status.success(),
            serde_json::from_slice::<Value>(&output.stdout).unwrap_or_else(|error| {
                panic!(
                    "{command:?}: {error}; stderr={}",
                    String::from_utf8_lossy(&output.stderr)
                )
            }),
        )
    };
    let (ok, payload) = invoke(&["plugin", "validate", "--path", f.manifest.to_str().unwrap()]);
    assert!(ok, "{payload}");
    let id = payload["candidateId"].as_str().unwrap();
    assert!(invoke(&["plugin", "activate", "--candidate", id]).0);
    let (ok, payload) = invoke(&["index", "--incremental"]);
    assert!(ok, "{payload}");
    assert_eq!(payload["executed"], json!([]));
    std::fs::write(f.root.join("repo/a.ts"), "// ROUTE:/cli\n").unwrap();
    let (ok, payload) = invoke(&["index", "--incremental"]);
    assert!(ok, "{payload}");
    assert_eq!(
        payload["executed"],
        json!(["smoke.plugin:extract", "smoke.plugin:link"])
    );
    assert!(invoke(&["collect", "--phase", "extract"]).0);
    assert_eq!(f.count("SELECT count(*) FROM batches b JOIN current_batches cb ON cb.batch_id=b.id WHERE cb.replacement_key LIKE '(smoke.plugin,link,%' AND b.state='stale'"),1);
    assert!(invoke(&["index", "--incremental"]).0);
    std::fs::write(f.root.join("repo/a.ts"), "// FAIL\n").unwrap();
    let (ok, payload) = invoke(&["collect"]);
    assert!(!ok, "{payload}");
    assert_eq!(payload["error"]["code"], "SOURCE_CHANGED");
    assert_eq!(f.count("SELECT count(*) FROM batches b JOIN current_batches cb ON cb.batch_id=b.id WHERE cb.replacement_key LIKE '(smoke.plugin,%' AND b.state='stale' AND b.invalidated_generation IS NOT NULL"),2);
}

#[cfg(windows)]
#[test]
fn incremental_cli_host_termination_keeps_invalidation_and_cleans_owned_descendants() {
    let mut f = collection::Fixture::new();
    std::fs::copy(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/plugins/incremental/collect.mjs"),
        f.manifest.parent().unwrap().join("collect.mjs"),
    )
    .unwrap();
    f.edit(|m| m["stages"][0]["entitySelectors"] = json!([]));
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let id = project_knowledge_core::collect::registry::submit(
        &f.project,
        &mut f.database,
        &f.manifest,
        "submit",
    )
    .unwrap()
    .id;
    runtime
        .block_on(project_knowledge_core::collect::registry::validate(
            &f.project,
            &mut f.database,
            &id,
            &tokio_util::sync::CancellationToken::new(),
            "validate",
        ))
        .unwrap();
    runtime
        .block_on(project_knowledge_core::collect::registry::activate(
            &f.project,
            &mut f.database,
            &id,
            "activate",
        ))
        .unwrap();
    let old = project_knowledge_core::storage::current_batches(f.database.connection()).unwrap();
    let marker = f.root.join("incremental-owned.json");
    std::fs::write(
        f.root.join("repo/a.ts"),
        format!("// HOLD:{}\n", marker.display()),
    )
    .unwrap();
    struct Host(std::process::Child);
    impl Drop for Host {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_project-knowledge"));
    command
        .args(["index", "--incremental", "--config"])
        .arg(&f.project.config_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    use std::os::windows::process::CommandExt;
    command.creation_flags(0x08000000);
    let mut host = Host(command.spawn().unwrap());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    let owned: Value = loop {
        if let Ok(bytes) = std::fs::read(&marker)
            && let Ok(pids) = serde_json::from_slice(&bytes)
        {
            break pids;
        }
        assert!(
            host.0.try_wait().unwrap().is_none(),
            "host exited before marker"
        );
        assert!(std::time::Instant::now() < deadline, "plugin did not start");
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    host.0.kill().unwrap();
    host.0.wait().unwrap();
    for field in ["parent", "descendant"] {
        let pid = owned[field].as_u64().unwrap().to_string();
        let mut stopped = false;
        for _ in 0..30 {
            stopped=std::process::Command::new("node").args(["-e","try { process.kill(Number(process.argv[1]),0); process.exit(1); } catch(e) { if(e.code==='ESRCH') process.exit(0); throw e; }",&pid]).output().unwrap().status.success();
            if stopped {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
        assert!(stopped, "owned {field} survived host termination");
    }
    for (key, id) in &old {
        if key.starts_with("(smoke.plugin,") {
            assert_eq!(
                project_knowledge_core::storage::batch_state(f.database.connection(), id).unwrap(),
                Some(project_knowledge_core::storage::BatchState::Stale)
            );
            assert!(
                project_knowledge_core::storage::batch_invalidated_generation(
                    f.database.connection(),
                    id
                )
                .unwrap()
                .is_some()
            );
        }
    }
    assert_eq!(
        f.count("SELECT count(*) FROM runs WHERE kind='index.incremental' AND status='running'"),
        1
    );
    project_knowledge_core::collect::registry::recover(&f.project, &mut f.database, "restart")
        .unwrap();
    assert_eq!(
        f.count(
            "SELECT count(*) FROM runs WHERE kind='index.incremental' AND status='interrupted'"
        ),
        1
    );
    std::fs::write(f.root.join("repo/a.ts"), "export const smoke = 1;\n").unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_project-knowledge"))
        .args(["index", "--incremental", "--config"])
        .arg(&f.project.config_path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    for (key, id) in old {
        if key.starts_with("(smoke.plugin,") {
            assert_ne!(
                project_knowledge_core::storage::current_batch(f.database.connection(), &key)
                    .unwrap(),
                Some(id)
            );
        }
    }
}

#[cfg(windows)]
#[test]
fn collector_host_termination_cleans_descendants_and_releases_maintenance_lock() {
    for validate_only in [false, true] {
        let mut f = collection::Fixture::new();
        let marker = f.root.join("owned-pids.json");
        f.edit(|m| m["stages"][0]["args"] = json!(["--hold", marker.to_str().unwrap()]));
        let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_project-knowledge"));
        if validate_only {
            command
                .args(["collect", "--config"])
                .arg(&f.project.config_path)
                .args(["--validate-only", "--manifest"])
                .arg(&f.manifest);
        } else {
            command
                .args(["plugin", "--config"])
                .arg(&f.project.config_path)
                .args(["validate", "--path"])
                .arg(&f.manifest);
        }
        let mut host = command
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let owned: Value = loop {
            if let Ok(bytes) = std::fs::read(&marker)
                && let Ok(pids) = serde_json::from_slice(&bytes)
            {
                break pids;
            }
            if std::time::Instant::now() >= deadline {
                let _ = host.kill();
                let _ = host.wait();
                panic!("collector did not create ownership marker");
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        };
        let before = f.database.generation().unwrap();
        assert_eq!(
            f.count("SELECT count(*) FROM plugin_candidates WHERE state='prepared'"),
            i64::from(!validate_only)
        );
        let staging = f.project.base_dir.join(".project-knowledge/work/staging");
        assert_eq!(std::fs::read_dir(&staging).unwrap().count(), 1);
        if validate_only {
            project_knowledge_core::collect::registry::recover(
                &f.project,
                &mut f.database,
                "concurrent maintenance",
            )
            .unwrap();
            assert_eq!(
                std::fs::read_dir(&staging).unwrap().count(),
                1,
                "live validate-only staging must retain ownership"
            );
        }
        host.kill().unwrap();
        host.wait().unwrap();
        for key in ["parent", "descendant"] {
            let pid = owned[key].as_u64().unwrap().to_string();
            let mut stopped = false;
            for _ in 0..30 {
                stopped=std::process::Command::new("node").args(["-e","try { process.kill(Number(process.argv[1]), 0); process.exit(1); } catch(e) { if(e.code==='ESRCH') process.exit(0); throw e; }",&pid])
                .output().unwrap().status.success();
                if stopped {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(30));
            }
            if !stopped {
                // 失败清理仅使用本次插件在唯一 marker 中登记的 owned PID。
                let _ = std::process::Command::new("node")
                    .args(["-e", "process.kill(Number(process.argv[1]));", &pid])
                    .output();
            }
            assert!(stopped, "owned {key} PID {pid} survived host termination");
        }
        assert_eq!(f.database.generation().unwrap(), before);
        assert_eq!(
            f.count("SELECT count(*) FROM plugin_candidates WHERE state='prepared'"),
            i64::from(!validate_only),
            "read-only inspection must not recover"
        );
        project_knowledge_core::collect::registry::recover(&f.project, &mut f.database, "restart")
            .unwrap();
        assert_eq!(
            std::fs::read_dir(staging).unwrap().count(),
            0,
            "abandoned staging must be reclaimed"
        );
        assert_eq!(
            f.count("SELECT count(*) FROM plugin_candidates WHERE state='failed'"),
            i64::from(!validate_only)
        );
    }
}

#[test]
fn collect_cli_validates_commits_and_returns_failure_for_invalid_output() {
    let f = collection::Fixture::new();
    let invoke = |extra: &[&str]| {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_project-knowledge"))
            .args(["collect", "--config"])
            .arg(&f.project.config_path)
            .arg("--manifest")
            .arg(&f.manifest)
            .args(extra)
            .output()
            .unwrap();
        let payload: Value = serde_json::from_slice(&output.stdout).expect("one JSON response");
        (output.status.code(), payload)
    };
    let before = f.database.generation().unwrap();
    let (code, payload) = invoke(&["--validate-only"]);
    assert_eq!(code, Some(0));
    assert_eq!(payload["committed"], false);
    assert_eq!(f.database.generation().unwrap(), before);
    let (code, payload) = invoke(&[]);
    assert_eq!(code, Some(1));
    assert_eq!(payload["error"]["code"], "INVALID_REQUEST");
    let activate = std::process::Command::new(env!("CARGO_BIN_EXE_project-knowledge"))
        .args(["plugin", "--config"])
        .arg(&f.project.config_path)
        .args(["validate", "--path"])
        .arg(&f.manifest)
        .output()
        .unwrap();
    assert!(
        activate.status.success(),
        "{}",
        String::from_utf8_lossy(&activate.stdout)
    );
    let activated: Value = serde_json::from_slice(&activate.stdout).unwrap();
    let id = activated["candidateId"].as_str().unwrap();
    assert!(
        std::process::Command::new(env!("CARGO_BIN_EXE_project-knowledge"))
            .args(["plugin", "--config"])
            .arg(&f.project.config_path)
            .args(["activate", "--candidate", id])
            .output()
            .unwrap()
            .status
            .success()
    );
    let (code, payload) = invoke(&[]);
    assert_eq!(code, Some(0), "{payload}");
    assert_eq!(payload["committed"], true);
    assert_eq!(f.database.generation().unwrap(), before + 2);
    f.edit(|m| m["stages"][1]["args"] = json!(["--bad"]));
    let (code, payload) = invoke(&[]);
    assert_eq!(code, Some(1));
    assert_eq!(payload["error"]["code"], "ARTIFACT_CHANGED");
    assert_eq!(f.database.generation().unwrap(), before + 2);
    let (code, payload) = invoke(&["--validate-only"]);
    assert_eq!(code, Some(1));
    assert_eq!(payload["error"]["code"], "EVIDENCE_INVALID");
}

#[test]
fn plugin_cli_submit_validate_activate_and_rollback() {
    let f = collection::Fixture::new();
    let invoke = |action: &str, args: &[&str]| {
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_project-knowledge"))
            .args(["plugin", "--config"])
            .arg(&f.project.config_path)
            .arg(action)
            .args(args)
            .output()
            .unwrap();
        let payload: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert!(out.status.success(), "{payload}");
        payload
    };
    let before = f.database.generation().unwrap();
    let candidate = invoke(
        "submit",
        &["--path", f.manifest.parent().unwrap().to_str().unwrap()],
    );
    let id = candidate["id"].as_str().unwrap();
    invoke("validate", &["--candidate", id]);
    assert_eq!(f.database.generation().unwrap(), before);
    invoke("activate", &["--candidate", id]);
    assert_eq!(f.database.generation().unwrap(), before + 1);
    invoke(
        "rollback",
        &["--plugin", "smoke.plugin", "--version", "1.0.0"],
    );
    assert_eq!(f.database.generation().unwrap(), before + 2);
}
