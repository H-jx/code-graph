//! Serial Core tool owner. Every model-visible failure retains the Core issue envelope.
use crate::runner::{SharedState, redact};
use project_knowledge_core::{
    collect::{
        candidate::{CandidateFile, read_bundle, write_bundle},
        manifest::{load_manifest, validate_manifest},
        registry,
    },
    contract::{
        collector::{
            AcceptanceCase, CollectorInput, CollectorOutput, PluginManifest, ReplacementScope,
            SourceSelector, StagePhase, StageSpec,
        },
        error::{CoreError, codes},
        evidence::SourceReference,
        identity::content_hash,
        knowledge::{ConventionsImport, TopicArtifact, TopicContextRequest},
        query::{CodeMapRequest, QueryRequest, ReadSourceRequest, SearchRequest},
    },
    knowledge::{self, KnowledgeControl},
    project::config::{IdentityDelta, LoadedProject},
    query::QueryContext,
    storage::{Database, plugins},
};
use rig::{
    message::ToolName,
    tool::{DynamicTool, ToolOutput},
};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Investigation {
    notes: String,
    source_ids: Vec<String>,
    #[serde(default)]
    expected_config_hash: Option<String>,
    #[serde(default)]
    identity_delta: IdentityDelta,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct TopicArgs {
    topic_id: String,
    query: String,
    #[serde(default)]
    view: TopicView,
    scope_ids: Option<Vec<String>>,
    kinds: Option<Vec<project_knowledge_core::contract::EntityKind>>,
    max_chars: Option<u64>,
}
#[derive(Default, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
enum TopicView {
    #[default]
    Search,
    LocateFeature,
    Page,
    Workflow,
    Report,
}
#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct SkillArgs {
    name: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct ReadArgs {
    source_id: String,
    max_chars: Option<u64>,
}
#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct SearchArgs {
    query: String,
    page_size: Option<u32>,
}
#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct MapArgs {
    query: Option<String>,
    max_files: Option<u32>,
    max_symbols: Option<u32>,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct Bundle {
    files: Vec<CandidateFile>,
    #[serde(default, rename = "includeIdentityHelper")]
    include_identity_helper: bool,
}

pub const TOOL_NAMES: &[&str] = &[
    "get-capabilities",
    "status",
    "search",
    "code-map",
    "read-source",
    "export-snapshot",
    "update-base",
    "import-conventions",
    "write-candidate",
    "prepare-plugin",
    "validate-plugin",
    "run-plugin-cases",
    "activate-plugin",
    "export-topic-context",
    "import-topic",
    "export-skill",
];

pub(crate) struct ToolHost {
    project: LoadedProject,
    database: Database,
    state: SharedState,
    cancel: CancellationToken,
    repair_plugin: Option<String>,
    references: std::collections::BTreeMap<String, SourceReference>,
}

impl ToolHost {
    fn control(&self) -> Result<KnowledgeControl, CoreError> {
        let mut state = self.state.lock().expect("run state");
        state.check()?;
        let remaining =
            ((state.progress.deadline_ms - chrono::Utc::now().timestamp_millis()).max(0) as u64)
                .min(
                    (state.progress.budget.timeout_seconds * 1000).saturating_sub(
                        state.prior_elapsed_ms + state.started.elapsed().as_millis() as u64,
                    ),
                );
        Ok(KnowledgeControl {
            cancel: self.cancel.clone(),
            deadline: Some(std::time::Instant::now() + std::time::Duration::from_millis(remaining)),
        })
    }
    fn require_knowledge(&self) -> Result<(), CoreError> {
        let state = self.state.lock().expect("run state");
        if !state.progress.with_skill || state.progress.activation.is_none() {
            return Err(CoreError::new(
                codes::INVALID_REQUEST,
                "knowledge requires --with-skill and a Core-verified activation in this run",
            ));
        }
        Ok(())
    }
    fn active_tool_names(&self) -> Vec<&'static str> {
        let with_skill = self.state.lock().expect("run state").progress.with_skill;
        available_tool_names(with_skill)
    }
    pub fn new(
        project: LoadedProject,
        database: Database,
        state: SharedState,
        cancel: CancellationToken,
        repair_plugin: Option<String>,
    ) -> Result<Self, CoreError> {
        if let Some(id) = &repair_plugin
            && !plugins::active_manifests(database.connection())?
                .iter()
                .any(|m| &m.plugin_id == id)
        {
            return Err(CoreError::new(
                codes::REFERENCE_MISSING,
                "repair requires an active plugin ID",
            ));
        }
        let saved_refs = state
            .lock()
            .expect("run state")
            .progress
            .investigation
            .as_ref()
            .and_then(|v| v.get("evidenceRefs"))
            .cloned()
            .unwrap_or_else(|| json!([]));
        let references = serde_json::from_value::<Vec<SourceReference>>(saved_refs)
            .unwrap_or_default()
            .into_iter()
            .map(|r| (r.source_id.clone(), r))
            .collect();
        Ok(Self {
            project,
            database,
            state,
            cancel,
            repair_plugin,
            references,
        })
    }

    fn query(&mut self, request: QueryRequest) -> Result<Value, CoreError> {
        let mut context = QueryContext {
            database: &mut self.database,
            project: Some(&self.project),
            now: now(),
        };
        project_knowledge_core::query::execute_with_cancellation(
            &mut context,
            &request,
            self.cancel.clone(),
        )
        .map(|r| serde_json::to_value(r).expect("query JSON"))
    }

    async fn dispatch(&mut self, name: &str, args: Value) -> Result<Value, CoreError> {
        if self.cancel.is_cancelled() {
            return Err(CoreError::new(codes::CANCELLED, "agent tool cancelled"));
        }
        {
            let mut state = self.state.lock().expect("run state");
            state.check()?;
            if state.progress.tool_calls >= state.progress.budget.max_tool_calls {
                return Err(state.fail(CoreError::new(
                    codes::BUDGET_EXHAUSTED,
                    "agent tool-call budget exhausted",
                )));
            }
            state.progress.tool_calls += 1;
            state.persist()?;
            if state.progress.activation.is_some()
                && matches!(
                    name,
                    "update-base"
                        | "write-candidate"
                        | "prepare-plugin"
                        | "validate-plugin"
                        | "run-plugin-cases"
                        | "import-conventions"
                )
            {
                return Err(CoreError::new(
                    codes::INVALID_REQUEST,
                    "plugin already activated; finish this run",
                ));
            }
        }
        match name {
            "get-capabilities" => {
                let schema = args
                    .get("schema")
                    .and_then(Value::as_str)
                    .unwrap_or("manifest");
                let value = match schema {
                    "candidate" => {
                        let (run, revision) = {
                            let state = self.state.lock().expect("run state");
                            (state.progress.run_id.clone(), state.progress.revision)
                        };
                        let files = read_bundle(&self.project, &run, revision)?;
                        if let Some(path) = args.get("path").and_then(Value::as_str) {
                            let file =
                                files.into_iter().find(|f| f.path == path).ok_or_else(|| {
                                    CoreError::new(
                                        codes::REFERENCE_MISSING,
                                        "candidate file not found",
                                    )
                                })?;
                            return Ok(
                                json!({"path":file.path,"contentHash":content_hash(file.content.as_bytes()),"content":file.content}),
                            );
                        }
                        return Ok(
                            json!({"revision":revision,"manifest":files.iter().find(|f|f.path=="manifest.json").and_then(|f|serde_json::from_str::<Value>(&f.content).ok()),"files":files.iter().map(|f|json!({"path":f.path,"contentHash":content_hash(f.content.as_bytes()),"bytes":f.content.len()})).collect::<Vec<_>>()}),
                        );
                    }
                    "identity" => {
                        use project_knowledge_core::contract::identity::{
                            IdentitySlot, compute_entity_id, entity_components,
                        };
                        if let Some(from) = args.get("fromId").and_then(Value::as_str) {
                            let kind = args
                                .get("kind")
                                .and_then(Value::as_str)
                                .and_then(project_knowledge_core::contract::RelationKind::parse)
                                .ok_or_else(|| {
                                    CoreError::new(codes::INVALID_REQUEST, "unknown relation kind")
                                        .with_issues(vec![
                                            project_knowledge_core::contract::Issue::new(
                                                codes::INVALID_REQUEST,
                                                "/kind",
                                            ),
                                        ])
                                })?;
                            let to: String =
                                parse(args.get("toId").cloned().unwrap_or(Value::Null))?;
                            let qualifiers: serde_json::Map<String, Value> = parse(
                                args.get("qualifiers").cloned().unwrap_or_else(|| json!({})),
                            )?;
                            let id =
                                project_knowledge_core::contract::identity::compute_relation_id(
                                    &self.project.config.project_id,
                                    kind,
                                    from,
                                    &to,
                                    &qualifiers,
                                )
                                .map_err(|_| {
                                    CoreError::new(
                                        codes::IDENTITY_CONFLICT,
                                        "invalid canonical relation qualifiers",
                                    )
                                })?;
                            return Ok(json!({"relationId":id}));
                        }
                        let kind: project_knowledge_core::contract::EntityKind =
                            parse(args.get("kind").cloned().unwrap_or(Value::Null))?;
                        let spec = entity_components(kind.clone());
                        if args.get("slots").is_none() {
                            return Ok(
                                json!({"kind":kind.as_str(),"identityComponents":spec.iter().map(|s|json!({"name":s.name,"required":s.required})).collect::<Vec<_>>(),"instructions":"Supply slots as a string-value object to compute an exact entityId; no hash guessing is needed."}),
                            );
                        }
                        let slots: std::collections::BTreeMap<String, String> =
                            parse(args.get("slots").cloned().unwrap_or(Value::Null))?;
                        if slots.keys().any(|k| !spec.iter().any(|s| s.name == k)) {
                            return Err(CoreError::new(
                                codes::IDENTITY_CONFLICT,
                                "identity contains an unknown slot",
                            ));
                        }
                        let ordered = spec
                            .iter()
                            .map(|s| match slots.get(s.name) {
                                Some(value) => Ok(IdentitySlot::required(s.name, value)),
                                None if !s.required => {
                                    Ok(IdentitySlot::optional(s.name, None::<String>))
                                }
                                None => Err(CoreError::new(
                                    codes::IDENTITY_CONFLICT,
                                    format!("missing identity slot {}", s.name),
                                )),
                            })
                            .collect::<Result<Vec<_>, CoreError>>()?;
                        let id = compute_entity_id(&self.project.config.project_id, kind, &ordered)
                            .map_err(|_| {
                                CoreError::new(
                                    codes::IDENTITY_CONFLICT,
                                    "invalid canonical identity slots",
                                )
                            })?;
                        return Ok(json!({"entityId":id,"identity":ordered}));
                    }
                    "relation" => match args.get("kind").and_then(Value::as_str) {
                        Some("transitions_to") => schemars::schema_for!(
                            project_knowledge_core::contract::TransitionQualifiers
                        )
                        .to_value(),
                        Some("joins") => {
                            schemars::schema_for!(project_knowledge_core::contract::JoinQualifiers)
                                .to_value()
                        }
                        _ => return Err(CoreError::new(
                            codes::INVALID_REQUEST,
                            "choose kind transitions_to or joins for its strict qualifiers schema",
                        )
                        .with_issues(vec![
                            project_knowledge_core::contract::Issue::new(
                                codes::INVALID_REQUEST,
                                "/kind",
                            ),
                        ])),
                    },
                    "manifest" => schemars::schema_for!(PluginManifest).to_value(),
                    "input" => schemars::schema_for!(CollectorInput).to_value(),
                    "output" => schemars::schema_for!(CollectorOutput).to_value(),
                    "case" => schemars::schema_for!(AcceptanceCase).to_value(),
                    "template" => {
                        json!({"manifest":include_str!("../../../collectors/templates/node/manifest.json"), "script":include_str!("../../../collectors/templates/node/src/collect.mjs"), "identity":include_str!("../../../scripts/identity.mjs")})
                    }
                    "repair" => {
                        let bundle = self.repair_bundle()?;
                        if let Some(path) = args.get("path").and_then(Value::as_str) {
                            return bundle["files"]
                                .as_array()
                                .and_then(|files| files.iter().find(|f| f["path"] == path))
                                .cloned()
                                .ok_or_else(|| {
                                    CoreError::new(
                                        codes::REFERENCE_MISSING,
                                        "active repair file not found",
                                    )
                                });
                        }
                        bundle
                    }
                    "topic" => schemars::schema_for!(TopicArtifact).to_value(),
                    "conventions" => schemars::schema_for!(ConventionsImport).to_value(),
                    _ => {
                        return Err(CoreError::new(
                            codes::INVALID_REQUEST,
                            "unknown capability schema",
                        ));
                    }
                };
                Ok(
                    json!({"protocolVersion":project_knowledge_core::contract::PROTOCOL_VERSION,"tools":self.active_tool_names(),"schemaHash":content_hash(value.to_string().as_bytes()),"schema":value,"knowledgeAvailable":true,"identityImportAvailable":true,"configHash":self.project.config_hash()?,"scopeIds":project_knowledge_core::index::effective_scopes(&self.project).iter().map(|s|&s.id).collect::<Vec<_>>()}),
                )
            }
            "status" => self.query(
                serde_json::from_value(json!({"operation":"status","checkFreshness":true}))
                    .expect("status DTO"),
            ),
            "search" => {
                let args: SearchArgs = parse(args)?;
                self.query(QueryRequest::Search(SearchRequest {
                    query: args.query,
                    page_size: args.page_size,
                    fields: None,
                    filters: None,
                    continuation: None,
                    budget: None,
                }))
            }
            "code-map" => {
                let args: MapArgs = parse(args)?;
                let result = self.query(QueryRequest::CodeMap(CodeMapRequest {
                    query: args.query,
                    max_files: args.max_files,
                    max_symbols: args.max_symbols,
                    roots: None,
                    filters: None,
                    budget: None,
                }))?;
                self.state.lock().expect("run state").progress.read_map = true;
                self.state.lock().expect("run state").progress.phase = "investigate".into();
                Ok(result)
            }
            "read-source" => {
                let args: ReadArgs = parse(args)?;
                let reference = self
                    .references
                    .get(&args.source_id)
                    .cloned()
                    .ok_or_else(|| {
                        CoreError::new(
                            codes::REFERENCE_MISSING,
                            "call export-snapshot and use a returned sourceId",
                        )
                    })?;
                let request = ReadSourceRequest {
                    references: vec![reference],
                    max_response_chars: Some(args.max_chars.unwrap_or(12000).min(12000)),
                    filters: None,
                };
                let result = self.query(QueryRequest::ReadSource(request))?;
                let current = result["data"]["reads"].as_array().is_some_and(|reads| {
                    !reads.is_empty() && reads.iter().all(|r| r["status"] == "current")
                });
                if !current {
                    return Err(CoreError::new(
                        codes::SOURCE_CHANGED,
                        "investigation requires current original sources",
                    ));
                }
                self.state.lock().expect("run state").progress.read_source = true;
                Ok(result)
            }
            "export-snapshot" => {
                let include: Vec<String> = parse(
                    args.get("include")
                        .cloned()
                        .unwrap_or_else(|| json!(["**/*"])),
                )?;
                let snapshot = self.database.current_source_snapshot_id()?.ok_or_else(|| {
                    CoreError::new(codes::INDEX_MISSING, "base snapshot is missing")
                })?;
                let stage = StageSpec {
                    id: "investigate".into(),
                    phase: StagePhase::Extract,
                    entry: "unused".into(),
                    args: vec![],
                    source_selectors: vec![SourceSelector {
                        id: "context".into(),
                        repository_ids: vec![],
                        include,
                        exclude: vec![],
                    }],
                    entity_selectors: vec![],
                    depends_on: vec![],
                    replacement: ReplacementScope::Project,
                };
                let run = self
                    .state
                    .lock()
                    .expect("run state")
                    .progress
                    .run_id
                    .clone();
                let input = project_knowledge_core::collect::import::export_input(
                    self.database.connection(),
                    &self.project,
                    &stage,
                    &snapshot,
                    &run,
                    "agent.context",
                    "1",
                )?;
                let references = project_knowledge_core::collect::import::context_references(
                    self.database.connection(),
                    &input,
                )?;
                for reference in &references {
                    self.references
                        .insert(reference.source_id.clone(), reference.clone());
                }
                Ok(json!({"input":input,"sourceReferences":references}))
            }
            "update-base" => {
                let result = project_knowledge_core::index::incremental::run_base_only(
                    &self.project,
                    &mut self.database,
                    &now(),
                )?;
                let mut state = self.state.lock().expect("run state");
                state.progress.source_snapshot_id = Some(result.base.snapshot.id.clone());
                state.progress.read_source = false;
                state.progress.investigation = None;
                state.progress.candidate_id = None;
                state.progress.validation_id = None;
                state.progress.status = "running".into();
                state.progress.phase = "investigate".into();
                self.references.clear();
                Ok(
                    json!({"snapshotId":result.base.snapshot.id,"generation":result.generation,"invalidated":result.invalidated,"nextSteps":["export-snapshot","read-source","import-conventions"],"instructions":"Prior investigation was invalidated. Read the new original snapshot and import conventions again before write-candidate."}),
                )
            }
            "import-conventions" => {
                let draft: Investigation = parse(args)?;
                if draft.identity_delta != IdentityDelta::default()
                    && draft.expected_config_hash.is_none()
                {
                    return Err(CoreError::new(
                        codes::REFERENCE_MISSING,
                        "identityDelta requires expectedConfigHash from current capabilities",
                    )
                    .with_issues(vec![
                        project_knowledge_core::contract::Issue::new(
                            codes::REFERENCE_MISSING,
                            "/expectedConfigHash",
                        ),
                    ]));
                }
                let evidence_refs = draft
                    .source_ids
                    .iter()
                    .map(|id| {
                        self.references.get(id).cloned().ok_or_else(|| {
                            CoreError::new(
                                codes::REFERENCE_MISSING,
                                "investigation source ID was not exported by Core",
                            )
                        })
                    })
                    .collect::<Result<Vec<_>, CoreError>>()?;
                {
                    let state = self.state.lock().expect("run state");
                    if !state.progress.read_map
                        || !state.progress.read_source
                        || evidence_refs.is_empty()
                        || draft.notes.len() > 16000
                    {
                        return Err(CoreError::new(
                            codes::EVIDENCE_INVALID,
                            "inspect code-map and current source, then supply bounded notes and evidenceRefs",
                        ));
                    }
                }
                if evidence_refs.len() > 32 {
                    return Err(CoreError::new(
                        codes::OUTPUT_LIMIT,
                        "at most 32 investigation references",
                    ));
                }
                for reference in &evidence_refs {
                    let reads = self.query(QueryRequest::ReadSource(ReadSourceRequest {
                        references: vec![reference.clone()],
                        max_response_chars: Some(4000),
                        filters: None,
                    }))?;
                    if !reads["data"]["reads"].as_array().is_some_and(|a| {
                        !a.is_empty() && a.iter().all(|r| r["status"] == "current")
                    }) {
                        return Err(CoreError::new(
                            codes::SOURCE_CHANGED,
                            "investigation evidence expired",
                        ));
                    }
                }
                let control = self.control()?;
                let result = knowledge::documents::import_conventions_with_control(
                    &mut self.database,
                    &self.project,
                    &ConventionsImport {
                        expected_config_hash: draft
                            .expected_config_hash
                            .unwrap_or(self.project.config_hash()?),
                        notes: draft.notes.clone(),
                        source_references: evidence_refs.clone(),
                        identity_delta: draft.identity_delta,
                    },
                    &now(),
                    &control,
                )?;
                self.project = LoadedProject::load(&self.project.config_path)?;
                let value = json!({"status":"imported","notes":draft.notes,"evidenceRefs":evidence_refs,"identityChangesApplied":result["indexInvalidated"],"convention":result});
                let mut state = self.state.lock().expect("run state");
                state.progress.config_hash = self.project.config_hash()?;
                state.progress.source_snapshot_id = self.database.current_source_snapshot_id()?;
                state.progress.investigation = Some(value.clone());
                state.progress.phase = "generate".into();
                Ok(value)
            }
            "export-topic-context" => {
                self.require_knowledge()?;
                let args: TopicArgs = parse(args)?;
                let allowed = (self
                    .state
                    .lock()
                    .expect("run state")
                    .progress
                    .budget
                    .max_input_chars
                    / 2)
                .min(256_000);
                let max_chars = args.max_chars.unwrap_or(80_000.min(allowed));
                if max_chars > allowed {
                    return Err(CoreError::new(
                        codes::OUTPUT_LIMIT,
                        "topic maxChars exceeds half of the run input budget",
                    ));
                }
                let operation = match args.view {
                    TopicView::Search => "search",
                    TopicView::LocateFeature => "locateFeature",
                    TopicView::Page => "page",
                    TopicView::Workflow => "workflow",
                    TopicView::Report => "report",
                };
                let mut value = json!({"operation":operation});
                value[if operation == "page" {
                    "route"
                } else {
                    "query"
                }] = json!(args.query);
                if let Some(ids) = args.scope_ids {
                    value["filters"] = json!({"scopeIds":ids});
                }
                if let Some(kinds) = args.kinds {
                    value["filters"]["kinds"] = json!(kinds);
                }
                let query: QueryRequest = parse(value)?;
                let control = self.control()?;
                let context = knowledge::context::export_with_control(
                    &mut self.database,
                    &self.project,
                    &TopicContextRequest {
                        topic_id: args.topic_id,
                        query,
                        max_chars,
                    },
                    &now(),
                    &control,
                )?;
                for r in &context.source_references {
                    if self.references.get(&r.source_id).is_none_or(|old| {
                        old.snapshot != r.snapshot
                            || old.end_byte - old.start_byte < r.end_byte - r.start_byte
                    }) {
                        self.references.insert(r.source_id.clone(), r.clone());
                    }
                }
                let mut state = self.state.lock().expect("run state");
                if !state
                    .progress
                    .topic_contexts
                    .contains(&context.input_digest)
                {
                    state
                        .progress
                        .topic_contexts
                        .push(context.input_digest.clone());
                }
                state.progress.phase = "knowledge".into();
                Ok(serde_json::to_value(context).expect("topic context"))
            }
            "import-topic" => {
                self.require_knowledge()?;
                let artifact: TopicArtifact = parse(args)?;
                if !self
                    .state
                    .lock()
                    .expect("run state")
                    .progress
                    .topic_contexts
                    .contains(&artifact.input_digest)
                {
                    return Err(CoreError::new(
                        codes::REFERENCE_MISSING,
                        "export this topic's Core context in the current run before import",
                    ));
                }
                let control = self.control()?;
                let result = knowledge::documents::import_topic_with_control(
                    &mut self.database,
                    &self.project,
                    &artifact,
                    &now(),
                    &control,
                )?;
                let mut state = self.state.lock().expect("run state");
                state
                    .progress
                    .topics
                    .retain(|t| t["topicId"] != result["topicId"]);
                state.progress.topics.push(result.clone());
                state.progress.skill = None;
                Ok(result)
            }
            "export-skill" => {
                self.require_knowledge()?;
                let args: SkillArgs = parse(args)?;
                if self
                    .state
                    .lock()
                    .expect("run state")
                    .progress
                    .topics
                    .is_empty()
                {
                    return Err(CoreError::new(
                        codes::REFERENCE_MISSING,
                        "import a topic from this run before exporting a Skill",
                    ));
                }
                let control = self.control()?;
                let output = self
                    .project
                    .base_dir
                    .join(".project-knowledge/skills")
                    .join(&args.name);
                let result = knowledge::skill::generate_with_checkpoint(
                    &mut self.database,
                    &self.project,
                    &args.name,
                    &output,
                    &now(),
                    |_| control.check(),
                )?;
                let mut state = self.state.lock().expect("run state");
                state.progress.skill = Some(result.clone());
                state.progress.phase = "knowledge".into();
                Ok(result)
            }
            "write-candidate" => {
                let mut bundle: Bundle = parse(args)?;
                if bundle.include_identity_helper {
                    if bundle
                        .files
                        .iter()
                        .any(|f| f.path.eq_ignore_ascii_case("identity.mjs"))
                    {
                        return Err(CoreError::new(
                            codes::INVALID_REQUEST,
                            "identity helper path is reserved when included",
                        ));
                    }
                    bundle.files.push(CandidateFile {
                        path: "identity.mjs".into(),
                        content: include_str!("../../../scripts/identity.mjs").into(),
                    });
                }
                let (run, revision) = {
                    let mut state = self.state.lock().expect("run state");
                    if state.progress.investigation.is_none() {
                        return Err(CoreError::new(
                            codes::EVIDENCE_INVALID,
                            "investigation is required before generation",
                        ));
                    }
                    if state.progress.activation.is_some() {
                        return Err(CoreError::new(
                            codes::INVALID_REQUEST,
                            "run already activated",
                        ));
                    }
                    if state.progress.needs_repair {
                        if state.progress.repairs >= state.progress.budget.max_repair_attempts {
                            return Err(state.fail(CoreError::new(
                                codes::BUDGET_EXHAUSTED,
                                "agent repair budget exhausted",
                            )));
                        }
                        state.progress.repairs += 1;
                        state.progress.needs_repair = false;
                    }
                    state.progress.revision += 1;
                    state.progress.status = "running".into();
                    state.progress.candidate_id = None;
                    state.progress.candidate_hash = None;
                    state.progress.validation_id = None;
                    state.progress.phase = "generate".into();
                    let pair = (state.progress.run_id.clone(), state.progress.revision);
                    state.persist()?;
                    pair
                };
                // The credential never crosses the candidate write boundary.
                let secret = self.state.lock().expect("run state").secret.clone();
                if bundle.files.iter().any(|f| f.content.contains(&secret)) {
                    return Err(CoreError::new(
                        codes::INVALID_REQUEST,
                        "candidate must not contain model credentials",
                    ));
                }
                // Assembly may span tool calls. Every write creates a full immutable
                // union; omission never deletes an existing file or assertion.
                let previous = (1..revision).rev().find(|n| {
                    self.project
                        .base_dir
                        .join("work/agent")
                        .join(&run)
                        .join(format!("revision-{n:03}"))
                        .is_dir()
                });
                if previous.is_some() || self.repair_plugin.is_some() {
                    let retained = match previous {
                        Some(previous) => read_bundle(&self.project, &run, previous)?,
                        None => serde_json::from_value::<Vec<CandidateFile>>(
                            self.repair_bundle()?["files"].clone(),
                        )
                        .map_err(|_| {
                            CoreError::new(codes::INTERNAL_ERROR, "invalid active repair bundle")
                        })?,
                    };
                    let mut merged = retained
                        .into_iter()
                        .map(|f| (f.path.clone(), f))
                        .collect::<std::collections::BTreeMap<_, _>>();
                    for file in bundle.files {
                        merged.insert(file.path.clone(), file);
                    }
                    bundle.files = merged.into_values().collect();
                }
                self.fill_truth_files(&mut bundle.files)?;
                let directory = write_bundle(&self.project, &run, revision, &bundle.files)?;
                if !directory.join("manifest.json").is_file() {
                    return Ok(
                        json!({"status":"draft","revision":revision,"needs":["manifest.json"]}),
                    );
                }
                let manifest = load_manifest(&directory.join("manifest.json"))?;
                if bundle.include_identity_helper
                    && !manifest.artifacts.iter().any(|p| p == "identity.mjs")
                {
                    return Err(CoreError::new(
                        codes::SCHEMA_INVALID,
                        "list identity.mjs in manifest.artifacts when including the helper",
                    ));
                }
                if self
                    .repair_plugin
                    .as_ref()
                    .is_some_and(|id| id != &manifest.plugin_id)
                {
                    return Err(CoreError::new(
                        codes::INVALID_REQUEST,
                        "repair must preserve plugin ID",
                    ));
                }
                if manifest.acceptance_cases.is_empty()
                    || manifest.stages.is_empty()
                    || manifest.capabilities.is_empty()
                {
                    return Err(CoreError::new(
                        codes::SCHEMA_INVALID,
                        "generated plugin requires capabilities, stages and acceptance cases",
                    ));
                }
                let checked =
                    validate_manifest(&manifest, &directory, &self.project.config.runtimes)?;
                self.check_cases(&directory, &manifest)?;
                let mut state = self.state.lock().expect("run state");
                state.progress.candidate_hash = Some(checked.artifact_hash.clone());
                state.progress.phase = "validate".into();
                Ok(
                    json!({"revision":revision,"artifactHash":checked.artifact_hash,"files":checked.files}),
                )
            }
            "prepare-plugin" | "validate-plugin" | "run-plugin-cases" => {
                self.ensure_submitted()?;
                let id = self
                    .state
                    .lock()
                    .expect("run state")
                    .progress
                    .candidate_id
                    .clone()
                    .expect("submitted");
                registry::prepare(&self.project, &self.database, &id).await?;
                if name == "prepare-plugin" {
                    return Ok(json!({"candidateId":id,"status":"prepared"}));
                }
                let report = registry::validate(
                    &self.project,
                    &mut self.database,
                    &id,
                    &self.cancel,
                    &now(),
                )
                .await?;
                let candidate = plugins::candidate(self.database.connection(), &id)?;
                {
                    let mut state = self.state.lock().expect("run state");
                    state.progress.validation_id = candidate.validation_id;
                    state.progress.phase = "activate".into();
                    state.persist()?;
                }
                let activation = self.activate().await?;
                Ok(json!({"validation":report,"activation":activation}))
            }
            "activate-plugin" => self.activate().await,
            _ => Err(CoreError::new(
                codes::INVALID_REQUEST,
                "tool is not allowed",
            )),
        }
    }

    fn check_cases(
        &self,
        directory: &std::path::Path,
        manifest: &PluginManifest,
    ) -> Result<(), CoreError> {
        // Revisions must retain the original assertions as a subset. Independent truth stays outside this writer.
        let state = self.state.lock().expect("run state");
        if state.progress.revision < 2 && self.repair_plugin.is_none() {
            return Ok(());
        }
        let previous = (1..state.progress.revision)
            .rev()
            .map(|n| {
                self.project
                    .base_dir
                    .join("work/agent")
                    .join(&state.progress.run_id)
                    .join(format!("revision-{n:03}"))
            })
            .find(|p| p.is_dir());
        let first = if let Some(previous) = previous {
            previous
        } else if let Some(id) = &self.repair_plugin {
            let active = plugins::active_manifests(self.database.connection())?
                .into_iter()
                .find(|m| &m.plugin_id == id)
                .ok_or_else(|| {
                    CoreError::new(
                        codes::REFERENCE_MISSING,
                        "repair plugin is no longer active",
                    )
                })?;
            plugins::published_manifest(self.database.connection(), id, &active.version)?
                .parent()
                .expect("active directory")
                .to_path_buf()
        } else {
            return Ok(());
        };
        let old_manifest = match load_manifest(&first.join("manifest.json")) {
            Ok(manifest) => manifest,
            Err(_) if self.repair_plugin.is_none() => return Ok(()),
            Err(error) => return Err(error),
        };
        if self.repair_plugin.is_some() {
            validate_manifest(&old_manifest, &first, &self.project.config.runtimes)?;
        }
        if old_manifest
            .capabilities
            .iter()
            .any(|c| !manifest.capabilities.contains(c))
            || old_manifest.stages.iter().any(|old| {
                manifest
                    .stages
                    .iter()
                    .find(|s| s.id == old.id)
                    .is_none_or(|new| {
                        new.source_selectors != old.source_selectors
                            || new.entity_selectors != old.entity_selectors
                            || new.phase != old.phase
                    })
            })
        {
            return Err(CoreError::new(
                codes::INVALID_REQUEST,
                "repair cannot narrow capabilities or input selectors",
            ));
        }
        for path in &old_manifest.acceptance_cases {
            // A missing/malformed initial artifact has no validated assertions to
            // freeze. Repair may fill it in; every existing valid case stays immutable.
            if project_knowledge_core::collect::manifest::validate_artifact_path(path).is_err() {
                continue;
            }
            if !manifest.acceptance_cases.contains(path) {
                return Err(CoreError::new(
                    codes::INVALID_REQUEST,
                    "repair cannot remove acceptance case paths",
                ));
            }
            let previous = match std::fs::read(first.join(path)) {
                Ok(bytes) => bytes,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(crate::config::io_error(e)),
            };
            let Ok(case) = serde_json::from_slice::<AcceptanceCase>(&previous) else {
                continue;
            };
            let current = std::fs::read(directory.join(path)).map_err(crate::config::io_error)?;
            if previous != current || !manifest.acceptance_cases.contains(path) {
                return Err(CoreError::new(
                    codes::INVALID_REQUEST,
                    "repair cannot remove or alter acceptance cases",
                ));
            }
            if project_knowledge_core::collect::manifest::validate_artifact_path(
                &case.input_fixture,
            )
            .is_err()
            {
                continue;
            }
            if first.join(&case.input_fixture).is_file()
                && std::fs::read(first.join(&case.input_fixture))
                    .map_err(crate::config::io_error)?
                    != std::fs::read(directory.join(&case.input_fixture))
                        .map_err(crate::config::io_error)?
            {
                return Err(CoreError::new(
                    codes::INVALID_REQUEST,
                    "repair cannot change retained acceptance inputs",
                ));
            }
        }
        Ok(())
    }

    // A case may package an exported original as its truthRef. Copy only exact,
    // unambiguous snapshot bytes with the already declared matching hash.
    fn fill_truth_files(&self, files: &mut Vec<CandidateFile>) -> Result<(), CoreError> {
        let Some(manifest) = files
            .iter()
            .find(|f| f.path == "manifest.json")
            .and_then(|f| serde_json::from_str::<PluginManifest>(&f.content).ok())
        else {
            return Ok(());
        };
        for path in &manifest.acceptance_cases {
            let Some(case) = files
                .iter()
                .find(|f| &f.path == path)
                .and_then(|f| serde_json::from_str::<AcceptanceCase>(&f.content).ok())
            else {
                continue;
            };
            let Some(truth) = case.truth_ref else {
                continue;
            };
            if files.iter().any(|f| f.path == truth)
                || project_knowledge_core::collect::manifest::validate_artifact_path(&truth)
                    .is_err()
            {
                continue;
            }
            let mut matches = Vec::new();
            for reference in self.references.values() {
                if case.truth_hash.as_deref() != Some(&reference.content_hash) {
                    continue;
                }
                if project_knowledge_core::storage::sources::source_path(
                    self.database.connection(),
                    &reference.source_id,
                )?
                .is_some_and(|(_, path)| path == truth)
                {
                    project_knowledge_core::storage::sources::validate_original_reference(
                        self.database.connection(),
                        reference,
                    )?;
                    if let Some((bytes, _)) =
                        project_knowledge_core::storage::sources::read_source_version(
                            self.database.connection(),
                            &reference.source_id,
                            &reference.content_hash,
                        )?
                    {
                        matches.push(bytes);
                    }
                }
            }
            if matches.len() == 1 {
                files.push(CandidateFile {
                    path: truth,
                    content: String::from_utf8(matches.pop().expect("one source")).map_err(
                        |_| CoreError::new(codes::EVIDENCE_INVALID, "truth source is not UTF-8"),
                    )?,
                });
            }
        }
        Ok(())
    }

    fn ensure_submitted(&mut self) -> Result<(), CoreError> {
        let (run, revision, existing) = {
            let state = self.state.lock().expect("run state");
            (
                state.progress.run_id.clone(),
                state.progress.revision,
                state.progress.candidate_id.clone(),
            )
        };
        if existing.is_some() {
            return Ok(());
        }
        if revision == 0 {
            return Err(CoreError::new(
                codes::INVALID_REQUEST,
                "write a candidate first",
            ));
        }
        let path = self
            .project
            .base_dir
            .join("work/agent")
            .join(run)
            .join(format!("revision-{revision:03}"));
        let manifest = load_manifest(&path.join("manifest.json"))?;
        validate_manifest(&manifest, &path, &self.project.config.runtimes)?;
        if manifest.acceptance_cases.is_empty()
            || manifest.capabilities.is_empty()
            || manifest.stages.is_empty()
        {
            return Err(CoreError::new(
                codes::SCHEMA_INVALID,
                "candidate requires capabilities, stages and cases",
            ));
        }
        self.check_cases(&path, &manifest)?;
        let candidate = registry::submit(&self.project, &mut self.database, &path, &now())?;
        let mut state = self.state.lock().expect("run state");
        state.progress.candidate_id = Some(candidate.id);
        state.progress.candidate_hash = Some(candidate.artifact_hash);
        state.persist()?;
        Ok(())
    }

    async fn activate(&mut self) -> Result<Value, CoreError> {
        let (id, auto) = {
            let state = self.state.lock().expect("run state");
            if let Some(value) = &state.progress.activation {
                return Ok(value.clone());
            }
            if state.progress.validation_id.is_none() {
                return Err(CoreError::new(
                    codes::INVALID_REQUEST,
                    "validation is required before activation",
                ));
            }
            (
                state
                    .progress
                    .candidate_id
                    .clone()
                    .expect("validated candidate"),
                state
                    .progress
                    .budget
                    .auto_activate
                    .unwrap_or(self.project.config.plugins.auto_activate),
            )
        };
        if !auto {
            let mut state = self.state.lock().expect("run state");
            state.progress.status = "pending_activation".into();
            return Ok(json!({"status":"pending_activation","candidateId":id}));
        }
        let result = registry::activate(&self.project, &mut self.database, &id, &now()).await?;
        let mut state = self.state.lock().expect("run state");
        state.progress.activation = Some(result.clone());
        state.progress.active_versions = plugins::active_version_map(self.database.connection())?;
        state.progress.source_snapshot_id = self.database.current_source_snapshot_id()?;
        state.progress.phase = "activated".into();
        state.persist()?;
        Ok(result)
    }

    fn repair_bundle(&self) -> Result<Value, CoreError> {
        let id = self
            .repair_plugin
            .as_ref()
            .ok_or_else(|| CoreError::new(codes::INVALID_REQUEST, "not a repair run"))?;
        let manifests = plugins::active_manifests(self.database.connection())?;
        let manifest = manifests
            .into_iter()
            .find(|m| &m.plugin_id == id)
            .ok_or_else(|| CoreError::new(codes::REFERENCE_MISSING, "active plugin not found"))?;
        let path = plugins::published_manifest(self.database.connection(), id, &manifest.version)?;
        let directory = path.parent().expect("active directory");
        let checked = validate_manifest(&manifest, directory, &self.project.config.runtimes)?;
        let files = checked
            .files
            .iter()
            .map(|file| {
                Ok(CandidateFile {
                    path: file.relative_path.clone(),
                    content: std::fs::read_to_string(directory.join(&file.relative_path))
                        .map_err(crate::config::io_error)?,
                })
            })
            .collect::<Result<Vec<_>, CoreError>>()?;
        Ok(
            json!({"files":files,"instructions":"Preserve plugin ID and acceptance cases. Choose a new version and repair extraction using current source."}),
        )
    }

    async fn call(&mut self, name: &str, args: Value) -> Value {
        let result = self.dispatch(name, args).await;
        let mut state = self.state.lock().expect("run state");
        let ok = result.is_ok();
        let mut result = match result {
            Ok(value) => value,
            Err(e) => {
                if matches!(
                    name,
                    "validate-plugin" | "run-plugin-cases" | "prepare-plugin"
                ) {
                    state.progress.needs_repair = true;
                }
                let value = json!({"error":e});
                state.progress.issues.push(value.clone());
                value
            }
        };
        redact(&mut result, &state.secret);
        let event = json!({"tool":name,"ok":ok,"turn":state.progress.model_turns,"revision":state.progress.revision});
        state.progress.events.push(event);
        if let Err(error) = state.persist() {
            state.fail(error.clone());
            return json!({"error":error});
        }
        let limit = if matches!(name, "get-capabilities" | "export-topic-context") {
            state.progress.budget.max_input_chars as usize / 2
        } else {
            16000
        };
        if result.to_string().chars().count() > limit {
            return json!({"error":{"code":"OUTPUT_LIMIT","message":"tool result exceeds context limit; request a narrower scope or schema"}});
        }
        result
    }
}

fn is_knowledge_tool(name: &str) -> bool {
    matches!(
        name,
        "export-topic-context" | "import-topic" | "export-skill"
    )
}
pub(crate) fn available_tool_names(with_skill: bool) -> Vec<&'static str> {
    TOOL_NAMES
        .iter()
        .copied()
        .filter(|name| with_skill || !is_knowledge_tool(name))
        .collect()
}
fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}
fn parse<T: serde::de::DeserializeOwned + schemars::JsonSchema>(
    value: Value,
) -> Result<T, CoreError> {
    project_knowledge_core::contract::schema::validate::<T>(&value, "")?;
    serde_json::from_value(value).map_err(|_| {
        CoreError::new(
            codes::INVALID_REQUEST,
            "tool arguments do not match the Core DTO",
        )
    })
}

pub(crate) struct Command {
    name: &'static str,
    args: Value,
    response: tokio::sync::oneshot::Sender<Value>,
}

pub(crate) async fn invoke(
    sender: tokio::sync::mpsc::UnboundedSender<Command>,
    name: &'static str,
    args: Value,
) -> Value {
    let (response, receiver) = tokio::sync::oneshot::channel();
    if sender
        .send(Command {
            name,
            args,
            response,
        })
        .is_err()
    {
        return json!({"error":{"code":"CLOSED","message":"Core tool owner is closed"}});
    }
    receiver.await.unwrap_or_else(
        |_| json!({"error":{"code":"CANCELLED","message":"Core tool owner stopped"}}),
    )
}

// rusqlite's connection is Send, not Sync. Its complete async lifecycle stays on
// one owned thread, including cancellation and subprocess cleanup.
pub(crate) fn start(
    mut host: ToolHost,
    cancel: CancellationToken,
) -> (
    tokio::sync::mpsc::UnboundedSender<Command>,
    std::thread::JoinHandle<()>,
) {
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<Command>();
    let handle = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Core tool runtime");
        runtime.block_on(async move {
            loop {
                let command=tokio::select! { biased; _=cancel.cancelled()=>break, value=receiver.recv()=>match value {Some(v)=>v,None=>break} };
                let result = if command.name == "__verify-completion" {
                    let result = (|| {
                        let control = host.control()?;
                        let progress = host.state.lock().expect("run state").progress.clone();
                        crate::runner::verify_completion(&host.project, &mut host.database, &progress, cancel.clone())?;
                        control.check()
                    })();
                    match result { Ok(()) => json!({"verified":true}), Err(error) => json!({"error":error}) }
                } else {
                    tokio::select! { biased; _=cancel.cancelled()=>break, value=host.call(command.name,command.args)=>value }
                };
                let _=command.response.send(result);
            }
        });
    });
    (sender, handle)
}

pub(crate) fn register(
    sender: tokio::sync::mpsc::UnboundedSender<Command>,
    smoke: bool,
    repair: bool,
    with_skill: bool,
) -> Vec<DynamicTool> {
    available_tool_names(with_skill).into_iter().filter(|name|!smoke||*name=="status").map(|name|{
        let parameters=match name {
            "search"=>schemars::schema_for!(SearchArgs).to_value(),
            "code-map"=>schemars::schema_for!(MapArgs).to_value(),
            "read-source"=>schemars::schema_for!(ReadArgs).to_value(),
            "write-candidate"=>schemars::schema_for!(Bundle).to_value(),
            "import-conventions"=>schemars::schema_for!(Investigation).to_value(),
            "export-topic-context"=>schemars::schema_for!(TopicArgs).to_value(),
            "import-topic"=>schemars::schema_for!(TopicArtifact).to_value(),
            "export-skill"=>schemars::schema_for!(SkillArgs).to_value(),
            "get-capabilities"=>json!({"type":"object","properties":{"schema":{"type":"string","enum":if repair{vec!["manifest","input","output","case","template","identity","relation","candidate","repair","topic","conventions"]}else{vec!["manifest","input","output","case","template","identity","relation","candidate","topic","conventions"]}},"kind":{"type":"string"},"slots":{"type":"object","additionalProperties":{"type":"string"}},"fromId":{"type":"string"},"toId":{"type":"string"},"qualifiers":{"type":"object","additionalProperties":true},"path":{"type":"string"}},"additionalProperties":false}),
            "export-snapshot"=>json!({"type":"object","properties":{"include":{"type":"array","items":{"type":"string"}}},"additionalProperties":false}),
            _=>json!({"type":"object","properties":{},"additionalProperties":false}),
        };
        let parameters=inline_parameters(parameters);
        let sender=sender.clone();
        let description=match name {
            "get-capabilities"=>"Read ONE schema: output, input, case, manifest, template, topic. schema=relation with kind=transitions_to or joins returns its strict qualifiers schema. schema=identity with entity kind returns required slots; add string slots object to compute entityId. For relationId, pass relation kind, fromId, toId and qualifiers instead of slots. Use exact IDs in cases; never guess hashes. schema=candidate lists draft files; add path to inspect one. Do not call every enum value.",
            "status"=>"Read Core project freshness and coverage. No arguments.",
            "code-map"=>"Read a bounded map. Pass {} or query; numeric limits must be JSON numbers. No roots or filters arguments.",
            "search"=>"Search indexed code. Required query is a string; optional pageSize is an integer.",
            "export-snapshot"=>"Export CollectorInput and references. Pass include as an array of globs. Returned source IDs work with read-source and import-conventions; input is a case fixture after changing stageId to your stage.",
            "read-source"=>"Read current original source. Pass sourceId from export-snapshot, optionally maxChars as a number. Host supplies and validates the full reference.",
            "import-conventions"=>"After code-map and read-source, import notes and sourceIds through Core. Optional identityDelta requires expectedConfigHash from capabilities. If identityChangesApplied, call update-base and inspect the new snapshot before building a candidate.",
            "export-topic-context"=>"After activation, select current original facts for a topic. topicId is a safe identifier; query is a search term or page route; view is search, locateFeature, page, workflow or report. Budget includes the whole context. Keep inputDigest, snapshot, hashes and all gaps for import.",
            "import-topic"=>"Import Core TopicArtifact from context exported in this run. Each explanation line cites [[claim:id]]. Retain context gaps; AI judgements must be inferred with gapIds. Return original source references unchanged. Inspect schema=topic when needed.",
            "export-skill"=>"After importing current topics, render a complete Skill. Pass name only; Core owns output directory, freshness and atomic publication. Partial or stale results are not complete.",
            "write-candidate"=>"Write file updates as path/content objects. Updates merge into a new immutable revision; omitted files remain. A repair run starts from the verified active bundle, so send only changed files (normally the new-version manifest); existing scripts/cases/inputs are retained. schema=repair with path=manifest.json reads that original manifest. includeIdentityHelper=true adds identity.mjs (list it in artifacts). Cases and input JSON must exist before validate-plugin. Repair budget counts modifications after a validation failure, not assembly calls.",
            "validate-plugin"|"run-plugin-cases"=>if with_skill {"Submit, prepare, validate cases and activate under project policy. No arguments. After activation continue export-topic-context, import-topic and export-skill in this same run; otherwise repair within budget."} else {"Submit candidate, prepare dependencies, run Core validators and cases, then activate under project policy. No arguments. On successful activation stop; otherwise repair within budget."},
            "activate-plugin"=>"Activate the validated candidate under project policy. No arguments; idempotent.",
            _=>"Core maintenance on this run. No arbitrary commands or business source edits.",
        };
        DynamicTool::new(ToolName::new(name).expect("static tool name"),description,parameters,move |args|{
            let sender=sender.clone();Box::pin(async move{
                let value=invoke(sender,name,args).await;
                Ok(ToolOutput::json(value))
            })
        })
    }).collect()
}

/// Provider tool schemas use the same Rust types, with local references expanded
/// for gateways that only understand inline function parameters.
fn inline_parameters(mut schema: Value) -> Value {
    let defs = schema.get("$defs").cloned().unwrap_or_else(|| json!({}));
    fn expand(value: &mut Value, defs: &Value, depth: usize) {
        assert!(depth < 32, "tool parameter schemas must not be recursive");
        if let Some(name) = value
            .get("$ref")
            .and_then(Value::as_str)
            .and_then(|r| r.strip_prefix("#/$defs/"))
        {
            let replacement = defs
                .get(name)
                .expect("local tool schema definition")
                .clone();
            *value = replacement;
        }
        if let Some(variants) = value.get("anyOf").and_then(Value::as_array)
            && variants.len() == 2
            && variants
                .iter()
                .any(|v| v.get("type") == Some(&json!("null")))
        {
            *value = variants
                .iter()
                .find(|v| v.get("type") != Some(&json!("null")))
                .expect("non-null parameter")
                .clone();
        }
        if let Some(types) = value.get("type").and_then(Value::as_array) {
            let types = types
                .iter()
                .filter(|t| t.as_str() != Some("null"))
                .cloned()
                .collect::<Vec<_>>();
            if types.len() == 1 {
                value["type"] = types[0].clone();
            }
        }
        match value {
            Value::Object(map) => {
                map.remove("$defs");
                map.remove("$schema");
                map.remove("title");
                for item in map.values_mut() {
                    expand(item, defs, depth + 1);
                }
            }
            Value::Array(items) => items
                .iter_mut()
                .for_each(|item| expand(item, defs, depth + 1)),
            _ => {}
        }
    }
    schema
        .as_object_mut()
        .expect("tool object schema")
        .remove("$defs");
    expand(&mut schema, &defs, 0);
    schema
}
