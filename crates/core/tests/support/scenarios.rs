use super::*;
use project_knowledge_core::{
    contract::query::{QueryRequest, ReadSourceRequest},
    query::{QueryContext, execute},
};

fn budget() -> Value {
    json!({"maxEntities":1000,"maxRelations":3000,"maxVisited":20000,"maxDepth":16,"maxResponseChars":200000,"timeoutMs":30000})
}
fn query(fixture: &mut FrontendFixture, request: Value) -> Value {
    serde_json::to_value(
        execute(
            &mut QueryContext {
                database: &mut fixture.database,
                project: Some(&fixture.project),
                now: "2026-10-28T00:00:00Z".into(),
            },
            &serde_json::from_value(request).unwrap(),
        )
        .unwrap(),
    )
    .unwrap()
}
fn normalized(mut value: Value) -> Value {
    fn clean(value: &mut Value) {
        match value {
            Value::Object(map) => {
                for field in ["requestId", "checkedAt", "elapsedMs", "responseChars"] {
                    map.remove(field);
                }
                for value in map.values_mut() {
                    clean(value);
                }
            }
            Value::Array(list) => {
                for value in list {
                    clean(value);
                }
            }
            _ => (),
        }
    }
    clean(&mut value);
    value
}
fn equivalent(label: &str, left: Value, right: Value) {
    let raw_left = left.clone();
    let raw_right = right.clone();
    let left = normalized(left);
    let right = normalized(right);
    if left != right {
        save(&format!("{label}-cli-raw-failed"), &raw_left);
        save(&format!("{label}-core-raw-failed"), &raw_right);
        save(&format!("{label}-cli-failed"), &left);
        save(&format!("{label}-core-failed"), &right);
    }
    assert!(
        left == right,
        "{label}: Core/CLI normalized JSON differs; see saved evidence"
    );
}
fn cli(fixture: &FrontendFixture, request: &Value) -> Option<Value> {
    let executable = std::env::var_os("PK_TEST_CLI")?;
    let path = fixture.temp.path().join("request.json");
    std::fs::write(&path, serde_json::to_vec(request).unwrap()).unwrap();
    let output = std::process::Command::new(executable)
        .args(["query", "--config"])
        .arg(fixture.temp.path().join("project.json"))
        .arg("--request")
        .arg(&path)
        .output()
        .unwrap();
    assert!(
        matches!(output.status.code(), Some(0 | 2)),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    if let Some(directory) = std::env::var_os("PK_SCENARIO_TRANSPORTS_DIR") {
        let expected = fixture.temp.path().join("cli-expected.json");
        std::fs::write(&expected, &output.stdout).unwrap();
        let result = std::process::Command::new("node")
            .arg(root().join("scripts/query-transports.mjs"))
            .arg(fixture.temp.path().join("project.json"))
            .arg(&path)
            .arg(expected)
            .arg(directory)
            .current_dir(root())
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "SDK/MCP mismatch: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    Some(serde_json::from_slice(&output.stdout).unwrap())
}
fn save(name: &str, value: &Value) {
    if let Some(directory) = std::env::var_os("PK_SCENARIO_EVIDENCE_DIR") {
        let directory = PathBuf::from(directory);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            directory.join(format!("{name}.json")),
            serde_json::to_vec_pretty(value).unwrap(),
        )
        .unwrap();
    }
}
fn entity<'a>(response: &'a Value, id: &str) -> &'a Value {
    response["data"]["entities"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["entityRef"]["entityId"] == id)
        .unwrap()
}

#[tokio::test]
async fn scenario_queries_two_layouts_match_original_facts_and_cli() {
    let frontend = read_json("fixtures/truth/frontend.json");
    let semantics = read_json("fixtures/truth-supplements/T14-semantics.json");
    for layout in ["a", "b"] {
        let mut fixture = super::semantics::fixture(layout).await;
        let project = format!("acceptance.{layout}");
        let original = frontend["layouts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|l| l["layout"] == layout)
            .unwrap();
        let route = original["routePath"].as_str().unwrap();
        let unbound = query(
            &mut fixture,
            json!({"operation":"page","route":route,"scopeIds":["app.web"],"budget":budget()}),
        );
        assert!(
            unbound["data"]["tableColumns"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert!(
            unbound["gaps"]
                .as_array()
                .unwrap()
                .iter()
                .any(|g| g["code"] == "PAGE_DATA_SOURCE_MISSING")
        );
        save(&format!("{layout}-unbound-source"), &unbound);
        let (binding, target) = if layout == "a" {
            ("service.ts", "api/service.ts")
        } else {
            ("queries.py", "server/domain/queries.py")
        };
        let review = read_json("fixtures/truth-supplements/T16-bindings/review.json");
        for (path, hash) in review["fileHashes"].as_object().unwrap() {
            assert_eq!(
                content_hash(&std::fs::read(root().join(path)).unwrap()),
                hash.as_str().unwrap()
            );
        }
        std::fs::copy(
            root()
                .join("fixtures/truth-supplements/T16-bindings")
                .join(binding),
            fixture.temp.path().join(target),
        )
        .unwrap();
        project_knowledge_core::index::incremental::run_incremental(
            &fixture.project,
            &mut fixture.database,
            &tokio_util::sync::CancellationToken::new(),
            "now",
        )
        .await
        .unwrap();
        let page_request = json!({"operation":"page","route":format!("https://example.invalid{hash}{route}?supplier=A",hash=if layout=="a" {"/#"} else {""}),"scopeIds":["app.web"],"budget":budget()});
        let page = query(&mut fixture, page_request.clone());
        assert_eq!(page["data"]["roots"].as_array().unwrap().len(), 1);
        assert!(!page["data"]["operations"].as_array().unwrap().is_empty());
        assert!(!page["data"]["requests"].as_array().unwrap().is_empty());
        assert!(!page["data"]["backend"].as_array().unwrap().is_empty());
        let expected_columns = review["columns"]
            .as_array()
            .unwrap()
            .iter()
            .map(|column| {
                truth_entity_id(
                    &project,
                    EntityKind::DataColumn,
                    &json!({"dataSourceId":"ds.erp","table":"purchase_detail","column":column}),
                )
            })
            .collect::<std::collections::BTreeSet<_>>();
        let actual_columns = page["data"]["tableColumns"]
            .as_array()
            .unwrap()
            .iter()
            .map(|column| column["entityId"].as_str().unwrap().to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            actual_columns, expected_columns,
            "{layout}: page must read the bound query's columns only"
        );
        for (kind, identity) in [
            (
                EntityKind::HttpEndpoint,
                json!({"serviceId":"erp","method":"GET","registeredPath":"/purchase/ledger"}),
            ),
            (
                EntityKind::GraphqlOperation,
                json!({"serviceId":"erp","operationType":original["graphql"]["type"],"operationName":original["graphql"]["name"]}),
            ),
            (
                EntityKind::GraphqlField,
                json!({"serviceId":"erp","schemaType":"Query","fieldName":original["graphql"]["field"]}),
            ),
        ] {
            let id = truth_entity_id(&project, kind, &identity);
            assert!(
                page["data"]["entities"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|e| e["entityRef"]["entityId"] == id)
            );
        }
        let method = truth_entity_id(
            &project,
            EntityKind::Operation,
            &json!({"repositoryId":"back","path":binding,"scopeId":"svc.erp","lexicalPath":"ledger","normalizedSignature":"()"}),
        );
        let reverse_root = json!({"snapshot":project_knowledge_core::query::snapshot_ref(&fixture.database).unwrap(),"entityId":method});
        let reverse = query(
            &mut fixture,
            json!({"operation":"trace","roots":[reverse_root],"direction":"in","budget":budget()}),
        );
        assert!(
            reverse["data"]["entities"]
                .as_array()
                .unwrap()
                .iter()
                .any(|e| e["entityRef"] == page["data"]["roots"][0])
        );
        save(&format!("{layout}-reverse-page"), &reverse);
        let title = &page["data"]["title"][0];
        assert!(
            title["assertions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|a| a["attributes"]["text"] == original["title"])
        );
        let alias = query(
            &mut fixture,
            json!({"operation":"page","route":original["alias"],"scopeIds":["app.web"],"budget":budget()}),
        );
        assert_eq!(alias["data"]["roots"], page["data"]["roots"]);
        let decoy = query(
            &mut fixture,
            json!({"operation":"page","route":format!("{}-suffix",route),"budget":budget()}),
        );
        assert!(decoy["data"]["roots"].as_array().unwrap().is_empty());
        let feature = query(
            &mut fixture,
            json!({"operation":"locateFeature","query":"采购","routeHint":route,"rootFilter":{"kinds":["route"],"scopeIds":["app.web"]},"budget":budget()}),
        );
        assert!(
            feature["data"]["entities"]
                .as_array()
                .unwrap()
                .iter()
                .any(|e| e["kind"] == "httpEndpoint")
        );
        assert!(
            feature["data"]["entities"]
                .as_array()
                .unwrap()
                .iter()
                .any(|e| e["kind"] == "operation" && e["scopeId"] == "svc.erp")
        );
        let natural_request =
            json!({"operation":"locateFeature","query":"采购台账增加供应商筛选","budget":budget()});
        let natural = query(&mut fixture, natural_request.clone());
        if let Some(response) = cli(&fixture, &natural_request) {
            equivalent(&format!("{layout}-natural"), response, natural.clone());
        }
        assert!(
            natural["data"]["entities"]
                .as_array()
                .unwrap()
                .iter()
                .any(|e| e["kind"] == "route")
        );
        save(&format!("{layout}-natural-feature"), &natural);
        let workflow_request = json!({"operation":"workflow","query":"purchase","scopeIds":["svc.erp"],"budget":budget()});
        let workflow = query(&mut fixture, workflow_request.clone());
        assert_eq!(workflow["data"]["states"].as_array().unwrap().len(), 6);
        assert_eq!(
            workflow["data"]["transitions"].as_array().unwrap().len(),
            14
        );
        let guards = workflow["data"]["transitions"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|h| h["qualifiers"]["guard"].as_str())
            .collect::<Vec<_>>();
        assert!(guards.iter().any(|s| s.contains("grossReceived")));
        assert!(guards.iter().any(|s| s.contains("netReceived")));
        assert!(
            workflow["gaps"]
                .as_array()
                .unwrap()
                .iter()
                .any(|g| g["code"] == "CONFLICTING_EVIDENCE")
        );
        assert!(
            workflow["data"]["transitions"]
                .as_array()
                .unwrap()
                .iter()
                .all(|h| h["kind"] == "transitions_to")
        );
        assert_eq!(
            workflow["data"]["documentLinks"].as_array().unwrap().len(),
            2
        );
        let correct = truth_entity_id(
            &project,
            EntityKind::QueryImplementation,
            &json!({"scopeId":"svc.erp","semanticKey":"back:correct.sql:0"}),
        );
        let root = json!({"snapshot":project_knowledge_core::query::snapshot_ref(&fixture.database).unwrap(),"entityId":correct});
        let report_request = json!({"operation":"report","query":"purchase","roots":[root],"scopeIds":["svc.erp"],"budget":budget()});
        let report = query(&mut fixture, report_request.clone());
        let first_request = json!({"operation":"search","query":"purchase","filters":{"kinds":["queryImplementation"]},"pageSize":1,"budget":budget()});
        let first = query(&mut fixture, first_request.clone());
        if let Some(cli_first) = cli(&fixture, &first_request) {
            equivalent(
                &format!("{layout}-search-first"),
                cli_first.clone(),
                first.clone(),
            );
            let mut second_request = first_request;
            second_request["continuation"] = first["continuation"].clone();
            assert!(second_request["continuation"].is_string());
            let second = query(&mut fixture, second_request.clone());
            let cli_second = cli(&fixture, &second_request).unwrap();
            equivalent(
                &format!("{layout}-search-cross-cursor"),
                cli_second.clone(),
                second.clone(),
            );
            save(
                &format!("{layout}-cross-client-cursor"),
                &json!({"coreFirst":first,"cliFirst":cli_first,"coreSecond":second,"cliSecond":cli_second}),
            );
        }
        assert_eq!(report["data"]["fields"].as_array().unwrap().len(), 9);
        assert_eq!(report["data"]["implementations"][0]["grainProven"], true);
        let policy = report["data"]["implementations"][0]["policies"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["grainColumnIds"].is_array())
            .unwrap();
        assert_eq!(policy["grainColumnIds"].as_array().unwrap().len(), 3);
        assert_eq!(policy["joins"].as_array().unwrap().len(), 4);
        assert_eq!(policy["validStates"], json!(["APPROVED", "POSTED"]));
        assert_eq!(policy["timeFields"]["status"], "unknown");
        for (name, columns) in semantics["fields"].as_object().unwrap() {
            let id = truth_entity_id(
                &project,
                EntityKind::ReportField,
                &json!({"scopeId":"svc.erp","semanticKey":format!("back:correct.sql:0:field:{name}")}),
            );
            let field = entity(&report, &id);
            let assertion = field["assertions"]
                .as_array()
                .unwrap()
                .iter()
                .find(|a| a["attributes"].get("sourceColumnIds").is_some())
                .unwrap();
            let mut expected = columns
                .as_array()
                .unwrap()
                .iter()
                .map(|c| {
                    let (table, column) = c.as_str().unwrap().split_once('.').unwrap();
                    truth_entity_id(
                        &project,
                        EntityKind::DataColumn,
                        &json!({"dataSourceId":"ds.erp","table":table,"column":column}),
                    )
                })
                .collect::<Vec<_>>();
            expected.sort();
            assert_eq!(assertion["attributes"]["sourceColumnIds"], json!(expected));
        }
        for (name, request, response) in [
            ("page", page_request, page),
            ("workflow", workflow_request, workflow),
            ("report", report_request, report),
        ] {
            let refs: Vec<project_knowledge_core::contract::evidence::SourceReference> =
                serde_json::from_value(response["sourceReferences"].clone()).unwrap();
            assert!(!refs.is_empty());
            let reads = execute(
                &mut QueryContext {
                    database: &mut fixture.database,
                    project: Some(&fixture.project),
                    now: "2026-10-28T00:00:00Z".into(),
                },
                &QueryRequest::ReadSource(ReadSourceRequest {
                    references: refs.clone(),
                    max_response_chars: Some(200000),
                    filters: None,
                }),
            )
            .unwrap();
            assert_eq!(reads.data["reads"].as_array().unwrap().len(), refs.len());
            assert!(
                reads.data["reads"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|r| r["status"] == "current")
            );
            let cli_response = cli(&fixture, &request);
            if let Some(value) = &cli_response {
                equivalent(&format!("{layout}-{name}"), value.clone(), response.clone());
            }
            save(
                &format!("{layout}-{name}"),
                &json!({"request":request,"core":response,"cli":cli_response,"sourceReferencesRead":refs.len()}),
            );
        }
        // Wrong join paths stay candidates with explicit counter-evidence; never selected by distance.
        let candidates = query(
            &mut fixture,
            json!({"operation":"report","query":"purchase","scopeIds":["svc.erp"],"budget":budget()}),
        );
        save(&format!("{layout}-report-candidates-failed"), &candidates);
        assert!(
            candidates["gaps"]
                .as_array()
                .unwrap()
                .iter()
                .any(|g| g["code"] == "JOIN_FANOUT_RISK")
        );
        assert!(
            candidates["gaps"]
                .as_array()
                .unwrap()
                .iter()
                .any(|g| g["code"] == "CHILD_FILTER_AFTER_LEFT_JOIN")
        );
        save(&format!("{layout}-report-candidates"), &candidates);
        if layout == "a" {
            let alternate = std::fs::read_to_string(fixture.temp.path().join("web/routes.ts"))
                .unwrap()
                .replace("id: 'main'", "id: 'alternate'")
                .replace("path: '/Other/purchaseLedger'", "path: '/Other/:name'");
            std::fs::write(fixture.temp.path().join("web/routes-alt.ts"), alternate).unwrap();
            std::fs::remove_file(fixture.temp.path().join("api/workflow.md")).unwrap();
            let flow = fixture.temp.path().join("api/flow.ts");
            let mut source = std::fs::read_to_string(&flow).unwrap();
            source.push_str("\nexport function audit(order) {}\nexport function save(order) {}\n");
            std::fs::write(flow, source).unwrap();
            project_knowledge_core::index::incremental::run_incremental(
                &fixture.project,
                &mut fixture.database,
                &tokio_util::sync::CancellationToken::new(),
                "now",
            )
            .await
            .unwrap();
            let request = json!({"operation":"page","route":route,"budget":budget()});
            let ambiguity = query(&mut fixture, request.clone());
            assert_eq!(ambiguity["data"]["roots"].as_array().unwrap().len(), 2);
            assert!(
                ambiguity["gaps"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|g| g["code"] == "AMBIGUOUS_BINDING")
            );
            if let Some(response) = cli(&fixture, &request) {
                equivalent("ambiguity", response, ambiguity.clone());
            }
            save("route-ambiguity", &ambiguity);
            let request = json!({"operation":"workflow","query":"purchase","budget":budget()});
            let code_only = query(&mut fixture, request.clone());
            assert!(
                code_only["data"]["transitions"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            );
            assert!(
                !code_only["data"]["codeCalls"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            );
            assert!(
                code_only["gaps"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|g| g["code"] == "WORKFLOW_EVIDENCE_MISSING")
            );
            if let Some(response) = cli(&fixture, &request) {
                equivalent("code-only", response, code_only.clone());
            }
            save("workflow-code-only", &code_only);
        }
    }
}

#[test]
fn scenario_route_normalization_keeps_case_and_encoded_separators() {
    use project_knowledge_core::query::page::{RouteMode, normalize_route, percent_decode_once};
    assert_eq!(
        normalize_route("https://example.invalid/Other/采购?x=1", RouteMode::History)
            .unwrap()
            .path,
        "/Other/采购"
    );
    assert_eq!(
        normalize_route(
            "https://example.invalid/app?outside=1#/Other/%E9%87%87%E8%B4%AD?inside=2",
            RouteMode::Hash
        )
        .unwrap()
        .path,
        "/Other/采购"
    );
    assert_eq!(percent_decode_once("/A%2fb%252F").unwrap(), "/A%2Fb%2F");
    assert!(percent_decode_once("/%中").is_err());
}
