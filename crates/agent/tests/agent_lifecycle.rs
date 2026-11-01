//! Provider protocol, bounded lifecycle and real Core activation (no business truth claims).
use project_knowledge_agent::{
    config::{Environment, validate},
    runner::{self, RunRequest},
};
use project_knowledge_core::{
    contract::{
        EntityKind,
        error::codes,
        identity::{IdentitySlot, compute_entity_id},
    },
    project::config::LoadedProject,
    storage::Database,
};
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread,
    time::Duration,
};
use tokio_util::sync::CancellationToken;

const SECRET: &str = "test-only-credential-never-persist";
struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    config: PathBuf,
    env: PathBuf,
}
impl Fixture {
    fn new(base: &str) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().to_path_buf();
        std::fs::create_dir(root.join("repo")).unwrap();
        std::fs::create_dir(root.join(".project-knowledge")).unwrap();
        std::fs::write(root.join("repo/a.ts"), "export const smoke = 1;\n").unwrap();
        std::fs::write(root.join("repo/.env"), format!("SECRET={SECRET}\n")).unwrap();
        let config = root.join(".project-knowledge/project.json");
        let env = root.join("test.env");
        std::fs::write(&env, format!("AGENT_TEST_SECRET='{SECRET}'\n")).unwrap();
        std::fs::write(&config,serde_json::to_vec(&json!({"schemaVersion":1,"projectId":"agent-test","repositories":[{"id":"r","root":"../repo"}],"selection":{},"agent":{"enabled":true,"provider":"openai-compatible","model":"test-model","baseUrl":base,"apiKeyEnv":"AGENT_TEST_SECRET","timeoutSeconds":60,"maxInputChars":180000}})).unwrap()).unwrap();
        let fixture = Self {
            _temp: temp,
            root,
            config,
            env,
        };
        fixture.index();
        fixture
    }
    fn project(&self) -> LoadedProject {
        LoadedProject::load(&self.config).unwrap()
    }
    fn edit(&self, change: impl FnOnce(&mut Value)) {
        let mut value: Value =
            serde_json::from_slice(&std::fs::read(&self.config).unwrap()).unwrap();
        change(&mut value);
        std::fs::write(&self.config, serde_json::to_vec(&value).unwrap()).unwrap();
    }
    fn index(&self) {
        let p = self.project();
        let mut db = Database::open_maintenance(
            &p.base_dir.join("index.sqlite"),
            "agent-test",
            &p.config_hash().unwrap(),
            "now",
        )
        .unwrap();
        project_knowledge_core::index::build_base_index(&p, &mut db, "now").unwrap();
    }
    async fn run(&self, request: RunRequest, cancel: CancellationToken) -> Value {
        let p = self.project();
        let env = Environment::load(&p, Some(&self.env)).unwrap();
        runner::run(p, env, request, cancel).await.unwrap()
    }
}
fn request(smoke: bool) -> RunRequest {
    RunRequest {
        goal: "Collect source-driven routes".into(),
        focus: Some("routes".into()),
        with_skill: false,
        resume: None,
        repair_plugin: None,
        smoke,
    }
}

struct Server {
    url: String,
    count: Arc<AtomicUsize>,
    requests: Arc<Mutex<Vec<Value>>>,
    stop: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}
impl Server {
    fn new(mut respond: impl FnMut(usize, Value) -> (u16, Value) + Send + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}/v1", listener.local_addr().unwrap());
        let count = Arc::new(AtomicUsize::new(0));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let (c, r, s) = (count.clone(), requests.clone(), stop.clone());
        let handle = thread::spawn(move || {
            while !s.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let Some(body) = read_request(&mut stream) else {
                            continue;
                        };
                        let i = c.fetch_add(1, Ordering::SeqCst);
                        r.lock().unwrap().push(body.clone());
                        let (status, value) = respond(i, body);
                        let text = value.to_string();
                        let response = format!(
                            "HTTP/1.1 {status} Response\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}",
                            text.len()
                        );
                        let _ = stream.write_all(response.as_bytes());
                        let _ = stream.shutdown(std::net::Shutdown::Write);
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5))
                    }
                    Err(_) => break,
                }
            }
        });
        Self {
            url,
            count,
            requests,
            stop,
            handle: Some(handle),
        }
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.handle.take().unwrap().join().unwrap();
    }
}
fn read_request(stream: &mut TcpStream) -> Option<Value> {
    // Windows accepted sockets inherit the listener's nonblocking flag.
    stream.set_nonblocking(false).ok()?;
    stream.set_read_timeout(Some(Duration::from_secs(3))).ok()?;
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        let n = match stream.read(&mut chunk) {
            Ok(n) => n,
            Err(error) => {
                eprintln!("mock request read: {error}; bytes={}", bytes.len());
                return None;
            }
        };
        if n == 0 {
            eprintln!("mock request EOF before body: {} bytes", bytes.len());
            return None;
        }
        bytes.extend_from_slice(&chunk[..n]);
        if let Some(offset) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
            let headers = String::from_utf8_lossy(&bytes[..offset]);
            let len = headers.lines().find_map(|l| {
                l.to_ascii_lowercase()
                    .strip_prefix("content-length:")
                    .map(str::trim)
                    .and_then(|v| v.parse::<usize>().ok())
            });
            let Some(len) = len else {
                eprintln!(
                    "mock missing length; header names: {:?}",
                    headers
                        .lines()
                        .map(|l| l.split(':').next().unwrap_or(""))
                        .collect::<Vec<_>>()
                );
                return None;
            };
            if bytes.len() >= offset + 4 + len {
                return match serde_json::from_slice(&bytes[offset + 4..offset + 4 + len]) {
                    Ok(value) => Some(value),
                    Err(error) => {
                        eprintln!("mock invalid JSON: {error}; bytes={}", bytes.len());
                        None
                    }
                };
            }
        }
        if bytes.len() > 2_000_000 {
            return None;
        }
    }
}
fn reply(tool: Option<(&str, Value)>) -> (u16, Value) {
    let (message, reason) = if let Some((name, args)) = tool {
        (
            json!({"role":"assistant","content":null,"tool_calls":[{"id":format!("call-{name}"),"type":"function","function":{"name":name,"arguments":args.to_string()}}]}),
            "tool_calls",
        )
    } else {
        (json!({"role":"assistant","content":"OK"}), "stop")
    };
    (
        200,
        json!({"id":"mock-response","object":"chat.completion","created":1,"model":"test-model","choices":[{"index":0,"message":message,"finish_reason":reason}],"usage":{"prompt_tokens":10,"completion_tokens":10,"total_tokens":20}}),
    )
}
fn last_tool(body: &Value) -> Value {
    body["messages"]
        .as_array()
        .unwrap()
        .iter()
        .rev()
        .find(|m| m["role"] == "tool")
        .and_then(|m| m["content"].as_str())
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or(Value::Null)
}

#[test]
fn config_dotenv_validation_precedence_and_secret_boundaries() {
    let f = Fixture::new("http://localhost:1/v1");
    std::fs::write(&f.env,format!("\u{feff}export AGENT_TEST_SECRET=\"{SECRET}\" # private\nPROJECT_KNOWLEDGE_PROVIDER=anthropic\nPROJECT_KNOWLEDGE_MODEL='configured-model'\nPROJECT_KNOWLEDGE_BASE_URL=https://example.test/apps/anthropic\nPATH=must-not-override-process\n")).unwrap();
    let env = Environment::load(&f.project(), Some(&f.env)).unwrap();
    let config = env
        .configuration(f.project().config.agent.as_ref())
        .unwrap();
    assert_eq!(config.provider, "anthropic");
    assert_eq!(config.model, "configured-model");
    assert_eq!(env.credential(&config).unwrap(), SECRET);
    assert_eq!(env.get("PATH"), std::env::var("PATH").ok());
    validate(&config).unwrap();
    project_knowledge_agent::config::save(&f.project(), config.clone()).unwrap();
    assert!(!std::fs::read_to_string(&f.config).unwrap().contains(SECRET));
    for (value, pointer) in [
        (json!({"model":""}), "/agent/model"),
        (json!({"provider":"unknown"}), "/agent/provider"),
        (
            json!({"baseUrl":"https://user:password@example.test"}),
            "/agent/baseUrl",
        ),
        (json!({"maxTurns":0}), "/agent/maxTurns"),
        (json!({"maxRepairAttempts":3}), "/agent/maxRepairAttempts"),
    ] {
        let mut v = serde_json::to_value(&config).unwrap();
        for (k, vv) in value.as_object().unwrap() {
            v[k] = vv.clone();
        }
        let error = validate(&serde_json::from_value(v).unwrap()).unwrap_err();
        assert_eq!(error.issues[0].instance_path, pointer);
    }
    std::fs::write(&f.env, "INVALID ENTRY '").unwrap();
    let error = Environment::load(&f.project(), Some(&f.env)).err().unwrap();
    assert!(!error.message.contains("INVALID ENTRY"));
    let options = project_knowledge_core::project::scan::ScanOptions::default();
    let scan =
        project_knowledge_core::project::scan::scan_repository("r", &f.root.join("repo"), &options)
            .unwrap();
    assert!(scan.sources.iter().all(|s| s.path != ".env"
        || s.selection_status == project_knowledge_core::project::scan::SelectionStatus::Excluded));
}

#[tokio::test]
async fn tools_roundtrip_and_no_tool_model_is_not_success() {
    let server = Server::new(|i, _| {
        if i == 0 {
            reply(Some(("status", json!({}))))
        } else {
            reply(None)
        }
    });
    let f = Fixture::new(&server.url);
    let result = f.run(request(true), CancellationToken::new()).await;
    assert_eq!(
        result["status"],
        "complete",
        "result={result:#}, requests={}",
        server.count.load(Ordering::SeqCst)
    );
    assert_eq!(result["modelTurns"], 2);
    assert_eq!(result["toolCalls"], 1);
    assert!(
        !server.requests.lock().unwrap()[0]
            .to_string()
            .contains(SECRET)
    );
    assert!(!result.to_string().contains(SECRET));
    assert!(
        !std::fs::read_to_string(
            f.project()
                .base_dir
                .join("work/agent")
                .join(result["runId"].as_str().unwrap())
                .join("run.json")
        )
        .unwrap()
        .contains(SECRET)
    );
    let no_tools = Server::new(|_, _| reply(None));
    let f = Fixture::new(&no_tools.url);
    let result = f.run(request(true), CancellationToken::new()).await;
    assert_eq!(result["status"], "failed");
    assert_eq!(result["issues"][0]["error"]["code"], "AGENT_INCOMPLETE");
}

#[tokio::test]
async fn config_provider_errors_cancellation_timeout_and_turn_budget() {
    for (status, code) in [
        (401, "MODEL_AUTH_FAILED"),
        (429, "MODEL_RATE_LIMITED"),
        (400, "MODEL_ENDPOINT_INCOMPATIBLE"),
    ] {
        let server =
            Server::new(move |_, _| (status, json!({"error":{"message":SECRET,"type":"test"}})));
        let f = Fixture::new(&server.url);
        let result = f.run(request(true), CancellationToken::new()).await;
        assert_eq!(result["status"], "failed");
        assert_eq!(result["issues"][0]["error"]["code"], code);
        assert!(!result.to_string().contains(SECRET));
        assert_eq!(server.count.load(Ordering::SeqCst), 1);
    }
    let server = Server::new(|_, _| reply(Some(("status", json!({})))));
    let f = Fixture::new(&server.url);
    f.edit(|v| v["agent"]["maxTurns"] = 1.into());
    let result = f.run(request(true), CancellationToken::new()).await;
    assert_eq!(
        result["issues"].as_array().unwrap().last().unwrap()["error"]["code"],
        codes::BUDGET_EXHAUSTED
    );
    assert_eq!(server.count.load(Ordering::SeqCst), 1);
    let cancel = CancellationToken::new();
    cancel.cancel();
    let p = f.project();
    let env = Environment::load(&p, Some(&f.env)).unwrap();
    let error = runner::run(p, env, request(true), cancel)
        .await
        .unwrap_err();
    assert_eq!(error.code, codes::CANCELLED);
    let slow = Server::new(|_, _| {
        thread::sleep(Duration::from_millis(1300));
        reply(None)
    });
    let f = Fixture::new(&slow.url);
    f.edit(|v| v["agent"]["timeoutSeconds"] = 1.into());
    let result = f.run(request(true), CancellationToken::new()).await;
    assert_eq!(
        result["issues"].as_array().unwrap().last().unwrap()["error"]["code"],
        codes::BUDGET_EXHAUSTED
    );
    let slow = Server::new(|_, _| {
        thread::sleep(Duration::from_millis(300));
        reply(None)
    });
    let f = Fixture::new(&slow.url);
    let cancel = CancellationToken::new();
    let signal = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        signal.cancel()
    });
    let result = f.run(request(true), cancel).await;
    assert_eq!(result["status"], "cancelled");
}

#[tokio::test]
async fn tools_errors_keep_json_pointers_and_budget_is_global() {
    let server = Server::new(|i, body| {
        if i == 0 {
            reply(Some(("search", json!({"query":42}))))
        } else {
            let error = last_tool(&body);
            assert_eq!(error["error"]["issues"][0]["instancePath"], "/query");
            reply(None)
        }
    });
    let f = Fixture::new(&server.url);
    let result = f.run(request(false), CancellationToken::new()).await;
    assert_eq!(result["status"], "failed");
    assert_eq!(
        result["issues"][0]["error"]["issues"][0]["instancePath"],
        "/query"
    );
    let server = Server::new(|_, _| reply(Some(("status", json!({})))));
    let f = Fixture::new(&server.url);
    f.edit(|v| v["agent"]["maxToolCalls"] = 1.into());
    let result = f.run(request(false), CancellationToken::new()).await;
    assert_eq!(result["status"], "failed");
    assert_eq!(result["toolCalls"], 1);
    assert_eq!(result["modelTurns"], 2);
    assert_eq!(server.count.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn tools_entity_and_relation_identity_and_qualifiers_use_core_contract() {
    let server = Server::new(|i, body| match i {
        0 => reply(Some((
            "get-capabilities",
            json!({"schema":"identity","kind":"workflowState"}),
        ))),
        1 => {
            let result = last_tool(&body);
            assert_eq!(result["identityComponents"][1]["name"], "workflowId");
            reply(Some((
                "get-capabilities",
                json!({"schema":"identity","kind":"workflowState","slots":{"scopeId":"repo.r","workflowId":"sample","semanticKey":"START"}}),
            )))
        }
        2 => {
            let result = last_tool(&body);
            assert!(result["entityId"].as_str().unwrap().starts_with("ent_"));
            reply(Some((
                "get-capabilities",
                json!({"schema":"identity","kind":"transitions_to","fromId":result["entityId"],"toId":result["entityId"],"qualifiers":{"trigger":"retry","guard":"allowed"}}),
            )))
        }
        3 => {
            let result = last_tool(&body);
            let slots = [
                IdentitySlot::required("scopeId", "repo.r"),
                IdentitySlot::required("workflowId", "sample"),
                IdentitySlot::required("semanticKey", "START"),
            ];
            let entity =
                compute_entity_id("agent-test", EntityKind::WorkflowState, &slots).unwrap();
            let qualifiers = json!({"trigger":"retry","guard":"allowed"})
                .as_object()
                .unwrap()
                .clone();
            let expected = project_knowledge_core::contract::identity::compute_relation_id(
                "agent-test",
                project_knowledge_core::contract::RelationKind::TransitionsTo,
                &entity,
                &entity,
                &qualifiers,
            )
            .unwrap();
            assert_eq!(result["relationId"], expected);
            reply(Some((
                "get-capabilities",
                json!({"schema":"relation","kind":"transitions_to"}),
            )))
        }
        4 => {
            let result = last_tool(&body);
            assert_eq!(result["schema"]["additionalProperties"], false);
            assert!(result["schema"]["properties"]["guard"].is_object());
            assert!(result["schema"]["properties"].get("orderedKeys").is_none());
            evidence("transition-qualifiers-schema.json", &result);
            reply(Some((
                "get-capabilities",
                json!({"schema":"relation","kind":"calls"}),
            )))
        }
        _ => {
            assert_eq!(
                last_tool(&body)["error"]["issues"][0]["instancePath"],
                "/kind"
            );
            reply(None)
        }
    });
    let f = Fixture::new(&server.url);
    let result = f.run(request(false), CancellationToken::new()).await;
    assert_eq!(result["status"], "failed");
    assert_eq!(result["toolCalls"], 5);
}

fn candidate_files(input: &Value, bad: bool) -> Value {
    let mut input = input.clone();
    input["stageId"] = "extract".into();
    let expected = compute_entity_id(
        "agent-test",
        EntityKind::Route,
        &[
            IdentitySlot::required("scopeId", "repo.r"),
            IdentitySlot::required("routerId", "router"),
            IdentitySlot::required("mode", "history"),
            IdentitySlot::required("declaredPath", "/smoke"),
        ],
    )
    .unwrap();
    let manifest = json!({"protocolVersion":"1.0","pluginId":"agent.fixture","version":"1.0.0","capabilities":["routes"],"runtime":{"kind":"node","versionRange":">=24 <25"},"artifacts":["collect.mjs","identity.mjs"],"stages":[{"id":"extract","phase":"extract","entry":"collect.mjs","args":if bad{vec!["--bad"]}else{vec![]},"replacement":"project","sourceSelectors":[{"id":"context","include":["**/*.ts"]}]}],"acceptanceCases":["case.json"]});
    json!({"includeIdentityHelper":true,"files":[{"path":"manifest.json","content":manifest.to_string()},{"path":"collect.mjs","content":include_str!("../../../fixtures/plugins/staging/collect.mjs")},{"path":"input.json","content":input.to_string()},{"path":"case.json","content":json!({"id":"required-route","inputFixture":"input.json","expected":{"entities":[expected]},"truthRef":"a.ts","truthHash":input["sources"][0]["contentHash"]}).to_string()}]})
}
fn lifecycle_server(repair: bool) -> Server {
    lifecycle_server_with_knowledge(repair, false, false)
}
fn lifecycle_server_with_knowledge(repair: bool, knowledge: bool, invalid_topic: bool) -> Server {
    lifecycle_scenario(repair, knowledge, invalid_topic, None, None)
}
fn lifecycle_scenario(
    repair: bool,
    knowledge: bool,
    invalid_topic: bool,
    pause: Option<(usize, Option<CancellationToken>)>,
    finish_change: Option<Arc<Mutex<PathBuf>>>,
) -> Server {
    let mut references = Value::Null;
    let mut input = Value::Null;
    Server::new(move |i, body| {
        let i = if let Some((step, cancel)) = &pause {
            if i == *step {
                if let Some(cancel) = cancel {
                    cancel.cancel();
                }
                return reply(None);
            }
            // Candidate recovery validates on the host before returning to the model.
            if i > *step {
                i - 1 + usize::from(*step == 5)
            } else {
                i
            }
        } else {
            i
        };
        if i == if knowledge { 9 } else { 6 }
            && let Some(root) = &finish_change
        {
            std::fs::write(
                root.lock().unwrap().join("repo/a.ts"),
                "changed while the model was finishing\n",
            )
            .unwrap();
        }
        match i {
            0 => reply(Some(("code-map", json!({})))),
            1 => reply(Some(("export-snapshot", json!({"include":["**/*.ts"]})))),
            2 => {
                let result = last_tool(&body);
                references = result["sourceReferences"].clone();
                input = result["input"].clone();
                assert!(!references.as_array().unwrap().is_empty());
                reply(Some((
                    "read-source",
                    json!({"sourceId":references[0]["sourceId"]}),
                )))
            }
            3 => reply(Some((
                "import-conventions",
                json!({"notes":"Fixture route evidence; mock tests mechanics only","sourceIds":[references[0]["sourceId"]]}),
            ))),
            4 => reply(Some(("write-candidate", candidate_files(&input, repair)))),
            5 => reply(Some(("validate-plugin", json!({})))),
            6 if repair => {
                assert!(last_tool(&body).get("error").is_some());
                reply(Some(("write-candidate", candidate_files(&input, false))))
            }
            7 if repair => reply(Some(("validate-plugin", json!({})))),
            6 if knowledge => reply(Some((
                "export-topic-context",
                json!({"topicId":"smoke","query":"smoke","maxChars":40_000}),
            ))),
            7 if knowledge => {
                let context = last_tool(&body);
                assert!(context["inputDigest"].is_string(), "context: {context}");
                let reference = context["sourceReferences"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .max_by_key(|r| {
                        r["endByte"].as_u64().unwrap() - r["startByte"].as_u64().unwrap()
                    })
                    .unwrap();
                let mut artifact = json!({"protocolVersion":"1.0","topicId":"smoke","title":"Fixture source declaration","snapshot":context["snapshot"],"inputDigest":context["inputDigest"],"promptHash":context["promptHash"],"templateHash":context["templateHash"],"markdown":"The fixture declares the smoke constant. [[claim:declaration]]","claims":[{"id":"declaration","text":"The fixture declares the smoke constant.","basis":"observed","origin":"human","evidenceRefs":[reference["evidenceId"]],"gapIds":[]}],"sourceReferences":context["sourceReferences"],"gaps":context["gaps"]});
                if invalid_topic {
                    artifact["claims"][0]["evidenceRefs"][0] = 42.into();
                }
                reply(Some(("import-topic", artifact)))
            }
            8 if knowledge => {
                if invalid_topic {
                    let error = last_tool(&body);
                    assert_eq!(
                        error["error"]["issues"][0]["instancePath"],
                        "/claims/0/evidenceRefs/0"
                    );
                    reply(None)
                } else {
                    assert!(last_tool(&body)["version"].is_number());
                    reply(Some(("export-skill", json!({"name":"fixture-knowledge"}))))
                }
            }
            _ => reply(None),
        }
    })
}

fn evidence(name: &str, value: &Value) {
    if let Some(path) = std::env::var_os("PK_AGENT_EVIDENCE_DIR") {
        let path = PathBuf::from(path);
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(path.join(name), serde_json::to_vec_pretty(value).unwrap()).unwrap();
    }
}

#[tokio::test]
async fn tools_knowledge_roundtrip_is_bound_to_core_activation_and_global_budget() {
    for limit in [80, 8] {
        let server = lifecycle_server_with_knowledge(false, true, false);
        let f = Fixture::new(&server.url);
        f.edit(|v| v["agent"]["maxToolCalls"] = limit.into());
        let mut req = request(false);
        req.with_skill = true;
        let report = f.run(req, CancellationToken::new()).await;
        assert!(report["activation"].is_object(), "{report:#}");
        assert_eq!(report["topics"].as_array().unwrap().len(), 1);
        assert!(!report.to_string().contains(SECRET));
        let project = f.project();
        let db = Database::open_readonly(&project.base_dir.join("index.sqlite")).unwrap();
        assert_eq!(
            project_knowledge_core::storage::knowledge::conventions(db.connection())
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            project_knowledge_core::storage::knowledge::topics(db.connection())
                .unwrap()
                .len(),
            1
        );
        if limit == 80 {
            assert_eq!(report["status"], "complete", "{report:#}");
            assert_eq!(report["modelTurns"], 10);
            assert_eq!(report["toolCalls"], 9);
            assert_eq!(report["skill"]["freshness"], "fresh");
            let directory = PathBuf::from(report["skill"]["directory"].as_str().unwrap());
            let manifest: Value = serde_json::from_slice(
                &std::fs::read(directory.join("references/index-manifest.json")).unwrap(),
            )
            .unwrap();
            for (name, hash) in manifest["files"].as_object().unwrap() {
                assert_eq!(
                    project_knowledge_core::contract::content_hash(
                        &std::fs::read(directory.join(name)).unwrap()
                    ),
                    hash.as_str().unwrap()
                );
            }
            let requests = server.requests.lock().unwrap();
            let definitions = &requests[0]["tools"];
            assert!(
                definitions
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|t| t["function"]["name"] == "export-topic-context")
            );
            assert!(requests.iter().all(|r| !r.to_string().contains(SECRET)));
            evidence("knowledge-roundtrip.json", &report);
            evidence("tool-definitions.json", definitions);
        } else {
            assert_eq!(report["status"], "partial");
            assert_eq!(report["toolCalls"], 8);
            assert_eq!(report["modelTurns"], 9);
            assert!(report["skill"].is_null());
            assert!(
                report["issues"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|v| v["error"]["code"] == "BUDGET_EXHAUSTED")
            );
            evidence("knowledge-budget.json", &report);
        }
    }
}

#[tokio::test]
async fn tools_knowledge_invalid_artifact_preserves_pointer_and_never_exports() {
    let server = lifecycle_server_with_knowledge(false, true, true);
    let f = Fixture::new(&server.url);
    let mut req = request(false);
    req.with_skill = true;
    let report = f.run(req, CancellationToken::new()).await;
    assert_eq!(report["status"], "partial");
    assert!(report["activation"].is_object());
    assert!(report["topics"].as_array().unwrap().is_empty());
    assert!(report["skill"].is_null());
    assert!(
        report["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["error"]["issues"][0]["instancePath"] == "/claims/0/evidenceRefs/0")
    );
    let p = f.project();
    let db = Database::open_readonly(&p.base_dir.join("index.sqlite")).unwrap();
    assert!(
        project_knowledge_core::storage::knowledge::topics(db.connection())
            .unwrap()
            .is_empty()
    );
    evidence("knowledge-invalid.json", &report);
}

#[tokio::test]
async fn tools_conventions_identity_cas_registers_original_provenance() {
    for variant in 0..3 {
        let wrong_hash = variant == 1;
        let missing_hash = variant == 2;
        let mut references = Value::Null;
        let server = Server::new(move |i, body| match i {
            0 => reply(Some(("code-map", json!({})))),
            1 => reply(Some(("export-snapshot", json!({})))),
            2 => {
                references = last_tool(&body)["sourceReferences"].clone();
                reply(Some((
                    "read-source",
                    json!({"sourceId":references[0]["sourceId"]}),
                )))
            }
            3 => reply(Some(("get-capabilities", json!({"schema":"conventions"})))),
            4 => {
                let capabilities = last_tool(&body);
                assert_eq!(capabilities["knowledgeAvailable"], true);
                assert_eq!(capabilities["identityImportAvailable"], true);
                let mut args = json!({"notes":"An explicit test module grounded in the original source.","sourceIds":[references[0]["sourceId"]],"expectedConfigHash":if wrong_hash{json!("wrong")}else{capabilities["configHash"].clone()},"identityDelta":{"scopes":[{"id":"fixture.module","kind":"module","repositoryIds":["r"]}]}});
                if missing_hash {
                    args.as_object_mut().unwrap().remove("expectedConfigHash");
                }
                reply(Some(("import-conventions", args)))
            }
            5 if !wrong_hash && !missing_hash => {
                assert_eq!(last_tool(&body)["identityChangesApplied"], true);
                reply(Some(("update-base", json!({}))))
            }
            _ => reply(None),
        });
        let f = Fixture::new(&server.url);
        let original = f.project().config_hash().unwrap();
        let report = f.run(request(false), CancellationToken::new()).await;
        assert_eq!(
            report["status"], "failed",
            "tool-only test must not claim activation"
        );
        let project = f.project();
        let db = Database::open_readonly(&project.base_dir.join("index.sqlite")).unwrap();
        let conventions =
            project_knowledge_core::storage::knowledge::conventions(db.connection()).unwrap();
        if wrong_hash || missing_hash {
            assert_eq!(project.config_hash().unwrap(), original);
            assert!(conventions.is_empty());
            assert!(
                report["issues"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|e| e["error"]["code"]
                        == if wrong_hash {
                            "ACTIVATION_CONFLICT"
                        } else {
                            "REFERENCE_MISSING"
                        })
            );
        } else {
            assert_ne!(project.config_hash().unwrap(), original);
            assert!(
                project
                    .config
                    .scopes
                    .iter()
                    .any(|s| s.id == "fixture.module")
            );
            assert_eq!(conventions.len(), 1);
            assert_eq!(conventions[0]["sourceClass"], "derivedConvention");
            assert_eq!(report["configHash"], project.config_hash().unwrap());
            assert_eq!(
                report["sourceSnapshotId"],
                db.current_source_snapshot_id().unwrap().unwrap()
            );
            evidence(
                "identity-import.json",
                &json!({"progress":report,"conventions":conventions}),
            );
        }
    }
}

#[tokio::test]
async fn lifecycle_repairs_validates_activates_and_runs_without_model() {
    let server = lifecycle_server(true);
    let f = Fixture::new(&server.url);
    let result = f.run(request(false), CancellationToken::new()).await;
    assert_eq!(result["status"], "complete", "{result:#}");
    assert_eq!(result["repairs"], 1);
    assert_eq!(result["revision"], 2);
    assert!(result["validationId"].is_string());
    assert_eq!(result["activation"]["status"], "active");
    let schemas = server.requests.lock().unwrap()[0]["tools"].clone();
    let write = schemas
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["function"]["name"] == "write-candidate")
        .unwrap();
    assert_eq!(
        write["function"]["parameters"]["properties"]["files"]["items"]["properties"]["content"]["type"],
        "string"
    );
    let map = schemas
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["function"]["name"] == "code-map")
        .unwrap();
    assert_eq!(
        map["function"]["parameters"]["properties"]["maxFiles"]["type"],
        "integer"
    );
    let calls = server.count.load(Ordering::SeqCst);
    let p = f.project();
    let mut db = Database::open_maintenance(
        &p.base_dir.join("index.sqlite"),
        "agent-test",
        &p.config_hash().unwrap(),
        "now",
    )
    .unwrap();
    std::fs::write(f.root.join("repo/a.ts"), "export const smoke = 2;\n").unwrap();
    let incremental = project_knowledge_core::index::incremental::run_incremental(
        &p,
        &mut db,
        &CancellationToken::new(),
        "later",
    )
    .await
    .unwrap();
    assert_eq!(incremental.executed, vec!["agent.fixture:extract"]);
    assert_eq!(server.count.load(Ordering::SeqCst), calls);
}

#[tokio::test]
async fn lifecycle_active_repair_retains_verified_files_without_model_echo() {
    let mut input = Value::Null;
    let mut refs = Value::Null;
    let server = Server::new(move |i, body| match i % 7 {
        0 => reply(Some(("code-map", json!({})))),
        1 => reply(Some(("export-snapshot", json!({"include":["**/*.ts"]})))),
        2 => {
            input = last_tool(&body)["input"].clone();
            refs = last_tool(&body)["sourceReferences"].clone();
            reply(Some((
                "read-source",
                json!({"sourceId":refs[0]["sourceId"]}),
            )))
        }
        3 => reply(Some((
            "import-conventions",
            json!({"notes":"current original fixture","sourceIds":[refs[0]["sourceId"]]}),
        ))),
        4 if i < 7 => reply(Some(("write-candidate", candidate_files(&input, false)))),
        4 => {
            let original = candidate_files(&input, false);
            let mut manifest: Value =
                serde_json::from_str(original["files"][0]["content"].as_str().unwrap()).unwrap();
            manifest["version"] = "2.0.0".into();
            reply(Some((
                "write-candidate",
                json!({"files":[{"path":"manifest.json","content":manifest.to_string()}]}),
            )))
        }
        5 => reply(Some(("validate-plugin", json!({})))),
        _ => reply(None),
    });
    let f = Fixture::new(&server.url);
    let first = f.run(request(false), CancellationToken::new()).await;
    assert_eq!(first["status"], "complete");
    let mut req = request(false);
    req.repair_plugin = Some("agent.fixture".into());
    let repaired = f.run(req, CancellationToken::new()).await;
    assert_eq!(repaired["status"], "complete", "{repaired:#}");
    assert_eq!(
        repaired["activeVersions"],
        json!([["agent.fixture", "2.0.0"]])
    );
    let project = f.project();
    let before = project
        .base_dir
        .join("work/agent")
        .join(first["runId"].as_str().unwrap())
        .join("revision-001");
    let after = project
        .base_dir
        .join("work/agent")
        .join(repaired["runId"].as_str().unwrap())
        .join("revision-001");
    for path in ["collect.mjs", "identity.mjs", "case.json", "input.json"] {
        assert_eq!(
            std::fs::read(before.join(path)).unwrap(),
            std::fs::read(after.join(path)).unwrap()
        );
    }
    evidence("active-repair-retained-files.json", &repaired);
}

#[tokio::test]
async fn lifecycle_pending_activation_and_skill_partial() {
    let server = lifecycle_server(false);
    let f = Fixture::new(&server.url);
    f.edit(|v| v["agent"]["autoActivate"] = false.into());
    let result = f.run(request(false), CancellationToken::new()).await;
    assert_eq!(result["status"], "pending_activation", "{result:#}");
    assert!(result["activation"].is_null());
    assert!(result["validationId"].is_string());
    let server = lifecycle_server(false);
    let f = Fixture::new(&server.url);
    let mut req = request(false);
    req.with_skill = true;
    let result = f.run(req, CancellationToken::new()).await;
    assert_eq!(result["status"], "partial", "{result:#}");
    assert!(result["activation"].is_object());
    assert_eq!(
        result["issues"].as_array().unwrap().last().unwrap()["error"]["code"],
        "KNOWLEDGE_INCOMPLETE"
    );
}

#[tokio::test]
async fn lifecycle_resume_rechecks_sources_and_retains_spent_budget() {
    let server = Server::new(|_, _| reply(None));
    let f = Fixture::new(&server.url);
    let result = f.run(request(false), CancellationToken::new()).await;
    let turns = result["modelTurns"].as_u64().unwrap();
    std::fs::create_dir_all(f.root.join("repo/.project-knowledge/work/agent")).unwrap();
    std::fs::write(
        f.root.join("repo/.project-knowledge/work/agent/run.json"),
        "generated control output",
    )
    .unwrap();
    let mut req = request(false);
    req.resume = result["runId"].as_str().map(str::to_string);
    req.goal = "Continue investigating current original sources; retain the original budget".into();
    let result = f.run(req, CancellationToken::new()).await;
    assert_eq!(result["modelTurns"], turns + 1);
    assert!(
        result["goal"]
            .as_str()
            .unwrap()
            .contains("Resume guidance (latest)")
    );
    assert_eq!(
        result["events"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["event"] == "resume-guidance")
            .count(),
        1
    );
    std::fs::write(f.root.join("repo/a.ts"), "changed\n").unwrap();
    let p = f.project();
    let env = Environment::load(&p, Some(&f.env)).unwrap();
    let mut req = request(false);
    req.resume = result["runId"].as_str().map(str::to_string);
    let error = runner::run(p, env, req, CancellationToken::new())
        .await
        .unwrap_err();
    assert_eq!(error.code, codes::DEPENDENCY_CHANGED);
}

#[tokio::test]
async fn lifecycle_resume_finishes_knowledge_after_candidate_activation_or_cancellation() {
    for (step, interrupted, repair_limit) in
        [(5, false, 0), (5, false, 2), (6, false, 2), (6, true, 2)]
    {
        let cancel = CancellationToken::new();
        let server = lifecycle_scenario(
            false,
            true,
            false,
            Some((step, interrupted.then(|| cancel.clone()))),
            None,
        );
        let f = Fixture::new(&server.url);
        f.edit(|v| v["agent"]["maxRepairAttempts"] = repair_limit.into());
        let mut req = request(false);
        req.with_skill = true;
        let before = f.run(req, cancel).await;
        assert_eq!(
            before["status"],
            if step == 5 { "failed" } else { "partial" },
            "{before:#}"
        );
        let mut req = request(false);
        req.resume = before["runId"].as_str().map(str::to_string);
        let resumed = f.run(req, CancellationToken::new()).await;
        assert_eq!(resumed["status"], "complete", "{resumed:#}");
        assert_eq!(resumed["skill"]["freshness"], "fresh");
        assert_eq!(resumed["repairs"], before["repairs"]);
        assert_eq!(resumed["deadlineMs"], before["deadlineMs"]);
        assert!(resumed["modelTurns"].as_u64().unwrap() > before["modelTurns"].as_u64().unwrap());
        let project = f.project();
        let db = Database::open_readonly(&project.base_dir.join("index.sqlite")).unwrap();
        let activations: i64 = db
            .connection()
            .query_row(
                "SELECT count(*) FROM activations WHERE status='committed'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(activations, 1);
        drop(db);
        let calls = server.count.load(Ordering::SeqCst);
        let mut req = request(false);
        req.resume = resumed["runId"].as_str().map(str::to_string);
        let repeated = f.run(req, CancellationToken::new()).await;
        assert_eq!(repeated["modelTurns"], resumed["modelTurns"]);
        assert_eq!(server.count.load(Ordering::SeqCst), calls);
        evidence(
            &format!("resume-knowledge-{step}-{interrupted}-{repair_limit}.json"),
            &resumed,
        );
    }
}

#[tokio::test]
async fn lifecycle_resume_recovers_committed_activation_and_rejects_unrelated_versions() {
    for conflict in [false, true] {
        let server = lifecycle_server(false);
        let f = Fixture::new(&server.url);
        let report = f.run(request(false), CancellationToken::new()).await;
        assert_eq!(report["status"], "complete", "{report:#}");
        let path = f
            .project()
            .base_dir
            .join("work/agent")
            .join(report["runId"].as_str().unwrap())
            .join("run.json");
        // Exact durable state immediately before the Core activation commit.
        let mut interrupted = report.clone();
        interrupted["activation"] = Value::Null;
        interrupted["activeVersions"] = json!([]);
        interrupted["status"] = "running".into();
        interrupted["phase"] = "activate".into();
        std::fs::write(path, serde_json::to_vec(&interrupted).unwrap()).unwrap();
        if conflict {
            let project = f.project();
            let db = Database::open_maintenance(
                &project.base_dir.join("index.sqlite"),
                "agent-test",
                &project.config_hash().unwrap(),
                "later",
            )
            .unwrap();
            let mut db = db;
            let mut files = project_knowledge_core::collect::candidate::read_bundle(
                &project,
                report["runId"].as_str().unwrap(),
                1,
            )
            .unwrap();
            let manifest = files
                .iter_mut()
                .find(|f| f.path == "manifest.json")
                .unwrap();
            let mut value: Value = serde_json::from_str(&manifest.content).unwrap();
            value["version"] = "2.0.0".into();
            manifest.content = value.to_string();
            let directory = project_knowledge_core::collect::candidate::write_bundle(
                &project,
                "agent_external",
                1,
                &files,
            )
            .unwrap();
            use project_knowledge_core::collect::registry;
            let other = registry::submit(&project, &mut db, &directory, "later").unwrap();
            registry::prepare(&project, &db, &other.id).await.unwrap();
            registry::validate(
                &project,
                &mut db,
                &other.id,
                &CancellationToken::new(),
                "later",
            )
            .await
            .unwrap();
            registry::activate(&project, &mut db, &other.id, "later")
                .await
                .unwrap();
        }
        let calls = server.count.load(Ordering::SeqCst);
        let mut req = request(false);
        req.resume = report["runId"].as_str().map(str::to_string);
        let project = f.project();
        let env = Environment::load(&project, Some(&f.env)).unwrap();
        let resumed = runner::run(project, env, req, CancellationToken::new()).await;
        if conflict {
            assert_eq!(resumed.unwrap_err().code, codes::DEPENDENCY_CHANGED);
        } else {
            let resumed = resumed.unwrap();
            assert_eq!(resumed["status"], "complete", "{resumed:#}");
            assert_eq!(resumed["activation"]["alreadyCommitted"], true);
            assert_eq!(resumed["modelTurns"], report["modelTurns"]);
            evidence("resume-committed-activation.json", &resumed);
        }
        assert_eq!(server.count.load(Ordering::SeqCst), calls);
    }
}

#[tokio::test]
async fn lifecycle_complete_report_rechecks_skill_files_and_source_after_model_stops() {
    let server = lifecycle_server_with_knowledge(false, true, false);
    let f = Fixture::new(&server.url);
    let mut req = request(false);
    req.with_skill = true;
    let report = f.run(req, CancellationToken::new()).await;
    assert_eq!(report["status"], "complete", "{report:#}");
    std::fs::write(
        PathBuf::from(report["skill"]["directory"].as_str().unwrap()).join("SKILL.md"),
        "changed after export",
    )
    .unwrap();
    let mut req = request(false);
    req.resume = report["runId"].as_str().map(str::to_string);
    let p = f.project();
    let env = Environment::load(&p, Some(&f.env)).unwrap();
    assert_eq!(
        runner::run(p, env, req, CancellationToken::new())
            .await
            .unwrap_err()
            .code,
        codes::DEPENDENCY_CHANGED
    );

    let root = Arc::new(Mutex::new(PathBuf::new()));
    let server = lifecycle_scenario(false, false, false, None, Some(root.clone()));
    let f = Fixture::new(&server.url);
    *root.lock().unwrap() = f.root.clone();
    let report = f.run(request(false), CancellationToken::new()).await;
    assert_eq!(report["status"], "partial", "{report:#}");
    assert!(report["activation"].is_object());
    assert!(
        report["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["error"]["code"] == codes::DEPENDENCY_CHANGED)
    );
    evidence("completion-source-race.json", &report);
}

#[tokio::test]
async fn config_missing_key_invalid_response_and_input_budget_fail_closed() {
    let server = Server::new(|_, _| (200, json!("malformed-provider-response")));
    let f = Fixture::new(&server.url);
    std::fs::write(&f.env, "UNUSED=1\n").unwrap();
    let p = f.project();
    let env = Environment::load(&p, Some(&f.env)).unwrap();
    let error = runner::run(p, env, request(true), CancellationToken::new())
        .await
        .unwrap_err();
    assert_eq!(error.code, codes::MODEL_CONFIG_INVALID);
    assert_eq!(server.count.load(Ordering::SeqCst), 0);
    std::fs::write(&f.env, format!("AGENT_TEST_SECRET={SECRET}\n")).unwrap();
    let result = f.run(request(true), CancellationToken::new()).await;
    assert_eq!(result["status"], "failed");
    assert_eq!(server.count.load(Ordering::SeqCst), 1);
    f.edit(|v| v["agent"]["maxInputChars"] = 1000.into());
    let result = f.run(request(false), CancellationToken::new()).await;
    assert_eq!(result["status"], "failed");
    assert_eq!(result["modelTurns"], 0);
    assert_eq!(server.count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn tools_invalid_large_bundle_keeps_bounded_error_pointers() {
    let invalid = SECRET.repeat(5000);
    let server = Server::new(move |i, body| {
        if i == 0 {
            reply(Some(("write-candidate", json!({"files":invalid}))))
        } else {
            let error = last_tool(&body);
            assert_eq!(error["error"]["issues"][0]["instancePath"], "/files");
            assert!(error.to_string().len() < 2000);
            reply(None)
        }
    });
    let f = Fixture::new(&server.url);
    let result = f.run(request(false), CancellationToken::new()).await;
    assert_eq!(result["status"], "failed");
    assert!(!result.to_string().contains(SECRET));
    assert!(result["issues"].to_string().len() < 2000);
}

#[tokio::test]
async fn tools_unknown_calls_return_feedback_without_execution_and_count_budget() {
    let server = Server::new(|i, body| match i {
        0 => reply(Some(("unsafe-shell", json!({})))),
        1 => {
            assert!(body.to_string().contains("MODEL_TOOL_INVALID"));
            reply(Some(("status", json!({}))))
        }
        _ => reply(None),
    });
    let f = Fixture::new(&server.url);
    let result = f.run(request(true), CancellationToken::new()).await;
    assert_eq!(result["status"], "complete", "{result:#}");
    assert_eq!(result["modelTurns"], 3);
    assert_eq!(result["toolCalls"], 2);
    assert_eq!(
        result["issues"][0]["error"]["issues"][0]["instancePath"],
        "/tool/name"
    );
}

#[tokio::test]
async fn lifecycle_stops_after_two_failed_repairs() {
    let mut refs = Value::Null;
    let mut input = Value::Null;
    let server = Server::new(move |i, body| match i {
        0 => reply(Some(("code-map", json!({})))),
        1 => reply(Some(("export-snapshot", json!({"include":["**/*.ts"]})))),
        2 => {
            let result = last_tool(&body);
            refs = result["sourceReferences"].clone();
            input = result["input"].clone();
            reply(Some((
                "read-source",
                json!({"sourceId":refs[0]["sourceId"]}),
            )))
        }
        3 => reply(Some((
            "import-conventions",
            json!({"notes":"bounded failing fixture","sourceIds":[refs[0]["sourceId"]]}),
        ))),
        4 | 6 | 8 | 10 => reply(Some(("write-candidate", candidate_files(&input, true)))),
        5 | 7 | 9 => reply(Some(("validate-plugin", json!({})))),
        _ => panic!("model budget must terminate before another request"),
    });
    let f = Fixture::new(&server.url);
    let result = f.run(request(false), CancellationToken::new()).await;
    assert_eq!(result["status"], "failed", "{result:#}");
    assert_eq!(result["repairs"], 2);
    assert_eq!(result["revision"], 3);
    assert!(result["activation"].is_null());
    assert_eq!(server.count.load(Ordering::SeqCst), 11);
}

#[tokio::test]
async fn lifecycle_fills_missing_initial_case_and_resume_validates_without_model() {
    let mut refs = Value::Null;
    let mut input = Value::Null;
    let server = Server::new(move |i, body| match i {
        0 => reply(Some(("code-map", json!({})))),
        1 => reply(Some(("export-snapshot", json!({"include":["**/*.ts"]})))),
        2 => {
            let result = last_tool(&body);
            refs = result["sourceReferences"].clone();
            input = result["input"].clone();
            reply(Some((
                "read-source",
                json!({"sourceId":refs[0]["sourceId"]}),
            )))
        }
        3 => reply(Some((
            "import-conventions",
            json!({"notes":"source-bound fixture","sourceIds":[refs[0]["sourceId"]]}),
        ))),
        4 => {
            let mut bundle = candidate_files(&input, false);
            bundle["files"]
                .as_array_mut()
                .unwrap()
                .retain(|f| f["path"] != "case.json");
            reply(Some(("write-candidate", bundle)))
        }
        5 => {
            let mut bundle = candidate_files(&input, false);
            bundle["files"]
                .as_array_mut()
                .unwrap()
                .retain(|f| f["path"] == "case.json");
            bundle["includeIdentityHelper"] = false.into();
            reply(Some(("write-candidate", bundle)))
        }
        6 => reply(None),
        _ => panic!("resume must revalidate the existing candidate without another model request"),
    });
    let f = Fixture::new(&server.url);
    let report = f.run(request(false), CancellationToken::new()).await;
    assert_eq!(report["status"], "failed");
    assert_eq!(report["repairs"], 0);
    assert_eq!(report["revision"], 2);
    let before = server.count.load(Ordering::SeqCst);
    let mut req = request(false);
    req.resume = report["runId"].as_str().map(str::to_string);
    let resumed = f.run(req, CancellationToken::new()).await;
    assert_eq!(resumed["status"], "complete", "{resumed:#}");
    assert_eq!(resumed["modelTurns"], report["modelTurns"]);
    assert_eq!(server.count.load(Ordering::SeqCst), before);
    assert_eq!(resumed["activation"]["status"], "active");
}

#[test]
fn tools_candidate_paths_reject_traversal_absolute_and_duplicate_files() {
    use project_knowledge_core::collect::candidate::{CandidateFile, write_bundle};
    let f = Fixture::new("http://localhost:1/v1");
    for path in [
        "../outside",
        "C:/outside",
        "/absolute",
        "src/../outside",
        "src\\outside",
        "file:stream",
    ] {
        let files = vec![
            CandidateFile {
                path: "manifest.json".into(),
                content: "{}".into(),
            },
            CandidateFile {
                path: path.into(),
                content: "x".into(),
            },
        ];
        assert!(write_bundle(&f.project(), "agent_test", 1, &files).is_err());
    }
    let files = vec![
        CandidateFile {
            path: "manifest.json".into(),
            content: "{}".into(),
        },
        CandidateFile {
            path: "MANIFEST.JSON".into(),
            content: "{}".into(),
        },
    ];
    assert!(write_bundle(&f.project(), "agent_test", 1, &files).is_err());
}
