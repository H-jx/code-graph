//! 知识材料组织与输出：主题上下文收集、LLM 产物导入校验与 Skill 渲染。

pub mod context;
pub mod documents;
pub mod skill;

/// A caller's cancellation and remaining deadline, shared by all knowledge steps.
#[derive(Default)]
pub struct KnowledgeControl {
    pub cancel: tokio_util::sync::CancellationToken,
    pub deadline: Option<std::time::Instant>,
}
impl KnowledgeControl {
    pub fn check(&self) -> Result<(), crate::contract::CoreError> {
        use crate::contract::{CoreError, codes};
        if self.cancel.is_cancelled() {
            return Err(CoreError::new(
                codes::CANCELLED,
                "knowledge operation cancelled",
            ));
        }
        if self
            .deadline
            .is_some_and(|end| std::time::Instant::now() >= end)
        {
            return Err(CoreError::new(
                codes::BUDGET_EXHAUSTED,
                "knowledge operation exceeded the caller deadline",
            ));
        }
        Ok(())
    }
    pub(crate) fn query(
        &self,
        database: &mut crate::storage::Database,
        project: &crate::project::LoadedProject,
        request: &crate::contract::QueryRequest,
        now: &str,
    ) -> Result<crate::contract::QueryResponse<serde_json::Value>, crate::contract::CoreError> {
        self.check()?;
        let mut value = serde_json::to_value(request).expect("query DTO");
        if let Some(end) = self.deadline
            && !matches!(value["operation"].as_str(), Some("status" | "readSource"))
        {
            let remaining = end
                .saturating_duration_since(std::time::Instant::now())
                .as_millis()
                .max(1) as u64;
            let timeout = value["budget"]["timeoutMs"]
                .as_u64()
                .unwrap_or(crate::contract::query::default_budget::TIMEOUT_MS)
                .min(remaining);
            if value["budget"].is_null() {
                value["budget"] = serde_json::json!({});
            }
            value["budget"]["timeoutMs"] = serde_json::json!(timeout);
        }
        let bounded = serde_json::from_value(value).expect("bounded query DTO");
        let response = crate::query::execute_with_cancellation(
            &mut crate::query::QueryContext {
                database,
                project: Some(project),
                now: now.into(),
            },
            &bounded,
            self.cancel.clone(),
        );
        self.check()?;
        response
    }
}
