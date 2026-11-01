use project_knowledge_core::collect::{load_manifest, staging::stage_collection};
use project_knowledge_core::contract::collector::StagePhase;
use project_knowledge_core::project::config::LoadedProject;
use project_knowledge_core::storage::Database;
use serde_json::{Value, json};
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;

pub struct Fixture {
    pub _temp: tempfile::TempDir,
    pub root: PathBuf,
    pub project: LoadedProject,
    pub database: Database,
    pub manifest: PathBuf,
}
impl Fixture {
    pub fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().to_path_buf();
        std::fs::create_dir(root.join("repo")).unwrap();
        std::fs::write(root.join("repo/a.ts"), "export const smoke = 1;\n").unwrap();
        let config = root.join("project.json");
        std::fs::write(
            &config,
            serde_json::to_vec(&json!({"schemaVersion":1,"projectId":"smoke",
            "repositories":[{"id":"r","root":"repo"}],"selection":{}}))
            .unwrap(),
        )
        .unwrap();
        let project = LoadedProject::load(&config).unwrap();
        let mut database = Database::open_maintenance(
            &root.join("index.sqlite"),
            "smoke",
            &project.config_hash().unwrap(),
            "now",
        )
        .unwrap();
        project_knowledge_core::index::build_base_index(&project, &mut database, "now").unwrap();
        let dir = root.join("plugin");
        std::fs::create_dir(&dir).unwrap();
        std::fs::copy(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../fixtures/plugins/staging/collect.mjs"),
            dir.join("collect.mjs"),
        )
        .unwrap();
        let manifest = dir.join("manifest.json");
        let stages: Vec<Value> = ["extract", "link"].iter().map(|stage| json!({
            "id":stage,"phase":stage,"entry":"collect.mjs","args":[],"replacement":"project",
            "sourceSelectors":[{"id":"sources","include":["**/*.ts"]}],
            "entitySelectors":[{"id":"targets","kinds":["route","file"]}],
            "dependsOn":if *stage == "link" {vec!["smoke.plugin:extract"]} else {vec![]}
        })).collect();
        std::fs::write(&manifest, serde_json::to_vec(&json!({"protocolVersion":"1.0","pluginId":"smoke.plugin","version":"1.0.0",
            "runtime":{"kind":"node","versionRange":">=24 <25"},"artifacts":["manifest.json","collect.mjs"],"stages":stages})).unwrap()).unwrap();
        Self {
            _temp: temp,
            root,
            project,
            database,
            manifest,
        }
    }
    pub fn edit(&self, edit: impl FnOnce(&mut Value)) {
        let mut manifest: Value =
            serde_json::from_slice(&std::fs::read(&self.manifest).unwrap()).unwrap();
        edit(&mut manifest);
        std::fs::write(&self.manifest, serde_json::to_vec(&manifest).unwrap()).unwrap();
    }
    pub fn use_extensions(&self) {
        let source =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/plugins/extensions");
        for name in ["manifest.json", "collect.mjs"] {
            std::fs::copy(
                source.join(name),
                self.manifest.parent().unwrap().join(name),
            )
            .unwrap();
        }
    }
    pub async fn stage(
        &self,
        phase: Option<StagePhase>,
    ) -> Result<
        project_knowledge_core::collect::staging::StagedCollection,
        project_knowledge_core::contract::error::CoreError,
    > {
        stage_collection(
            &self.project,
            &self.database,
            std::slice::from_ref(&self.manifest),
            phase,
            &CancellationToken::new(),
            "now",
        )
        .await
    }
    pub fn add_case(&self, required_entity: &str) {
        let manifest = load_manifest(&self.manifest).unwrap();
        let input = project_knowledge_core::collect::import::export_input(
            self.database.connection(),
            &self.project,
            &manifest.stages[0],
            &self.database.current_source_snapshot_id().unwrap().unwrap(),
            "case",
            "smoke.plugin",
            "1.0.0",
        )
        .unwrap();
        let dir = self.manifest.parent().unwrap();
        std::fs::write(dir.join("input.json"), serde_json::to_vec(&input).unwrap()).unwrap();
        std::fs::write(dir.join("case.json"), serde_json::to_vec(&json!({"id":"smoke","inputFixture":"input.json","expected":{"entities":[required_entity]}})).unwrap()).unwrap();
        self.edit(|m| m["acceptanceCases"] = json!(["case.json"]));
    }
    pub fn count(&self, sql: &str) -> i64 {
        self.database
            .connection()
            .query_row(sql, [], |r| r.get(0))
            .unwrap()
    }
}
