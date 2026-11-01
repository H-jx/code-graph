use super::*;
use project_knowledge_core::storage::{StoredEntityAssertion, load_entity_assertions};
use tokio_util::sync::CancellationToken;

pub(super) async fn fixture(layout: &str) -> FrontendFixture {
    let mut fixture = FrontendFixture::new_for(layout, "database").await;
    let destination = fixture.temp.path().join(if layout == "a" {
        "api"
    } else {
        "server/domain"
    });
    for (source, target) in [
        ("fixtures/purchase-workflow/flow.ts", "flow.ts"),
        ("fixtures/purchase-workflow/rules.md", "workflow.md"),
        ("fixtures/purchase-ledger/policy.md", "policy.md"),
    ] {
        std::fs::copy(root().join(source), destination.join(target)).unwrap();
    }
    fixture.reindex();
    let mut paths = FrontendFixture::group_paths(layout);
    paths.push(root().join(format!(
        "collectors/examples/frontend-backend-{layout}/semantics/manifest.json"
    )));
    fixture.activate_group(&paths).await;
    fixture
}

fn semantic_assertion(fixture: &FrontendFixture, id: &str) -> StoredEntityAssertion {
    load_entity_assertions(fixture.database.connection(), id, true)
        .unwrap()
        .into_iter()
        .find(|assertion| assertion.producer_key.contains(".semantics@"))
        .unwrap()
}

#[tokio::test]
async fn semantics_workflow_report_preserve_original_conflicts_and_all_field_lineage() {
    let review = read_json("fixtures/truth-supplements/T14-semantics.json");
    for (path, hash) in review["sourceHashes"].as_object().unwrap() {
        assert_eq!(
            content_hash(&std::fs::read(root().join(path)).unwrap()),
            hash.as_str().unwrap()
        );
    }
    for key in ["a", "b"] {
        let fixture = fixture(key).await;
        let project = format!("acceptance.{key}");
        for state in review["states"].as_array().unwrap() {
            let id = truth_entity_id(
                &project,
                EntityKind::WorkflowState,
                &json!({"scopeId":"svc.erp","workflowId":"purchase","semanticKey":state}),
            );
            assert_eq!(
                semantic_assertion(&fixture, &id).attributes["state"],
                *state
            );
        }
        assert_eq!(fixture.count("SELECT COUNT(*) FROM relations r JOIN relation_assertions ra ON ra.relation_id=r.id JOIN current_batches cb ON cb.batch_id=ra.batch_id WHERE r.kind='transitions_to'"),14);
        assert_eq!(fixture.count("SELECT COUNT(*) FROM relations r JOIN relation_assertions ra ON ra.relation_id=r.id JOIN current_batches cb ON cb.batch_id=ra.batch_id WHERE r.kind='precedes'"),2);
        assert_eq!(fixture.count("SELECT COUNT(*) FROM relations r JOIN relation_assertions ra ON ra.relation_id=r.id JOIN current_batches cb ON cb.batch_id=ra.batch_id WHERE r.kind='relates_document' AND json_array_length(r.qualifiers_json,'$.keyPairs')=3"),2);
        let guards:Vec<String>=fixture.database.connection().prepare("SELECT json_extract(r.qualifiers_json,'$.guard') FROM relations r JOIN relation_assertions ra ON ra.relation_id=r.id JOIN current_batches cb ON cb.batch_id=ra.batch_id WHERE r.kind='transitions_to' AND json_extract(r.qualifiers_json,'$.trigger')='finish'").unwrap().query_map([],|row|row.get(0)).unwrap().map(Result::unwrap).collect();
        assert!(guards.iter().any(|guard| guard.contains("grossReceived")));
        assert!(guards.iter().any(|guard| guard.contains("netReceived")));
        assert!(fixture.count("SELECT COUNT(*) FROM gaps g JOIN current_batches cb ON cb.batch_id=g.batch_id WHERE g.code='CONFLICTING_EVIDENCE'")>0);
        let query = truth_entity_id(
            &project,
            EntityKind::QueryImplementation,
            &json!({"scopeId":"svc.erp","semanticKey":"back:correct.sql:0"}),
        );
        let attributes = semantic_assertion(&fixture, &query).attributes;
        let grain: Vec<_> = review["documentKeys"]
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
            .collect();
        assert_eq!(attributes["grainColumnIds"], json!(grain));
        assert_eq!(attributes["aggregates"].as_array().unwrap().len(), 3);
        assert_eq!(attributes["validStates"], json!(["APPROVED", "POSTED"]));
        assert_eq!(attributes["timeFields"]["status"], "unknown");
        assert!(
            attributes["nullHandling"]
                .as_str()
                .unwrap()
                .contains("zero")
        );
        assert_eq!(attributes["units"].as_array().unwrap().len(), 2);
        for (name, columns) in review["fields"].as_object().unwrap() {
            let id = truth_entity_id(
                &project,
                EntityKind::ReportField,
                &json!({"scopeId":"svc.erp","semanticKey":format!("back:correct.sql:0:field:{name}")}),
            );
            let assertion = semantic_assertion(&fixture, &id);
            let mut expected: Vec<_> = columns
                .as_array()
                .unwrap()
                .iter()
                .map(|column| {
                    let (table, column) = column.as_str().unwrap().split_once('.').unwrap();
                    truth_entity_id(
                        &project,
                        EntityKind::DataColumn,
                        &json!({"dataSourceId":"ds.erp","table":table,"column":column}),
                    )
                })
                .collect();
            expected.sort();
            assert_eq!(
                assertion.attributes["sourceColumnIds"],
                json!(expected),
                "{key}: {name}"
            );
            assert!(
                !assertion.attributes["expression"]
                    .as_str()
                    .unwrap()
                    .is_empty()
            );
        }
        for code in [
            "JOIN_FANOUT_RISK",
            "CHILD_FILTER_AFTER_LEFT_JOIN",
            "REPORT_TIME_FIELD_UNKNOWN",
            "REPORT_CARDINALITY_UNKNOWN",
            "REPORT_UNIT_UNKNOWN",
        ] {
            let count:i64=fixture.database.connection().query_row("SELECT COUNT(*) FROM gaps g JOIN current_batches cb ON cb.batch_id=g.batch_id WHERE g.code=?1",[code],|row|row.get(0)).unwrap();
            assert!(count > 0, "{key}: {code}");
        }
        if let Some(directory) = std::env::var_os("PK_SCENARIO_EVIDENCE_DIR") {
            let directory = PathBuf::from(directory);
            std::fs::create_dir_all(&directory).unwrap();
            std::fs::write(directory.join(format!("semantic-{key}-report.json")),serde_json::to_vec_pretty(&json!({"expectedStates":6,"declaredAndCodeTransitions":14,"documentKeyPairs":3,"expectedFields":9,"queryAttributes":attributes,"forbiddenConfirmed":0})).unwrap()).unwrap();
        }
    }
}

#[tokio::test]
async fn semantics_code_calls_without_flow_and_missing_or_unsafe_report_policy_remain_gaps() {
    for key in ["a", "b"] {
        let mut fixture = fixture(key).await;
        let repo = if key == "a" { "api" } else { "server/domain" };
        std::fs::remove_file(fixture.temp.path().join(repo).join("workflow.md")).unwrap();
        let flow = fixture.temp.path().join(repo).join("flow.ts");
        let mut code = std::fs::read_to_string(&flow).unwrap();
        code.push_str("\nexport function audit(order) {}\nexport function save(order) {}\n");
        code = code.replace(
            "approve(order, actor)",
            "approve( order /* signature stays stable */,\n actor )",
        );
        code = code.replace("order.state = 'APPROVED'", "snapshot.state = 'APPROVED'");
        std::fs::write(&flow, code).unwrap();
        fixture.replace(
            &format!("{repo}/correct.sql"),
            "GROUP BY tenant_id,purchase_no,line_no",
            "GROUP BY purchase_no,line_no",
        );
        fixture.replace(
            &format!("{repo}/correct.sql"),
            "status='POSTED'",
            "status='DRAFT'",
        );
        fixture.replace(&format!("{repo}/correct.sql"), " AND deleted=0", "");
        fixture.replace(
            &format!("{repo}/correct.sql"),
            "COALESCE(i.amount,0)",
            "COALESCE(i.amount,999)",
        );
        project_knowledge_core::index::run_incremental(
            &fixture.project,
            &mut fixture.database,
            &CancellationToken::new(),
            "semantic change",
        )
        .await
        .unwrap();
        assert_eq!(fixture.count("SELECT COUNT(*) FROM relations r JOIN relation_assertions ra ON ra.relation_id=r.id JOIN current_batches cb ON cb.batch_id=ra.batch_id WHERE r.kind IN ('transitions_to','precedes')"),0);
        assert!(fixture.count("SELECT COUNT(*) FROM relations r JOIN relation_assertions ra ON ra.relation_id=r.id JOIN current_batches cb ON cb.batch_id=ra.batch_id JOIN assertions a ON a.id=ra.id WHERE r.kind='calls' AND a.producer_key LIKE 'example.%.semantics@%'")>=2);
        let approve = truth_entity_id(
            &format!("acceptance.{key}"),
            EntityKind::Operation,
            &json!({"repositoryId":"back","path":"flow.ts","scopeId":"svc.erp","lexicalPath":"approve","normalizedSignature":"(order,actor)"}),
        );
        assert_eq!(
            semantic_assertion(&fixture, &approve).name.as_deref(),
            Some("approve")
        );
        if key == "a" {
            assert!(
                load_entity_assertions(fixture.database.connection(), &approve, true)
                    .unwrap()
                    .iter()
                    .any(|assertion| assertion.producer_key.starts_with("example.a.api@"))
            );
        }
        for code in [
            "WORKFLOW_EVIDENCE_MISSING",
            "WORKFLOW_STATE_OWNER_UNKNOWN",
            "PREAGGREGATION_GRAIN_MISMATCH",
            "REPORT_STATES_UNKNOWN",
            "REPORT_VALID_FILTER_UNKNOWN",
            "REPORT_NULL_POLICY_UNKNOWN",
        ] {
            let count:i64=fixture.database.connection().query_row("SELECT COUNT(*) FROM gaps g JOIN current_batches cb ON cb.batch_id=g.batch_id WHERE g.code=?1",[code],|row|row.get(0)).unwrap();
            assert!(count > 0, "{key}: {code}");
        }
        std::fs::remove_file(fixture.temp.path().join(repo).join("policy.md")).unwrap();
        project_knowledge_core::index::run_incremental(
            &fixture.project,
            &mut fixture.database,
            &CancellationToken::new(),
            "missing policy",
        )
        .await
        .unwrap();
        let id = truth_entity_id(
            &format!("acceptance.{key}"),
            EntityKind::QueryImplementation,
            &json!({"scopeId":"svc.erp","semanticKey":"back:correct.sql:0"}),
        );
        let attributes = semantic_assertion(&fixture, &id).attributes;
        for field in [
            "grainColumnIds",
            "validStates",
            "nullHandling",
            "timeFields",
            "units",
        ] {
            assert_eq!(attributes[field]["status"], "unknown", "{key}: {field}");
        }
    }
}
