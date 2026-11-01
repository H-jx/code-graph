//! T15 综合验收：独立构造图压力向量，不作为业务正确性真值。
use super::*;
use project_knowledge_core::contract::query::{CommonFilters, FreshnessPolicy, ResponseStatus};
use std::sync::{Arc, Mutex};

fn fixture() -> (tempfile::TempDir, LoadedProject, Database, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let (config, web) = build_fixture(temp.path());
    let source = (0..30)
        .map(|n| format!("export function budgetPurchase{n}(value: string) {{ return value; }}\n"))
        .collect::<String>();
    write_file(&web.join("src/采购😀.ts"), &source);
    let project = LoadedProject::load(&config).unwrap();
    let path = config.parent().unwrap().join("index.sqlite");
    let mut database = Database::open_maintenance(&path, "erp", "cfg", "now").unwrap();
    build_base_index(&project, &mut database, "now").unwrap();
    (temp, project, database, path)
}
fn run(
    database: &mut Database,
    project: &LoadedProject,
    value: Value,
) -> project_knowledge_core::contract::query::QueryResponse<Value> {
    execute(
        &mut QueryContext {
            database,
            project: Some(project),
            now: "now".into(),
        },
        &serde_json::from_value(value).unwrap(),
    )
    .unwrap()
}
fn budget() -> Value {
    json!({"maxResponseChars":200000,"maxEntities":100,"maxRelations":100,"maxVisited":2000,"timeoutMs":30000})
}
fn save(name: &str, value: Value) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/evidence/T15");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(format!("{name}.json")),
        serde_json::to_string_pretty(&value).unwrap(),
    )
    .unwrap();
}
fn root(database: &Database, name: &str) -> Value {
    let id: String = database
        .connection()
        .query_row(
            "SELECT entity_id FROM entity_assertions WHERE name=?1 ORDER BY id LIMIT 1",
            [name],
            |r| r.get(0),
        )
        .unwrap();
    json!({"snapshot":project_knowledge_core::query::snapshot_ref(database).unwrap(),"entityId":id})
}
fn logical(database: &Database) -> Value {
    let connection = database.connection();
    let mut stmt=connection.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE '%_fts%' ORDER BY name").unwrap();
    let names = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let mut all = serde_json::Map::new();
    for name in names {
        let mut stmt = connection
            .prepare(&format!("SELECT * FROM \"{}\"", name.replace('"', "\"\"")))
            .unwrap();
        let columns = stmt.column_count();
        let mut rows = stmt
            .query_map([], |r| {
                (0..columns)
                    .map(|i| Ok(format!("{:?}", r.get_ref(i)?)))
                    .collect::<rusqlite::Result<Vec<_>>>()
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        rows.sort();
        all.insert(name, json!(rows));
    }
    Value::Object(all)
}

#[test]
fn query_search_keyset_fields_unicode_generation_and_readonly() {
    let (_temp, project, database, path) = fixture();
    let before = logical(&database);
    let before_bytes = std::fs::read(&path).unwrap();
    let mut database = Database::open_readonly(&path).unwrap();
    let trace = Arc::new(Mutex::new(Vec::<String>::new()));
    let sink = trace.clone();
    database
        .observe_sql(move |sql| sink.lock().unwrap().push(sql.into()))
        .unwrap();
    let request = json!({"operation":"search","query":"budgetPurchase","fields":["symbol"],"pageSize":100,"budget":budget()});
    let full = run(&mut database, &project, request.clone());
    assert_eq!(full.data["results"].as_array().unwrap().len(), 30);
    assert_eq!(full.status, ResponseStatus::Complete);
    let expected = full.data["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| {
            (
                v["entityRef"]["entityId"].as_str().unwrap().to_string(),
                v["assertions"][0]["assertionId"]
                    .as_str()
                    .unwrap()
                    .to_string(),
            )
        })
        .collect::<Vec<_>>();
    let mut page = request.clone();
    page["pageSize"] = json!(3);
    let mut actual = Vec::new();
    let mut first_token = None;
    for _ in 0..15 {
        let response = run(&mut database, &project, page.clone());
        for v in response.data["results"].as_array().unwrap() {
            actual.push((
                v["entityRef"]["entityId"].as_str().unwrap().to_string(),
                v["assertions"][0]["assertionId"]
                    .as_str()
                    .unwrap()
                    .to_string(),
            ));
        }
        if let Some(token) = response.continuation {
            first_token.get_or_insert(token.clone());
            page["continuation"] = json!(token);
        } else {
            break;
        }
    }
    assert_eq!(actual, expected);
    let mut compact = request.clone();
    compact["budget"]["maxResponseChars"] = json!(7000);
    let mut compact_order = Vec::new();
    for _ in 0..40 {
        let response = run(&mut database, &project, compact.clone());
        assert!(serde_json::to_string(&response).unwrap().chars().count() <= 7000);
        for v in response.data["results"].as_array().unwrap() {
            compact_order.push((
                v["entityRef"]["entityId"].as_str().unwrap().to_string(),
                v["assertions"][0]["assertionId"]
                    .as_str()
                    .unwrap()
                    .to_string(),
            ));
        }
        if let Some(token) = response.continuation {
            compact["continuation"] = json!(token);
        } else {
            break;
        }
    }
    assert_eq!(
        compact_order, expected,
        "character trimming must advance the key only to the last returned row"
    );
    assert_eq!(
        actual
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        30
    );
    let comment = run(
        &mut database,
        &project,
        json!({"operation":"search","query":"供应商筛选","fields":["comment"],"budget":budget()}),
    );
    assert!(!comment.data["results"].as_array().unwrap().is_empty());
    let restricted = run(
        &mut database,
        &project,
        json!({"operation":"search","query":"budgetPurchase","filters":{"repositoryIds":["repo.api"]},"budget":budget()}),
    );
    assert!(restricted.data["results"].as_array().unwrap().is_empty());
    let identity: String = database
        .connection()
        .query_row(
            "SELECT identity_json FROM entities WHERE id=?1",
            [expected[0].0.as_str()],
            |r| r.get(0),
        )
        .unwrap();
    let exact = run(
        &mut database,
        &project,
        json!({"operation":"search","query":identity,"budget":budget()}),
    );
    assert_eq!(
        exact.data["results"][0]["entityRef"]["entityId"],
        expected[0].0
    );
    let mut invalid = page.clone();
    invalid["query"] = json!("different");
    invalid["continuation"] = json!(first_token.clone());
    let error = execute(
        &mut QueryContext {
            database: &mut database,
            project: Some(&project),
            now: "now".into(),
        },
        &serde_json::from_value(invalid).unwrap(),
    )
    .unwrap_err();
    assert_eq!(error.code, codes::INVALID_REQUEST);
    let no_match = run(
        &mut database,
        &project,
        json!({"operation":"search","query":"NoSuchMethod","budget":budget()}),
    );
    assert_eq!(no_match.status, ResponseStatus::Complete);
    assert!(no_match.data["results"].as_array().unwrap().is_empty());
    let code_root = root(&database, "budgetPurchase1");
    let global_map = run(
        &mut database,
        &project,
        json!({"operation":"codeMap","budget":budget()}),
    );
    assert!(global_map.data["files"].as_array().unwrap().len() >= 3);
    assert!(global_map.data["symbols"].as_array().unwrap().len() >= 30);
    let symbols = run(
        &mut database,
        &project,
        json!({"operation":"codeMap","roots":[code_root],"budget":budget()}),
    );
    assert!(symbols.data["symbols"].as_array().unwrap().len() >= 30);
    assert!(
        symbols.data["symbols"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["signature"].as_str().is_some_and(|s| s.contains("value")))
    );
    let uncovered = run(
        &mut database,
        &project,
        json!({"operation":"codeMap","query":"src/schema.sql","budget":budget()}),
    );
    assert_eq!(uncovered.status, ResponseStatus::Partial);
    assert!(
        !uncovered.data["uncoveredLanguages"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let small = run(
        &mut database,
        &project,
        json!({"operation":"search","query":"采购😀","budget":{"maxResponseChars":3000,"timeoutMs":30000}}),
    );
    let serialized = serde_json::to_string(&small).unwrap();
    assert!(serialized.chars().count() <= 3000);
    assert_eq!(
        small.budget_used.response_chars,
        serialized.chars().count() as u64
    );
    assert_eq!(before, logical(&database));
    assert_eq!(before_bytes, std::fs::read(&path).unwrap());
    let sql = trace.lock().unwrap().clone();
    let actual_search = sql.iter().find(|s| s.starts_with("WITH hits")).unwrap();
    let mut explain = database
        .connection()
        .prepare(&format!("EXPLAIN QUERY PLAN {actual_search}"))
        .unwrap();
    let actual_plan = explain
        .query_map(
            rusqlite::params!["\"budgetPurchase\"", "budgetPurchase", false, 2001],
            |r| r.get::<_, String>(3),
        )
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(
        actual_plan
            .iter()
            .any(|p| p.contains("idx_search_documents_lookup"))
    );
    drop(explain);
    assert!(sql.iter().any(|s| s.contains("BEGIN DEFERRED")));
    assert!(!sql.iter().any(|s| {
        matches!(
            s.split_whitespace()
                .next()
                .unwrap_or("")
                .to_uppercase()
                .as_str(),
            "INSERT" | "UPDATE" | "DELETE" | "CREATE" | "ALTER" | "REPLACE"
        )
    }));
    let before_hash = content_hash(serde_json::to_string(&before).unwrap().as_bytes());
    save(
        "search",
        json!({"full":full,"codeMap":symbols,"small":small,"beforeHash":before_hash,"afterHash":content_hash(serde_json::to_string(&logical(&database)).unwrap().as_bytes()),"mainFileUnchanged":true,"sql":sql,"actualSearchPlan":actual_plan,"orderedCount":actual.len()}),
    );
    drop(database);
    let mut writer = Database::open_maintenance(&path, "erp", "cfg", "later").unwrap();
    writer.bump_generation().unwrap();
    let mut expired = request;
    expired["continuation"] = json!(first_token);
    expired["pageSize"] = json!(3);
    let error = execute(
        &mut QueryContext {
            database: &mut writer,
            project: Some(&project),
            now: "later".into(),
        },
        &serde_json::from_value(expired).unwrap(),
    )
    .unwrap_err();
    assert_eq!(error.code, codes::SNAPSHOT_EXPIRED);
}

fn graph(database: &Database) -> Value {
    let connection = database.connection();
    let root_value = root(database, "budgetPurchase0");
    let root_id = root_value["entityId"].as_str().unwrap();
    let batch: String = connection
        .query_row(
            "SELECT ea.batch_id FROM entity_assertions ea WHERE ea.entity_id=?1",
            [root_id],
            |r| r.get(0),
        )
        .unwrap();
    let evidence:String=connection.query_row("SELECT evidence_id FROM assertion_evidence WHERE assertion_id IN (SELECT id FROM entity_assertions WHERE entity_id=?1) LIMIT 1",[root_id],|r|r.get(0)).unwrap();
    for n in 1..30 {
        let target = root(database, &format!("budgetPurchase{n}"));
        let target = target["entityId"].as_str().unwrap();
        for (from, to) in [(root_id, target), (target, root_id)] {
            let qualifiers = json!({"guard":"value != null"})
                .as_object()
                .unwrap()
                .clone();
            let id =
                compute_relation_id("erp", RelationKind::Calls, from, to, &qualifiers).unwrap();
            let assertion = format!("a_stress_{}", id);
            let relation = Relation {
                id: id.clone(),
                kind: RelationKind::Calls,
                from_id: from.into(),
                to_id: to.into(),
                qualifiers,
                basis: Basis::Observed,
                origin: Origin::Static,
                resolution: Resolution::Confirmed,
                evidence_refs: vec![evidence.clone()],
                extensions: Default::default(),
            };
            project_knowledge_core::storage::upsert_relation(connection, &relation).unwrap();
            project_knowledge_core::storage::insert_relation_assertion(
                connection,
                &project_knowledge_core::storage::RelationAssertionRow {
                    assertion_id: &assertion,
                    relation_id: &id,
                    batch_id: &batch,
                    basis: Basis::Observed,
                    origin: Origin::Static,
                    resolution: Resolution::Confirmed,
                },
                &content_hash(id.as_bytes()),
                "stress",
            )
            .unwrap();
            project_knowledge_core::storage::link_assertion_evidence(
                connection, &assertion, &evidence,
            )
            .unwrap();
        }
    }
    root_value
}

#[test]
fn query_read_source_rejects_forgery_and_preserves_expired_versions_explicitly() {
    let (_temp, project, mut database, _) = fixture();
    let result = run(
        &mut database,
        &project,
        json!({"operation":"search","query":"budgetPurchase1","fields":["symbol"],"budget":budget()}),
    );
    let reference = serde_json::to_value(&result.source_references[0]).unwrap();
    let read = run(
        &mut database,
        &project,
        json!({"operation":"readSource","references":[reference],"maxResponseChars":8000}),
    );
    assert_eq!(read.data["reads"][0]["status"], "current");
    let mut forged = reference.clone();
    forged["endByte"] = json!(forged["endByte"].as_u64().unwrap() + 1);
    let error = execute(
        &mut QueryContext {
            database: &mut database,
            project: Some(&project),
            now: "now".into(),
        },
        &serde_json::from_value(
            json!({"operation":"readSource","references":[forged],"maxResponseChars":8000}),
        )
        .unwrap(),
    )
    .unwrap_err();
    assert_eq!(error.code, codes::EVIDENCE_INVALID);
    std::fs::remove_file(project.repositories["repo.web"].join("src/采购😀.ts")).unwrap();
    let expired = run(
        &mut database,
        &project,
        json!({"operation":"readSource","references":[reference],"maxResponseChars":8000}),
    );
    assert_eq!(expired.status, ResponseStatus::Partial);
    assert_eq!(expired.data["reads"][0]["status"], "missing");
    assert_eq!(expired.data["reads"][0]["snippet"], "");
    let historical = run(
        &mut database,
        &project,
        json!({"operation":"readSource","references":[reference],"maxResponseChars":8000,"filters":{"freshnessPolicy":"allowStale"}}),
    );
    assert_eq!(historical.status, ResponseStatus::Partial);
    assert!(
        !historical.data["reads"][0]["snippet"]
            .as_str()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        historical.data["reads"][0]["storedVersion"],
        reference["contentHash"]
    );
    save(
        "evidence-reads",
        json!({"current":read,"expired":expired,"historical":historical,"forgedCode":error.code}),
    );
}

#[test]
fn query_neighbors_bfs_cycles_filters_and_strict_budgets() {
    let (_temp, project, mut database, _) = fixture();
    let file_id:String=database.connection().query_row("SELECT ea.entity_id FROM entity_assertions ea JOIN entities e ON e.id=ea.entity_id WHERE e.kind='file' AND json_extract(ea.attributes_json,'$.path')='src/采购😀.ts'",[],|r|r.get(0)).unwrap();
    let selected_file = json!({"snapshot":project_knowledge_core::query::snapshot_ref(&database).unwrap(),"entityId":file_id});
    let cross_kind = run(
        &mut database,
        &project,
        json!({"operation":"trace","roots":[selected_file],"direction":"out","relationKinds":["contains"],"maxDepth":1,"budget":budget()}),
    );
    assert!(
        cross_kind.data["entities"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["kind"] == "symbol")
    );
    let root = graph(&database);
    let statements = Arc::new(Mutex::new(Vec::<String>::new()));
    let sink = statements.clone();
    database
        .observe_sql(move |sql| {
            if sql.starts_with("SELECT r.id,r.kind") {
                sink.lock().unwrap().push(sql.into());
            }
        })
        .unwrap();
    let request = json!({"operation":"neighbors","root":root,"direction":"both","relationKinds":["calls"],"pageSize":100,"budget":budget()});
    let full = run(&mut database, &project, request.clone());
    assert_eq!(full.data["hops"].as_array().unwrap().len(), 58);
    let mut actual_plans = Vec::new();
    for sql in statements.lock().unwrap().iter() {
        let mut statement = database
            .connection()
            .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
            .unwrap();
        let plan = statement
            .query_map(
                rusqlite::params![
                    root["entityId"].as_str().unwrap(),
                    false,
                    false,
                    "[\"observed\",\"declared\"]",
                    "[\"calls\"]",
                    "",
                    "",
                    "",
                    "",
                    201
                ],
                |r| r.get::<_, String>(3),
            )
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let expected = if sql.contains("WHERE r.from_id") {
            "idx_relations_from"
        } else {
            "idx_relations_to"
        };
        assert!(plan.iter().any(|p| p.contains(expected)));
        actual_plans.push(plan);
    }
    let mut request = request;
    request["pageSize"] = json!(5);
    let mut keys = Vec::new();
    for _ in 0..15 {
        let page = run(&mut database, &project, request.clone());
        for hop in page.data["hops"].as_array().unwrap() {
            keys.push(hop["assertionId"].as_str().unwrap().to_string());
            assert_eq!(hop["qualifiers"]["guard"], "value != null");
            assert!(!hop["evidenceRefs"].as_array().unwrap().is_empty());
        }
        if let Some(token) = page.continuation {
            request["continuation"] = json!(token);
        } else {
            break;
        }
    }
    assert_eq!(keys.len(), 58);
    let mut compact = request.clone();
    compact.as_object_mut().unwrap().remove("continuation");
    compact["pageSize"] = json!(100);
    compact["budget"]["maxResponseChars"] = json!(7000);
    let mut compact_keys = Vec::new();
    for _ in 0..60 {
        let response = run(&mut database, &project, compact.clone());
        assert!(serde_json::to_string(&response).unwrap().chars().count() <= 7000);
        for hop in response.data["hops"].as_array().unwrap() {
            compact_keys.push(hop["assertionId"].as_str().unwrap().to_string());
        }
        if let Some(token) = response.continuation {
            compact["continuation"] = json!(token);
        } else {
            break;
        }
    }
    assert_eq!(
        compact_keys, keys,
        "adjacency character pages must not skip trimmed edges"
    );
    assert_eq!(
        keys.iter().collect::<std::collections::BTreeSet<_>>().len(),
        58
    );
    let bfs = run(
        &mut database,
        &project,
        json!({"operation":"trace","roots":[root],"direction":"both","relationKinds":["calls"],"maxDepth":8,"budget":budget()}),
    );
    assert_eq!(bfs.data["hops"].as_array().unwrap().len(), 58);
    assert_eq!(bfs.data["entities"].as_array().unwrap().len(), 30);
    assert!(bfs.continuation.is_none());
    let tiny = run(
        &mut database,
        &project,
        json!({"operation":"trace","roots":[root],"direction":"both","relationKinds":["calls"],"budget":{"maxRelations":3,"maxEntities":4,"maxVisited":10,"maxResponseChars":200000,"timeoutMs":30000}}),
    );
    assert!(tiny.budget_used.relations <= 3);
    assert!(tiny.budget_used.entities <= 4);
    assert!(tiny.budget_used.visited <= 10);
    assert_eq!(tiny.status, ResponseStatus::Partial);
    assert!(!tiny.data["frontier"].as_array().unwrap().is_empty());
    assert!(tiny.continuation.is_none());
    let mut other = root.clone();
    other["snapshot"]["indexId"] = json!("other-index");
    let error = execute(
        &mut QueryContext {
            database: &mut database,
            project: Some(&project),
            now: "now".into(),
        },
        &serde_json::from_value(json!({"operation":"trace","roots":[other],"direction":"out"}))
            .unwrap(),
    )
    .unwrap_err();
    assert_eq!(error.code, codes::SNAPSHOT_EXPIRED);
    let mut missing = root.clone();
    missing["entityId"] = json!("ent_missing");
    let error = execute(
        &mut QueryContext {
            database: &mut database,
            project: Some(&project),
            now: "now".into(),
        },
        &serde_json::from_value(json!({"operation":"trace","roots":[missing],"direction":"out"}))
            .unwrap(),
    )
    .unwrap_err();
    assert_eq!(error.code, codes::REFERENCE_MISSING);
    save(
        "graph",
        json!({"neighbors":full,"bfs":bfs,"tiny":tiny,"keysetCount":keys.len(),"actualNeighborPlans":actual_plans}),
    );
}

#[test]
fn query_stale_unknown_cancel_zero_and_index_plans_are_explicit() {
    let (_temp, project, mut database, _) = fixture();
    let root = root(&database, "budgetPurchase1");
    let plans = [
        "EXPLAIN QUERY PLAN SELECT id FROM relations WHERE from_id='x' ORDER BY kind,to_id",
        "EXPLAIN QUERY PLAN SELECT id FROM relations WHERE to_id='x' ORDER BY kind,from_id",
        "EXPLAIN QUERY PLAN SELECT entity_id FROM search_documents WHERE scope_id='app.web' AND route_exact='/a'",
    ];
    let mut details = Vec::new();
    for sql in plans {
        let mut stmt = database.connection().prepare(sql).unwrap();
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(3))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        details.extend(rows);
    }
    for index in [
        "idx_relations_from",
        "idx_relations_to",
        "idx_search_documents_lookup",
    ] {
        assert!(details.iter().any(|p| p.contains(index)));
    }
    write_file(
        &project.repositories["repo.web"].join("src/new.ts"),
        "export function newFile() {}\n",
    );
    let stale = run(
        &mut database,
        &project,
        json!({"operation":"search","query":"budgetPurchase","budget":budget()}),
    );
    assert_eq!(
        stale.completeness.freshness,
        project_knowledge_core::contract::query::Freshness::Stale
    );
    assert!(stale.data["results"].as_array().unwrap().is_empty());
    let indexed = run(
        &mut database,
        &project,
        json!({"operation":"search","query":"budgetPurchase","filters":{"freshnessPolicy":"indexed"},"budget":budget()}),
    );
    assert_eq!(indexed.status, ResponseStatus::Partial);
    assert!(!indexed.completeness.disk_checked);
    assert!(!indexed.data["results"].as_array().unwrap().is_empty());
    let allow = run(
        &mut database,
        &project,
        json!({"operation":"search","query":"budgetPurchase","filters":{"freshnessPolicy":"allowStale"},"budget":budget()}),
    );
    assert_eq!(allow.status, ResponseStatus::Partial);
    assert!(!allow.data["results"].as_array().unwrap().is_empty());
    let zero = run(
        &mut database,
        &project,
        json!({"operation":"trace","roots":[root],"direction":"out","budget":{"maxVisited":0,"maxResponseChars":4000}}),
    );
    assert_eq!(zero.status, ResponseStatus::Partial);
    assert_eq!(zero.budget_used.visited, 0);
    let inflight = tokio_util::sync::CancellationToken::new();
    let trigger = inflight.clone();
    database
        .observe_sql(move |sql| {
            if sql.starts_with("WITH hits") {
                trigger.cancel();
            }
        })
        .unwrap();
    let inflight_error = project_knowledge_core::query::execute_with_cancellation(
        &mut QueryContext {
            database: &mut database,
            project: Some(&project),
            now: "now".into(),
        },
        &serde_json::from_value(
            json!({"operation":"search","query":"budgetPurchase","budget":budget()}),
        )
        .unwrap(),
        inflight,
    )
    .unwrap_err();
    assert_eq!(inflight_error.code, codes::CANCELLED);
    assert!(database.connection().is_autocommit());
    let cancel = tokio_util::sync::CancellationToken::new();
    cancel.cancel();
    let error = project_knowledge_core::query::execute_with_cancellation(
        &mut QueryContext {
            database: &mut database,
            project: Some(&project),
            now: "now".into(),
        },
        &QueryRequest::Status(StatusRequest {
            check_freshness: true,
        }),
        cancel,
    )
    .unwrap_err();
    assert_eq!(error.code, codes::CANCELLED);
    save(
        "boundaries",
        json!({"plans":details,"stale":stale,"indexed":indexed,"allowStale":allow,"zero":zero}),
    );
    // A non-indexed request has no basis to claim a covered empty result.
    let path = tempfile::tempdir().unwrap();
    let mut empty =
        Database::open_maintenance(&path.path().join("empty.sqlite"), "erp", "cfg", "now").unwrap();
    let response = execute(
        &mut QueryContext {
            database: &mut empty,
            project: None,
            now: "now".into(),
        },
        &QueryRequest::Search(SearchRequest {
            query: "budgetPurchase".into(),
            fields: None,
            filters: Some(CommonFilters {
                freshness_policy: Some(FreshnessPolicy::Indexed),
                ..Default::default()
            }),
            page_size: None,
            continuation: None,
            budget: None,
        }),
    )
    .unwrap();
    assert_eq!(response.status, ResponseStatus::Partial);
}
