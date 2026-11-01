use rig::{
    AgentBuilder,
    agent::{AgentHook, HookContext, OutcomeAction, OutcomeEvent},
    completion::PromptError,
    test_utils::{MockCompletionModel, MockTurn},
    tool::{Tool, ToolContext, ToolExecutionError, ToolOutput},
};
use serde::Deserialize;
use serde_json::json;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Deserialize)]
struct Args {
    value: i32,
    #[serde(default)]
    fail: bool,
}
#[derive(Clone, Default)]
struct Inspect(Arc<AtomicUsize>);
impl Tool for Inspect {
    const NAME: &'static str = "inspect";
    type Args = Args;
    type Output = serde_json::Value;
    type Error = ToolExecutionError;
    fn description(&self) -> String {
        "Return a checked value".into()
    }
    fn parameters(&self) -> serde_json::Value {
        json!({"type":"object","properties":{"value":{"type":"integer"},"fail":{"type":"boolean"}},"required":["value"]})
    }
    async fn call(
        &self,
        _context: &mut ToolContext,
        args: Args,
    ) -> Result<Self::Output, Self::Error> {
        self.0.fetch_add(1, Ordering::SeqCst);
        if args.fail {
            return Err(ToolExecutionError::invalid_args("repair the supplied value").with_model_output(ToolOutput::json(json!({
                "error":{"code":"SCHEMA_INVALID","issues":[{"instancePath":"/value","repairHint":"provide a nonnegative value"}]}
            }))));
        }
        Ok(json!({"value":args.value}))
    }
}

#[tokio::test]
async fn rig_two_turn_tool_loop_and_structured_error_reach_the_next_request() {
    for fail in [false, true] {
        let model = MockCompletionModel::from_turns([
            MockTurn::tool_call("call-1", "inspect", json!({"value":42,"fail":fail})),
            MockTurn::text("done"),
        ]);
        let tool = Inspect::default();
        let agent = AgentBuilder::new(model.clone()).tool(tool.clone()).build();
        let response = agent
            .prompt("inspect the value")
            .max_turns(2)
            .run()
            .await
            .expect("tool loop");
        assert_eq!(response.output(), "done");
        assert_eq!(tool.0.load(Ordering::SeqCst), 1);
        assert_eq!(model.request_count(), 2);
        let requests = model.requests();
        let second = serde_json::to_value(&requests[1]).unwrap().to_string();
        if fail {
            assert!(second.contains("SCHEMA_INVALID"));
            assert!(second.contains("/value"));
            assert!(second.contains("repairHint"));
        } else {
            assert!(second.contains("42"));
        }
    }
}

#[tokio::test]
async fn rig_max_turns_counts_real_model_requests() {
    let model = MockCompletionModel::from_turns([
        MockTurn::tool_call("call-1", "inspect", json!({"value":42})),
        MockTurn::text("must not run"),
    ]);
    let tool = Inspect::default();
    let agent = AgentBuilder::new(model.clone()).tool(tool.clone()).build();
    let error = agent
        .prompt("inspect")
        .max_turns(1)
        .run()
        .await
        .expect_err("budget");
    assert!(matches!(error, PromptError::MaxTurns { max_turns: 1, .. }));
    assert_eq!(model.request_count(), 1);
    assert_eq!(tool.0.load(Ordering::SeqCst), 1);
}

struct StopAfterTool;
impl AgentHook for StopAfterTool {
    async fn on_outcome(&self, _context: &HookContext, event: OutcomeEvent<'_>) -> OutcomeAction {
        if event.tool_name() == Some("inspect") {
            OutcomeAction::stop("probe cancelled")
        } else {
            OutcomeAction::proceed()
        }
    }
}
#[tokio::test]
async fn rig_hook_cancellation_stops_further_model_requests() {
    let model = MockCompletionModel::from_turns([
        MockTurn::tool_call("call-1", "inspect", json!({"value":42})),
        MockTurn::text("must not run"),
    ]);
    let agent = AgentBuilder::new(model.clone())
        .tool(Inspect::default())
        .build();
    let error = agent
        .prompt("inspect")
        .max_turns(4)
        .add_hook(StopAfterTool)
        .run()
        .await
        .expect_err("cancel");
    assert!(matches!(error, PromptError::Cancelled { .. }));
    assert_eq!(model.request_count(), 1);
}

#[test]
fn rig_provider_accepts_explicit_key_and_custom_base_url_without_network() {
    let key = std::env::var("PROJECT_KNOWLEDGE_PROBE_KEY")
        .unwrap_or_else(|_| "nonsecret-probe-key".into());
    let config = rig::providers::openai::OpenAIConfig::new(key.clone())
        .with_base_url("http://127.0.0.1:9/v1");
    assert!(config.api_key.expose() == key);
    assert_eq!(config.base_url, "http://127.0.0.1:9/v1");
    let _agent = AgentBuilder::new(config.client().chat("probe-only")).build();
}
