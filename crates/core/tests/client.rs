//! Transport entry points use Core validation, read transactions and cancellation.
#[allow(dead_code)]
#[path = "support/collection.rs"]
mod collection;

use project_knowledge_core::{
    client,
    contract::{CoreError, codes},
    storage::Database,
};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;

#[test]
fn client_schema_and_maintenance_fields_have_one_rust_owner() {
    let cap = client::capabilities();
    project_knowledge_core::contract::schema::validate::<
        project_knowledge_core::contract::client::Capabilities,
    >(&cap, "")
    .unwrap();
    assert_eq!(
        cap["schema"],
        project_knowledge_core::contract::schema::document()
    );
    let request = client::decode::<project_knowledge_core::contract::client::MaintenanceRequest>(
        r#"{"operation":"validatePlugin","candidateIds":["c"]}"#,
    )
    .unwrap();
    assert_eq!(json!(request)["candidateIds"], json!(["c"]));
    let error = client::decode::<project_knowledge_core::contract::client::MaintenanceRequest>(
        r#"{"operation":"validatePlugin","candidate_ids":["secret"]}"#,
    )
    .unwrap_err();
    assert_eq!(error.code, codes::SCHEMA_INVALID);
    assert!(!error.issues.is_empty());
    assert!(!json!(error).to_string().contains("secret"));
}

#[test]
fn client_query_is_readonly_and_cancels_inside_sqlite_vm() {
    let f = collection::Fixture::new();
    let before = std::fs::read(f.database.path()).unwrap();
    let mut database = Database::open_readonly(f.database.path()).unwrap();
    let statements = Arc::new(Mutex::new(Vec::new()));
    let observed = statements.clone();
    database
        .observe_sql(move |sql| observed.lock().unwrap().push(sql.to_string()))
        .unwrap();
    let response = client::execute_query(
        &f.project,
        &mut database,
        r#"{"operation":"search","query":"smoke"}"#,
        &CancellationToken::new(),
        "caller-id",
    )
    .unwrap();
    assert_eq!(response["requestId"], "caller-id");
    assert!(!response["sourceReferences"].as_array().unwrap().is_empty());
    let logs = statements.lock().unwrap().clone();
    assert!(
        logs.iter()
            .any(|sql| sql.trim_start().starts_with("BEGIN DEFERRED"))
    );
    assert!(!logs.iter().any(|sql| {
        ["INSERT", "UPDATE", "DELETE", "CREATE", "ALTER", "REPLACE"]
            .iter()
            .any(|verb| sql.to_uppercase().starts_with(verb))
    }));
    let cancel = CancellationToken::new();
    let signal = cancel.clone();
    database
        .observe_sql(move |sql| {
            if sql.starts_with("WITH hits") {
                signal.cancel();
            }
        })
        .unwrap();
    let error = client::execute_query(
        &f.project,
        &mut database,
        r#"{"operation":"search","query":"smoke"}"#,
        &cancel,
        "running-sql",
    )
    .unwrap_err();
    assert_eq!(error.code, codes::CANCELLED);
    assert!(cancel.is_cancelled());
    assert!(database.connection().is_autocommit());
    assert_eq!(std::fs::read(f.database.path()).unwrap(), before);
    if let Some(dir) = std::env::var_os("PK_SDK_EVIDENCE_DIR") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            std::path::Path::new(&dir).join("readonly-sql.json"),
            serde_json::to_vec_pretty(
                &json!({"passed":true,"sql":logs,"inflightCancellation":error}),
            )
            .unwrap(),
        )
        .unwrap();
    }
}

#[tokio::test]
async fn maintenance_cancellation_rolls_back_base_and_activation_transactions() {
    let mut f = collection::Fixture::new();
    let generation = f.database.generation().unwrap();
    std::fs::write(f.root.join("repo/new.ts"), "export const newSource = 1;\n").unwrap();
    let cancel = CancellationToken::new();
    let signal = cancel.clone();
    f.database
        .observe_sql(move |sql| {
            if sql.starts_with("INSERT INTO repositories") {
                signal.cancel();
            }
        })
        .unwrap();
    let error = client::execute_maintenance(
        &f.project,
        &mut f.database,
        r#"{"operation":"index","mode":"base"}"#,
        &cancel,
    )
    .await
    .unwrap_err();
    assert_eq!(error.code, codes::CANCELLED);
    assert!(cancel.is_cancelled());
    assert_eq!(f.database.generation().unwrap(), generation);
    assert!(f.database.connection().is_autocommit());
    f.database.observe_sql(|_| {}).unwrap();
    std::fs::remove_file(f.root.join("repo/new.ts")).unwrap();
    let candidate = client::execute_maintenance(
        &f.project,
        &mut f.database,
        &json!({"operation":"submitPlugin","path":f.manifest}).to_string(),
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    client::execute_maintenance(
        &f.project,
        &mut f.database,
        &json!({"operation":"validatePlugin","candidateIds":[candidate["id"]]}).to_string(),
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    let cancel = CancellationToken::new();
    let signal = cancel.clone();
    f.database
        .observe_sql(move |sql| {
            if sql.contains("UPDATE index_meta SET generation") {
                signal.cancel();
            }
        })
        .unwrap();
    let result: Result<Value, CoreError> = client::execute_maintenance(
        &f.project,
        &mut f.database,
        &json!({"operation":"activatePlugin","candidateId":candidate["id"]}).to_string(),
        &cancel,
    )
    .await;
    assert_eq!(result.unwrap_err().code, codes::CANCELLED);
    assert!(cancel.is_cancelled());
    assert_eq!(f.database.generation().unwrap(), generation);
    assert!(
        project_knowledge_core::storage::active_version_map(f.database.connection())
            .unwrap()
            .is_empty()
    );
    assert!(f.database.connection().is_autocommit());
}
