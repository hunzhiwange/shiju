//! Workflow tool boundary validation and execution over the existing typed runtime.
use serde_json::Value;
use vw_api_transport::TenantScope;
use vw_api_workflow::WorkflowRunRequest;
use tool_assembly::runtime::{RuntimeHost, ToolRuntimeContext, execute_tool_call};
use tool_assembly::ToolCallError;
use tool_context::context::ToolUseContext;
use workflow_engine::{WorkflowToolProvider, WorkflowToolRequest, WorkflowToolResult};
pub const WORKFLOW_INPUT_ROOT: &str = "__vw_root";
pub struct GatewayWorkflowToolProvider<H: RuntimeHost> {
    pub ctx: ToolRuntimeContext<H>,
    pub tenant: TenantScope,
}

#[async_trait::async_trait]
impl<H: RuntimeHost> WorkflowToolProvider for GatewayWorkflowToolProvider<H> {
    async fn call(&self, request: WorkflowToolRequest) -> Result<WorkflowToolResult, String> {
        session_store::ui_store::with_tenant_scope_sync(self.tenant.clone(), || {
            if !matches!(request.provider.as_str(), "shiju" | "vibewindow")
                || request.action.is_some()
                || request.credential_id.is_some()
            {
                return Err(
                    "工作流工具必须绑定已有工具，不能覆盖 provider/action/credential".into()
                );
            }
            let input = Value::Object(request.inputs.into_iter().collect());
            let input_text = serde_json::to_string(&input)
                .map_err(|error| format!("workflow tool 输入序列化失败: {error}"))?;
            let result = execute_tool_call(request.tool_name.as_str(), &input_text, &self.ctx)
                .map_err(workflow_tool_call_error_text)?;
            // Shell terminal snapshots remain protocol data even on failure.
            // A workflow must still stop before downstream side effects.
            if !result.is_success() {
                return Err(result.error_text().unwrap_or_else(|| "workflow tool failed".into()));
            }
            let model_result = result.default_model_result();
            let model_text = result.model_text();
            let json = (!result.data.is_null()).then_some(result.data.clone());

            Ok(WorkflowToolResult {
                result: if result.data.is_null() { model_result } else { result.data },
                text: (!model_text.trim().is_empty()).then_some(model_text),
                json,
                files: Vec::new(),
            })
        })
    }
}

pub fn workflow_tool_call_error_text(error: ToolCallError) -> String {
    match error {
        ToolCallError::Denied { message, .. } | ToolCallError::Failed(message) => message,
    }
}

pub fn workflow_tool_runtime_context<H: RuntimeHost>(request: &WorkflowRunRequest) -> ToolRuntimeContext<H> {
    let root = workflow_request_root(request);
    // Workflow inputs are data, never an approval or a session capability.
    let tool_use_context = ToolUseContext::<H>::new("workflow", root.clone()).with_channel("workflow");
    ToolRuntimeContext::new("workflow", root).with_tool_use_context(tool_use_context)
}

pub fn workflow_request_root(request: &WorkflowRunRequest) -> Option<String> {
    request
        .inputs
        .get(WORKFLOW_INPUT_ROOT)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}
