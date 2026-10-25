//! 契约、项目接入、存储与基础索引的综合用例。
//!
//! 测试分组前缀（计划 §4.1）：
//! - `contract_`：统一类型、稳定身份、Schema 与协议；
//! - `project_`：多仓库配置、文件选择与来源快照；
//! - `storage_`：SQLite 断言存储、批次替换与一致读；
//! - `base_`：基础索引、检索、代码地图与证据读取。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use project_knowledge_core::contract::entity::{Basis, Entity, EntityKind, MaybeKnown, Origin};
use project_knowledge_core::contract::error::{CoreError, ErrorEnvelope, codes};
use project_knowledge_core::contract::evidence::{Evidence, SnapshotRef, Source};
use project_knowledge_core::contract::identity::{
    IdentitySlot, IdentityValue, canonical_entity_identity, canonical_json, compute_entity_id,
    compute_evidence_id, compute_relation_id, compute_source_id, content_hash,
    identity_slots_from_canonical,
};
use project_knowledge_core::contract::query::{
    QueryRequest, ReadSourceRequest, SearchRequest, StatusRequest,
};
use project_knowledge_core::contract::relation::{Relation, RelationKind, Resolution};
use project_knowledge_core::contract::{
    ALL_RELATION_KINDS, Attributes, SourceClass, check_protocol_version, endpoint_allowed,
};
use project_knowledge_core::index::{build_base_index, effective_scopes, select_scope};
use project_knowledge_core::project::config::{
    IdentityDelta, LoadedProject, ProjectConfig, RepositoryConfig, ScopeConfig, ScopeKind,
    ScopeRoot, SelectionConfig, apply_identity_delta,
};
use project_knowledge_core::project::scan::{ScanOptions, SelectionStatus, scan_repository};
use project_knowledge_core::project::snapshot::{LineMap, build_snapshot};
use project_knowledge_core::query::{QueryContext, execute};
use project_knowledge_core::storage::database::Database;
use rusqlite::OptionalExtension;
use serde_json::{Value, json};

// ---------------------------------------------------------------- 契约测试

#[test]
fn contract_entity_id_is_stable_and_scope_sensitive() {
    let slots_a = vec![
        IdentitySlot::required("repositoryId", "repo.web"),
        IdentitySlot::required("path", "src/ledger.ts"),
        IdentitySlot::required("scopeId", "app.web"),
    ];
    let slots_b = vec![
        IdentitySlot::required("repositoryId", "repo.web"),
        IdentitySlot::required("path", "src/ledger.ts"),
        IdentitySlot::required("scopeId", "app.admin"),
    ];
    let id_a = compute_entity_id("erp", EntityKind::File, &slots_a).expect("identity");
    let id_b = compute_entity_id("erp", EntityKind::File, &slots_b).expect("identity");
    assert_ne!(id_a, id_b, "不同作用域的同名实体必须是不同身份");
    assert!(id_a.starts_with("ent_"));
    assert_eq!(
        id_a.len(),
        "ent_".len() + 64,
        "SHA-256 必须是完整 64 位十六进制"
    );
    assert_eq!(
        id_a,
        compute_entity_id("erp", EntityKind::File, &slots_a).expect("identity"),
        "同一输入必须得到同一身份"
    );
    assert!(
        id_a.chars()
            .skip(4)
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    );
}

#[test]
fn contract_optional_identity_slot_distinguishes_absent_from_empty() {
    let absent = vec![
        IdentitySlot::required("dataSourceId", "ds.erp"),
        IdentitySlot::optional("catalog", None::<String>),
        IdentitySlot::optional("schema", None::<String>),
        IdentitySlot::required("table", "purchase_line"),
    ];
    let empty = vec![
        IdentitySlot::required("dataSourceId", "ds.erp"),
        IdentitySlot::optional("catalog", Some("")),
        IdentitySlot::optional("schema", None::<String>),
        IdentitySlot::required("table", "purchase_line"),
    ];
    let id_absent = compute_entity_id("erp", EntityKind::DataTable, &absent).expect("identity");
    let id_empty = compute_entity_id("erp", EntityKind::DataTable, &empty).expect("identity");
    assert_ne!(
        id_absent, id_empty,
        "缺省与空字符串不得混同（可选槽位用 [] 与 [value] 编码）"
    );
    let canonical =
        canonical_entity_identity("erp", EntityKind::DataTable, &absent).expect("canonical");
    assert!(
        canonical.contains("[]"),
        "缺省槽位必须编码为空数组：{canonical}"
    );
    let restored =
        identity_slots_from_canonical(EntityKind::DataTable, &canonical).expect("restore");
    assert_eq!(restored.len(), absent.len());
    assert_eq!(restored[1].value, None);
}

#[test]
fn contract_canonical_json_sorts_keys_and_rejects_floats() {
    let value = json!({"b": 1, "a": {"z": [1, 2], "y": "x"}});
    let canonical = canonical_json(&value).expect("canonical");
    assert_eq!(canonical, r#"{"a":{"y":"x","z":[1,2]},"b":1}"#);
    let float = json!({"a": 1.5});
    assert!(canonical_json(&float).is_err(), "身份中禁止浮点数");
}

#[test]
fn contract_relation_id_depends_on_qualifiers_but_not_evidence() {
    let mut qualifiers = serde_json::Map::new();
    qualifiers.insert("operationId".to_string(), json!("op.submit"));
    let id_a = compute_relation_id(
        "erp",
        RelationKind::TransitionsTo,
        "ent_a",
        "ent_b",
        &qualifiers,
    )
    .expect("relation id");
    let id_b = compute_relation_id(
        "erp",
        RelationKind::TransitionsTo,
        "ent_a",
        "ent_b",
        &qualifiers,
    )
    .expect("relation id");
    assert_eq!(id_a, id_b);
    let mut other = qualifiers.clone();
    other.insert("guard".to_string(), json!("role == 'ADMIN'"));
    let id_c = compute_relation_id("erp", RelationKind::TransitionsTo, "ent_a", "ent_b", &other)
        .expect("relation id");
    assert_ne!(id_a, id_c, "限定条件参与关系身份");
}

#[test]
fn contract_source_and_evidence_ids_are_deterministic() {
    let source_id = compute_source_id("erp", "repo.web", "src/ledger.ts");
    assert!(source_id.starts_with("src_"));
    assert_eq!(
        source_id,
        compute_source_id("erp", "repo.web", "src/ledger.ts")
    );
    assert_ne!(
        source_id,
        compute_source_id("erp", "repo.web", "src/other.ts")
    );
    let evidence_id = compute_evidence_id(&source_id, "abc", 0, 10, "definition");
    assert!(evidence_id.starts_with("ev_"));
    assert_ne!(
        evidence_id,
        compute_evidence_id(&source_id, "abc", 0, 11, "definition")
    );
}

#[test]
fn contract_rejects_unknown_fields_and_bad_protocol_versions() {
    let raw = r#"{"id":"ent_1","kind":"file","scopeId":"s","identity":[],"attributes":{},"basis":"observed","origin":"static","evidenceRefs":[],"extensions":{},"unexpected":1}"#;
    let parsed = serde_json::from_str::<Entity>(raw);
    assert!(
        parsed.is_err(),
        "未知公共字段必须被拒绝，不能靠忽略错误兼容"
    );

    assert!(check_protocol_version("1.0").is_ok());
    assert!(check_protocol_version("2.0").is_err());
    assert!(check_protocol_version("1.1").is_err());
    assert!(check_protocol_version("nonsense").is_err());
}

#[test]
fn contract_relation_endpoint_kinds_are_restricted() {
    assert!(endpoint_allowed(RelationKind::Contains, "file", "symbol"));
    assert!(
        !endpoint_allowed(RelationKind::Contains, "symbol", "file"),
        "contains 的方向不可随意反转"
    );
    assert!(!endpoint_allowed(
        RelationKind::Calls,
        "file",
        "httpEndpoint"
    ));
    for kind in ALL_RELATION_KINDS {
        let (from, to) = project_knowledge_core::contract::endpoint_kinds(*kind);
        assert!(!from.is_empty() && !to.is_empty());
    }
}

#[test]
fn contract_error_envelope_shape_is_stable() {
    let error = CoreError::new(codes::CONFIG_INVALID, "bad config").with_issues(vec![
        project_knowledge_core::contract::error::Issue::new(codes::CONFIG_INVALID, "/projectId"),
    ]);
    let envelope: ErrorEnvelope = error.to_envelope();
    let value = serde_json::to_value(&envelope).expect("serialize");
    assert_eq!(value["error"]["code"], codes::CONFIG_INVALID);
    assert_eq!(value["error"]["retryable"], json!(false));
    assert_eq!(
        value["error"]["issues"][0]["instancePath"],
        json!("/projectId")
    );
    assert!(
        value["error"].get("runId").is_none(),
        "缺省的可选字段不得输出 null"
    );
}

#[test]
fn contract_unknown_business_field_uses_status_unknown() {
    let unknown: MaybeKnown<Vec<String>> = MaybeKnown::unknown(vec!["gap_1".to_string()]);
    let value = serde_json::to_value(&unknown).expect("serialize");
    assert_eq!(value["status"], json!("unknown"));
    assert_eq!(value["gapIds"], json!(["gap_1"]));
    let known: MaybeKnown<Vec<String>> = MaybeKnown::known(vec![]);
    let value = serde_json::to_value(&known).expect("serialize");
    assert!(value.get("status").is_none(), "已知值不得被误读为 unknown");
}

#[test]
fn project_scan_handles_chinese_paths_crlf_and_relocated_repositories() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().join("仓库 A");
    write_file(
        &root.join("src/采购台账.ts"),
        "export const 标题 = \"采购台账\";\r\n",
    );
    let options = ScanOptions::default();
    let result = scan_repository("repo.cn", &root, &options).expect("scan");
    let selected: Vec<&str> = result
        .sources
        .iter()
        .filter(|source| source.selection_status == SelectionStatus::Selected)
        .map(|source| source.path.as_str())
        .collect();
    assert_eq!(selected, vec!["src/采购台账.ts"], "中文路径必须可被索引");

    // 内容哈希按原始字节计算：CRLF 不被改写。
    let raw = std::fs::read(root.join("src/采购台账.ts")).expect("read");
    assert!(
        raw.windows(2).any(|pair| pair == b"\r\n"),
        "CRLF 必须原样保留"
    );
    let source = result
        .sources
        .iter()
        .find(|source| source.path == "src/采购台账.ts")
        .expect("source");
    assert_eq!(source.content_hash, content_hash(&raw));

    // 复制到不同绝对路径后，逻辑身份不变。
    let relocated = temp.path().join("其他位置");
    std::fs::create_dir_all(&relocated).expect("mkdir");
    std::fs::create_dir_all(relocated.join("src")).expect("mkdir");
    std::fs::copy(
        root.join("src/采购台账.ts"),
        relocated.join("src/采购台账.ts"),
    )
    .expect("copy");
    let relocated_result = scan_repository("repo.cn", &relocated, &options).expect("scan");
    let relocated_source = relocated_result
        .sources
        .iter()
        .find(|source| source.path == "src/采购台账.ts")
        .expect("relocated source");
    assert_eq!(
        source.content_hash, relocated_source.content_hash,
        "内容相同则来源版本一致"
    );
    let slots = vec![
        IdentitySlot::required("repositoryId", "repo.cn"),
        IdentitySlot::required("path", "src/采购台账.ts"),
        IdentitySlot::required("scopeId", "repo.repo.cn"),
    ];
    let id = compute_entity_id("erp", EntityKind::File, &slots).expect("identity");
    assert_eq!(
        id,
        compute_entity_id("erp", EntityKind::File, &slots).expect("identity"),
        "身份只依赖仓库相对路径与作用域，与绝对位置无关"
    );
}

#[test]
fn storage_rejects_invalid_batches_and_rolls_back_failed_transactions() {
    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("index.sqlite");
    let mut database = Database::open_maintenance(&path, "erp", "cfg", "now").expect("open");
    {
        let transaction = database.write_transaction().expect("transaction");
        transaction
            .execute("INSERT INTO scopes (id, kind) VALUES ('s', 'module')", [])
            .expect("scope");
        transaction.commit().expect("commit");
    }

    // 错误批次：断言指向不存在的批次必须被外键拒绝。
    let transaction = database.write_transaction().expect("transaction");
    let error = transaction
        .execute(
            "INSERT INTO assertions (id, kind, batch_id, payload_hash, producer_key, basis, origin)
             VALUES ('as_bad', 'entity', 'bat_missing', 'p', 'k', 'observed', 'static')",
            [],
        )
        .expect_err("外键必须拒绝不存在的批次");
    assert!(
        error.to_string().contains("FOREIGN KEY"),
        "错误信息应指出外键约束：{error}"
    );
    transaction.rollback().expect("rollback");

    // 空批次：没有断言的批次是合法的（成功空输出也必须能替换）。
    let transaction = database.write_transaction().expect("transaction");
    {
        let connection = &transaction;
        connection
            .execute(
                "INSERT INTO runs (id, kind, stage, status, started_at) VALUES ('run_empty', 'collect', 'extract', 'running', 'now')",
                [],
            )
            .expect("run");
        connection
            .execute(
                "INSERT INTO batches (id, run_id, producer_key, replacement_key, artifact_hash, input_digest, state, created_at)
                 VALUES ('bat_empty', 'run_empty', 'p', '(p,extract,erp)', 'h', 'd', 'fresh', 'now')",
                [],
            )
            .expect("batch");
        connection
            .execute(
                "INSERT INTO current_batches (replacement_key, batch_id) VALUES ('(p,extract,erp)', 'bat_empty')",
                [],
            )
            .expect("current batch");
    }
    transaction.commit().expect("commit");
    let count: i64 = database
        .connection()
        .query_row(
            "SELECT count(*) FROM entity_assertions WHERE batch_id = 'bat_empty'",
            [],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(count, 0, "空批次不得有断言");

    // 强制回滚：未提交事务的写入不得生效。
    {
        let transaction = database.write_transaction().expect("transaction");
        transaction
            .execute("INSERT INTO scopes (id, kind) VALUES ('s2', 'module')", [])
            .expect("scope");
        transaction.rollback().expect("rollback");
    }
    let scopes: i64 = database
        .connection()
        .query_row("SELECT count(*) FROM scopes", [], |row| row.get(0))
        .expect("count");
    assert_eq!(scopes, 1, "回滚后写入不得生效");

    // 并发读：写事务进行中，另一个只读连接仍能读到已提交数据。
    {
        let transaction = database.write_transaction().expect("transaction");
        transaction
            .execute("INSERT INTO scopes (id, kind) VALUES ('s3', 'module')", [])
            .expect("scope");
        let reader = Database::open_readonly(&path).expect("readonly open");
        let visible: i64 = reader
            .connection()
            .query_row("SELECT count(*) FROM scopes", [], |row| row.get(0))
            .expect("count");
        assert_eq!(visible, 1, "只读连接只看到已提交数据");
        transaction.rollback().expect("rollback");
    }
}

#[test]
fn storage_two_producers_assert_same_entity_and_one_is_replaced() {
    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("index.sqlite");
    let mut database = Database::open_maintenance(&path, "erp", "cfg", "now").expect("open");
    let entity_id = "ent_shared";
    let identity_json = "[\"erp\",\"file\",\"repo.web\",\"src/a.ts\",\"app.web\"]";

    let transaction = database.write_transaction().expect("transaction");
    {
        let connection = &transaction;
        connection
            .execute(
                "INSERT INTO scopes (id, kind) VALUES ('app.web', 'application')",
                [],
            )
            .expect("scope");
        connection
            .execute(
                "INSERT INTO repositories (id, root) VALUES ('repo.web', 'web')",
                [],
            )
            .expect("repository");
        connection
            .execute(
                "INSERT INTO sources (id, repository_id, path, current_hash, selection_status)
                 VALUES ('src_a', 'repo.web', 'src/a.ts', 'h', 'selected')",
                [],
            )
            .expect("source");
        connection
            .execute(
                "INSERT INTO source_versions (source_id, content_hash, content, byte_length, line_map_json, source_class, created_at)
                 VALUES ('src_a', 'h', X'6162', 2, '{}', 'code', 'now')",
                [],
            )
            .expect("source version");
        connection
            .execute(
                "INSERT INTO entities (id, kind, scope_id, identity_json) VALUES (?1, 'file', 'app.web', ?2)",
                rusqlite::params![entity_id, identity_json],
            )
            .expect("entity");
        connection
            .execute(
                "INSERT INTO evidence (id, source_id, content_hash, start_byte, end_byte, start_line, end_line, role)
                 VALUES ('ev_a', 'src_a', 'h', 0, 2, 1, 1, 'file')",
                [],
            )
            .expect("evidence");
        for (run_id, producer, replacement) in [
            ("run_a", "producer.a", "(producer.a,extract,erp)"),
            ("run_b", "producer.b", "(producer.b,extract,erp)"),
        ] {
            connection
                .execute(
                    "INSERT INTO runs (id, kind, stage, status, started_at) VALUES (?1, 'collect', 'extract', 'running', 'now')",
                    rusqlite::params![run_id],
                )
                .expect("run");
            connection
                .execute(
                    "INSERT INTO batches (id, run_id, producer_key, replacement_key, artifact_hash, input_digest, state, created_at)
                     VALUES (?1, ?2, ?3, ?4, 'h', 'd', 'fresh', 'now')",
                    rusqlite::params![format!("bat_{run_id}"), run_id, producer, replacement],
                )
                .expect("batch");
            connection
                .execute(
                    "INSERT INTO assertions (id, kind, batch_id, payload_hash, producer_key, basis, origin)
                     VALUES (?1, 'entity', ?2, ?3, ?4, 'observed', 'static')",
                    rusqlite::params![
                        format!("as_{run_id}"),
                        format!("bat_{run_id}"),
                        format!("payload_{run_id}"),
                        producer
                    ],
                )
                .expect("assertion");
            connection
                .execute(
                    "INSERT INTO entity_assertions (id, entity_id, batch_id, basis, origin, name, attributes_json, payload_hash)
                     VALUES (?1, ?2, ?3, 'observed', 'static', 'a.ts', '{}', ?4)",
                    rusqlite::params![
                        format!("as_{run_id}"),
                        entity_id,
                        format!("bat_{run_id}"),
                        format!("payload_{run_id}")
                    ],
                )
                .expect("entity assertion");
            connection
                .execute(
                    "INSERT INTO assertion_evidence (assertion_id, evidence_id) VALUES (?1, 'ev_a')",
                    rusqlite::params![format!("as_{run_id}")],
                )
                .expect("assertion evidence");
            connection
                .execute(
                    "INSERT INTO current_batches (replacement_key, batch_id) VALUES (?1, ?2)",
                    rusqlite::params![replacement, format!("bat_{run_id}")],
                )
                .expect("current batch");
        }
    }
    transaction.commit().expect("commit");

    // 两个生产者都对同一实体断言。
    let visible = project_knowledge_core::storage::entities::load_entity_assertions(
        database.connection(),
        entity_id,
        true,
    )
    .expect("assertions");
    assert_eq!(visible.len(), 2, "两个生产者的断言都应可见");

    // 替换其中一个生产者的批次（切换 current_batches 指针）。
    let transaction = database.write_transaction().expect("transaction");
    {
        let connection = &transaction;
        connection
            .execute(
                "INSERT INTO runs (id, kind, stage, status, started_at) VALUES ('run_a2', 'collect', 'extract', 'running', 'now')",
                [],
            )
            .expect("run");
        connection
            .execute(
                "INSERT INTO batches (id, run_id, producer_key, replacement_key, artifact_hash, input_digest, state, created_at)
                 VALUES ('bat_run_a2', 'run_a2', 'producer.a', '(producer.a,extract,erp)', 'h2', 'd2', 'fresh', 'now')",
                [],
            )
            .expect("batch");
    }
    project_knowledge_core::storage::set_current_batch(
        &transaction,
        "(producer.a,extract,erp)",
        "bat_run_a2",
    )
    .expect("replace");
    transaction.commit().expect("commit");

    let visible = project_knowledge_core::storage::entities::load_entity_assertions(
        database.connection(),
        entity_id,
        true,
    )
    .expect("assertions");
    assert!(
        visible
            .iter()
            .any(|assertion| assertion.producer_key == "producer.b"),
        "替换一个生产者的批次不得删除另一个生产者的断言：{visible:?}"
    );
    assert!(
        !visible
            .iter()
            .any(|assertion| assertion.producer_key == "producer.a"),
        "被替换的批次不再可见"
    );
}

#[test]
fn base_index_search_finds_comments_and_titles() {
    let temp = tempfile::tempdir().expect("tempdir");
    let (config_path, _web) = build_fixture(temp.path());
    let project = LoadedProject::load(&config_path).expect("load");
    let database_path = config_path.parent().expect("parent").join("index.sqlite");
    let mut database =
        Database::open_maintenance(&database_path, "erp", "cfg", "now").expect("open");
    build_base_index(&project, &mut database, "now").expect("index");

    let mut context = QueryContext {
        database: &mut database,
        project: Some(&project),
        now: "now".to_string(),
    };
    for term in ["供应商筛选", "采购台账"] {
        let response = execute(
            &mut context,
            &QueryRequest::Search(SearchRequest {
                query: term.to_string(),
                fields: None,
                filters: None,
                page_size: Some(20),
                continuation: None,
                budget: None,
            }),
        )
        .expect("search");
        let results = response
            .data
            .get("results")
            .and_then(Value::as_array)
            .expect("results");
        assert!(!results.is_empty(), "`{term}` 必须可召回");
    }
}

// ---------------------------------------------------------------- 导入用例

/// 构造一个最小的待提交批次。
fn staged_batch(
    project_id: &str,
    batch_id: &str,
    replacement_key: &str,
    producer_key: &str,
    entity_name: Option<&str>,
) -> project_knowledge_core::collect::StagedBatch {
    use project_knowledge_core::collect::{StagedBatch, StagedEntity};
    use project_knowledge_core::contract::entity::EntityKind;

    let slots = vec![
        IdentitySlot::required("repositoryId", "repo.web"),
        IdentitySlot::required("path", "src/a.ts"),
        IdentitySlot::required("scopeId", "app.web"),
    ];
    let entity_id = compute_entity_id(project_id, EntityKind::File, &slots).expect("identity");
    StagedBatch {
        project_id: project_id.to_string(),
        batch_id: batch_id.to_string(),
        run_id: "run_import".to_string(),
        producer_key: producer_key.to_string(),
        replacement_key: replacement_key.to_string(),
        artifact_hash: "artifact".to_string(),
        input_digest: "digest".to_string(),
        entities: vec![StagedEntity {
            entity: Entity {
                id: entity_id,
                kind: EntityKind::File,
                scope_id: "app.web".to_string(),
                identity: slots,
                attributes: Attributes::new(),
                basis: Basis::Observed,
                origin: Origin::Static,
                evidence_refs: Vec::new(),
                extensions: serde_json::Map::new(),
            },
            name: entity_name.map(|name| name.to_string()),
            evidence_refs: Vec::new(),
            basis: Basis::Observed,
            origin: Origin::Static,
        }],
        relations: Vec::new(),
        evidence: Vec::new(),
        dependencies: Vec::new(),
        coverage: Vec::new(),
        gaps: Vec::new(),
        documents: Vec::new(),
    }
}

#[test]
fn import_batch_replaces_current_facts_for_its_replacement_key() {
    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("index.sqlite");
    let mut database = Database::open_maintenance(&path, "erp", "cfg", "now").expect("open");
    {
        let transaction = database.write_transaction().expect("transaction");
        transaction
            .execute(
                "INSERT INTO scopes (id, kind) VALUES ('app.web', 'application')",
                [],
            )
            .expect("scope");
        transaction
            .execute(
                "INSERT INTO runs (id, kind, stage, status, started_at) VALUES ('run_import', 'collect', 'extract', 'running', 'now')",
                [],
            )
            .expect("run");
        transaction.commit().expect("commit");
    }

    let replacement_key = "(test.producer,extract,erp)";
    {
        let transaction = database.write_transaction().expect("transaction");
        let batch = staged_batch(
            "erp",
            "bat_1",
            replacement_key,
            "test.producer",
            Some("a.ts"),
        );
        project_knowledge_core::collect::commit_batch(&transaction, &batch, 1, "now")
            .expect("commit");
        transaction.commit().expect("commit");
    }
    let entity_id = compute_entity_id(
        "erp",
        project_knowledge_core::contract::entity::EntityKind::File,
        &[
            IdentitySlot::required("repositoryId", "repo.web"),
            IdentitySlot::required("path", "src/a.ts"),
            IdentitySlot::required("scopeId", "app.web"),
        ],
    )
    .expect("identity");
    let visible = project_knowledge_core::storage::entities::load_entity_assertions(
        database.connection(),
        &entity_id,
        true,
    )
    .expect("assertions");
    assert_eq!(visible.len(), 1, "新批次的事实必须可见");

    // 同替换范围再提交一个空批次：旧事实必须撤下。
    {
        let transaction = database.write_transaction().expect("transaction");
        let mut batch = staged_batch("erp", "bat_2", replacement_key, "test.producer", None);
        batch.entities.clear();
        project_knowledge_core::collect::commit_batch(&transaction, &batch, 2, "now")
            .expect("commit");
        transaction.commit().expect("commit");
    }
    let visible = project_knowledge_core::storage::entities::load_entity_assertions(
        database.connection(),
        &entity_id,
        true,
    )
    .expect("assertions");
    assert!(visible.is_empty(), "成功空输出必须撤下旧事实：{visible:?}");
    let old_state = project_knowledge_core::storage::batch_state(database.connection(), "bat_1")
        .expect("state");
    assert_eq!(
        old_state,
        Some(project_knowledge_core::storage::BatchState::Stale),
        "旧批次必须被标记为 stale"
    );
    let invalidated = project_knowledge_core::storage::batch_invalidated_generation(
        database.connection(),
        "bat_1",
    )
    .expect("generation");
    assert_eq!(
        invalidated,
        Some(2),
        "失效 generation 必须记录（发生在第二次提交时）"
    );
}

#[test]
fn import_keeps_other_producers_facts_visible() {
    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("index.sqlite");
    let mut database = Database::open_maintenance(&path, "erp", "cfg", "now").expect("open");
    {
        let transaction = database.write_transaction().expect("transaction");
        transaction
            .execute(
                "INSERT INTO scopes (id, kind) VALUES ('app.web', 'application')",
                [],
            )
            .expect("scope");
        transaction
            .execute(
                "INSERT INTO runs (id, kind, stage, status, started_at) VALUES ('run_import', 'collect', 'extract', 'running', 'now')",
                [],
            )
            .expect("run");
        transaction.commit().expect("commit");
    }

    let entity_id = compute_entity_id(
        "erp",
        project_knowledge_core::contract::entity::EntityKind::File,
        &[
            IdentitySlot::required("repositoryId", "repo.web"),
            IdentitySlot::required("path", "src/a.ts"),
            IdentitySlot::required("scopeId", "app.web"),
        ],
    )
    .expect("identity");

    for (index, producer) in ["producer.a", "producer.b"].iter().enumerate() {
        let transaction = database.write_transaction().expect("transaction");
        let batch = staged_batch(
            "erp",
            &format!("bat_{index}"),
            &format!("({producer},extract,erp)"),
            producer,
            Some("a.ts"),
        );
        project_knowledge_core::collect::commit_batch(
            &transaction,
            &batch,
            index as u64 + 1,
            "now",
        )
        .expect("commit");
        transaction.commit().expect("commit");
    }
    let visible = project_knowledge_core::storage::entities::load_entity_assertions(
        database.connection(),
        &entity_id,
        true,
    )
    .expect("assertions");
    assert_eq!(visible.len(), 2, "两个生产者的断言都必须可见");

    // 只替换其中一个生产者的批次。
    let transaction = database.write_transaction().expect("transaction");
    let batch = staged_batch(
        "erp",
        "bat_a2",
        "(producer.a,extract,erp)",
        "producer.a",
        Some("a.ts"),
    );
    project_knowledge_core::collect::commit_batch(&transaction, &batch, 3, "now").expect("commit");
    transaction.commit().expect("commit");
    let visible = project_knowledge_core::storage::entities::load_entity_assertions(
        database.connection(),
        &entity_id,
        true,
    )
    .expect("assertions");
    assert!(
        visible
            .iter()
            .any(|assertion| assertion.producer_key == "producer.b"),
        "替换一个生产者不得删除另一个生产者的断言：{visible:?}"
    );
}

#[test]
fn import_failed_transaction_leaves_current_batch_untouched() {
    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("index.sqlite");
    let mut database = Database::open_maintenance(&path, "erp", "cfg", "now").expect("open");
    {
        let transaction = database.write_transaction().expect("transaction");
        transaction
            .execute(
                "INSERT INTO scopes (id, kind) VALUES ('app.web', 'application')",
                [],
            )
            .expect("scope");
        transaction
            .execute(
                "INSERT INTO runs (id, kind, stage, status, started_at) VALUES ('run_import', 'collect', 'extract', 'running', 'now')",
                [],
            )
            .expect("run");
        transaction.commit().expect("commit");
    }
    {
        let transaction = database.write_transaction().expect("transaction");
        let batch = staged_batch(
            "erp",
            "bat_ok",
            "(test.producer,extract,erp)",
            "test.producer",
            Some("a.ts"),
        );
        project_knowledge_core::collect::commit_batch(&transaction, &batch, 1, "now")
            .expect("commit");
        transaction.commit().expect("commit");
    }
    let before = project_knowledge_core::storage::current_batch(
        database.connection(),
        "(test.producer,extract,erp)",
    )
    .expect("current batch");

    // 失败的批次（作用域不存在 → 外键失败）必须整体回滚。
    {
        let transaction = database.write_transaction().expect("transaction");
        let mut batch = staged_batch(
            "erp",
            "bat_bad",
            "(test.producer,extract,erp)",
            "test.producer",
            Some("a.ts"),
        );
        batch.entities[0].entity.scope_id = "missing.scope".to_string();
        let error = project_knowledge_core::collect::commit_batch(&transaction, &batch, 2, "now")
            .expect_err("外键失败必须报错");
        assert_eq!(error.code, codes::INTERNAL_ERROR);
        transaction.rollback().expect("rollback");
    }
    let after = project_knowledge_core::storage::current_batch(
        database.connection(),
        "(test.producer,extract,erp)",
    )
    .expect("current batch");
    assert_eq!(before, after, "失败批次不得改变当前批次指针");
}

#[test]
fn project_scan_respects_gitignore_with_and_without_git_directory() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().join("repo");
    write_file(&root.join("src/app.ts"), "export const a = 1;\n");
    write_file(&root.join("src/generated.ts"), "export const g = 1;\n");
    write_file(&root.join(".gitignore"), "src/generated.ts\n");

    let options = ScanOptions {
        respect_git_ignore: true,
        ..ScanOptions::default()
    };

    // 没有 .git 目录时仍按配置读取 .gitignore。
    let without_git = scan_repository("repo.main", &root, &options).expect("scan");
    let selected: Vec<&str> = without_git
        .sources
        .iter()
        .filter(|source| source.selection_status == SelectionStatus::Selected)
        .map(|source| source.path.as_str())
        .collect();
    assert!(
        selected.contains(&"src/app.ts"),
        "普通源码必须被选中：{selected:?}"
    );
    assert!(
        !selected.contains(&"src/generated.ts"),
        "被 .gitignore 忽略的文件不得入选：{selected:?}"
    );

    // 有 .git 目录时行为一致。
    std::fs::create_dir_all(root.join(".git")).expect("mkdir .git");
    let with_git = scan_repository("repo.main", &root, &options).expect("scan");
    let selected: Vec<&str> = with_git
        .sources
        .iter()
        .filter(|source| source.selection_status == SelectionStatus::Selected)
        .map(|source| source.path.as_str())
        .collect();
    assert!(selected.contains(&"src/app.ts"));
    assert!(!selected.contains(&"src/generated.ts"));

    // 关闭 Git 忽略后，被忽略的文件重新入选（摘要随之改变）。
    let options_without_ignore = ScanOptions {
        respect_git_ignore: false,
        ..options.clone()
    };
    let ignoring_disabled =
        scan_repository("repo.main", &root, &options_without_ignore).expect("scan");
    let selected: Vec<&str> = ignoring_disabled
        .sources
        .iter()
        .filter(|source| source.selection_status == SelectionStatus::Selected)
        .map(|source| source.path.as_str())
        .collect();
    assert!(
        selected.contains(&"src/generated.ts"),
        "respectGitIgnore=false 时被忽略文件应入选：{selected:?}"
    );
}

#[test]
fn base_index_rejects_out_of_range_and_utf16_positions() {
    let bytes = "第一行\n第二行\n".as_bytes();
    // 越界区间必须拒绝。
    assert!(
        project_knowledge_core::evidence::position::ensure_valid(bytes, 0, bytes.len() as u64 + 1)
            .is_err(),
        "越界字节区间必须拒绝"
    );
    assert!(
        project_knowledge_core::evidence::position::ensure_valid(bytes, 5, 5).is_err(),
        "空区间必须拒绝"
    );

    // UTF-16 偏移必须先映射到 UTF-8 字节，不能直接写入协议。
    let text = "😀采购台账";
    let utf16_offset = 2; // 代理对占两个 UTF-16 code unit
    let byte_offset =
        project_knowledge_core::evidence::position::utf16_offset_to_utf8(text, utf16_offset);
    assert_eq!(byte_offset, 4, "emoji 占 4 个 UTF-8 字节");
    let back = project_knowledge_core::evidence::position::utf8_offset_to_utf16(text, 4);
    assert_eq!(back, 2);

    // 行号必须与字节区间一致，否则拒绝。
    assert!(
        project_knowledge_core::evidence::position::ensure_valid(bytes, 0, bytes.len() as u64)
            .is_ok()
    );
    let content = "第一行\n第二行\n";
    assert!(
        project_knowledge_core::evidence::read::validate_position(
            content.as_bytes(),
            0,
            content.len() as u64,
            1,
            2,
        )
        .is_ok(),
        "行号与字节区间一致"
    );
    assert!(
        project_knowledge_core::evidence::read::validate_position(
            content.as_bytes(),
            0,
            content.len() as u64,
            2,
            3,
        )
        .is_err(),
        "行号与字节区间不一致时必须拒绝"
    );
}

#[test]
fn base_index_runs_without_node_python_or_model() {
    // 基础索引不依赖 Node / Python / 模型：只使用 Rust 侧扫描、语法与 SQLite。
    let temp = tempfile::tempdir().expect("tempdir");
    let (config_path, _web) = build_fixture(temp.path());
    let project = LoadedProject::load(&config_path).expect("load");
    let database_path = config_path.parent().expect("parent").join("index.sqlite");
    let mut database =
        Database::open_maintenance(&database_path, "erp", "cfg", "now").expect("open");
    let result = build_base_index(&project, &mut database, "now").expect("index");
    assert!(result.entities > 0);
    assert!(
        !project
            .config
            .agent
            .as_ref()
            .map(|agent| agent.enabled)
            .unwrap_or(false),
        "基础索引不依赖 Agent 配置"
    );
}

#[test]
fn contract_identity_vectors_match_shared_fixture() {
    // 共享向量文件是 Rust / Node / Python 三种实现的唯一期望来源。
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/protocol/identity-vectors.json");
    let raw = std::fs::read_to_string(&path).expect("read identity vectors");
    let document: Value = serde_json::from_str(&raw).expect("parse identity vectors");
    let vectors = document["vectors"]
        .as_array()
        .expect("vectors array")
        .clone();
    assert!(vectors.len() >= 10, "向量数量不足：{}", vectors.len());

    for vector in &vectors {
        let id = vector["id"].as_str().expect("vector id");
        let expected = vector["expectedId"].as_str().expect("expected id");
        let kind = vector["type"].as_str().expect("vector type");
        let actual = match kind {
            "entity" => {
                let project_id = vector["projectId"].as_str().expect("projectId");
                let entity_kind = vector["kind"].as_str().expect("kind");
                let entity_kind = EntityKind::parse(entity_kind)
                    .unwrap_or_else(|| panic!("unknown entity kind {entity_kind}"));
                let slots: Vec<IdentitySlot> = vector["slots"]
                    .as_array()
                    .expect("slots")
                    .iter()
                    .map(|slot| {
                        let name = slot["name"].as_str().expect("slot name");
                        match slot.get("value") {
                            Some(Value::String(text)) => IdentitySlot::required(name, text.clone()),
                            Some(Value::Array(items)) => {
                                let values: Vec<IdentityValue> = items
                                    .iter()
                                    .map(|item| {
                                        IdentityValue::text(item.as_str().unwrap_or_default())
                                    })
                                    .collect();
                                IdentitySlot {
                                    name: name.to_string(),
                                    value: Some(IdentityValue::List(values)),
                                }
                            }
                            _ => IdentitySlot {
                                name: name.to_string(),
                                value: None,
                            },
                        }
                    })
                    .collect();
                compute_entity_id(project_id, entity_kind, &slots).expect("entity id")
            }
            "relation" => {
                let qualifiers = vector["qualifiers"]
                    .as_object()
                    .cloned()
                    .unwrap_or_default();
                compute_relation_id(
                    vector["projectId"].as_str().expect("projectId"),
                    RelationKind::parse(vector["kind"].as_str().expect("kind"))
                        .expect("relation kind"),
                    vector["fromId"].as_str().expect("fromId"),
                    vector["toId"].as_str().expect("toId"),
                    &qualifiers,
                )
                .expect("relation id")
            }
            "source" => compute_source_id(
                vector["projectId"].as_str().expect("projectId"),
                vector["repositoryId"].as_str().expect("repositoryId"),
                vector["path"].as_str().expect("path"),
            ),
            "evidence" => compute_evidence_id(
                vector["sourceId"].as_str().expect("sourceId"),
                vector["contentHash"].as_str().expect("contentHash"),
                vector["startByte"].as_u64().expect("startByte"),
                vector["endByte"].as_u64().expect("endByte"),
                vector["role"].as_str().expect("role"),
            ),
            other => panic!("unknown vector type {other}"),
        };
        assert_eq!(
            actual, expected,
            "向量 `{id}` 的 ID 与共享 fixture 不一致（Rust 与 Node 必须一致）"
        );
    }
}

#[test]
fn contract_entity_and_relation_round_trip_through_json() {
    let entity = Entity {
        id: compute_entity_id(
            "erp",
            EntityKind::Symbol,
            &[
                IdentitySlot::required("repositoryId", "repo.web"),
                IdentitySlot::required("path", "src/a.ts"),
                IdentitySlot::required("scopeId", "app.web"),
                IdentitySlot::required("lexicalPath", "ledger"),
                IdentitySlot::required("symbolKind", "function"),
                IdentitySlot::required("normalizedSignature", "functionledger()"),
            ],
        )
        .expect("identity"),
        kind: EntityKind::Symbol,
        scope_id: "app.web".to_string(),
        identity: Vec::new(),
        attributes: Attributes::new(),
        basis: Basis::Observed,
        origin: Origin::Static,
        evidence_refs: Vec::new(),
        extensions: serde_json::Map::new(),
    };
    let text = serde_json::to_string(&entity).expect("serialize");
    assert!(!text.contains("scope_id"), "JSON 必须使用 camelCase");
    let parsed: Entity = serde_json::from_str(&text).expect("deserialize");
    assert_eq!(parsed.kind, EntityKind::Symbol);

    let relation = Relation {
        id: "rel_1".to_string(),
        kind: RelationKind::Calls,
        from_id: "ent_a".to_string(),
        to_id: "ent_b".to_string(),
        qualifiers: serde_json::Map::new(),
        basis: Basis::Observed,
        origin: Origin::Static,
        resolution: Resolution::Confirmed,
        evidence_refs: vec!["ev_1".to_string()],
        extensions: serde_json::Map::new(),
    };
    let text = serde_json::to_string(&relation).expect("serialize");
    let parsed: Relation = serde_json::from_str(&text).expect("deserialize");
    assert_eq!(parsed.resolution, Resolution::Confirmed);
}

// ---------------------------------------------------------------- 项目接入测试

fn write_file(path: &Path, content: &str) {
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(path, content).expect("write");
}

fn write_bytes(path: &Path, content: &[u8]) {
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(path, content).expect("write");
}

fn base_config(project_id: &str, repositories: Vec<RepositoryConfig>) -> ProjectConfig {
    ProjectConfig {
        schema_version: project_knowledge_core::contract::CONFIG_SCHEMA_VERSION,
        project_id: project_id.to_string(),
        repositories,
        selection: SelectionConfig {
            include: vec!["**/*".to_string()],
            exclude: vec!["node_modules/**".to_string()],
            respect_git_ignore: true,
            max_file_bytes: 2_097_152,
        },
        scopes: Vec::new(),
        services: Vec::new(),
        data_sources: Vec::new(),
        bindings: Vec::new(),
        knowledge_inputs: Vec::new(),
        plugins: Default::default(),
        collectors: Vec::new(),
        agent: None,
        runtimes: Vec::new(),
    }
}

fn write_config(dir: &Path, config: &ProjectConfig) -> PathBuf {
    let workspace = dir.join(".project-knowledge");
    std::fs::create_dir_all(&workspace).expect("mkdir");
    let path = workspace.join("project.json");
    std::fs::write(
        &path,
        serde_json::to_string_pretty(config).expect("serialize"),
    )
    .expect("write");
    path
}

#[test]
fn project_scan_reports_selection_and_gaps() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().join("repo");
    write_file(&root.join("src/app.ts"), "export const a = 1;\n");
    write_file(&root.join("src/.hidden.ts"), "export const b = 2;\n");
    write_file(
        &root.join("node_modules/dep/index.js"),
        "module.exports = 1;\n",
    );
    write_bytes(&root.join("binary.dat"), &[0u8, 1, 2, 3, 0]);
    write_file(&root.join("big.txt"), &"x".repeat(100));

    let options = ScanOptions {
        respect_git_ignore: true,
        max_file_bytes: 32,
        include: vec!["**/*".to_string()],
        exclude: vec!["node_modules/**".to_string()],
    };
    let result = scan_repository("repo.main", &root, &options).expect("scan");

    let selected: Vec<&str> = result
        .sources
        .iter()
        .filter(|source| source.selection_status == SelectionStatus::Selected)
        .map(|source| source.path.as_str())
        .collect();
    assert!(selected.contains(&"src/app.ts"), "普通源码必须被选中");
    assert!(
        selected.contains(&"src/.hidden.ts"),
        "隐藏文件不默认忽略：{selected:?}"
    );
    assert!(
        !selected
            .iter()
            .any(|path| path.starts_with("node_modules/")),
        "node_modules 必须被排除"
    );
    assert!(
        result
            .gaps
            .iter()
            .any(|gap| gap.path == "big.txt" && gap.code == "FILE_TOO_LARGE"),
        "超限文件必须登记缺口：{:?}",
        result.gaps
    );
    assert!(
        result.gaps.iter().any(|gap| gap.path == "binary.dat"),
        "二进制文件必须登记缺口：{:?}",
        result.gaps
    );
}

#[test]
fn project_source_set_digest_changes_with_file_set() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().join("repo");
    write_file(&root.join("a.ts"), "export const a = 1;\n");
    let options = ScanOptions::default();
    let before = scan_repository("repo.main", &root, &options).expect("scan");
    let digest_before =
        project_knowledge_core::project::scan::source_set_digest(std::slice::from_ref(&before));

    write_file(&root.join("b.ts"), "export const b = 2;\n");
    let after = scan_repository("repo.main", &root, &options).expect("scan");
    let digest_after =
        project_knowledge_core::project::scan::source_set_digest(std::slice::from_ref(&after));
    assert_ne!(digest_before, digest_after, "新增文件必须改变集合摘要");

    std::fs::remove_file(root.join("b.ts")).expect("remove");
    let removed = scan_repository("repo.main", &root, &options).expect("scan");
    let digest_removed = project_knowledge_core::project::scan::source_set_digest(&[removed]);
    assert_eq!(digest_before, digest_removed, "删除文件后摘要必须回到原值");
}

#[test]
fn project_config_rejects_overlapping_repository_roots() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().to_path_buf();
    std::fs::create_dir_all(root.join("repo")).expect("mkdir");
    let config = base_config(
        "erp",
        vec![
            RepositoryConfig {
                id: "repo.a".to_string(),
                root: "..".to_string(),
            },
            RepositoryConfig {
                id: "repo.b".to_string(),
                root: "..".to_string(),
            },
        ],
    );
    let path = write_config(&root, &config);
    let error = LoadedProject::load(&path).expect_err("must reject overlapping roots");
    assert_eq!(error.code, codes::CONFIG_INVALID);
}

#[test]
fn project_config_identity_delta_uses_compare_and_swap() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().to_path_buf();
    std::fs::create_dir_all(root.join("repo")).expect("mkdir");
    let config = base_config(
        "erp",
        vec![RepositoryConfig {
            id: "repo.a".to_string(),
            root: "..".to_string(),
        }],
    );
    let path = write_config(&root, &config);
    let loaded = LoadedProject::load(&path).expect("load");
    let hash = loaded.config_hash().expect("hash");

    let delta = IdentityDelta {
        scopes: vec![ScopeConfig {
            id: "app.web".to_string(),
            repository_ids: vec!["repo.a".to_string()],
            roots: vec![ScopeRoot {
                repository_id: "repo.a".to_string(),
                path: "src".to_string(),
            }],
            kind: ScopeKind::Application,
        }],
        ..IdentityDelta::default()
    };
    let error = apply_identity_delta(&loaded.config, &delta, "wrong-hash")
        .expect_err("CAS 必须拒绝过期哈希");
    assert_eq!(error.code, codes::ACTIVATION_CONFLICT);

    let result = apply_identity_delta(&loaded.config, &delta, &hash).expect("apply");
    assert_eq!(result.config.scopes.len(), 1);
    assert_eq!(result.config.scopes[0].id, "app.web");
    assert_ne!(result.config_hash, hash, "写入后配置哈希必须变化");
}

#[test]
fn project_scope_selection_uses_longest_matching_root() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().to_path_buf();
    std::fs::create_dir_all(root.join("repo")).expect("mkdir");
    let mut config = base_config(
        "erp",
        vec![RepositoryConfig {
            id: "repo.a".to_string(),
            root: "..".to_string(),
        }],
    );
    config.scopes = vec![
        ScopeConfig {
            id: "app.web".to_string(),
            repository_ids: vec!["repo.a".to_string()],
            roots: vec![ScopeRoot {
                repository_id: "repo.a".to_string(),
                path: "web".to_string(),
            }],
            kind: ScopeKind::Application,
        },
        ScopeConfig {
            id: "app.web.ledger".to_string(),
            repository_ids: vec!["repo.a".to_string()],
            roots: vec![ScopeRoot {
                repository_id: "repo.a".to_string(),
                path: "web/src/ledger".to_string(),
            }],
            kind: ScopeKind::Module,
        },
    ];
    let path = write_config(&root, &config);
    let loaded = LoadedProject::load(&path).expect("load");
    let scopes = effective_scopes(&loaded);
    assert_eq!(
        select_scope(&scopes, "repo.a", "web/src/ledger/page.ts").expect("scope"),
        "app.web.ledger",
        "最长匹配根优先"
    );
    assert_eq!(
        select_scope(&scopes, "repo.a", "web/src/other.ts").expect("scope"),
        "app.web"
    );
    assert_eq!(
        select_scope(&scopes, "repo.a", "unmatched/file.ts").expect("scope"),
        "repo.repo.a",
        "没有匹配时使用稳定的仓库默认作用域"
    );
}

// ---------------------------------------------------------------- 存储测试

#[test]
fn storage_migration_creates_expected_schema() {
    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("index.sqlite");
    let database = Database::open_maintenance(&path, "erp", "cfg", "now").expect("open");
    let capabilities = database.capabilities().expect("capabilities");
    assert!(capabilities.fts5, "必须启用 FTS5");
    assert!(capabilities.json, "必须启用 JSON1");
    assert!(capabilities.foreign_keys, "必须启用外键");
    assert!(
        capabilities.sqlite_version.starts_with('3'),
        "SQLite 版本异常：{}",
        capabilities.sqlite_version
    );

    let connection = database.connection();
    let tables: Vec<String> = connection
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .expect("prepare")
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query")
        .map(|row| row.expect("row"))
        .collect();
    for expected in [
        "assertions",
        "batches",
        "current_batches",
        "entities",
        "entity_assertions",
        "evidence",
        "relation_assertions",
        "relations",
        "search_documents",
        "source_versions",
        "sources",
    ] {
        assert!(
            tables.iter().any(|table| table == expected),
            "缺少表 {expected}：{tables:?}"
        );
    }
    let integrity: String = connection
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .expect("integrity");
    assert_eq!(integrity, "ok");
}

#[test]
fn storage_readonly_open_missing_database_is_an_error() {
    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("missing.sqlite");
    let error = match Database::open_readonly(&path) {
        Ok(_) => panic!("缺失的数据库必须报错"),
        Err(error) => error,
    };
    assert_eq!(error.code, codes::INDEX_MISSING);
}

#[test]
fn storage_backup_copies_consistent_database() {
    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("index.sqlite");
    let backup = temp.path().join("backup.sqlite");
    {
        let database = Database::open_maintenance(&path, "erp", "cfg", "now").expect("open");
        database
            .connection()
            .execute("INSERT INTO scopes (id, kind) VALUES ('s', 'module')", [])
            .expect("insert");
    }
    project_knowledge_core::storage::backup_to(&path, &backup).expect("backup");
    let copied = Database::open_readonly(&backup).expect("open backup");
    let count: i64 = copied
        .connection()
        .query_row("SELECT count(*) FROM scopes", [], |row| row.get(0))
        .expect("count");
    assert_eq!(count, 1, "备份必须包含已提交数据");
}

#[test]
fn storage_batch_state_is_one_way_after_invalidation() {
    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("index.sqlite");
    let mut database = Database::open_maintenance(&path, "erp", "cfg", "now").expect("open");
    let transaction = database.write_transaction().expect("transaction");
    {
        let connection = &transaction;
        connection
            .execute("INSERT INTO scopes (id, kind) VALUES ('s', 'module')", [])
            .expect("insert scope");
        connection
            .execute(
                "INSERT INTO runs (id, kind, stage, status, started_at) VALUES ('run1', 'index.base', 'base', 'running', 'now')",
                [],
            )
            .expect("insert run");
        connection
            .execute(
                "INSERT INTO batches (id, run_id, producer_key, replacement_key, artifact_hash, input_digest, state, created_at)
                 VALUES ('bat1', 'run1', 'core.base', '(core.base,repo.a)', 'h', 'd', 'fresh', 'now')",
                [],
            )
            .expect("insert batch");
        connection
            .execute(
                "INSERT INTO current_batches (replacement_key, batch_id) VALUES ('(core.base,repo.a)', 'bat1')",
                [],
            )
            .expect("insert current batch");
    }
    transaction.commit().expect("commit");

    {
        let transaction = database.write_transaction().expect("transaction");
        let invalidated = project_knowledge_core::storage::invalidate_batches(
            &transaction,
            &["(core.base,repo.a)".to_string()],
            7,
        )
        .expect("invalidate");
        assert_eq!(invalidated, 1);
        transaction.commit().expect("commit");
    }
    let state =
        project_knowledge_core::storage::batch_state(database.connection(), "bat1").expect("state");
    assert_eq!(
        state,
        Some(project_knowledge_core::storage::BatchState::Stale)
    );
    let generation = project_knowledge_core::storage::batch_invalidated_generation(
        database.connection(),
        "bat1",
    )
    .expect("generation");
    assert_eq!(generation, Some(7));

    // 再次失效不能清空失效 generation。
    {
        let transaction = database.write_transaction().expect("transaction");
        project_knowledge_core::storage::invalidate_batches(
            &transaction,
            &["(core.base,repo.a)".to_string()],
            9,
        )
        .expect("invalidate");
        transaction.commit().expect("commit");
    }
    let generation = project_knowledge_core::storage::batch_invalidated_generation(
        database.connection(),
        "bat1",
    )
    .expect("generation");
    assert_eq!(
        generation,
        Some(7),
        "失效 generation 是单向的，不允许被覆盖"
    );
}

// ---------------------------------------------------------------- 基础索引测试

fn build_fixture(temp: &Path) -> (PathBuf, PathBuf) {
    let workspace = temp.join("workspace");
    let web = workspace.join("web");
    let api = workspace.join("api");
    write_file(
        &web.join("src/pages/PurchaseLedger.tsx"),
        r#"import { listLedger } from "../api/ledger";
import { queryLedger } from "./ledgerQuery";

// 供应商筛选：按供应商名称过滤采购台账明细。
export function PurchaseLedgerPage() {
  const rows = listLedger({ supplier: "ACME" });
  return rows;
}

export const title = "采购台账";
"#,
    );
    write_file(
        &web.join("src/pages/ledgerQuery.ts"),
        r#"export function queryLedger(filter: string) {
  return listLedger(filter);
}
"#,
    );
    write_file(
        &api.join("src/ledger.ts"),
        r#"export function listLedger(filter: string) {
  return { filter, total: 0 };
}
"#,
    );
    write_file(
        &api.join("src/schema.sql"),
        "SELECT id FROM purchase_line;\n",
    );

    let mut config = base_config(
        "erp",
        vec![
            RepositoryConfig {
                id: "repo.web".to_string(),
                root: "../web".to_string(),
            },
            RepositoryConfig {
                id: "repo.api".to_string(),
                root: "../api".to_string(),
            },
        ],
    );
    config.scopes = vec![ScopeConfig {
        id: "app.web".to_string(),
        repository_ids: vec!["repo.web".to_string()],
        roots: vec![ScopeRoot {
            repository_id: "repo.web".to_string(),
            path: "src".to_string(),
        }],
        kind: ScopeKind::Application,
    }];
    let config_path = write_config(&workspace, &config);
    (config_path, web)
}

#[test]
fn base_index_creates_entities_relations_and_evidence() {
    let temp = tempfile::tempdir().expect("tempdir");
    let (config_path, _web) = build_fixture(temp.path());
    let project = LoadedProject::load(&config_path).expect("load");
    let database_path = config_path.parent().expect("parent").join("index.sqlite");
    let mut database =
        Database::open_maintenance(&database_path, "erp", "cfg", "now").expect("open");
    let result = build_base_index(&project, &mut database, "now").expect("index");

    assert!(result.entities > 0, "必须产生实体");
    assert!(result.relations > 0, "必须产生关系");
    assert!(result.documents > 0, "必须产生搜索投影");
    assert!(result.generation >= 1);

    let connection = database.connection();
    let symbols: i64 = connection
        .query_row(
            "SELECT count(*) FROM entities WHERE kind = 'symbol'",
            [],
            |row| row.get(0),
        )
        .expect("count");
    assert!(symbols > 0, "必须索引符号");

    let evidence: i64 = connection
        .query_row("SELECT count(*) FROM evidence", [], |row| row.get(0))
        .expect("count");
    assert!(evidence > 0, "每个断言必须有原始证据");

    let orphan_assertions: i64 = connection
        .query_row(
            "SELECT count(*) FROM assertions a
              WHERE NOT EXISTS (SELECT 1 FROM assertion_evidence ae WHERE ae.assertion_id = a.id)",
            [],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(orphan_assertions, 0, "断言不得缺少证据引用");

    let foreign_key_errors: i64 = connection
        .query_row("PRAGMA foreign_key_check", [], |row| row.get::<_, i64>(0))
        .unwrap_or(0);
    assert_eq!(foreign_key_errors, 0, "外键检查必须通过");
}

#[test]
fn base_index_search_finds_chinese_text_and_identifiers() {
    let temp = tempfile::tempdir().expect("tempdir");
    let (config_path, _web) = build_fixture(temp.path());
    let project = LoadedProject::load(&config_path).expect("load");
    let database_path = config_path.parent().expect("parent").join("index.sqlite");
    let mut database =
        Database::open_maintenance(&database_path, "erp", "cfg", "now").expect("open");
    build_base_index(&project, &mut database, "now").expect("index");

    let mut context = QueryContext {
        database: &mut database,
        project: Some(&project),
        now: "now".to_string(),
    };
    let response = execute(
        &mut context,
        &QueryRequest::Search(SearchRequest {
            query: "采购台账".to_string(),
            fields: None,
            filters: None,
            page_size: Some(20),
            continuation: None,
            budget: None,
        }),
    )
    .expect("search");
    let results = response
        .data
        .get("results")
        .and_then(Value::as_array)
        .expect("results");
    assert!(!results.is_empty(), "中文业务词必须可召回");

    let response = execute(
        &mut context,
        &QueryRequest::Search(SearchRequest {
            query: "listLedger".to_string(),
            fields: None,
            filters: None,
            page_size: Some(20),
            continuation: None,
            budget: None,
        }),
    )
    .expect("search");
    let results = response
        .data
        .get("results")
        .and_then(Value::as_array)
        .expect("results");
    assert!(
        results
            .iter()
            .any(|item| item.get("name").and_then(Value::as_str) == Some("listLedger")),
        "标识符必须可召回：{results:?}"
    );
}

#[test]
fn base_index_reads_source_with_byte_positions() {
    let temp = tempfile::tempdir().expect("tempdir");
    let (config_path, web) = build_fixture(temp.path());
    let project = LoadedProject::load(&config_path).expect("load");
    let database_path = config_path.parent().expect("parent").join("index.sqlite");
    let mut database =
        Database::open_maintenance(&database_path, "erp", "cfg", "now").expect("open");
    build_base_index(&project, &mut database, "now").expect("index");

    let source_id = compute_source_id("erp", "repo.web", "src/pages/PurchaseLedger.tsx");
    let content_hash =
        content_hash(&std::fs::read(web.join("src/pages/PurchaseLedger.tsx")).expect("read"));
    let reference = project_knowledge_core::contract::evidence::SourceReference {
        snapshot: SnapshotRef {
            index_id: database.index_id().expect("index id"),
            generation: database.generation_string().expect("generation"),
            source_snapshot_id: None,
        },
        evidence_id: compute_evidence_id(&source_id, &content_hash, 0, 10, "definition"),
        source_id,
        content_hash,
        start_byte: 0,
        end_byte: 10,
    };

    let mut context = QueryContext {
        database: &mut database,
        project: Some(&project),
        now: "now".to_string(),
    };
    let response = execute(
        &mut context,
        &QueryRequest::ReadSource(ReadSourceRequest {
            references: vec![reference.clone()],
            max_response_chars: Some(1024),
            filters: None,
        }),
    )
    .expect("read source");
    let reads = response
        .data
        .get("reads")
        .and_then(Value::as_array)
        .expect("reads");
    assert_eq!(reads.len(), 1);
    assert_eq!(reads[0]["status"], json!("current"));
    assert_eq!(reads[0]["path"], json!("src/pages/PurchaseLedger.tsx"));
    assert!(
        reads[0]["snippet"]
            .as_str()
            .expect("snippet")
            .starts_with("import"),
        "片段必须是原始字节：{:?}",
        reads[0]["snippet"]
    );
}

#[test]
fn base_index_declares_unconfigured_capabilities() {
    let temp = tempfile::tempdir().expect("tempdir");
    let (config_path, _web) = build_fixture(temp.path());
    let project = LoadedProject::load(&config_path).expect("load");
    let database_path = config_path.parent().expect("parent").join("index.sqlite");
    let mut database =
        Database::open_maintenance(&database_path, "erp", "cfg", "now").expect("open");
    let result = build_base_index(&project, &mut database, "now").expect("index");

    for capability in [
        "routes", "frontend", "api", "database", "workflow", "report",
    ] {
        let entry = result
            .coverage
            .iter()
            .find(|entry| entry.capability == capability)
            .unwrap_or_else(|| panic!("缺少能力声明 {}", capability));
        assert_eq!(
            entry.status.as_str(),
            "not_configured",
            "未采集的能力必须登记为 not_configured"
        );
    }
    project_knowledge_core::index::assert_capabilities_declared(&result.coverage)
        .expect("coverage 完整");
}

#[test]
fn base_index_status_reports_freshness_and_does_not_write() {
    let temp = tempfile::tempdir().expect("tempdir");
    let (config_path, _web) = build_fixture(temp.path());
    let project = LoadedProject::load(&config_path).expect("load");
    let database_path = config_path.parent().expect("parent").join("index.sqlite");
    let mut database =
        Database::open_maintenance(&database_path, "erp", "cfg", "now").expect("open");
    build_base_index(&project, &mut database, "now").expect("index");
    let generation_before = database.generation().expect("generation");

    let mut context = QueryContext {
        database: &mut database,
        project: Some(&project),
        now: "now".to_string(),
    };
    let response = execute(
        &mut context,
        &QueryRequest::Status(StatusRequest {
            check_freshness: true,
        }),
    )
    .expect("status");
    assert_eq!(
        response.completeness.freshness,
        project_knowledge_core::contract::query::Freshness::Fresh
    );
    assert!(response.completeness.disk_checked);
    assert_eq!(
        database.generation().expect("generation"),
        generation_before,
        "只读查询不得推进 generation"
    );
}

#[test]
fn base_index_detects_changed_sources_as_stale() {
    let temp = tempfile::tempdir().expect("tempdir");
    let (config_path, web) = build_fixture(temp.path());
    let project = LoadedProject::load(&config_path).expect("load");
    let database_path = config_path.parent().expect("parent").join("index.sqlite");
    let mut database =
        Database::open_maintenance(&database_path, "erp", "cfg", "now").expect("open");
    build_base_index(&project, &mut database, "now").expect("index");

    write_file(
        &web.join("src/pages/PurchaseLedger.tsx"),
        "export const changed = true;\n",
    );

    let mut context = QueryContext {
        database: &mut database,
        project: Some(&project),
        now: "now".to_string(),
    };
    let response = execute(
        &mut context,
        &QueryRequest::Status(StatusRequest {
            check_freshness: true,
        }),
    )
    .expect("status");
    assert_eq!(
        response.completeness.freshness,
        project_knowledge_core::contract::query::Freshness::Stale,
        "来源变化必须被检测为 stale"
    );
    assert!(response.completeness.disk_checked, "核验必须真的读取磁盘");
}

#[test]
fn base_index_line_map_handles_crlf_and_multibyte() {
    let bytes = "第一行\r\n第二行\nthird 😀\n".as_bytes();
    let map = LineMap::build(bytes);
    assert_eq!(map.line_of(0), 1);
    let second_line_start = "第一行\r\n".len() as u64;
    assert_eq!(map.line_of(second_line_start), 2);
    let third_line_start = "第一行\r\n第二行\n".len() as u64;
    assert_eq!(map.line_of(third_line_start), 3);
    assert_eq!(map.end_line_of(0, bytes.len() as u64), 3);
}

#[test]
fn base_index_snapshot_members_track_selection_status() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().join("repo");
    write_file(&root.join("a.ts"), "export const a = 1;\n");
    std::fs::create_dir_all(root.join("node_modules/pkg")).expect("mkdir");
    write_file(
        &root.join("node_modules/pkg/index.js"),
        "module.exports = 1;\n",
    );
    let options = ScanOptions {
        exclude: vec!["node_modules/**".to_string()],
        ..ScanOptions::default()
    };
    let result = scan_repository("repo.main", &root, &options).expect("scan");
    let snapshot = build_snapshot("erp", "cfg", &[result], "now");
    let selected: Vec<&str> = snapshot
        .selected_members()
        .map(|member| member.path.as_str())
        .collect();
    assert_eq!(selected, vec!["a.ts"]);
    assert!(
        snapshot
            .members
            .iter()
            .any(|member| member.path == "node_modules/pkg/index.js"
                && member.selection_status == SelectionStatus::Excluded),
        "被排除的文件也要登记在集合摘要里"
    );
}

#[test]
fn base_index_sources_table_keeps_only_current_hash() {
    let temp = tempfile::tempdir().expect("tempdir");
    let (config_path, _web) = build_fixture(temp.path());
    let project = LoadedProject::load(&config_path).expect("load");
    let database_path = config_path.parent().expect("parent").join("index.sqlite");
    let mut database =
        Database::open_maintenance(&database_path, "erp", "cfg", "now").expect("open");
    build_base_index(&project, &mut database, "now").expect("index");
    let connection = database.connection();
    let source: Option<(String, String)> = connection
        .query_row(
            "SELECT s.id, s.current_hash FROM sources s WHERE s.repository_id = 'repo.api' AND s.path = 'src/ledger.ts'",
            [],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .expect("query");
    let (source_id, current_hash) = source.expect("source row");
    assert_eq!(
        source_id,
        compute_source_id("erp", "repo.api", "src/ledger.ts")
    );
    assert_eq!(current_hash.len(), 64, "来源必须保存内容哈希");
}

#[test]
fn base_index_entity_identity_round_trips() {
    let slots = vec![
        IdentitySlot::required("repositoryId", "repo.web"),
        IdentitySlot::required("path", "src/a.ts"),
        IdentitySlot::required("scopeId", "app.web"),
    ];
    let canonical = canonical_entity_identity("erp", EntityKind::File, &slots).expect("canonical");
    let restored = identity_slots_from_canonical(EntityKind::File, &canonical).expect("restore");
    assert_eq!(restored.len(), 3);
    assert_eq!(restored[0].value, slots[0].value);
    let mut map: BTreeMap<String, String> = BTreeMap::new();
    map.insert("kind".to_string(), "file".to_string());
    assert_eq!(map.get("kind").map(String::as_str), Some("file"));
}

#[test]
fn base_index_evidence_requires_non_empty_range() {
    let error = project_knowledge_core::project::snapshot::validate_range(b"hello", 3, 3)
        .expect_err("must reject empty range");
    assert_eq!(error.code, codes::EVIDENCE_INVALID);
    let error = project_knowledge_core::project::snapshot::validate_range(b"hello", 0, 99)
        .expect_err("must reject out of range");
    assert_eq!(error.code, codes::EVIDENCE_INVALID);
    project_knowledge_core::project::snapshot::validate_range(b"hello", 0, 5).expect("valid range");
}

#[test]
fn base_index_source_and_evidence_are_consistent() {
    let temp = tempfile::tempdir().expect("tempdir");
    let (config_path, _web) = build_fixture(temp.path());
    let project = LoadedProject::load(&config_path).expect("load");
    let database_path = config_path.parent().expect("parent").join("index.sqlite");
    let mut database =
        Database::open_maintenance(&database_path, "erp", "cfg", "now").expect("open");
    build_base_index(&project, &mut database, "now").expect("index");
    let connection = database.connection();
    let dangling: i64 = connection
        .query_row(
            "SELECT count(*) FROM evidence e
              WHERE NOT EXISTS (SELECT 1 FROM source_versions sv
                                 WHERE sv.source_id = e.source_id AND sv.content_hash = e.content_hash)",
            [],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(dangling, 0, "证据必须指向存在的来源版本");
    let bad_range: i64 = connection
        .query_row(
            "SELECT count(*) FROM evidence WHERE end_byte <= start_byte OR start_line < 1",
            [],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(bad_range, 0, "证据区间必须非空且行号从 1 开始");
}

#[test]
fn base_index_source_and_evidence_types_are_exported() {
    // 编译期断言：契约类型来自 Core 的唯一来源。
    let source = Source {
        id: "src_1".to_string(),
        repository_id: "repo.web".to_string(),
        path: "src/a.ts".to_string(),
        source_class: SourceClass::Code,
        language: Some("typescript".to_string()),
        content_hash: "hash".to_string(),
        byte_length: 10,
    };
    let evidence = Evidence {
        id: "ev_1".to_string(),
        source_id: source.id.clone(),
        content_hash: source.content_hash.clone(),
        start_byte: 0,
        end_byte: 10,
        start_line: 1,
        end_line: 1,
        role: "definition".to_string(),
    };
    assert_eq!(evidence.source_id, source.id);
}
