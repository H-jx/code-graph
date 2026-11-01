//! Frozen source truth is checked independently of business collector output.
use project_knowledge_core::{
    collect::candidate::{CandidateFile, write_bundle},
    contract::{
        entity::EntityKind,
        identity::{
            IdentitySlot, IdentityValue, compute_entity_id, compute_relation_id, content_hash,
            entity_components,
        },
    },
    project::config::LoadedProject,
};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

#[path = "support/linking.rs"]
mod linking;

#[path = "support/semantics.rs"]
mod semantics;

#[path = "support/scenarios.rs"]
mod scenarios;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_json(path: &str) -> Value {
    serde_json::from_slice(&std::fs::read(root().join(path)).unwrap()).unwrap()
}

fn truth_entity_id(project: &str, kind: EntityKind, fields: &Value) -> String {
    let slots: Vec<_> = entity_components(kind.clone())
        .iter()
        .map(|spec| IdentitySlot {
            name: spec.name.into(),
            value: fields
                .get(spec.name)
                .map(|value| serde_json::from_value::<IdentityValue>(value.clone()).unwrap()),
        })
        .collect();
    compute_entity_id(project, kind, &slots).unwrap()
}

#[test]
fn truth_frontend_expected_and_forbidden_identities_are_complete() {
    let truth = read_json("fixtures/truth/frontend.json");
    for layout in truth["layouts"].as_array().unwrap() {
        let project = layout["projectId"].as_str().unwrap();
        let mut entities = std::collections::BTreeMap::new();
        for entity in layout["expectedEntities"].as_array().unwrap() {
            assert!(!entity["evidence"].as_array().unwrap().is_empty());
            let id = truth_entity_id(
                project,
                serde_json::from_value(entity["kind"].clone()).unwrap(),
                &entity["identity"],
            );
            assert!(
                entities
                    .insert(entity["key"].as_str().unwrap(), id)
                    .is_none()
            );
        }
        assert_eq!(entities.len(), 13);
        let mut relations = std::collections::BTreeSet::new();
        for relation in layout["expectedRelations"].as_array().unwrap() {
            assert!(!relation["evidence"].as_array().unwrap().is_empty());
            let id = compute_relation_id(
                project,
                serde_json::from_value(relation["kind"].clone()).unwrap(),
                &entities[relation["from"].as_str().unwrap()],
                &entities[relation["to"].as_str().unwrap()],
                relation["qualifiers"].as_object().unwrap(),
            )
            .unwrap();
            assert!(relations.insert(id));
        }
        assert_eq!(relations.len(), 10);
        for forbidden in layout["forbiddenRelations"].as_array().unwrap() {
            let target = truth_entity_id(
                project,
                serde_json::from_value(forbidden["toKind"].clone()).unwrap(),
                &forbidden["toIdentity"],
            );
            let id = compute_relation_id(
                project,
                serde_json::from_value(forbidden["kind"].clone()).unwrap(),
                &entities[forbidden["from"].as_str().unwrap()],
                &target,
                forbidden["qualifiers"].as_object().unwrap(),
            )
            .unwrap();
            assert!(!relations.contains(&id));
        }
    }
}

fn verify_citations(value: &Value, files: &serde_json::Map<String, Value>) -> usize {
    match value {
        Value::Object(object) if object.contains_key("startLine") => {
            let path = object["path"].as_str().unwrap();
            assert!(files.contains_key(path), "unfrozen citation: {path}");
            let content = std::fs::read_to_string(root().join(path)).unwrap();
            let lines: Vec<_> = content.lines().collect();
            let start = object["startLine"].as_u64().unwrap() as usize;
            let end = object["endLine"].as_u64().unwrap() as usize;
            assert!(start > 0 && start <= end && end <= lines.len());
            assert!(
                lines[start - 1..end]
                    .join("\n")
                    .contains(object["text"].as_str().unwrap())
            );
            1
        }
        Value::Object(object) => object.values().map(|v| verify_citations(v, files)).sum(),
        Value::Array(array) => array.iter().map(|v| verify_citations(v, files)).sum(),
        _ => 0,
    }
}

#[test]
fn truth_frozen_sources_citations_and_provenance() {
    let manifest = read_json("fixtures/truth/manifest.json");
    let files = manifest["files"].as_object().unwrap();
    assert!(files.len() >= 45);
    for (path, expected) in files {
        let bytes = std::fs::read(root().join(path)).unwrap();
        assert_eq!(content_hash(&bytes), expected.as_str().unwrap(), "{path}");
    }
    assert_eq!(
        content_hash(
            project_knowledge_core::contract::identity::canonical_json(&Value::Object(
                files.clone()
            ))
            .unwrap()
            .as_bytes()
        ),
        manifest["truthHash"].as_str().unwrap()
    );
    let mut citations = 0;
    for name in [
        "scenario1",
        "scenario2",
        "scenario3",
        "negative",
        "frontend",
    ] {
        citations += verify_citations(&read_json(&format!("fixtures/truth/{name}.json")), files);
    }
    assert!(
        citations >= 30,
        "every reviewed positive and negative needs a source"
    );
    let provenance = read_json("fixtures/original/provenance.json");
    assert_eq!(provenance["revision"], manifest["sourceRevision"]);
    for file in provenance["files"].as_array().unwrap() {
        assert!(!file["lastCommit"].as_str().unwrap().is_empty());
        assert_eq!(files[file["path"].as_str().unwrap()], file["sha256"]);
    }
}

#[test]
fn truth_numeric_sql_matches_frozen_rows_and_rejects_both_mutants() {
    let db = rusqlite::Connection::open_in_memory().unwrap();
    db.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
    for file in ["schema.sql", "data.sql"] {
        db.execute_batch(
            &std::fs::read_to_string(root().join("fixtures/purchase-ledger").join(file)).unwrap(),
        )
        .unwrap();
    }
    let truth = read_json("fixtures/truth/scenario3.json");
    let sql = std::fs::read_to_string(root().join("fixtures/purchase-ledger/correct.sql")).unwrap();
    let mut statement = db.prepare(&sql).unwrap();
    let rows: Vec<Value> = statement
        .query_map(rusqlite::named_params! {":tenant":"A"}, |r| {
            Ok(json!([
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, i64>(4)?,
                r.get::<_, i64>(5)?,
                r.get::<_, i64>(6)?,
                r.get::<_, i64>(7)?,
                r.get::<_, i64>(8)?
            ]))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(json!(rows), truth["rows"]);
    let totals: Vec<i64> = (4..9)
        .map(|column| rows.iter().map(|r| r[column].as_i64().unwrap()).sum())
        .collect();
    assert_eq!(json!(totals), truth["totals"]);
    let fanout =
        std::fs::read_to_string(root().join("fixtures/purchase-ledger/fanout.sql")).unwrap();
    let (gross, invoice): (i64, i64) = db
        .query_row(&fanout, rusqlite::named_params! {":tenant":"A"}, |r| {
            Ok((r.get(2)?, r.get(3)?))
        })
        .unwrap();
    assert_eq!(gross, 10);
    assert_eq!(invoice, 4000);
    assert_ne!(gross, rows[0][4].as_i64().unwrap());
    let where_child =
        std::fs::read_to_string(root().join("fixtures/purchase-ledger/where-child.sql")).unwrap();
    let count = db
        .prepare(&where_child)
        .unwrap()
        .query_map(rusqlite::named_params! {":tenant":"A"}, |_| Ok(()))
        .unwrap()
        .count();
    assert_eq!(count, 2);
    assert_ne!(count, rows.len());
    if let Some(directory) = std::env::var_os("PK_SCENARIO_EVIDENCE_DIR") {
        let directory = PathBuf::from(directory);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join("numeric-sql.json"),serde_json::to_vec_pretty(&json!({"actualRows":rows,"expectedRows":truth["rows"],"actualTotals":totals,"expectedTotals":truth["totals"],"rowDifferences":[],"totalDifferences":[],"fanout":{"firstGrossReceived":gross,"firstInvoiceCents":invoice},"whereChildRowCount":count,"businessDatabaseConnections":0})).unwrap()).unwrap();
    }
}

#[test]
fn truth_candidate_writer_cannot_modify_the_baseline() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::create_dir(temp.path().join("repo")).unwrap();
    let config = temp.path().join("project.json");
    std::fs::write(
        &config,
        serde_json::to_vec(&json!({"schemaVersion":1,"projectId":"truth-boundary",
        "repositories":[{"id":"r","root":"repo"}],"selection":{}}))
        .unwrap(),
    )
    .unwrap();
    let project = LoadedProject::load(&config).unwrap();
    let before = std::fs::read(root().join("fixtures/truth/manifest.json")).unwrap();
    for path in [
        "../../fixtures/truth/scenario1.json".to_string(),
        root()
            .join("fixtures/truth/scenario1.json")
            .to_string_lossy()
            .to_string(),
    ] {
        assert!(
            write_bundle(
                &project,
                "agent_truth",
                1,
                &[CandidateFile {
                    path,
                    content: "{}".into()
                }]
            )
            .is_err()
        );
    }
    let owned = write_bundle(
        &project,
        "agent_truth",
        2,
        &[CandidateFile {
            path: "fixtures/truth/scenario1.json".into(),
            content: "{}".into(),
        }],
    )
    .unwrap();
    assert!(owned.starts_with(std::fs::canonicalize(&project.base_dir).unwrap()));
    assert_eq!(
        std::fs::read(root().join("fixtures/truth/manifest.json")).unwrap(),
        before
    );
}

struct FrontendFixture {
    temp: tempfile::TempDir,
    project: LoadedProject,
    database: project_knowledge_core::storage::Database,
    manifest: PathBuf,
}

fn copy_tree(source: &Path, target: &Path) {
    std::fs::create_dir_all(target).unwrap();
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target.join(entry.file_name()));
        } else {
            std::fs::copy(entry.path(), target.join(entry.file_name())).unwrap();
        }
    }
}

impl FrontendFixture {
    async fn new(layout: &str) -> Self {
        Self::new_for(layout, "frontend").await
    }

    async fn new_for(layout: &str, capability: &str) -> Self {
        use project_knowledge_core::collect::{deps, manifest, staging};
        let temp = tempfile::tempdir().unwrap();
        copy_tree(
            &root().join(format!("fixtures/page-location/{layout}")),
            temp.path(),
        );
        if capability == "database" {
            let repository = if layout == "a" {
                "api"
            } else {
                "server/domain"
            };
            let destination = temp.path().join(repository);
            for (from, to) in [
                ("schema.sql", "schema.sql"),
                ("schema.sql", "analytics.sql"),
                ("correct.sql", "correct.sql"),
                ("fanout.sql", "fanout.sql"),
                ("where-child.sql", "where-child.sql"),
            ] {
                std::fs::copy(
                    root().join("fixtures/purchase-ledger").join(from),
                    destination.join(to),
                )
                .unwrap();
            }
            let name = if layout == "a" {
                "database.ts"
            } else {
                "database.py"
            };
            std::fs::copy(
                root()
                    .join("fixtures/truth-supplements/T12-database")
                    .join(name),
                destination.join(name),
            )
            .unwrap();
        }
        if (layout == "b" && capability == "api") || capability == "database" {
            let python = std::process::Command::new("python")
                .arg("--version")
                .output()
                .unwrap();
            if !String::from_utf8_lossy(&python.stdout).starts_with("Python 3.12.") {
                let executable = std::env::var("PK_TEST_PYTHON").unwrap_or_else(|_| {
                    let found = std::process::Command::new("uv")
                        .args(["python", "find", "3.12"])
                        .output()
                        .expect("Python 3.12 or uv is required for the Python examples");
                    assert!(found.status.success());
                    String::from_utf8(found.stdout).unwrap().trim().to_string()
                });
                let config_path = temp.path().join("project.json");
                let mut config: Value =
                    serde_json::from_slice(&std::fs::read(&config_path).unwrap()).unwrap();
                config["runtimes"] =
                    json!([{"id":"python312","kind":"python","executable":executable}]);
                std::fs::write(config_path, serde_json::to_vec(&config).unwrap()).unwrap();
            }
        }
        let project = LoadedProject::load(&temp.path().join("project.json")).unwrap();
        let mut database = project_knowledge_core::storage::Database::open_maintenance(
            &temp.path().join("index.sqlite"),
            &project.config.project_id,
            &project.config_hash().unwrap(),
            "now",
        )
        .unwrap();
        project_knowledge_core::index::build_base_index(&project, &mut database, "now").unwrap();
        let manifest = root().join(format!(
            "collectors/examples/frontend-backend-{layout}/{capability}/manifest.json"
        ));
        let parsed = manifest::load_manifest(&manifest).unwrap();
        let validated = manifest::validate_manifest(
            &parsed,
            manifest.parent().unwrap(),
            &project.config.runtimes,
        )
        .unwrap();
        let runtime = staging::resolve_runtime(&parsed, &project).await.unwrap();
        if parsed.dependencies.is_some() {
            let (plan, inputs) = deps::plugin_plan(&validated, &runtime).unwrap();
            deps::prepare_dependencies(&plan, &inputs).await.unwrap();
        }
        Self {
            temp,
            project,
            database,
            manifest,
        }
    }

    async fn collect(&mut self) -> Value {
        let staged = project_knowledge_core::collect::staging::stage_collection(
            &self.project,
            &self.database,
            std::slice::from_ref(&self.manifest),
            None,
            &tokio_util::sync::CancellationToken::new(),
            "now",
        )
        .await
        .unwrap();
        let report = staged.report();
        assert_eq!(report["cases"].as_array().unwrap().len(), 1);
        staged
            .commit(&self.project, &mut self.database, "now")
            .await
            .unwrap();
        report
    }

    fn replace(&self, relative: &str, before: &str, after: &str) {
        let file = self.temp.path().join(relative);
        let content = std::fs::read_to_string(&file).unwrap();
        assert!(content.contains(before), "{relative}: {before}");
        std::fs::write(file, content.replace(before, after)).unwrap();
    }

    fn reindex(&mut self) {
        project_knowledge_core::index::build_base_index(&self.project, &mut self.database, "now")
            .unwrap();
    }

    fn count(&self, sql: &str) -> i64 {
        self.database
            .connection()
            .query_row(sql, [], |r| r.get(0))
            .unwrap()
    }
}

#[tokio::test]
async fn frontend_two_layouts_match_frozen_identities_attributes_relations_and_evidence() {
    use project_knowledge_core::storage::{load_entity_assertions, load_relation_assertions};
    let truth = read_json("fixtures/truth/frontend.json");
    for layout in truth["layouts"].as_array().unwrap() {
        let key = layout["layout"].as_str().unwrap();
        let mut fixture = FrontendFixture::new(key).await;
        let report = fixture.collect().await;
        let project = layout["projectId"].as_str().unwrap();
        let connection = fixture.database.connection();
        let mut ids = std::collections::BTreeMap::new();
        for expected in layout["expectedEntities"].as_array().unwrap() {
            let id = truth_entity_id(
                project,
                serde_json::from_value(expected["kind"].clone()).unwrap(),
                &expected["identity"],
            );
            let assertions = load_entity_assertions(connection, &id, true).unwrap();
            assert!(!assertions.is_empty(), "{key}: {}", expected["key"]);
            for (attribute, value) in expected["attributes"].as_object().unwrap() {
                assert!(
                    assertions
                        .iter()
                        .any(|a| a.attributes.get(attribute) == Some(value)),
                    "{key}: {}.{attribute}",
                    expected["key"]
                );
            }
            let cited: i64 = connection.query_row("SELECT COUNT(*) FROM assertion_evidence ae JOIN entity_assertions ea ON ea.id=ae.assertion_id JOIN evidence e ON e.id=ae.evidence_id WHERE ea.entity_id=?1", [&id], |r| r.get(0)).unwrap();
            assert!(cited > 0);
            ids.insert(expected["key"].as_str().unwrap(), id);
        }
        for expected in layout["expectedRelations"].as_array().unwrap() {
            let id = compute_relation_id(
                project,
                serde_json::from_value(expected["kind"].clone()).unwrap(),
                &ids[expected["from"].as_str().unwrap()],
                &ids[expected["to"].as_str().unwrap()],
                expected["qualifiers"].as_object().unwrap(),
            )
            .unwrap();
            assert!(
                !load_relation_assertions(connection, &id, true)
                    .unwrap()
                    .is_empty(),
                "{key}: {expected}"
            );
        }
        for forbidden in layout["forbiddenRelations"].as_array().unwrap() {
            let target = truth_entity_id(
                project,
                serde_json::from_value(forbidden["toKind"].clone()).unwrap(),
                &forbidden["toIdentity"],
            );
            let id = compute_relation_id(
                project,
                serde_json::from_value(forbidden["kind"].clone()).unwrap(),
                &ids[forbidden["from"].as_str().unwrap()],
                &target,
                forbidden["qualifiers"].as_object().unwrap(),
            )
            .unwrap();
            assert!(
                load_relation_assertions(connection, &id, true)
                    .unwrap()
                    .is_empty()
            );
        }
        assert_eq!(fixture.count("SELECT COUNT(*) FROM coverage c JOIN current_batches cb ON cb.batch_id=c.batch_id WHERE c.capability='frontend' AND c.status='partial'"),1);
        assert!(fixture.count("SELECT COUNT(*) FROM gaps g JOIN current_batches cb ON cb.batch_id=g.batch_id WHERE code='DYNAMIC_TARGET'")>0);
        assert_eq!(fixture.count("SELECT COUNT(*) FROM entities e JOIN entity_assertions ea ON ea.entity_id=e.id JOIN current_batches cb ON cb.batch_id=ea.batch_id WHERE e.kind='graphqlOperation' AND ea.name='PurchaseSummary'"),1);
        let title: (Vec<u8>,i64,i64) = connection.query_row("SELECT sv.content,e.start_byte,e.end_byte FROM entity_assertions ea JOIN entities n ON n.id=ea.entity_id JOIN assertion_evidence ae ON ae.assertion_id=ea.id JOIN evidence e ON e.id=ae.evidence_id JOIN source_versions sv ON sv.source_id=e.source_id AND sv.content_hash=e.content_hash WHERE n.kind='title' LIMIT 1",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap();
        assert!(
            std::str::from_utf8(&title.0[title.1 as usize..title.2 as usize])
                .unwrap()
                .contains("采购台账 🧾")
        );
        if let Some(directory) = std::env::var_os("PK_SCENARIO_EVIDENCE_DIR") {
            let directory = PathBuf::from(directory);
            std::fs::create_dir_all(&directory).unwrap();
            std::fs::write(directory.join(format!("frontend-{key}.json")),serde_json::to_vec_pretty(&json!({"truthHash":read_json("fixtures/truth/manifest.json")["truthHash"],"classification":"synthetic-source-reviewed-smoke","expectedEntities":ids.len(),"expectedRelations":layout["expectedRelations"].as_array().unwrap().len(),"forbiddenHits":0,"report":report})).unwrap()).unwrap();
            let mut stage =
                project_knowledge_core::collect::manifest::load_manifest(&fixture.manifest)
                    .unwrap()
                    .stages
                    .remove(0);
            stage.entity_selectors =
                vec![serde_json::from_value(json!({"id":"all","kinds":[],"scopeIds":[]})).unwrap()];
            let exported = project_knowledge_core::collect::import::export_input(
                connection,
                &fixture.project,
                &stage,
                &fixture
                    .database
                    .current_source_snapshot_id()
                    .unwrap()
                    .unwrap(),
                "evidence",
                "unused",
                "1.0.0",
            )
            .unwrap();
            std::fs::write(
                directory.join(format!("frontend-{key}-snapshot.json")),
                serde_json::to_vec_pretty(&exported).unwrap(),
            )
            .unwrap();
        }
    }
}

#[tokio::test]
async fn frontend_moves_renames_and_ambiguous_dynamic_registrations_remain_conservative() {
    let mut fixture = FrontendFixture::new("a").await;
    fixture.replace("web/routes.ts", "/Other/purchaseLedger", "/采购/供应商");
    fixture.replace("web/routes.ts", "./pages/Ledger", "./screens/Supplier");
    std::fs::create_dir(fixture.temp.path().join("web/screens")).unwrap();
    std::fs::rename(
        fixture.temp.path().join("web/pages/Ledger.tsx"),
        fixture.temp.path().join("web/screens/Supplier.tsx"),
    )
    .unwrap();
    fixture.replace("web/screens/Supplier.tsx", "Ledger", "SupplierBoard");
    fixture.replace("web/screens/Supplier.tsx", "query", "filterSupplier");
    fixture.reindex();
    fixture.collect().await;
    let moved = truth_entity_id(
        "acceptance.a",
        EntityKind::Component,
        &json!({"repositoryId":"front","path":"screens/Supplier.tsx","scopeId":"app.web","lexicalPath":"SupplierBoard"}),
    );
    assert!(
        !project_knowledge_core::storage::load_entity_assertions(
            fixture.database.connection(),
            &moved,
            true
        )
        .unwrap()
        .is_empty()
    );
    assert_eq!(fixture.count("SELECT COUNT(*) FROM entity_assertions ea JOIN current_batches cb ON cb.batch_id=ea.batch_id JOIN entities e ON e.id=ea.entity_id WHERE e.kind='route' AND ea.name='/采购/供应商'"),1);
    // A second title definition must not be arbitrarily selected.
    std::fs::write(
        fixture.temp.path().join("web/ambiguous.ts"),
        "export const messages = { 'purchase.title': '不应默认挑选' };\n",
    )
    .unwrap();
    fixture.reindex();
    fixture.collect().await;
    assert_eq!(fixture.count("SELECT COUNT(*) FROM entity_assertions ea JOIN current_batches cb ON cb.batch_id=ea.batch_id JOIN entities e ON e.id=ea.entity_id WHERE e.kind='title'"),0);
    fixture.replace(
        "web/routes.ts",
        "component: Ledger",
        "component: runtimeComponent",
    );
    fixture.reindex();
    fixture.collect().await;
    assert_eq!(fixture.count("SELECT COUNT(*) FROM relations r JOIN relation_assertions ra ON ra.relation_id=r.id JOIN current_batches cb ON cb.batch_id=ra.batch_id WHERE r.kind='renders'"),0);
}

#[tokio::test]
async fn frontend_shadowed_imports_nested_registrations_and_parse_errors_do_not_confirm_targets() {
    let mut fixture = FrontendFixture::new("b").await;
    fixture.replace(
        "client/ui/network.js",
        "function loadRows()",
        "function loadRows(http)",
    );
    fixture.replace(
        "client/ui/views/Board.tsx",
        "function search()",
        "function search(loadRows)",
    );
    let startup = fixture.temp.path().join("client/ui/startup.js");
    let mut content = std::fs::read_to_string(&startup).unwrap();
    content
        .push_str("\nexport function later(scheduler) { scheduler.register('fake', search); }\n");
    std::fs::write(startup, content).unwrap();
    fixture.reindex();
    fixture.collect().await;
    assert_eq!(fixture.count("SELECT COUNT(*) FROM entities e JOIN entity_assertions ea ON ea.entity_id=e.id JOIN current_batches cb ON cb.batch_id=ea.batch_id WHERE e.kind='request' AND ea.name='loadRows' AND json_extract(ea.attributes_json,'$.serviceId') IS NOT NULL"),0);
    assert_eq!(fixture.count("SELECT COUNT(*) FROM entities e JOIN entity_assertions ea ON ea.entity_id=e.id JOIN current_batches cb ON cb.batch_id=ea.batch_id WHERE e.kind='entrypoint' AND ea.name='fake'"),0);
    let shadowed = truth_entity_id(
        "acceptance.b",
        EntityKind::Operation,
        &json!({"repositoryId":"front","path":"views/Board.tsx","scopeId":"app.web","lexicalPath":"Board.search","normalizedSignature":"(loadRows)"}),
    );
    let false_calls:i64=fixture.database.connection().query_row("SELECT COUNT(*) FROM relations r JOIN relation_assertions ra ON ra.relation_id=r.id JOIN current_batches cb ON cb.batch_id=ra.batch_id WHERE r.kind='calls' AND r.from_id=?1",[shadowed],|r|r.get(0)).unwrap();
    assert_eq!(false_calls, 0);
    fixture.replace(
        "client/ui/navigation.js",
        "export const router =",
        "export const broken = ( ;\nexport const router =",
    );
    fixture.reindex();
    fixture.collect().await;
    assert_eq!(fixture.count("SELECT COUNT(*) FROM entities e JOIN entity_assertions ea ON ea.entity_id=e.id JOIN current_batches cb ON cb.batch_id=ea.batch_id WHERE e.kind='route'"),0);
    assert!(fixture.count("SELECT COUNT(*) FROM gaps g JOIN current_batches cb ON cb.batch_id=g.batch_id WHERE g.code='UNSUPPORTED_SYNTAX'")>0);
}

#[tokio::test]
async fn frontend_literal_title_alias_array_and_proxy_are_preserved_with_dynamic_gaps() {
    let mut fixture = FrontendFixture::new("a").await;
    fixture.replace(
        "web/routes.ts",
        "titleKey: 'purchase.title'",
        "title: '直接文案 🧾'",
    );
    fixture.replace(
        "web/routes.ts",
        "alias: '/purchase'",
        "alias: ['/purchase', '/alias']",
    );
    fixture.replace("web/transport.ts","serviceId: 'erp'","serviceId: 'erp', proxy: { prefix: '/erp', target: 'https://erp.example.test', rewrite: '/api' }");
    fixture.reindex();
    fixture.collect().await;
    assert_eq!(fixture.count("SELECT COUNT(*) FROM entities e JOIN entity_assertions ea ON ea.entity_id=e.id JOIN current_batches cb ON cb.batch_id=ea.batch_id WHERE e.kind='title' AND ea.name='直接文案 🧾'"),1);
    assert_eq!(fixture.count("SELECT COUNT(*) FROM entities e JOIN entity_assertions ea ON ea.entity_id=e.id JOIN current_batches cb ON cb.batch_id=ea.batch_id WHERE e.kind='route' AND json_extract(ea.attributes_json,'$.alias[1]')='/alias'"),1);
    assert_eq!(fixture.count("SELECT COUNT(*) FROM entities e JOIN entity_assertions ea ON ea.entity_id=e.id JOIN current_batches cb ON cb.batch_id=ea.batch_id WHERE e.kind='request' AND ea.name='fetchLedger' AND json_extract(ea.attributes_json,'$.proxy.target')='https://erp.example.test' AND json_extract(ea.attributes_json,'$.bindingComplete')=1"),1);
    fixture.replace(
        "web/transport.ts",
        "target: 'https://erp.example.test'",
        "target: runtimeTarget",
    );
    fixture.replace(
        "web/routes.ts",
        "title: '直接文案 🧾'",
        "title: runtimeTitle",
    );
    fixture.reindex();
    fixture.collect().await;
    assert_eq!(fixture.count("SELECT COUNT(*) FROM entities e JOIN entity_assertions ea ON ea.entity_id=e.id JOIN current_batches cb ON cb.batch_id=ea.batch_id WHERE e.kind='title'"),0);
    assert_eq!(fixture.count("SELECT COUNT(*) FROM entities e JOIN entity_assertions ea ON ea.entity_id=e.id JOIN current_batches cb ON cb.batch_id=ea.batch_id WHERE e.kind='request' AND ea.name='fetchLedger' AND json_extract(ea.attributes_json,'$.bindingComplete')=1"),0);
    fixture.replace(
        "web/routes.ts",
        "export const routes =",
        "export const unsupportedRegistry =",
    );
    fixture.reindex();
    fixture.collect().await;
    assert_eq!(fixture.count("SELECT COUNT(*) FROM coverage c JOIN current_batches cb ON cb.batch_id=c.batch_id WHERE c.capability='routes' AND c.status='not_configured' AND cb.replacement_key LIKE '(example.%'"),1);
}

#[tokio::test]
async fn backend_api_two_languages_bind_registrations_and_reject_decoys_and_dynamic_targets() {
    use project_knowledge_core::storage::{load_entity_assertions, load_relation_assertions};
    let truth = read_json("fixtures/truth-supplements/T12-api.json");
    for layout in truth["layouts"].as_array().unwrap() {
        let key = layout["layout"].as_str().unwrap();
        let mut fixture = FrontendFixture::new_for(key, "api").await;
        let repo = if key == "a" { "api" } else { "server/domain" };
        let handler = layout["handlerFile"].as_str().unwrap();
        std::fs::create_dir(fixture.temp.path().join(repo).join("decoy")).unwrap();
        std::fs::copy(
            fixture.temp.path().join(repo).join(handler),
            fixture.temp.path().join(repo).join("decoy").join(handler),
        )
        .unwrap();
        fixture.reindex();
        let report = fixture.collect().await;
        let project = format!("acceptance.{key}");
        let service = truth_entity_id(
            &project,
            EntityKind::Service,
            &json!({"scopeId":"svc.erp","configuredId":"erp"}),
        );
        let endpoint = truth_entity_id(
            &project,
            EntityKind::HttpEndpoint,
            &json!({"serviceId":"erp","method":"GET","registeredPath":"/purchase/ledger"}),
        );
        let field = truth_entity_id(
            &project,
            EntityKind::GraphqlField,
            &json!({"serviceId":"erp","schemaType":"Query","fieldName":"purchaseSummary"}),
        );
        let operation = truth_entity_id(
            &project,
            EntityKind::Operation,
            &json!({"repositoryId":"back","path":handler,"scopeId":"svc.erp","lexicalPath":"ledger","normalizedSignature":"()"}),
        );
        let decoy = truth_entity_id(
            &project,
            EntityKind::Operation,
            &json!({"repositoryId":"back","path":format!("decoy/{handler}"),"scopeId":"svc.erp","lexicalPath":"ledger","normalizedSignature":"()"}),
        );
        for id in [&service, &endpoint, &field, &operation] {
            assert!(
                !load_entity_assertions(fixture.database.connection(), id, true)
                    .unwrap()
                    .is_empty(),
                "{key}: {id}"
            );
        }
        for (kind, from, to) in [
            (
                project_knowledge_core::contract::RelationKind::Serves,
                &service,
                &endpoint,
            ),
            (
                project_knowledge_core::contract::RelationKind::Handles,
                &endpoint,
                &operation,
            ),
            (
                project_knowledge_core::contract::RelationKind::Resolves,
                &field,
                &operation,
            ),
        ] {
            let relation =
                compute_relation_id(&project, kind, from, to, &serde_json::Map::new()).unwrap();
            assert!(
                !load_relation_assertions(fixture.database.connection(), &relation, true)
                    .unwrap()
                    .is_empty()
            );
        }
        let forbidden = compute_relation_id(
            &project,
            project_knowledge_core::contract::RelationKind::Handles,
            &endpoint,
            &decoy,
            &serde_json::Map::new(),
        )
        .unwrap();
        assert!(
            load_relation_assertions(fixture.database.connection(), &forbidden, true)
                .unwrap()
                .is_empty()
        );
        if let Some(directory) = std::env::var_os("PK_SCENARIO_EVIDENCE_DIR") {
            std::fs::create_dir_all(&directory).unwrap();
            let directory = PathBuf::from(directory);
            std::fs::write(directory.join(format!("api-{key}.json")),serde_json::to_vec_pretty(&json!({"report":report,"expectedEntities":4,"expectedRelations":3,"forbiddenHits":0,"classification":"source-reviewed synthetic smoke"})).unwrap()).unwrap();
            let mut stage =
                project_knowledge_core::collect::manifest::load_manifest(&fixture.manifest)
                    .unwrap()
                    .stages
                    .remove(0);
            stage.entity_selectors =
                vec![serde_json::from_value(json!({"id":"all","kinds":[],"scopeIds":[]})).unwrap()];
            let exported = project_knowledge_core::collect::import::export_input(
                fixture.database.connection(),
                &fixture.project,
                &stage,
                &fixture
                    .database
                    .current_source_snapshot_id()
                    .unwrap()
                    .unwrap(),
                "evidence",
                "unused",
                "1.0.0",
            )
            .unwrap();
            std::fs::write(
                directory.join(format!("api-{key}-snapshot.json")),
                serde_json::to_vec_pretty(&exported).unwrap(),
            )
            .unwrap();
        }
        let registration = format!("{repo}/{}", layout["registrationFile"].as_str().unwrap());
        if key == "a" {
            fixture.replace(
                &registration,
                "http.get('/purchase/ledger', ledger)",
                "http.get('/purchase/ledger', handlers[target])",
            );
        } else {
            fixture.replace(
                &registration,
                "app.get('/purchase/ledger')(ledger)",
                "app.get('/purchase/ledger')(handlers[target])",
            );
        }
        fixture.reindex();
        fixture.collect().await;
        assert_eq!(fixture.count("SELECT COUNT(*) FROM relations r JOIN relation_assertions ra ON ra.relation_id=r.id JOIN current_batches cb ON cb.batch_id=ra.batch_id WHERE r.kind='handles'"),0);
        fixture.replace(&registration, "'/purchase/ledger'", "runtimePath");
        fixture.reindex();
        fixture.collect().await;
        assert_eq!(fixture.count("SELECT COUNT(*) FROM entities e JOIN entity_assertions ea ON ea.entity_id=e.id JOIN current_batches cb ON cb.batch_id=ea.batch_id WHERE e.kind='httpEndpoint'"),0);
        assert!(fixture.count("SELECT COUNT(*) FROM gaps g JOIN current_batches cb ON cb.batch_id=g.batch_id WHERE g.code='DYNAMIC_TARGET'")>0);
    }
}

#[tokio::test]
async fn backend_api_and_frontend_share_service_scoped_graphql_fields_without_scope_overwrite() {
    for key in ["a", "b"] {
        let mut fixture = FrontendFixture::new_for(key, "api").await;
        let frontend = root().join(format!(
            "collectors/examples/frontend-backend-{key}/frontend/manifest.json"
        ));
        let manifest = project_knowledge_core::collect::manifest::load_manifest(&frontend).unwrap();
        let validated = project_knowledge_core::collect::manifest::validate_manifest(
            &manifest,
            frontend.parent().unwrap(),
            &fixture.project.config.runtimes,
        )
        .unwrap();
        let runtime =
            project_knowledge_core::collect::staging::resolve_runtime(&manifest, &fixture.project)
                .await
                .unwrap();
        let (plan, inputs) =
            project_knowledge_core::collect::deps::plugin_plan(&validated, &runtime).unwrap();
        project_knowledge_core::collect::deps::prepare_dependencies(&plan, &inputs)
            .await
            .unwrap();
        let staged = project_knowledge_core::collect::staging::stage_collection(
            &fixture.project,
            &fixture.database,
            &[fixture.manifest.clone(), frontend],
            None,
            &tokio_util::sync::CancellationToken::new(),
            "now",
        )
        .await
        .unwrap();
        assert_eq!(staged.report()["cases"].as_array().unwrap().len(), 2);
        staged
            .commit(&fixture.project, &mut fixture.database, "now")
            .await
            .unwrap();
        let id = truth_entity_id(
            &format!("acceptance.{key}"),
            EntityKind::GraphqlField,
            &json!({"serviceId":"erp","schemaType":"Query","fieldName":"purchaseSummary"}),
        );
        let assertions = project_knowledge_core::storage::load_entity_assertions(
            fixture.database.connection(),
            &id,
            true,
        )
        .unwrap();
        assert_eq!(assertions.len(), 2);
        assert!(assertions.iter().all(|a| a.scope_id == "svc.erp"));
    }
}

#[tokio::test]
async fn backend_database_two_asts_preserve_datasources_ddl_keys_query_joins_and_original_ranges() {
    use project_knowledge_core::storage::{load_entity_assertions, load_relation_assertions};
    let truth = read_json("fixtures/truth-supplements/T12-database/review.json");
    for (path, hash) in truth["fileHashes"].as_object().unwrap() {
        assert_eq!(
            content_hash(&std::fs::read(root().join(path)).unwrap()),
            hash.as_str().unwrap()
        );
    }
    for key in ["a", "b"] {
        let mut fixture = FrontendFixture::new_for(key, "database").await;
        let report = fixture.collect().await;
        let project = format!("acceptance.{key}");
        let connection = fixture.database.connection();
        let mut expected_entities = 0;
        for ds in ["ds.erp", "ds.analytics"] {
            let source_id = truth_entity_id(
                &project,
                EntityKind::DataSource,
                &json!({"scopeId":"svc.erp","configuredId":ds}),
            );
            assert_eq!(
                load_entity_assertions(connection, &source_id, true)
                    .unwrap()
                    .len(),
                1
            );
            expected_entities += 1;
            for (table, columns) in truth["tables"].as_object().unwrap() {
                let table_id = truth_entity_id(
                    &project,
                    EntityKind::DataTable,
                    &json!({"dataSourceId":ds,"table":table}),
                );
                assert_eq!(
                    load_entity_assertions(connection, &table_id, true)
                        .unwrap()
                        .len(),
                    1
                );
                expected_entities += 1;
                for column in columns.as_array().unwrap() {
                    let id = truth_entity_id(
                        &project,
                        EntityKind::DataColumn,
                        &json!({"dataSourceId":ds,"table":table,"column":column}),
                    );
                    let assertions = load_entity_assertions(connection, &id, true).unwrap();
                    assert_eq!(assertions.len(), 1);
                    assert_eq!(assertions[0].attributes["nullable"], "unknown");
                    expected_entities += 1;
                }
                for (kind, names) in [
                    ("primary", &truth["primaryKeys"][table]),
                    ("foreign", &truth["foreignKeys"][table]),
                ] {
                    if names.is_null() {
                        continue;
                    }
                    let id = truth_entity_id(
                        &project,
                        EntityKind::DataKey,
                        &json!({"dataSourceId":ds,"table":table,"keyColumns":names}),
                    );
                    let assertions = load_entity_assertions(connection, &id, true).unwrap();
                    assert_eq!(assertions.len(), 1);
                    assert_eq!(assertions[0].attributes["keyType"], kind);
                    assert_eq!(assertions[0].attributes["orderedColumns"], *names);
                    if kind == "foreign" {
                        assert_eq!(
                            assertions[0].attributes["keyPairs"]
                                .as_array()
                                .unwrap()
                                .len(),
                            3
                        );
                        let target = truth_entity_id(
                            &project,
                            EntityKind::DataTable,
                            &json!({"dataSourceId":ds,"table":"purchase_detail"}),
                        );
                        let relation = compute_relation_id(
                            &project,
                            project_knowledge_core::contract::RelationKind::MapsTo,
                            &id,
                            &target,
                            &serde_json::Map::new(),
                        )
                        .unwrap();
                        assert_eq!(
                            load_relation_assertions(connection, &relation, true)
                                .unwrap()
                                .len(),
                            1
                        );
                    }
                    expected_entities += 1;
                }
            }
        }
        assert_eq!(expected_entities, 90);
        let query = truth_entity_id(
            &project,
            EntityKind::QueryImplementation,
            &json!({"scopeId":"svc.erp","semanticKey":"back:correct.sql:0"}),
        );
        let assertions = load_entity_assertions(connection, &query, true).unwrap();
        assert_eq!(assertions.len(), 1);
        assert_eq!(
            assertions[0].attributes["joins"].as_array().unwrap().len(),
            4
        );
        assert_eq!(
            assertions[0].attributes["grainColumnIds"]["status"],
            "unknown"
        );
        for expected in truth["correctQueryJoins"].as_array().unwrap() {
            let mut pairs = Vec::new();
            for column in expected["columns"].as_array().unwrap() {
                let left = truth_entity_id(
                    &project,
                    EntityKind::DataColumn,
                    &json!({"dataSourceId":"ds.erp","table":expected["leftTable"],"column":column}),
                );
                let right = truth_entity_id(
                    &project,
                    EntityKind::DataColumn,
                    &json!({"dataSourceId":"ds.erp","table":expected["rightTable"],"column":column}),
                );
                pairs.push(json!({"leftColumnId":left,"rightColumnId":right,"operator":"equals"}));
            }
            assert!(
                assertions[0].attributes["joins"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|join| join["joinType"] == "left"
                        && join["cardinality"] == "unknown"
                        && join["keyPairs"] == json!(pairs))
            );
        }
        assert_eq!(fixture.count("SELECT COUNT(*) FROM entities e JOIN entity_assertions ea ON ea.entity_id=e.id JOIN current_batches cb ON cb.batch_id=ea.batch_id WHERE e.kind='queryImplementation'"), 6);
        let language = if key == "a" { "ts" } else { "py" };
        let wrong_instance = truth_entity_id(
            &project,
            EntityKind::QueryImplementation,
            &json!({"scopeId":"svc.erp","semanticKey":format!("back:database.{language}:wrongInstance:query:0:0")}),
        );
        for (ds, expected) in [("ds.analytics", 1), ("ds.erp", 0)] {
            let table = truth_entity_id(
                &project,
                EntityKind::DataTable,
                &json!({"dataSourceId":ds,"table":"purchase_detail"}),
            );
            let relation = compute_relation_id(
                &project,
                project_knowledge_core::contract::RelationKind::Reads,
                &wrong_instance,
                &table,
                &serde_json::Map::new(),
            )
            .unwrap();
            assert_eq!(
                load_relation_assertions(connection, &relation, true)
                    .unwrap()
                    .len(),
                expected
            );
        }
        for code in truth["requiredGaps"].as_array().unwrap() {
            let count:i64=connection.query_row("SELECT COUNT(*) FROM gaps g JOIN current_batches cb ON cb.batch_id=g.batch_id WHERE g.code=?1",[code.as_str().unwrap()],|row|row.get(0)).unwrap();
            assert!(count > 0, "{key}: {code}");
        }
        let literal_query = truth_entity_id(
            &project,
            EntityKind::QueryImplementation,
            &json!({"scopeId":"svc.erp","semanticKey":format!("back:database.{language}:boundLedger:query:0:0")}),
        );
        let literal = load_entity_assertions(connection, &literal_query, true).unwrap();
        assert!(
            literal[0].attributes["statement"]["text"]
                .as_str()
                .unwrap()
                .contains("\\n")
        );
        assert_eq!(fixture.count("SELECT COUNT(*) FROM coverage c JOIN current_batches cb ON cb.batch_id=c.batch_id WHERE c.capability='database' AND c.status='partial'"),1);
        if let Some(directory) = std::env::var_os("PK_SCENARIO_EVIDENCE_DIR") {
            let directory = PathBuf::from(directory);
            std::fs::create_dir_all(&directory).unwrap();
            let mut stage =
                project_knowledge_core::collect::manifest::load_manifest(&fixture.manifest)
                    .unwrap()
                    .stages
                    .remove(0);
            stage.entity_selectors =
                vec![serde_json::from_value(json!({"id":"all","kinds":[],"scopeIds":[]})).unwrap()];
            let snapshot = project_knowledge_core::collect::import::export_input(
                connection,
                &fixture.project,
                &stage,
                &fixture
                    .database
                    .current_source_snapshot_id()
                    .unwrap()
                    .unwrap(),
                "evidence",
                "unused",
                "1.0.0",
            )
            .unwrap();
            std::fs::write(
                directory.join(format!("database-{key}-snapshot.json")),
                serde_json::to_vec_pretty(&snapshot).unwrap(),
            )
            .unwrap();
            std::fs::write(directory.join(format!("database-{key}.json")),serde_json::to_vec_pretty(&json!({"report":report,"expectedDdlEntities":90,"expectedCorrectJoins":4,"forbiddenCrossDatasourceReads":0,"classification":"source-reviewed synthetic smoke"})).unwrap()).unwrap();
        }
    }
}

#[tokio::test]
async fn backend_database_incomplete_keys_dynamic_instances_and_invalid_sql_never_confirm_mappings()
{
    for key in ["a", "b"] {
        let mut fixture = FrontendFixture::new_for(key, "database").await;
        let repo = if key == "a" { "api" } else { "server/domain" };
        let models = if key == "a" { "models.ts" } else { "models.py" };
        fixture.replace(
            &format!("{repo}/{models}"),
            "erp.model('purchase_detail', ['tenant_id', 'purchase_no', 'line_no'])",
            "erp.model('purchase_detail', ['purchase_no', 'line_no'])",
        );
        fixture.replace(
            &format!("{repo}/correct.sql"),
            "s.tenant_id=p.tenant_id AND ",
            "",
        );
        std::fs::write(
            fixture.temp.path().join(repo).join("unbound.sql"),
            "SELECT * FROM purchase_detail",
        )
        .unwrap();
        fixture.reindex();
        fixture.collect().await;
        assert!(fixture.count("SELECT COUNT(*) FROM gaps g JOIN current_batches cb ON cb.batch_id=g.batch_id WHERE code='INCOMPLETE_MODEL_KEY'")>0);
        assert!(fixture.count("SELECT COUNT(*) FROM gaps g JOIN current_batches cb ON cb.batch_id=g.batch_id WHERE code='INCOMPLETE_JOIN_KEY'")>0);
        assert!(fixture.count("SELECT COUNT(*) FROM gaps g JOIN current_batches cb ON cb.batch_id=g.batch_id WHERE code='UNBOUND_DATASOURCE'")>0);
        assert_eq!(fixture.count("SELECT COUNT(*) FROM entities e JOIN entity_assertions ea ON ea.entity_id=e.id JOIN current_batches cb ON cb.batch_id=ea.batch_id WHERE e.kind='symbol' AND ea.name='Purchase' AND cb.replacement_key LIKE '(example.%'"),0);
        fixture.replace(
            &format!("{repo}/{models}"),
            "orm.connect('ds.erp')",
            "orm.connect(runtimeSource)",
        );
        fixture.replace(
            &format!("{repo}/correct.sql"),
            "SELECT p.tenant_id,p.purchase_no,p.line_no,s.name,",
            "SELECT ( FROM ; SELECT p.tenant_id,p.purchase_no,p.line_no,s.name,",
        );
        fixture.reindex();
        fixture.collect().await;
        assert!(fixture.count("SELECT COUNT(*) FROM gaps g JOIN current_batches cb ON cb.batch_id=g.batch_id WHERE code='DYNAMIC_INSTANCE'")>0);
        assert!(fixture.count("SELECT COUNT(*) FROM gaps g JOIN current_batches cb ON cb.batch_id=g.batch_id WHERE code='UNSUPPORTED_SQL'")>0);
        let id = truth_entity_id(
            &format!("acceptance.{key}"),
            EntityKind::QueryImplementation,
            &json!({"scopeId":"svc.erp","semanticKey":"back:correct.sql:0"}),
        );
        assert!(
            project_knowledge_core::storage::load_entity_assertions(
                fixture.database.connection(),
                &id,
                true
            )
            .unwrap()
            .is_empty()
        );
    }
}
