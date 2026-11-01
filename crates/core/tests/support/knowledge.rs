use project_knowledge_core::{
    contract::{
        Basis, Origin,
        error::{CoreError, codes},
        knowledge::*,
        query::{CommonFilters, QueryRequest, SearchRequest, StatusRequest},
    },
    index::build_base_index,
    knowledge::{context, documents, skill},
    project::config::{IdentityDelta, LoadedProject},
    storage::Database,
};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

struct Fixture {
    _temp: tempfile::TempDir,
    main: PathBuf,
    other: PathBuf,
    project: LoadedProject,
    database: Database,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let main = temp.path().join("main");
        let other = temp.path().join("other");
        std::fs::create_dir_all(main.join(".project-knowledge")).unwrap();
        std::fs::create_dir_all(main.join("src")).unwrap();
        std::fs::create_dir_all(&other).unwrap();
        std::fs::write(
            main.join("src/ledger.sql"),
            "-- ledger quantity from original local SQL\nSELECT ordered_qty FROM purchase_line;\n",
        )
        .unwrap();
        std::fs::write(other.join("other.ts"), "export const unrelated = 1;\n").unwrap();
        let config = main.join(".project-knowledge/project.json");
        std::fs::write(&config,serde_json::to_vec(&json!({"schemaVersion":1,"projectId":"knowledge-fixture","repositories":[{"id":"main","root":".."},{"id":"other","root":"../../other"}],"selection":{}})).unwrap()).unwrap();
        let project = LoadedProject::load(&config).unwrap();
        let mut database = Database::open_maintenance(
            &main.join(".project-knowledge/index.sqlite"),
            "knowledge-fixture",
            &project.config_hash().unwrap(),
            "now",
        )
        .unwrap();
        build_base_index(&project, &mut database, "now").unwrap();
        Self {
            _temp: temp,
            main,
            other,
            project,
            database,
        }
    }
    fn context(&mut self) -> TopicContext {
        context::export(
            &mut self.database,
            &self.project,
            &TopicContextRequest {
                topic_id: "ledger".into(),
                query: QueryRequest::Search(SearchRequest {
                    query: "ledger".into(),
                    fields: None,
                    filters: Some(CommonFilters {
                        repository_ids: Some(vec!["main".into()]),
                        ..Default::default()
                    }),
                    page_size: Some(8),
                    continuation: None,
                    budget: None,
                }),
                max_chars: 80_000,
            },
            "now",
        )
        .unwrap()
    }
    fn artifact(&self, context: &TopicContext) -> TopicArtifact {
        assert!(!context.source_references.is_empty());
        TopicArtifact{protocol_version:"1.0".into(),topic_id:context.topic_id.clone(),title:"Local ledger SQL".into(),snapshot:context.snapshot.clone(),input_digest:context.input_digest.clone(),markdown:"The original local SQL selects ordered_qty from purchase_line. [[claim:select_quantity]]".into(),claims:vec![TopicClaim{id:"select_quantity".into(),text:"The original local SQL selects ordered_qty from purchase_line.".into(),basis:Basis::Observed,origin:Origin::Human,evidence_refs:vec![context.source_references.iter().max_by_key(|r|r.end_byte-r.start_byte).unwrap().evidence_id.clone()],gap_ids:vec![]}],source_references:context.source_references.clone(),gaps:context.gaps.clone(),prompt_hash:context.prompt_hash.clone(),template_hash:context.template_hash.clone(),model_info:None}
    }
    fn output(&self) -> PathBuf {
        self.main.join(".agents/skills/ledger")
    }
    fn status(&mut self) -> Value {
        let response = project_knowledge_core::query::execute(
            &mut project_knowledge_core::query::QueryContext {
                database: &mut self.database,
                project: Some(&self.project),
                now: "now".into(),
            },
            &QueryRequest::Status(StatusRequest {
                check_freshness: true,
            }),
        )
        .unwrap();
        response.data["knowledge"].clone()
    }
}
fn evidence(name: &str, value: &Value) {
    if let Some(root) = std::env::var_os("PK_KNOWLEDGE_EVIDENCE_DIR") {
        let path = PathBuf::from(root);
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(path.join(name), serde_json::to_vec_pretty(value).unwrap()).unwrap();
    }
}
fn verify_files(directory: &Path) -> Value {
    let manifest: Value = serde_json::from_slice(
        &std::fs::read(directory.join("references/index-manifest.json")).unwrap(),
    )
    .unwrap();
    for (path, hash) in manifest["files"].as_object().unwrap() {
        let bytes = std::fs::read(directory.join(path)).unwrap();
        assert_eq!(
            project_knowledge_core::contract::content_hash(&bytes),
            hash.as_str().unwrap()
        );
    }
    let complete: Value =
        serde_json::from_slice(&std::fs::read(directory.join("COMPLETE.json")).unwrap()).unwrap();
    assert_eq!(
        complete["manifestSha256"],
        project_knowledge_core::contract::content_hash(
            &std::fs::read(directory.join("references/index-manifest.json")).unwrap()
        )
    );
    let links = regex::Regex::new(r"\]\(([^)]+)\)").unwrap();
    for name in manifest["files"]
        .as_object()
        .unwrap()
        .keys()
        .filter(|n| n.ends_with(".md"))
    {
        let file = directory.join(name);
        let text = std::fs::read_to_string(&file).unwrap();
        for cap in links.captures_iter(&text) {
            let (target, anchor) = cap[1]
                .split_once('#')
                .map_or((cap[1].as_ref(), None), |(p, a)| (p, Some(a)));
            let path = if target.is_empty() {
                file.clone()
            } else {
                file.parent().unwrap().join(target)
            };
            assert!(path.is_file(), "broken link {} in {}", &cap[1], name);
            if let Some(anchor) = anchor {
                let body = std::fs::read_to_string(path).unwrap();
                assert!(
                    body.contains(&format!("id=\"{anchor}\"")),
                    "missing anchor {anchor}"
                );
            }
        }
    }
    manifest
}

#[test]
fn knowledge_external_context_topic_skill_and_original_references() {
    let mut f = Fixture::new();
    let context = f.context();
    let repeat = context::export(
        &mut f.database,
        &f.project,
        &context.request,
        "different-clock",
    )
    .unwrap();
    assert_eq!(
        context.input_digest, repeat.input_digest,
        "input receipts must ignore checking clocks and retain semantic material"
    );
    assert!(serde_json::to_string(&context).unwrap().chars().count() <= 80_000);
    let artifact = f.artifact(&context);
    let imported = documents::import_topic(&mut f.database, &f.project, &artifact, "now").unwrap();
    assert_eq!(imported["version"], 1);
    let output = f.output();
    let result = skill::generate(&mut f.database, &f.project, "ledger", &output, "now").unwrap();
    assert_eq!(result["freshness"], "fresh");
    let manifest = verify_files(Path::new(result["directory"].as_str().unwrap()));
    for name in [
        "conventions",
        "routes",
        "frontend",
        "api",
        "database",
        "code-map",
    ] {
        assert!(
            manifest["files"]
                .get(format!("references/{name}.md"))
                .is_some()
        );
    }
    assert!(
        manifest["files"]
            .get("references/topics/ledger.md")
            .is_some()
    );
    let generation = f.database.generation().unwrap();
    let status = f.status();
    assert_eq!(status["topics"][0]["freshness"], "fresh");
    assert_eq!(generation, f.database.generation().unwrap());
    let indexed = documents::status(f.database.connection(), Some(&f.project), false).unwrap();
    assert_eq!(indexed["topics"][0]["freshness"], "unknown");
    assert_eq!(indexed["exports"][0]["freshness"], "unknown");
    let skill_path = Path::new(result["directory"].as_str().unwrap()).join("SKILL.md");
    let original = std::fs::read(&skill_path).unwrap();
    std::fs::write(&skill_path, "tampered").unwrap();
    let tampered = f.status();
    assert_eq!(tampered["exports"][0]["integrity"], false);
    assert_eq!(tampered["exports"][0]["freshness"], "stale");
    std::fs::write(skill_path, original).unwrap();
    let scan = project_knowledge_core::project::snapshot::scan_project(&f.project).unwrap();
    assert!(
        !scan
            .iter()
            .flat_map(|s| &s.sources)
            .any(|s| s.path.contains(".agents/skills") || s.path.contains(".project-knowledge"))
    );
    assert!(
        skill::generate(
            &mut f.database,
            &f.project,
            "ledger",
            &f.main.join("generated"),
            "now"
        )
        .is_err()
    );
    evidence("context.json", &serde_json::to_value(context).unwrap());
    evidence("topic.json", &serde_json::to_value(artifact).unwrap());
    evidence("manifest.json", &manifest);
    evidence("fresh-status.json", &status);
}

#[test]
fn knowledge_code_map_uses_selected_facts_instead_of_topic_label() {
    let mut f = Fixture::new();
    std::fs::write(
        f.main.join("src/actual.ts"),
        "export function preciseCode() { return 1; }\n",
    )
    .unwrap();
    std::fs::write(
        f.main.join("misleading_topic.md"),
        "# Topic label decoy\nNo code declarations here.\n",
    )
    .unwrap();
    build_base_index(&f.project, &mut f.database, "next").unwrap();
    f.database.connection().execute(
        "UPDATE entity_assertions SET attributes_json=json_set(attributes_json,'$.entityId','ent_project_metadata') WHERE entity_id IN (SELECT id FROM entities WHERE kind='symbol')",
        [],
    ).unwrap();
    let context = context::export(
        &mut f.database,
        &f.project,
        &TopicContextRequest {
            topic_id: "misleading_topic".into(),
            query: QueryRequest::Search(SearchRequest {
                query: "preciseCode".into(),
                fields: None,
                filters: Some(CommonFilters {
                    repository_ids: Some(vec!["main".into()]),
                    ..Default::default()
                }),
                page_size: Some(8),
                continuation: None,
                budget: None,
            }),
            max_chars: 80_000,
        },
        "next",
    )
    .unwrap();
    assert_eq!(
        context.code_map.status,
        project_knowledge_core::contract::query::ResponseStatus::Complete
    );
    let files = context.code_map.data["files"].as_array().unwrap();
    assert!(!files.is_empty());
    assert!(files.iter().all(|f| f["path"] == "src/actual.ts"));
    assert!(
        context.code_map.data["uncoveredLanguages"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    evidence(
        "context-fact-roots.json",
        &serde_json::to_value(context).unwrap(),
    );
}

#[test]
fn knowledge_rejects_forged_cyclic_and_unmarked_explanations() {
    let mut f = Fixture::new();
    let context = f.context();
    let original = f.artifact(&context);
    let mut failures = Vec::new();
    for mutation in 0..14 {
        let mut artifact = original.clone();
        match mutation {
            0 => artifact.input_digest = "forged".into(),
            1 => artifact.snapshot.index_id = "other-index".into(),
            2 => artifact.source_references[0].end_byte += 1,
            3 => artifact.claims[0].evidence_refs.clear(),
            4 => artifact.claims[0].origin = Origin::Ai,
            5 => {
                artifact.claims[0].basis = Basis::Inferred;
                artifact.claims[0].evidence_refs.clear();
            }
            6 => artifact.markdown = "uncited explanation".into(),
            7 => artifact.markdown = "topic:other proves this. [[claim:select_quantity]]".into(),
            8 => {
                artifact.markdown =
                    "```mermaid\nflowchart LR\na-->b\n```\n[[claim:select_quantity]]".into()
            }
            9 => artifact.claims.push(artifact.claims[0].clone()),
            10 => artifact.claims[0].gap_ids = vec!["missing".into()],
            11 => artifact.prompt_hash = "changed".into(),
            12 => artifact.template_hash = "changed".into(),
            13 => artifact.gaps.clear(),
            _ => unreachable!(),
        }
        let error =
            documents::import_topic(&mut f.database, &f.project, &artifact, "now").unwrap_err();
        failures.push(json!({"case":mutation,"code":error.code}));
    }
    for class in ["generatedKnowledge", "derivedConvention"] {
        f.database
            .connection()
            .execute(
                "UPDATE source_versions SET source_class=?1 WHERE source_id=?2",
                rusqlite::params![class, original.source_references[0].source_id],
            )
            .unwrap();
        assert_eq!(
            documents::import_topic(&mut f.database, &f.project, &original, "now")
                .unwrap_err()
                .code,
            codes::EVIDENCE_INVALID
        );
    }
    let count: i64 = f
        .database
        .connection()
        .query_row("SELECT count(*) FROM topics", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 0);
    evidence("rejections.json", &json!(failures));
}

#[test]
fn knowledge_relevant_sql_stales_rerender_does_not_refresh_and_unrelated_generation_survives() {
    let mut f = Fixture::new();
    let context = f.context();
    let artifact = f.artifact(&context);
    documents::import_topic(&mut f.database, &f.project, &artifact, "now").unwrap();
    std::fs::write(f.other.join("other.ts"), "export const unrelated = 2;\n").unwrap();
    build_base_index(&f.project, &mut f.database, "later").unwrap();
    let unrelated = f.status();
    assert_eq!(unrelated["topics"][0]["freshness"], "fresh");
    assert_eq!(
        unrelated["topics"][0]["snapshot"],
        serde_json::to_value(&context.snapshot).unwrap()
    );
    assert_eq!(unrelated["topics"][0]["dependenciesRechecked"], true);
    let sql = f.main.join("src/ledger.sql");
    let old = std::fs::read(&sql).unwrap();
    std::fs::write(
        &sql,
        "-- ledger changed\nSELECT sum(ordered_qty) FROM purchase_line;\n",
    )
    .unwrap();
    let read_only_stale = f.status();
    assert_eq!(read_only_stale["topics"][0]["freshness"], "stale");
    let output = f.output();
    let stale = skill::generate(&mut f.database, &f.project, "ledger", &output, "later").unwrap();
    assert_eq!(stale["freshness"], "stale");
    verify_files(Path::new(stale["directory"].as_str().unwrap()));
    std::fs::write(&sql, old).unwrap();
    assert_eq!(
        f.status()["topics"][0]["freshness"],
        "stale",
        "restoring source must not revive persisted explanation"
    );
    build_base_index(&f.project, &mut f.database, "later").unwrap();
    assert_eq!(f.status()["topics"][0]["freshness"], "stale");
    let current = f.context();
    let updated = f.artifact(&current);
    let imported = documents::import_topic(&mut f.database, &f.project, &updated, "later").unwrap();
    assert_eq!(imported["version"], 2);
    let fresh = skill::generate(&mut f.database, &f.project, "ledger", &output, "later").unwrap();
    assert_eq!(fresh["freshness"], "fresh");
    let status = f.status();
    let old = status["exports"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == stale["exportId"])
        .unwrap();
    assert_eq!(old["freshness"], "stale");
    evidence(
        "freshness-timeline.json",
        &json!({"unrelated":unrelated,"relatedReadOnly":read_only_stale,"rerender":stale,"reimport":imported,"newExport":fresh}),
    );
}

#[test]
fn knowledge_atomic_export_interruption_retains_last_complete_generation() {
    let mut f = Fixture::new();
    let context = f.context();
    let artifact = f.artifact(&context);
    documents::import_topic(&mut f.database, &f.project, &artifact, "now").unwrap();
    let output = f.output();
    let first = skill::generate(&mut f.database, &f.project, "ledger", &output, "now").unwrap();
    let pointer = std::fs::read(output.join("CURRENT.json")).unwrap();
    for phase in [
        "file-written",
        "before-publish",
        "before-pointer",
        "transaction",
    ] {
        let error = skill::generate_with_checkpoint(
            &mut f.database,
            &f.project,
            "ledger",
            &output,
            "later",
            |at| {
                if at == phase {
                    Err(CoreError::new(codes::CANCELLED, "injected cancellation"))
                } else {
                    Ok(())
                }
            },
        )
        .unwrap_err();
        assert_eq!(error.code, codes::CANCELLED);
        assert_eq!(pointer, std::fs::read(output.join("CURRENT.json")).unwrap());
        verify_files(Path::new(first["directory"].as_str().unwrap()));
    }
    let sql = f.main.join("src/ledger.sql");
    let error = skill::generate_with_checkpoint(
        &mut f.database,
        &f.project,
        "ledger",
        &output,
        "later",
        |phase| {
            if phase == "before-pointer" {
                std::fs::write(&sql, "-- ledger changed during rendering\nSELECT 1;\n").unwrap();
            }
            Ok(())
        },
    )
    .unwrap_err();
    assert_eq!(error.code, codes::DEPENDENCY_CHANGED);
    assert_eq!(pointer, std::fs::read(output.join("CURRENT.json")).unwrap());
    let status = f.status();
    assert_eq!(
        status["attempts"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|a| a["state"] == "failed")
            .count(),
        5
    );
    let second = skill::generate(&mut f.database, &f.project, "ledger", &output, "later").unwrap();
    assert_ne!(pointer, std::fs::read(output.join("CURRENT.json")).unwrap());
    verify_files(Path::new(second["directory"].as_str().unwrap()));
    evidence("atomic-export.json", &status);
}

#[test]
fn knowledge_conventions_expand_original_evidence_and_identity_cas() {
    let mut f = Fixture::new();
    let context = f.context();
    let artifact = f.artifact(&context);
    documents::import_topic(&mut f.database, &f.project, &artifact, "now").unwrap();
    let mut input = ConventionsImport {
        expected_config_hash: f.project.config_hash().unwrap(),
        notes: "Original fixture query reads ordered_qty; production units remain unknown.".into(),
        source_references: context.source_references.clone(),
        identity_delta: IdentityDelta::default(),
    };
    let invalid = ConventionsImport {
        expected_config_hash: "wrong".into(),
        ..input.clone()
    };
    assert_eq!(
        documents::import_conventions(&mut f.database, &f.project, &invalid, "now")
            .unwrap_err()
            .code,
        codes::ACTIVATION_CONFLICT
    );
    input
        .identity_delta
        .scopes
        .push(project_knowledge_core::project::config::ScopeConfig {
            id: "sql".into(),
            repository_ids: vec!["main".into()],
            roots: vec![],
            kind: project_knowledge_core::project::config::ScopeKind::Module,
        });
    let result = documents::import_conventions(&mut f.database, &f.project, &input, "now").unwrap();
    assert_eq!(result["indexInvalidated"], true);
    assert_eq!(f.status()["topics"][0]["freshness"], "stale");
    f.project = LoadedProject::load(&f.project.config_path).unwrap();
    assert!(f.project.config.scopes.iter().any(|s| s.id == "sql"));
    assert_eq!(result["configHash"], f.project.config_hash().unwrap());
    assert_eq!(
        documents::import_conventions(&mut f.database, &f.project, &input, "now")
            .unwrap_err()
            .code,
        codes::SNAPSHOT_EXPIRED
    );
    build_base_index(&f.project, &mut f.database, "now").unwrap();
    let updated = f.context();
    assert_eq!(updated.conventions.len(), 1);
    assert_eq!(updated.conventions[0]["sourceClass"], "derivedConvention");
    let stored =
        project_knowledge_core::storage::knowledge::conventions(f.database.connection()).unwrap();
    assert_eq!(
        stored[0]["sourceReferences"],
        serde_json::to_value(&input.source_references).unwrap()
    );
    let mut current = input.source_references.clone();
    for r in &mut current {
        r.snapshot = updated.snapshot.clone();
    }
    assert_eq!(
        updated.conventions[0]["sourceReferences"],
        serde_json::to_value(current).unwrap()
    );
    evidence("conventions.json", &result);
}

#[test]
fn knowledge_control_aborts_inflight_query_and_expired_import_without_commit() {
    use project_knowledge_core::knowledge::KnowledgeControl;
    let mut f = Fixture::new();
    for point in ["search_documents", "BEGIN IMMEDIATE"] {
        let mut f = Fixture::new();
        let control = KnowledgeControl::default();
        let signal = control.cancel.clone();
        f.database
            .observe_sql(move |sql| {
                if sql.contains(point) {
                    signal.cancel();
                }
            })
            .unwrap();
        let request = TopicContextRequest {
            topic_id: "ledger".into(),
            query: QueryRequest::Search(SearchRequest {
                query: "ledger".into(),
                fields: None,
                filters: None,
                page_size: Some(8),
                continuation: None,
                budget: None,
            }),
            max_chars: 80_000,
        };
        let error =
            context::export_with_control(&mut f.database, &f.project, &request, "now", &control)
                .unwrap_err();
        assert_eq!(error.code, codes::CANCELLED);
        let receipts: i64 = f
            .database
            .connection()
            .query_row("SELECT count(*) FROM knowledge_contexts", [], |r| r.get(0))
            .unwrap();
        assert_eq!(receipts, 0);
    }
    let context = f.context();
    let artifact = f.artifact(&context);
    let control = KnowledgeControl::default();
    let signal = control.cancel.clone();
    f.database
        .observe_sql(move |sql| {
            if sql.contains("BEGIN IMMEDIATE") {
                signal.cancel();
            }
        })
        .unwrap();
    let error = documents::import_topic_with_control(
        &mut f.database,
        &f.project,
        &artifact,
        "now",
        &control,
    )
    .unwrap_err();
    assert_eq!(error.code, codes::CANCELLED);
    assert!(
        project_knowledge_core::storage::knowledge::topics(f.database.connection())
            .unwrap()
            .is_empty()
    );
    let expired = KnowledgeControl {
        cancel: tokio_util::sync::CancellationToken::new(),
        deadline: Some(std::time::Instant::now()),
    };
    let input = ConventionsImport {
        expected_config_hash: f.project.config_hash().unwrap(),
        notes: "not committed".into(),
        source_references: vec![],
        identity_delta: IdentityDelta::default(),
    };
    let error = documents::import_conventions_with_control(
        &mut f.database,
        &f.project,
        &input,
        "now",
        &expired,
    )
    .unwrap_err();
    assert_eq!(error.code, codes::BUDGET_EXHAUSTED);
    assert!(
        project_knowledge_core::storage::knowledge::conventions(f.database.connection())
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn knowledge_plugin_fact_and_candidate_sets_invalidate_zero_matches() {
    let mut f = Fixture::new();
    std::fs::write(f.main.join("src/plugin.ts"), "export const smoke = 1;\n").unwrap();
    build_base_index(&f.project, &mut f.database, "now").unwrap();
    let context = f.context();
    let mut artifact = f.artifact(&context);
    artifact.markdown.push_str("\n```mermaid\nflowchart LR\nquantity[Quantity] %% [[claim:select_quantity]]\n```\nUnit is unknown. [[claim:unknown_unit]]");
    artifact
        .gaps
        .push(project_knowledge_core::contract::QueryGap {
            id: "unknown_unit".into(),
            code: "UNIT_UNKNOWN".into(),
            capability: "knowledge".into(),
            scope_id: "repo.main".into(),
            source_refs: vec![],
            candidate_ids: vec![],
            reason: "The original SQL does not specify quantity units".into(),
            required_evidence: vec!["original unit declaration".into()],
        });
    artifact.claims.push(TopicClaim {
        id: "unknown_unit".into(),
        text: "Unit is unknown.".into(),
        basis: Basis::Inferred,
        origin: Origin::Ai,
        evidence_refs: vec![],
        gap_ids: vec!["unknown_unit".into()],
    });
    documents::import_topic(&mut f.database, &f.project, &artifact, "now").unwrap();
    let directory = f._temp.path().join("collector");
    std::fs::create_dir(&directory).unwrap();
    let script = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/plugins/staging/collect.mjs"),
    )
    .unwrap()
    .replace("'repo.r'", "'repo.main'");
    std::fs::write(directory.join("collect.mjs"), script).unwrap();
    let manifest = directory.join("manifest.json");
    std::fs::write(&manifest,serde_json::to_vec(&json!({"protocolVersion":"1.0","pluginId":"smoke.plugin","version":"1.0.0","runtime":{"kind":"node","versionRange":">=24 <25"},"artifacts":["manifest.json","collect.mjs"],"stages":[{"id":"extract","phase":"extract","entry":"collect.mjs","args":[],"replacement":"project","sourceSelectors":[{"id":"ts","repositoryIds":["main"],"include":["**/*.ts"]}],"entitySelectors":[{"id":"targets","kinds":["route","file"]}]}]})).unwrap()).unwrap();
    let staged = project_knowledge_core::collect::staging::stage_collection(
        &f.project,
        &f.database,
        std::slice::from_ref(&manifest),
        None,
        &tokio_util::sync::CancellationToken::new(),
        "now",
    )
    .await
    .unwrap();
    staged
        .commit(&f.project, &mut f.database, "now")
        .await
        .unwrap();
    assert_eq!(
        f.status()["topics"][0]["freshness"],
        "stale",
        "new plugin facts invalidate an earlier zero-match set"
    );
    let request = TopicContextRequest {
        topic_id: "smoke".into(),
        query: QueryRequest::Search(SearchRequest {
            query: "smoke".into(),
            fields: None,
            filters: None,
            page_size: Some(8),
            continuation: None,
            budget: None,
        }),
        max_chars: 120_000,
    };
    let current = context::export(&mut f.database, &f.project, &request, "now").unwrap();
    assert!(current.dependencies.iter().any(|d| d.kind == "selectors"));
    assert!(current.dependencies.iter().any(|d| d.kind == "batch"));
    let mut topic = f.artifact(&current);
    topic.markdown = "The local fixture has a Smoke route. [[claim:select_quantity]]".into();
    topic.claims[0].text = "The local fixture has a Smoke route.".into();
    documents::import_topic(&mut f.database, &f.project, &topic, "now").unwrap();
    let key = current
        .dependencies
        .iter()
        .find(|d| d.kind == "batch")
        .unwrap()
        .key
        .clone();
    let generation = f.database.generation().unwrap() + 1;
    let tx = f.database.write_transaction().unwrap();
    project_knowledge_core::storage::invalidate_batches(&tx, &[key], generation).unwrap();
    tx.commit().unwrap();
    let status = f.status();
    assert_eq!(
        status["topics"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["topicId"] == "smoke")
            .unwrap()["freshness"],
        "stale"
    );
    evidence(
        "plugin-dependencies.json",
        &serde_json::to_value(current).unwrap(),
    );
}
