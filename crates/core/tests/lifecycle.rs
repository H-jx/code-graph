//! 候选与激活的生命周期，使用真实 Node 子进程和 SQLite WAL。
#[allow(dead_code)]
#[path = "support/collection.rs"]
mod collection;
use project_knowledge_core::collect::registry::{self, FaultPoint, MaintenanceLock};
use project_knowledge_core::contract::error::codes;
use project_knowledge_core::storage::{self, Database};
use serde_json::json;
use tokio_util::sync::CancellationToken;

fn incremental_fixture() -> collection::Fixture {
    let f = collection::Fixture::new();
    std::fs::copy(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/plugins/incremental/collect.mjs"),
        f.manifest.parent().unwrap().join("collect.mjs"),
    )
    .unwrap();
    f.edit(|m| {
        m["stages"][0]["entitySelectors"] = json!([]);
    });
    f
}
async fn incremental(
    f: &mut collection::Fixture,
) -> project_knowledge_core::index::IncrementalResult {
    project_knowledge_core::index::run_incremental(
        &f.project,
        &mut f.database,
        &CancellationToken::new(),
        "update",
    )
    .await
    .unwrap()
}
fn normalized(f: &collection::Fixture) -> serde_json::Value {
    let mut entities = f.database.connection().prepare("SELECT e.id,e.kind,e.scope_id,ea.name,ea.attributes_json FROM entities e JOIN entity_assertions ea ON ea.entity_id=e.id JOIN current_batches cb ON cb.batch_id=ea.batch_id JOIN batches b ON b.id=cb.batch_id WHERE b.state='fresh' ORDER BY e.id,ea.name,ea.attributes_json").unwrap();
    let entities: Vec<_> = entities
        .query_map([], |r| {
            Ok(json!([
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, String>(4)?
            ]))
        })
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let mut relations = f.database.connection().prepare("SELECT r.id,r.kind,r.from_id,r.to_id,ra.resolution FROM relations r JOIN relation_assertions ra ON ra.relation_id=r.id JOIN current_batches cb ON cb.batch_id=ra.batch_id JOIN batches b ON b.id=cb.batch_id WHERE b.state='fresh' ORDER BY r.id").unwrap();
    let relations: Vec<_> = relations
        .query_map([], |r| {
            Ok(json!([
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?
            ]))
        })
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let rows = |sql: &str| -> Vec<serde_json::Value> {
        f.database
            .connection()
            .prepare(sql)
            .unwrap()
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .map(|row| serde_json::from_str(&row.unwrap()).unwrap())
            .collect()
    };
    let search = rows(
        "SELECT json_array(d.entity_id,d.scope_id,d.route_exact,d.text,d.tokens,d.projection_kind) FROM search_documents d JOIN assertions a ON a.id=d.assertion_id JOIN current_batches cb ON cb.batch_id=a.batch_id JOIN batches b ON b.id=cb.batch_id WHERE b.state='fresh' ORDER BY d.entity_id,d.projection_kind,d.text",
    );
    let evidence = rows(
        "SELECT DISTINCT json_array(e.source_id,e.content_hash,e.start_byte,e.end_byte,e.start_line,e.end_line,e.role) AS value FROM evidence e JOIN assertion_evidence ae ON ae.evidence_id=e.id JOIN assertions a ON a.id=ae.assertion_id JOIN current_batches cb ON cb.batch_id=a.batch_id JOIN batches b ON b.id=cb.batch_id WHERE b.state='fresh' ORDER BY value",
    );
    let sources = rows(
        "SELECT json_array(s.repository_id,s.path,ss.content_hash,ss.selection_status) FROM snapshot_sources ss JOIN sources s ON s.id=ss.source_id JOIN index_meta im ON im.current_source_snapshot_id=ss.snapshot_id ORDER BY s.repository_id,s.path",
    );
    let coverage = rows(
        "SELECT json_array(c.capability,c.scope_id,c.source_set_id,c.status,c.counts_json,c.gap_ids_json) FROM coverage c JOIN current_batches cb ON cb.batch_id=c.batch_id JOIN batches b ON b.id=cb.batch_id WHERE b.state='fresh' ORDER BY c.capability,c.scope_id,c.source_set_id,c.status",
    );
    json!({"entities":entities,"relations":relations,"search":search,"evidence":evidence,"sources":sources,"coverage":coverage})
}
fn current_plugin_batches(f: &collection::Fixture) -> Vec<(String, String)> {
    storage::current_batches(f.database.connection())
        .unwrap()
        .into_iter()
        .filter(|(k, _)| !k.starts_with("(core.base,"))
        .collect()
}

#[tokio::test]
async fn incremental_unchanged_stages_and_repositories_retain_batch_ids_and_ignore_changes_are_detected()
 {
    let mut f = incremental_fixture();
    std::fs::create_dir(f.root.join("other-repo")).unwrap();
    std::fs::write(
        f.root.join("other-repo/b.ts"),
        "export const unrelated=1;\n",
    )
    .unwrap();
    let mut config: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&f.project.config_path).unwrap()).unwrap();
    config["repositories"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"other","root":"other-repo"}));
    std::fs::write(&f.project.config_path, serde_json::to_vec(&config).unwrap()).unwrap();
    f.project =
        project_knowledge_core::project::config::LoadedProject::load(&f.project.config_path)
            .unwrap();
    f.edit(|m| {
        for stage in m["stages"].as_array_mut().unwrap() {
            stage["sourceSelectors"][0]["repositoryIds"] = json!(["r"]);
            stage["entitySelectors"] = json!([]);
        }
    });
    project_knowledge_core::index::build_base_index(&f.project, &mut f.database, "base").unwrap();
    active(&mut f, "1.0.0").await;
    let before = current_plugin_batches(&f);
    let base = storage::current_batch(f.database.connection(), &storage::base_replacement_key("r"))
        .unwrap();
    std::fs::write(
        f.root.join("other-repo/b.ts"),
        "export const unrelated=2;\n",
    )
    .unwrap();
    assert!(incremental(&mut f).await.executed.is_empty());
    assert_eq!(current_plugin_batches(&f), before);
    assert_eq!(
        storage::current_batch(f.database.connection(), &storage::base_replacement_key("r"))
            .unwrap(),
        base
    );
    std::fs::write(f.root.join("repo/.gitignore"), "a.ts\n").unwrap();
    assert_eq!(incremental(&mut f).await.executed.len(), 2);
    assert_eq!(f.count("SELECT count(*) FROM entity_assertions ea JOIN current_batches cb ON cb.batch_id=ea.batch_id WHERE cb.replacement_key LIKE '(smoke.plugin,%'"),0);
    std::fs::write(f.root.join("repo/.gitignore"), "\n").unwrap();
    assert_eq!(incremental(&mut f).await.executed.len(), 2);
    assert!(incremental(&mut f).await.executed.is_empty());
}

#[tokio::test]
async fn incremental_artifact_tampering_and_missing_runtime_never_restore_old_freshness() {
    let mut f = incremental_fixture();
    active(&mut f, "1.0.0").await;
    let old = current_plugin_batches(&f);
    let published =
        storage::plugins::published_manifest(f.database.connection(), "smoke.plugin", "1.0.0")
            .unwrap();
    let entry = published.parent().unwrap().join("collect.mjs");
    let original = std::fs::read(&entry).unwrap();
    std::fs::write(&entry, b"tampered").unwrap();
    let error = project_knowledge_core::index::run_incremental(
        &f.project,
        &mut f.database,
        &CancellationToken::new(),
        "tampered",
    )
    .await
    .unwrap_err();
    assert_eq!(error.code, codes::ARTIFACT_CHANGED);
    for (_, id) in &old {
        assert_eq!(
            storage::batch_state(f.database.connection(), id).unwrap(),
            Some(storage::BatchState::Stale)
        );
    }
    std::fs::write(&entry, original).unwrap();
    assert_eq!(incremental(&mut f).await.executed.len(), 2);
    assert_ne!(current_plugin_batches(&f), old);
    let old = current_plugin_batches(&f);
    let original = std::fs::read(&f.project.config_path).unwrap();
    let mut config: serde_json::Value = serde_json::from_slice(&original).unwrap();
    config["runtimes"] = json!([{"id":"missing","kind":"node","executable":f.root.join("missing-node").to_str().unwrap()}]);
    std::fs::write(&f.project.config_path, serde_json::to_vec(&config).unwrap()).unwrap();
    f.project =
        project_knowledge_core::project::config::LoadedProject::load(&f.project.config_path)
            .unwrap();
    assert!(
        project_knowledge_core::index::run_incremental(
            &f.project,
            &mut f.database,
            &CancellationToken::new(),
            "missing runtime"
        )
        .await
        .is_err()
    );
    for (_, id) in &old {
        assert_eq!(
            storage::batch_state(f.database.connection(), id).unwrap(),
            Some(storage::BatchState::Stale)
        );
    }
    std::fs::write(&f.project.config_path, original).unwrap();
    f.project =
        project_knowledge_core::project::config::LoadedProject::load(&f.project.config_path)
            .unwrap();
    assert_eq!(incremental(&mut f).await.executed.len(), 2);
    assert_ne!(current_plugin_batches(&f), old);
}

#[tokio::test]
async fn incremental_candidates_zero_one_two_rename_attributes_delete_and_clean_rebuild_agree() {
    let mut f = incremental_fixture();
    active(&mut f, "1.0.0").await;
    let before = current_plugin_batches(&f);
    let generation = f.database.generation().unwrap();
    assert!(incremental(&mut f).await.executed.is_empty());
    assert_eq!(current_plugin_batches(&f), before);
    assert_eq!(f.database.generation().unwrap(), generation);
    for (a, b) in [
        (Some("/one"), None),
        (Some("/one"), Some("/two")),
        (None, Some("/two")),
        (None, Some("/renamed")),
        (None, None),
    ] {
        for (file, route) in [("a.ts", a), ("b.ts", b)] {
            if let Some(route) = route {
                std::fs::write(
                    f.root.join("repo").join(file),
                    format!("// ROUTE:{route}\n"),
                )
                .unwrap();
            } else if f.root.join("repo").join(file).exists() {
                std::fs::remove_file(f.root.join("repo").join(file)).unwrap();
            }
        }
        let result = incremental(&mut f).await;
        assert_eq!(
            result.executed,
            vec!["smoke.plugin:extract", "smoke.plugin:link"]
        );
        let count = i64::from(a.is_some()) + i64::from(b.is_some());
        assert_eq!(f.count("SELECT count(*) FROM relation_assertions ra JOIN current_batches cb ON cb.batch_id=ra.batch_id JOIN batches b ON b.id=cb.batch_id WHERE b.state='fresh' AND ra.resolution='confirmed' AND cb.replacement_key LIKE '(smoke.plugin,%'"),i64::from(count==1));
        let mut clean = incremental_fixture();
        std::fs::remove_file(clean.root.join("repo/a.ts")).unwrap();
        for entry in std::fs::read_dir(f.root.join("repo")).unwrap() {
            let entry = entry.unwrap();
            std::fs::copy(
                entry.path(),
                clean.root.join("repo").join(entry.file_name()),
            )
            .unwrap();
        }
        project_knowledge_core::index::build_base_index(
            &clean.project,
            &mut clean.database,
            "clean",
        )
        .unwrap();
        active(&mut clean, "1.0.0").await;
        let actual = normalized(&f);
        let expected = normalized(&clean);
        assert_eq!(actual, expected);
        if let Some(directory) = std::env::var_os("PK_INCREMENTAL_EVIDENCE") {
            let directory = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&directory).unwrap();
            let index = f.count(
                "SELECT count(*) FROM runs WHERE kind='index.incremental' AND status='succeeded'",
            );
            std::fs::write(directory.join(format!("comparison-{index}.json")),serde_json::to_vec_pretty(&json!({"executed":result.executed,"invalidated":result.invalidated,"incremental":actual,"clean":expected,"equal":true,"modelRequests":0,"modelCounterBasis":"Core has no Agent/Rig dependency and constructs no model client"})).unwrap()).unwrap();
        }
        assert!(
            incremental(&mut f).await.executed.is_empty(),
            "same inputs must not rerun collectors"
        );
    }
    std::fs::write(f.root.join("repo/a.ts"), "// ROUTE:/same initial\n").unwrap();
    incremental(&mut f).await;
    let before = current_plugin_batches(&f);
    std::fs::write(
        f.root.join("repo/a.ts"),
        "// ROUTE:/same changed attribute\n",
    )
    .unwrap();
    incremental(&mut f).await;
    assert_ne!(current_plugin_batches(&f), before);
    std::fs::rename(f.root.join("repo/a.ts"), f.root.join("repo/moved.ts")).unwrap();
    assert_eq!(incremental(&mut f).await.executed.len(), 2);
    let mut clean = incremental_fixture();
    std::fs::remove_file(clean.root.join("repo/a.ts")).unwrap();
    std::fs::copy(
        f.root.join("repo/moved.ts"),
        clean.root.join("repo/moved.ts"),
    )
    .unwrap();
    project_knowledge_core::index::build_base_index(
        &clean.project,
        &mut clean.database,
        "clean move",
    )
    .unwrap();
    active(&mut clean, "1.0.0").await;
    assert_eq!(normalized(&f), normalized(&clean));
}

#[tokio::test]
async fn incremental_failure_cancel_base_only_and_restored_content_keep_old_batches_stale() {
    let mut f = incremental_fixture();
    std::fs::write(f.root.join("repo/a.ts"), "// ROUTE:/one\n").unwrap();
    project_knowledge_core::index::build_base_index(&f.project, &mut f.database, "base").unwrap();
    active(&mut f, "1.0.0").await;
    let old = current_plugin_batches(&f);
    std::fs::write(f.root.join("repo/a.ts"), "// FAIL\n").unwrap();
    let error = project_knowledge_core::index::run_incremental(
        &f.project,
        &mut f.database,
        &CancellationToken::new(),
        "failure",
    )
    .await
    .unwrap_err();
    assert_eq!(error.code, codes::PLUGIN_EXIT);
    assert_eq!(
        registry::rollback(
            &f.project,
            &mut f.database,
            "smoke.plugin",
            "1.0.0",
            &CancellationToken::new(),
            "failed rollback"
        )
        .await
        .unwrap_err()
        .code,
        codes::PLUGIN_EXIT
    );
    assert_eq!(current_plugin_batches(&f), old);
    let invalidated: Vec<_> = old
        .iter()
        .map(|(_, id)| {
            storage::batch_invalidated_generation(f.database.connection(), id)
                .unwrap()
                .unwrap()
        })
        .collect();
    std::fs::write(f.root.join("repo/a.ts"), "// ROUTE:/one\n").unwrap();
    registry::recover(&f.project, &mut f.database, "restart").unwrap();
    for (_, id) in &old {
        assert_eq!(
            storage::batch_state(f.database.connection(), id).unwrap(),
            Some(storage::BatchState::Stale)
        );
    }
    let cancelled = CancellationToken::new();
    cancelled.cancel();
    assert_eq!(
        project_knowledge_core::index::run_incremental(
            &f.project,
            &mut f.database,
            &cancelled,
            "cancel"
        )
        .await
        .unwrap_err()
        .code,
        codes::CANCELLED
    );
    incremental(&mut f).await;
    assert_ne!(current_plugin_batches(&f), old);
    for ((_, id), generation) in old.iter().zip(invalidated) {
        assert_eq!(
            storage::batch_invalidated_generation(f.database.connection(), id).unwrap(),
            Some(generation)
        );
    }
    let old = current_plugin_batches(&f);
    let result =
        project_knowledge_core::index::run_base_only(&f.project, &mut f.database, "base-only")
            .unwrap();
    assert!(result.executed.is_empty());
    assert_eq!(current_plugin_batches(&f), old);
    for (_, id) in &old {
        assert_eq!(
            storage::batch_state(f.database.connection(), id).unwrap(),
            Some(storage::BatchState::Stale)
        );
    }
    incremental(&mut f).await;
    assert_ne!(current_plugin_batches(&f), old);
}

#[tokio::test]
async fn activation_new_candidate_group_validates_and_switches_together() {
    for bad_link in [false, true] {
        let mut f = collection::Fixture::new();
        let second = f.root.join("second");
        std::fs::create_dir(&second).unwrap();
        std::fs::copy(
            f.manifest.parent().unwrap().join("collect.mjs"),
            second.join("collect.mjs"),
        )
        .unwrap();
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&f.manifest).unwrap()).unwrap();
        manifest["pluginId"] = json!("smoke.links");
        manifest["stages"] = json!([manifest["stages"][1].clone()]);
        if bad_link {
            manifest["stages"][0]["args"] = json!(["--bad"]);
        }
        std::fs::write(
            second.join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        let a = registry::submit(&f.project, &mut f.database, &f.manifest, "now").unwrap();
        let b = registry::submit(&f.project, &mut f.database, &second, "now").unwrap();
        let generation = f.database.generation().unwrap();
        let ids = vec![a.id, b.id];
        let result = registry::validate_many(
            &f.project,
            &mut f.database,
            &ids,
            &CancellationToken::new(),
            "now",
        )
        .await;
        assert_eq!(f.database.generation().unwrap(), generation);
        if bad_link {
            assert_eq!(result.unwrap_err().code, codes::EVIDENCE_INVALID);
            assert!(
                storage::active_version_map(f.database.connection())
                    .unwrap()
                    .is_empty()
            );
            assert_eq!(
                f.count("SELECT count(*) FROM plugin_candidates WHERE state='failed'"),
                2
            );
        } else {
            let result = result.unwrap();
            assert_eq!(result["candidateIds"].as_array().unwrap().len(), 2);
            registry::activate(&f.project, &mut f.database, &ids[1], "now")
                .await
                .unwrap();
            assert_eq!(f.database.generation().unwrap(), generation + 1);
            assert_eq!(
                storage::active_version_map(f.database.connection())
                    .unwrap()
                    .len(),
                2
            );
            assert_eq!(
                f.count("SELECT count(*) FROM plugin_candidates WHERE state='active'"),
                2
            );
            assert_eq!(
                registry::activate(&f.project, &mut f.database, &ids[0], "again")
                    .await
                    .unwrap()["alreadyCommitted"],
                true
            );
        }
    }
}

#[tokio::test]
async fn activation_source_change_is_persistently_stale_even_when_content_is_restored() {
    let mut f = collection::Fixture::new();
    active(&mut f, "1.0.0").await;
    let old = storage::current_batch(f.database.connection(), "(smoke.plugin,extract,smoke)")
        .unwrap()
        .unwrap();
    let id = validated(&mut f, "2.0.0").await;
    let source = f.root.join("repo/a.ts");
    let original = std::fs::read(&source).unwrap();
    std::fs::write(&source, "export const changed=1;").unwrap();
    assert_eq!(
        registry::activate(&f.project, &mut f.database, &id, "now")
            .await
            .unwrap_err()
            .code,
        codes::SOURCE_CHANGED
    );
    assert_eq!(
        storage::batch_state(f.database.connection(), &old).unwrap(),
        Some(storage::BatchState::Stale)
    );
    let invalidated = storage::batch_invalidated_generation(f.database.connection(), &old).unwrap();
    std::fs::write(source, original).unwrap();
    assert_eq!(
        registry::activate(&f.project, &mut f.database, &id, "now")
            .await
            .unwrap_err()
            .code,
        codes::ACTIVATION_CONFLICT
    );
    registry::recover(&f.project, &mut f.database, "restart").unwrap();
    assert_eq!(
        storage::batch_invalidated_generation(f.database.connection(), &old).unwrap(),
        invalidated
    );
    registry::rollback(
        &f.project,
        &mut f.database,
        "smoke.plugin",
        "1.0.0",
        &CancellationToken::new(),
        "rollback",
    )
    .await
    .unwrap();
    assert_eq!(
        storage::batch_state(f.database.connection(), &old).unwrap(),
        Some(storage::BatchState::Stale)
    );
    let new = storage::current_batch(f.database.connection(), "(smoke.plugin,extract,smoke)")
        .unwrap()
        .unwrap();
    assert_ne!(old, new);
    assert_eq!(
        storage::batch_state(f.database.connection(), &new).unwrap(),
        Some(storage::BatchState::Fresh)
    );
}

async fn validated(f: &mut collection::Fixture, version: &str) -> String {
    f.edit(|m| m["version"] = json!(version));
    let candidate = registry::submit(&f.project, &mut f.database, &f.manifest, "now").unwrap();
    registry::validate(
        &f.project,
        &mut f.database,
        &candidate.id,
        &CancellationToken::new(),
        "now",
    )
    .await
    .unwrap();
    candidate.id
}
async fn active(f: &mut collection::Fixture, version: &str) -> String {
    let id = validated(f, version).await;
    registry::activate(&f.project, &mut f.database, &id, "now")
        .await
        .unwrap();
    id
}

#[tokio::test]
async fn activation_submit_validate_publish_and_atomic_version_switch() {
    let mut f = collection::Fixture::new();
    let generation = f.database.generation().unwrap();
    let id = validated(&mut f, "1.0.0").await;
    assert_eq!(f.database.generation().unwrap(), generation);
    assert!(
        storage::active_version_map(f.database.connection())
            .unwrap()
            .is_empty()
    );
    let result = registry::activate(&f.project, &mut f.database, &id, "now")
        .await
        .unwrap();
    assert_eq!(result["generation"], (generation + 1).to_string());
    let stored = storage::plugins::candidate(f.database.connection(), &id).unwrap();
    assert_eq!(stored.state, "active");
    let path =
        storage::plugins::published_manifest(f.database.connection(), "smoke.plugin", "1.0.0")
            .unwrap();
    assert!(path.is_file());
    assert!(path.to_string_lossy().contains(&stored.artifact_hash));
    assert_eq!(f.count("SELECT count(*) FROM current_batches"), 3);
    assert_eq!(f.count("SELECT count(*) FROM batches WHERE producer_key LIKE 'smoke.plugin@%' AND state = 'fresh'"),2);
    assert_eq!(f.count("SELECT count(*) FROM pragma_foreign_key_check"), 0);
    assert_eq!(
        f.database
            .connection()
            .query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
}

#[tokio::test]
async fn activation_concurrent_candidates_and_readers_never_mix_versions() {
    let mut f = collection::Fixture::new();
    active(&mut f, "1.0.0").await;
    let a = validated(&mut f, "2.0.0").await;
    let b = validated(&mut f, "3.0.0").await;
    let mut reader = Database::open_readonly(f.database.path()).unwrap();
    let snapshot = reader.read_transaction().unwrap();
    let version = || {
        snapshot
            .query_row(
                "SELECT version FROM active_plugins WHERE plugin_id = 'smoke.plugin'",
                [],
                |r| r.get::<_, String>(0),
            )
            .unwrap()
    };
    assert_eq!(version(), "1.0.0");
    registry::activate(&f.project, &mut f.database, &a, "now")
        .await
        .unwrap();
    assert_eq!(version(), "1.0.0");
    assert_eq!(
        registry::activate(&f.project, &mut f.database, &b, "now")
            .await
            .unwrap_err()
            .code,
        codes::ACTIVATION_CONFLICT
    );
    let keys:Vec<String> = f.database.connection().prepare("SELECT b.producer_key FROM batches b JOIN current_batches cb ON cb.batch_id = b.id WHERE b.producer_key LIKE 'smoke.plugin@%' ORDER BY b.producer_key").unwrap()
        .query_map([],|r|r.get(0)).unwrap().collect::<Result<_,_>>().unwrap();
    assert_eq!(keys.len(), 2);
    assert!(keys.iter().all(|p| p.starts_with("smoke.plugin@2.0.0:")));
}

#[tokio::test]
async fn activation_failure_points_recover_to_complete_old_or_new_state() {
    for point in [
        FaultPoint::BeforePublish,
        FaultPoint::AfterPublish,
        FaultPoint::DuringCommit(0),
        FaultPoint::DuringCommit(1),
        FaultPoint::AfterCommit,
    ] {
        let mut f = collection::Fixture::new();
        active(&mut f, "1.0.0").await;
        let generation = f.database.generation().unwrap();
        let id = validated(&mut f, "2.0.0").await;
        registry::activate_with_fault(&f.project, &mut f.database, &id, point.clone(), "now")
            .await
            .unwrap_err();
        // 查询打开不会执行中断恢复。
        let readonly = Database::open_readonly(f.database.path()).unwrap();
        assert_eq!(
            readonly.generation().unwrap(),
            generation + u64::from(point == FaultPoint::AfterCommit)
        );
        drop(readonly);
        registry::recover(&f.project, &mut f.database, "restart").unwrap();
        let expected = if point == FaultPoint::AfterCommit {
            "2.0.0"
        } else {
            "1.0.0"
        };
        assert_eq!(
            storage::active_version_map(f.database.connection()).unwrap(),
            vec![("smoke.plugin".into(), expected.into())]
        );
        assert_eq!(f.count("SELECT count(*) FROM pragma_foreign_key_check"), 0);
        assert_eq!(f.count("SELECT count(*) FROM current_batches"), 3);
        assert_eq!(
            f.count("SELECT count(*) FROM activations WHERE status = 'pending'"),
            0
        );
        if point == FaultPoint::AfterCommit {
            assert_eq!(
                registry::activate(&f.project, &mut f.database, &id, "retry")
                    .await
                    .unwrap()["alreadyCommitted"],
                true
            );
            assert_eq!(f.database.generation().unwrap(), generation + 1);
        }
    }
}

fn serving_state(f: &collection::Fixture) -> serde_json::Value {
    let text:String=f.database.connection().query_row("SELECT coalesce(group_concat(text, '|'),'') FROM (SELECT text FROM search_documents ORDER BY rowid)",[],|r|r.get(0)).unwrap();
    json!({"generation":f.database.generation().unwrap(),"active":storage::active_version_map(f.database.connection()).unwrap(),
        "batches":storage::current_batches(f.database.connection()).unwrap(),"entityAssertions":f.count("SELECT count(*) FROM entity_assertions"),
        "relationAssertions":f.count("SELECT count(*) FROM relation_assertions"),"search":text})
}

#[cfg(unix)]
#[test]
#[allow(unsafe_code)]
fn activation_lock_release_does_not_wait_for_a_forked_descriptor_to_close() {
    let f = collection::Fixture::new();
    let lock = MaintenanceLock::acquire(&f.project).unwrap();
    // 子进程只调用 async-signal-safe 的 libc 函数，不运行分配器或继承的 Rust Drop。
    let mut descriptors = [-1; 2];
    unsafe {
        assert_eq!(libc::pipe(descriptors.as_mut_ptr()), 0);
        let child = libc::fork();
        if child == 0 {
            libc::close(descriptors[1]);
            let mut byte = 0_u8;
            libc::read(descriptors[0], std::ptr::from_mut(&mut byte).cast(), 1);
            libc::_exit(0);
        }
        if child < 0 {
            libc::close(descriptors[0]);
            libc::close(descriptors[1]);
            panic!("fork failed");
        }
        libc::close(descriptors[0]);
        drop(lock);
        let acquired = MaintenanceLock::acquire(&f.project);
        let byte = 1_u8;
        libc::write(descriptors[1], std::ptr::from_ref(&byte).cast(), 1);
        libc::close(descriptors[1]);
        let mut status = 0;
        assert_eq!(libc::waitpid(child, &mut status, 0), child);
        assert!(
            acquired.is_ok(),
            "a forked copy must not prolong a released maintenance lock"
        );
    }
}

#[tokio::test]
async fn activation_real_host_kill_inside_c_preserves_old_snapshot_and_retries() {
    const CHILD_CONFIG: &str = "PK_TEST_ACTIVATION_CHILD_CONFIG";
    if let Some(path) = std::env::var_os(CHILD_CONFIG) {
        let project = project_knowledge_core::project::config::LoadedProject::load(
            std::path::Path::new(&path),
        )
        .unwrap();
        let mut database = Database::open_maintenance(
            &project.base_dir.join("index.sqlite"),
            &project.config.project_id,
            &project.config_hash().unwrap(),
            "child",
        )
        .unwrap();
        let id = std::env::var("PK_TEST_ACTIVATION_CANDIDATE").unwrap();
        let marker =
            std::path::PathBuf::from(std::env::var_os("PK_TEST_ACTIVATION_MARKER").unwrap());
        let step = std::env::var("PK_TEST_ACTIVATION_STEP")
            .unwrap()
            .parse()
            .unwrap();
        registry::activate_with_fault(
            &project,
            &mut database,
            &id,
            FaultPoint::HoldDuringCommit { step, marker },
            "child",
        )
        .await
        .unwrap();
        panic!("child must remain at its C barrier until killed");
    }
    struct Host(std::process::Child);
    impl Drop for Host {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    for step in [0, 1] {
        let mut f = collection::Fixture::new();
        active(&mut f, "1.0.0").await;
        let id = validated(&mut f, "2.0.0").await;
        let before = serving_state(&f);
        let marker = f.root.join("activation-c-barrier");
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "activation_real_host_kill_inside_c_preserves_old_snapshot_and_retries",
                "--nocapture",
            ])
            .env(CHILD_CONFIG, &f.project.config_path)
            .env("PK_TEST_ACTIVATION_CANDIDATE", &id)
            .env("PK_TEST_ACTIVATION_MARKER", &marker)
            .env("PK_TEST_ACTIVATION_STEP", step.to_string())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        let mut host = Host(command.spawn().unwrap());
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        while !marker.exists() {
            assert!(
                host.0.try_wait().unwrap().is_none(),
                "activation child exited before C barrier"
            );
            assert!(
                std::time::Instant::now() < deadline,
                "activation child never reached C barrier"
            );
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        assert_eq!(
            std::fs::read_to_string(&marker).unwrap(),
            format!("{}:{step}", host.0.id())
        );
        // WAL 读者在 C 写入一半与写完待提交时都只能看到旧整组。
        assert_eq!(serving_state(&f), before);
        host.0.kill().unwrap();
        assert!(!host.0.wait().unwrap().success());
        assert_eq!(serving_state(&f), before);
        registry::recover(&f.project, &mut f.database, "restart").unwrap();
        assert_eq!(serving_state(&f), before);
        let integrity: String = f
            .database
            .connection()
            .query_row("PRAGMA integrity_check", [], |r| r.get(0))
            .unwrap();
        assert_eq!(integrity, "ok");
        assert_eq!(f.count("SELECT count(*) FROM pragma_foreign_key_check"), 0);
        assert_eq!(
            f.count("SELECT count(*) FROM activations WHERE status='pending'"),
            0
        );
        registry::activate(&f.project, &mut f.database, &id, "retry")
            .await
            .unwrap();
        assert_eq!(
            f.database.generation().unwrap(),
            before["generation"].as_u64().unwrap() + 1
        );
        assert_eq!(
            storage::active_version_map(f.database.connection()).unwrap(),
            vec![("smoke.plugin".into(), "2.0.0".into())]
        );
    }
}

#[tokio::test]
async fn activation_sqlite_full_rolls_back_c_and_can_retry_after_capacity_returns() {
    let mut f = collection::Fixture::new();
    active(&mut f, "1.0.0").await;
    let entry = f.manifest.parent().unwrap().join("collect.mjs");
    let script = std::fs::read_to_string(&entry).unwrap();
    let replacement = "for (const entity of output.entities) entity.assertion.attributes.capacityProbe='x'.repeat(1024*1024); process.stdout.write(JSON.stringify(output));";
    assert!(script.contains("process.stdout.write(JSON.stringify(output));"));
    std::fs::write(
        &entry,
        script.replace("process.stdout.write(JSON.stringify(output));", replacement),
    )
    .unwrap();
    let id = validated(&mut f, "2.0.0").await;
    let before = serving_state(&f);
    let maximum: i64 = f
        .database
        .connection()
        .query_row("PRAGMA max_page_count", [], |r| r.get(0))
        .unwrap();
    let pages: i64 = f
        .database
        .connection()
        .query_row("PRAGMA page_count", [], |r| r.get(0))
        .unwrap();
    f.database
        .connection()
        .pragma_update(None, "max_page_count", pages)
        .unwrap();
    let error = registry::activate(&f.project, &mut f.database, &id, "full")
        .await
        .unwrap_err();
    assert!(
        error.message.contains("database or disk is full"),
        "{error:?}"
    );
    assert_eq!(
        f.count("SELECT count(*) FROM activations WHERE status='pending'"),
        1,
        "the pending record must precede the failing C transaction"
    );
    assert_eq!(serving_state(&f), before);
    f.database
        .connection()
        .pragma_update(None, "max_page_count", maximum)
        .unwrap();
    registry::recover(&f.project, &mut f.database, "restart").unwrap();
    assert_eq!(serving_state(&f), before);
    assert_eq!(f.count("SELECT count(*) FROM pragma_foreign_key_check"), 0);
    let integrity: String = f
        .database
        .connection()
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .unwrap();
    assert_eq!(integrity, "ok");
    registry::activate(&f.project, &mut f.database, &id, "capacity restored")
        .await
        .unwrap();
    assert_eq!(
        f.database.generation().unwrap(),
        before["generation"].as_u64().unwrap() + 1
    );
}

/// 必须由独立 mount namespace 提供已满的限额文件系统，不填满宿主磁盘。
#[cfg(unix)]
#[tokio::test]
#[ignore = "requires isolated bounded full filesystem; linux-probe/full-filesystem.sh"]
async fn activation_publish_real_filesystem_full_preserves_old_snapshot_and_retries() {
    let full = std::path::PathBuf::from(
        std::env::var_os("PK_TEST_FULL_FS_ROOT").expect("isolated full filesystem"),
    );
    let mut f = collection::Fixture::new();
    active(&mut f, "1.0.0").await;
    let id = validated(&mut f, "2.0.0").await;
    let before = serving_state(&f);
    let target = f
        .project
        .base_dir
        .join(".project-knowledge/collectors/smoke.plugin/2.0.0");
    std::fs::create_dir_all(&target).unwrap();
    struct Mount {
        target: std::path::PathBuf,
        active: bool,
    }
    impl Drop for Mount {
        fn drop(&mut self) {
            if self.active {
                let _ = std::process::Command::new("umount")
                    .arg(&self.target)
                    .status();
            }
        }
    }
    assert!(
        std::process::Command::new("mount")
            .arg("--bind")
            .arg(&full)
            .arg(&target)
            .status()
            .unwrap()
            .success()
    );
    let mut mount = Mount {
        target,
        active: true,
    };
    let error = registry::activate(&f.project, &mut f.database, &id, "filesystem full")
        .await
        .unwrap_err();
    assert!(
        error.message.contains("No space left on device"),
        "{error:?}"
    );
    assert_eq!(serving_state(&f), before);
    assert_eq!(
        storage::plugins::candidate(f.database.connection(), &id)
            .unwrap()
            .state,
        "validated"
    );
    assert!(
        std::process::Command::new("umount")
            .arg(&mount.target)
            .status()
            .unwrap()
            .success()
    );
    mount.active = false;
    registry::recover(&f.project, &mut f.database, "restart").unwrap();
    assert_eq!(serving_state(&f), before);
    registry::activate(&f.project, &mut f.database, &id, "space restored")
        .await
        .unwrap();
    assert_eq!(
        f.database.generation().unwrap(),
        before["generation"].as_u64().unwrap() + 1
    );
}

#[tokio::test]
async fn activation_current_sources_artifacts_and_project_lock_are_rechecked() {
    let mut f = collection::Fixture::new();
    let id = validated(&mut f, "1.0.0").await;
    let lock = MaintenanceLock::acquire(&f.project).unwrap();
    assert_eq!(
        registry::activate(&f.project, &mut f.database, &id, "now")
            .await
            .unwrap_err()
            .code,
        codes::BUSY
    );
    drop(lock);
    std::fs::write(f.root.join("repo/new.ts"), "const x=1;").unwrap();
    assert_eq!(
        registry::activate(&f.project, &mut f.database, &id, "now")
            .await
            .unwrap_err()
            .code,
        codes::SOURCE_CHANGED
    );
    std::fs::remove_file(f.root.join("repo/new.ts")).unwrap();
    let c = storage::plugins::candidate(f.database.connection(), &id).unwrap();
    std::fs::write(
        std::path::Path::new(&c.directory).join("collect.mjs"),
        "process.exit(1);",
    )
    .unwrap();
    assert_eq!(
        registry::activate(&f.project, &mut f.database, &id, "now")
            .await
            .unwrap_err()
            .code,
        codes::ARTIFACT_CHANGED
    );
}

#[tokio::test]
async fn activation_rollback_runs_new_batches_and_tampering_persists_stale() {
    let mut f = collection::Fixture::new();
    active(&mut f, "1.0.0").await;
    let old = storage::current_batch(f.database.connection(), "(smoke.plugin,extract,smoke)")
        .unwrap()
        .unwrap();
    active(&mut f, "2.0.0").await;
    registry::rollback(
        &f.project,
        &mut f.database,
        "smoke.plugin",
        "1.0.0",
        &CancellationToken::new(),
        "rollback",
    )
    .await
    .unwrap();
    let new = storage::current_batch(f.database.connection(), "(smoke.plugin,extract,smoke)")
        .unwrap()
        .unwrap();
    assert_ne!(old, new);
    assert_eq!(
        storage::batch_state(f.database.connection(), &old).unwrap(),
        Some(storage::BatchState::Stale)
    );
    let path =
        storage::plugins::published_manifest(f.database.connection(), "smoke.plugin", "1.0.0")
            .unwrap();
    std::fs::write(path.parent().unwrap().join("collect.mjs"), "modified").unwrap();
    assert_eq!(
        registry::check_active_artifacts(&f.project, &mut f.database)
            .unwrap_err()
            .code,
        codes::ARTIFACT_CHANGED
    );
    assert_eq!(
        storage::batch_state(f.database.connection(), &new).unwrap(),
        Some(storage::BatchState::Stale)
    );
}

#[tokio::test]
async fn activation_includes_affected_active_link_from_other_plugin() {
    let mut f = collection::Fixture::new();
    active(&mut f, "1.0.0").await;
    let link_dir = f.root.join("link-plugin");
    std::fs::create_dir(&link_dir).unwrap();
    std::fs::copy(
        f.manifest.parent().unwrap().join("collect.mjs"),
        link_dir.join("collect.mjs"),
    )
    .unwrap();
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&f.manifest).unwrap()).unwrap();
    manifest["pluginId"] = json!("smoke.links");
    manifest["stages"] = json!([manifest["stages"][1].clone()]);
    std::fs::write(
        link_dir.join("manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    let c = registry::submit(&f.project, &mut f.database, &link_dir, "now").unwrap();
    registry::validate(
        &f.project,
        &mut f.database,
        &c.id,
        &CancellationToken::new(),
        "now",
    )
    .await
    .unwrap();
    registry::activate(&f.project, &mut f.database, &c.id, "now")
        .await
        .unwrap();
    let old = storage::current_batch(f.database.connection(), "(smoke.links,link,smoke)")
        .unwrap()
        .unwrap();
    let id = validated(&mut f, "2.0.0").await;
    registry::activate(&f.project, &mut f.database, &id, "now")
        .await
        .unwrap();
    let new = storage::current_batch(f.database.connection(), "(smoke.links,link,smoke)")
        .unwrap()
        .unwrap();
    assert_ne!(old, new);
    assert_eq!(
        storage::batch_state(f.database.connection(), &new).unwrap(),
        Some(storage::BatchState::Fresh)
    );
    let deps = storage::load_dependencies(f.database.connection(), &new).unwrap();
    let extract = storage::current_batch(f.database.connection(), "(smoke.plugin,extract,smoke)")
        .unwrap()
        .unwrap();
    assert!(
        deps.iter()
            .any(|(kind, key, _)| kind == "upstreamBatch" && key == &extract)
    );
}

#[tokio::test]
async fn activation_auto_activate_false_keeps_validated_candidate() {
    let mut f = collection::Fixture::new();
    let config = &f.project.config_path;
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(config).unwrap()).unwrap();
    value["plugins"] = json!({"autoLoad":false,"autoActivate":false});
    std::fs::write(config, serde_json::to_vec(&value).unwrap()).unwrap();
    f.project = project_knowledge_core::project::config::LoadedProject::load(config).unwrap();
    project_knowledge_core::index::build_base_index(&f.project, &mut f.database, "now").unwrap();
    let c = registry::submit(&f.project, &mut f.database, &f.manifest, "now").unwrap();
    registry::process_candidate(
        &f.project,
        &mut f.database,
        &c.id,
        &CancellationToken::new(),
        "now",
    )
    .await
    .unwrap();
    assert_eq!(
        storage::plugins::candidate(f.database.connection(), &c.id)
            .unwrap()
            .state,
        "validated"
    );
    assert!(
        storage::active_version_map(f.database.connection())
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn activation_discovery_deduplicates_and_auto_load_false_skips_directories() {
    let mut f = collection::Fixture::new();
    let config = f.project.config_path.clone();
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&config).unwrap()).unwrap();
    value["collectors"] = json!(["plugin/manifest.json"]);
    value["plugins"] = json!({"directories":["plugin"],"autoLoad":true,"autoActivate":false});
    std::fs::write(&config, serde_json::to_vec(&value).unwrap()).unwrap();
    f.project = project_knowledge_core::project::config::LoadedProject::load(&config).unwrap();
    project_knowledge_core::index::build_base_index(&f.project, &mut f.database, "now").unwrap();
    let found = registry::discover(&f.project, &mut f.database, "now").unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].state, "generated");
    assert_eq!(f.count("SELECT count(*) FROM validations"), 0);
    assert!(
        registry::discover(&f.project, &mut f.database, "now")
            .unwrap()
            .is_empty()
    );
    f.project.config.plugins.auto_load = false;
    assert!(
        registry::discover(&f.project, &mut f.database, "now")
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn activation_publish_io_failure_preserves_active_state_and_published_versions_are_immutable()
{
    let mut f = collection::Fixture::new();
    active(&mut f, "1.0.0").await;
    let id = validated(&mut f, "2.0.0").await;
    let generation = f.database.generation().unwrap();
    let blocker = f
        .project
        .base_dir
        .join(".project-knowledge/collectors/smoke.plugin/2.0.0");
    std::fs::write(
        &blocker,
        "this file prevents creating the version directory",
    )
    .unwrap();
    assert_eq!(
        registry::activate(&f.project, &mut f.database, &id, "now")
            .await
            .unwrap_err()
            .code,
        codes::INTERNAL_ERROR
    );
    assert_eq!(f.database.generation().unwrap(), generation);
    assert_eq!(
        storage::active_version_map(f.database.connection()).unwrap(),
        vec![("smoke.plugin".into(), "1.0.0".into())]
    );
    f.edit(|m| {
        m["version"] = json!("1.0.0");
        m["stages"][0]["args"] = json!(["--empty"]);
    });
    assert_eq!(
        registry::submit(&f.project, &mut f.database, &f.manifest, "now")
            .unwrap_err()
            .code,
        codes::ARTIFACT_CHANGED
    );
}
