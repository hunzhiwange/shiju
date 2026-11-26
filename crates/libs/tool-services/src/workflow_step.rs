//! Explicit workflow action for a scheduler-owned execution.
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use tool_types::{Tool, ToolCallResult, ToolResult, ToolSpec};

pub struct MoveWorkflowStepTool<H>(std::marker::PhantomData<H>);
impl<H> MoveWorkflowStepTool<H> {
    pub fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    action: Action,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Action {
    Complete,
    Return,
}
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<H: tool_context::host::ContextHost> Tool for MoveWorkflowStepTool<H> {
    fn name(&self) -> &str {
        "move_workflow_step"
    }
    fn description(&self) -> &str {
        "Explicitly complete or return the current workflow step using its configured target. Available only to the current scheduler run."
    }
    fn parameters_schema(&self) -> Value {
        json!({"type":"object", "additionalProperties":false,
        "required":["action"], "properties":{"action":{"type":"string","enum":["complete","return"]}}})
    }
    fn spec(&self) -> ToolSpec {
        ToolSpec::new(self.name(), self.description(), self.parameters_schema())
            .with_read_only(false)
            .with_concurrency_safe(false)
            .with_requires_user_interaction(false)
            .with_strict(true)
    }
    async fn execute(&self, input: Value) -> anyhow::Result<ToolResult> {
        let result = self.call(input).await?;
        Ok(ToolResult { success: true, output: result.data.to_string(), error: None })
    }

    async fn call(&self, input: Value) -> anyhow::Result<ToolCallResult> {
        let args: Args = serde_json::from_value(input)?;
        let context = tool_context::context::current_tool_use_context::<H>()
            .ok_or_else(|| anyhow::anyhow!("missing tool context"))?;
        #[cfg(not(target_arch = "wasm32"))]
        {
            let owner = context
                .run_worktree_owner()
                .ok_or_else(|| anyhow::anyhow!("workflow action requires a scheduler run"))?;
            let data = owner.move_workflow_step(matches!(args.action, Action::Return))?;
            Ok(ToolCallResult {
                data: data.clone(),
                model_result: data.clone(),
                content_blocks: vec![vw_api_tools::ToolResultContentDto::Json { value: data }],
                ..Default::default()
            })
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (args, context);
            anyhow::bail!("workflow action requires the native scheduler")
        }
    }
}
