use super::*;
use project_knowledge_core::{
    collect::{
        deps, manifest, registry,
        runner::{PluginRunner, ResolvedRuntime, RunRequest},
        staging,
    },
    contract::{CollectorInput, CollectorOutput, RelationKind, Resolution},
};
use tokio_util::sync::CancellationToken;

impl FrontendFixture {
    pub(super) fn group_paths(layout: &str) -> Vec<PathBuf> {
        ["frontend", "api", "database", "link"]
            .iter()
            .map(|kind| {
                root().join(format!(
                    "collectors/examples/frontend-backend-{layout}/{kind}/manifest.json"
                ))
            })
            .collect()
    }
    pub(super) async fn activate_group(&mut self, paths: &[PathBuf]) {
        let mut ids = Vec::new();
        for path in paths {
            let parsed = manifest::load_manifest(path).unwrap();
            if parsed.dependencies.is_some() {
                let validated = manifest::validate_manifest(
                    &parsed,
                    path.parent().unwrap(),
                    &self.project.config.runtimes,
                )
                .unwrap();
                let runtime = staging::resolve_runtime(&parsed, &self.project)
                    .await
                    .unwrap();
                let (plan, inputs) = deps::plugin_plan(&validated, &runtime).unwrap();
                deps::prepare_dependencies(&plan, &inputs).await.unwrap();
            }
            ids.push(
                registry::submit(&self.project, &mut self.database, path, "now")
                    .unwrap()
                    .id,
            );
        }
        registry::validate_many(
            &self.project,
            &mut self.database,
            &ids,
            &CancellationToken::new(),
            "now",
        )
        .await
        .unwrap();
        registry::activate(&self.project, &mut self.database, &ids[0], "now")
            .await
            .unwrap();
    }
    fn link_input(&self, layout: &str) -> CollectorInput {
        let manifest = manifest::load_manifest(&Self::group_paths(layout)[3]).unwrap();
        project_knowledge_core::collect::import::export_input(
            self.database.connection(),
            &self.project,
            &manifest.stages[0],
            &self.database.current_source_snapshot_id().unwrap().unwrap(),
            "link-test",
            &manifest.plugin_id,
            &manifest.version,
        )
        .unwrap()
    }
    fn normalized_business(&self) -> Value {
        let rows = |sql: &str| -> Vec<Value> {
            let mut values: Vec<Value> = self
                .database
                .connection()
                .prepare(sql)
                .unwrap()
                .query_map([], |row| row.get::<_, String>(0))
                .unwrap()
                .map(|row| serde_json::from_str(&row.unwrap()).unwrap())
                .collect();
            values.sort_by_key(|value| serde_json::to_string(value).unwrap());
            values
        };
        json!({"entities":rows("SELECT json_array(e.id,e.kind,e.scope_id,a.producer_key,ea.name,ea.attributes_json) FROM entities e JOIN entity_assertions ea ON ea.entity_id=e.id JOIN assertions a ON a.id=ea.id JOIN current_batches cb ON cb.batch_id=ea.batch_id JOIN batches b ON b.id=cb.batch_id WHERE b.state='fresh' ORDER BY e.id,a.producer_key"),
            "relations":rows("SELECT json_array(r.id,r.kind,r.from_id,r.to_id,r.qualifiers_json,ra.resolution) FROM relations r JOIN relation_assertions ra ON ra.relation_id=r.id JOIN current_batches cb ON cb.batch_id=ra.batch_id JOIN batches b ON b.id=cb.batch_id WHERE b.state='fresh' ORDER BY r.id,ra.resolution"),
            "coverage":rows("SELECT json_array(c.capability,c.scope_id,c.status,c.counts_json,c.gap_ids_json) FROM coverage c JOIN current_batches cb ON cb.batch_id=c.batch_id ORDER BY c.capability,c.scope_id")})
    }
}

fn compare_normalized(
    actual: &FrontendFixture,
    expected: &FrontendFixture,
    layout: &str,
    state: &str,
) {
    let actual = actual.normalized_business();
    let expected = expected.normalized_business();
    if let Some(directory) = std::env::var_os("PK_SCENARIO_EVIDENCE_DIR") {
        let directory = PathBuf::from(directory);
        std::fs::create_dir_all(&directory).unwrap();
        for (label, value) in [("incremental", &actual), ("clean", &expected)] {
            std::fs::write(
                directory.join(format!("normalized-{layout}-{state}-{label}.json")),
                serde_json::to_vec_pretty(value).unwrap(),
            )
            .unwrap();
        }
    }
    assert!(
        actual == expected,
        "{layout}/{state}: normalized entities, relations or coverage differ; see saved comparisons"
    );
}

async fn run_link_case(input: CollectorInput, layout: &str) -> CollectorOutput {
    let work = tempfile::tempdir().unwrap();
    PluginRunner::default()
        .run(
            &RunRequest {
                runtime: ResolvedRuntime {
                    executable: PathBuf::from("node"),
                    version: "24".into(),
                },
                entry: FrontendFixture::group_paths(layout)[3]
                    .parent()
                    .unwrap()
                    .join("collect.mjs"),
                args: vec![],
                directory: work.path().into(),
                limits: Default::default(),
                input,
                extra_env: Default::default(),
            },
            &CancellationToken::new(),
        )
        .await
        .unwrap()
        .output
}

#[tokio::test]
async fn linking_two_layouts_use_original_bindings_and_reject_missing_or_incomplete_proof() {
    for key in ["a", "b"] {
        let mut fixture = FrontendFixture::new_for(key, "database").await;
        let paths = FrontendFixture::group_paths(key);
        fixture.activate_group(&paths).await;
        let input = fixture.link_input(key);
        assert!(!input.evidence.is_empty());
        assert!(input.evidence.iter().all(|evidence| {
            input.sources.iter().any(|source| {
                source.source.id == evidence.source_id
                    && source.source.content_hash == evidence.content_hash
            })
        }));
        let project = format!("acceptance.{key}");
        let (file, method, handler) = if key == "a" {
            ("requests.ts", "fetchLedger", "service.ts")
        } else {
            ("network.js", "loadRows", "queries.py")
        };
        let request = truth_entity_id(
            &project,
            EntityKind::Request,
            &json!({"scopeId":"app.web","requestKey":format!("{file}#{method}")}),
        );
        let endpoint = truth_entity_id(
            &project,
            EntityKind::HttpEndpoint,
            &json!({"serviceId":"erp","method":"GET","registeredPath":"/purchase/ledger"}),
        );
        let graphql = truth_entity_id(
            &project,
            EntityKind::GraphqlOperation,
            &json!({"serviceId":"erp","operationType":"query","operationName":"PurchaseSummary"}),
        );
        let operation = truth_entity_id(
            &project,
            EntityKind::Operation,
            &json!({"repositoryId":"back","path":handler,"scopeId":"svc.erp","lexicalPath":"ledger","normalizedSignature":"()"}),
        );
        for (kind, from, to) in [
            (RelationKind::References, &request, &endpoint),
            (RelationKind::Calls, &graphql, &operation),
        ] {
            let id =
                compute_relation_id(&project, kind, from, to, &serde_json::Map::new()).unwrap();
            let assertions = project_knowledge_core::storage::load_relation_assertions(
                fixture.database.connection(),
                &id,
                true,
            )
            .unwrap();
            assert!(
                assertions
                    .iter()
                    .any(|assertion| assertion.resolution == Resolution::Confirmed)
            );
        }
        let mut missing = input.clone();
        missing.config_subset["bindings"] = json!([]);
        let output = run_link_case(missing, key).await;
        assert!(
            output
                .relations
                .iter()
                .filter(|relation| relation.from_id == request || relation.from_id == graphql)
                .all(|relation| relation.resolution == Resolution::Candidate)
        );
        assert!(
            output
                .gaps
                .iter()
                .any(|gap| gap.code == "BINDING_EVIDENCE_MISSING")
        );
        let mut missing = input.clone();
        missing
            .relations
            .retain(|relation| relation.kind != RelationKind::Requests);
        let output = run_link_case(missing, key).await;
        assert!(
            output
                .relations
                .iter()
                .filter(|relation| relation.from_id == request)
                .all(|relation| relation.resolution == Resolution::Candidate)
        );
        let mut human = input.clone();
        for entity in &mut human.entities {
            if entity.entity_id == request {
                for assertion in &mut entity.assertions {
                    assertion.basis = project_knowledge_core::contract::Basis::Declared;
                    assertion.origin = project_knowledge_core::contract::Origin::Human;
                }
            }
        }
        let output = run_link_case(human, key).await;
        assert!(
            output
                .relations
                .iter()
                .any(|relation| relation.from_id == request
                    && relation.resolution == Resolution::Confirmed
                    && relation.assertion.basis
                        == project_knowledge_core::contract::Basis::Declared
                    && relation.assertion.origin
                        == project_knowledge_core::contract::Origin::Human)
        );
        let mut incomplete = input.clone();
        incomplete.candidate_sets[0].complete = false;
        let output = run_link_case(incomplete.clone(), key).await;
        assert!(output.relations.is_empty());
        let manifest = manifest::load_manifest(&paths[3]).unwrap();
        let error = project_knowledge_core::collect::validate::validate_output(
            &project_knowledge_core::collect::validate::ValidationContext {
                project_id: &project,
                input: &incomplete,
                manifest: &manifest,
                stage: &manifest.stages[0],
                registry: None,
            },
            &output,
        )
        .unwrap_err();
        assert_eq!(error.code, "CANDIDATE_SET_INCOMPLETE");
        incomplete.candidate_sets.clear();
        let output = run_link_case(incomplete.clone(), key).await;
        let error = project_knowledge_core::collect::validate::validate_output(
            &project_knowledge_core::collect::validate::ValidationContext {
                project_id: &project,
                input: &incomplete,
                manifest: &manifest,
                stage: &manifest.stages[0],
                registry: None,
            },
            &output,
        )
        .unwrap_err();
        assert_eq!(error.code, "CANDIDATE_SET_INCOMPLETE");
        if let Some(directory) = std::env::var_os("PK_SCENARIO_EVIDENCE_DIR") {
            let directory = PathBuf::from(directory);
            std::fs::create_dir_all(&directory).unwrap();
            std::fs::write(
                directory.join(format!("link-{key}-snapshot.json")),
                serde_json::to_vec_pretty(&input).unwrap(),
            )
            .unwrap();
        }
    }
}

#[tokio::test]
async fn linking_new_resolver_proxy_and_datasource_changes_match_clean_rebuild() {
    for key in ["a", "b"] {
        let mut fixture = FrontendFixture::new_for(key, "database").await;
        let repo = if key == "a" { "api" } else { "server/domain" };
        let alternate = if key == "a" {
            "alternate.ts"
        } else {
            "alternate.py"
        };
        let alternate_text = if key == "a" {
            "export function otherLedger() { return []; }\n"
        } else {
            "def otherLedger():\n    return []\n"
        };
        std::fs::write(
            fixture.temp.path().join(repo).join(alternate),
            alternate_text,
        )
        .unwrap();
        fixture.reindex();
        fixture
            .activate_group(&FrontendFixture::group_paths(key))
            .await;
        let first = fixture.link_input(key);
        let registration = if key == "a" {
            "register.ts"
        } else {
            "expose.py"
        };
        let path = fixture.temp.path().join(repo).join(registration);
        let original = std::fs::read_to_string(&path).unwrap();
        let added = if key == "a" {
            "\nimport { otherLedger } from './alternate';\ngraphql.register('Query', 'purchaseSummary', otherLedger);\n"
        } else {
            "\nfrom .alternate import otherLedger\ngraphql.register('Query', 'purchaseSummary', otherLedger)\n"
        };
        std::fs::write(&path, format!("{original}{added}")).unwrap();
        let update = project_knowledge_core::index::run_incremental(
            &fixture.project,
            &mut fixture.database,
            &CancellationToken::new(),
            "changed",
        )
        .await
        .unwrap();
        assert!(
            update
                .executed
                .contains(&format!("example.{key}.link:link"))
        );
        let ambiguous = fixture.link_input(key);
        assert_ne!(
            first.candidate_sets[0].digest,
            ambiguous.candidate_sets[0].digest
        );
        let graphql = truth_entity_id(
            &format!("acceptance.{key}"),
            EntityKind::GraphqlOperation,
            &json!({"serviceId":"erp","operationType":"query","operationName":"PurchaseSummary"}),
        );
        let count:i64=fixture.database.connection().query_row("SELECT COUNT(*) FROM relation_assertions ra JOIN relations r ON r.id=ra.relation_id JOIN current_batches cb ON cb.batch_id=ra.batch_id WHERE r.kind='calls' AND r.from_id=?1 AND ra.resolution='confirmed'",[&graphql],|row|row.get(0)).unwrap();
        assert_eq!(count, 0);
        assert!(fixture.count("SELECT COUNT(*) FROM gaps g JOIN current_batches cb ON cb.batch_id=g.batch_id WHERE g.code='AMBIGUOUS_BINDING'")>0);
        let mut clean = FrontendFixture::new_for(key, "database").await;
        std::fs::write(clean.temp.path().join(repo).join(alternate), alternate_text).unwrap();
        std::fs::write(
            clean.temp.path().join(repo).join(registration),
            format!("{original}{added}"),
        )
        .unwrap();
        clean.reindex();
        clean
            .activate_group(&FrontendFixture::group_paths(key))
            .await;
        compare_normalized(&fixture, &clean, key, "ambiguous");
        std::fs::write(&path, &original).unwrap();
        fixture.replace(
            &format!("{repo}/models.{}", if key == "a" { "ts" } else { "py" }),
            "orm.connect('ds.erp')",
            "orm.connect('ds.analytics')",
        );
        let client = if key == "a" {
            "web/transport.ts"
        } else {
            "client/ui/client.js"
        };
        fixture.replace(client,"serviceId: 'erp'","serviceId: 'erp', proxy: { prefix: '/erp', target: 'https://unregistered.test', rewrite: '/api' }");
        project_knowledge_core::index::run_incremental(
            &fixture.project,
            &mut fixture.database,
            &CancellationToken::new(),
            "changed bindings",
        )
        .await
        .unwrap();
        let mut clean = FrontendFixture::new_for(key, "database").await;
        std::fs::write(clean.temp.path().join(repo).join(alternate), alternate_text).unwrap();
        clean.replace(
            &format!("{repo}/models.{}", if key == "a" { "ts" } else { "py" }),
            "orm.connect('ds.erp')",
            "orm.connect('ds.analytics')",
        );
        clean.replace(client,"serviceId: 'erp'","serviceId: 'erp', proxy: { prefix: '/erp', target: 'https://unregistered.test', rewrite: '/api' }");
        clean.reindex();
        clean
            .activate_group(&FrontendFixture::group_paths(key))
            .await;
        compare_normalized(&fixture, &clean, key, "proxy-datasource");
    }
}
