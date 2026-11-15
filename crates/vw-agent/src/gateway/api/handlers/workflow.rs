//! Workflow 路由组合与宿主工具适配。
use std::sync::Arc;
use axum::Router;
use serde_json::Value;
use vw_api_transport::TenantScope;
use vw_api_workflow::WorkflowRunRequest;
use crate::app::agent::gateway::ApiError;
use crate::app::agent::gateway::state::AppState;
use crate::app::agent::tools::{
    ToolCallError, ToolRuntimeContext, ToolUseContext, execute_tool_call,
};
use workflow_engine::{WorkflowRuntime, WorkflowToolProvider, WorkflowToolRequest, WorkflowToolResult};

pub(crate) fn router() -> Router<AppState> {
    gateway_workflow::server::router(gateway_workflow::server::RuntimeFactory(workflow_runtime))
}
pub(crate) use gateway_workflow::server::applications_router;
const WORKFLOW_INPUT_ROOT: &str = "__vw_root";
fn workflow_runtime(
    state: &AppState,
    tenant: TenantScope,
    request: &WorkflowRunRequest,
) -> Result<WorkflowRuntime, ApiError> {
    Ok(WorkflowRuntime {
        video_edit_budget: crate::workflow::budget::VideoEditRunBudget::default(),
        provider: state.provider.clone(),
        knowledge_provider: Some(Arc::new(agent_knowledge::runtime_store::knowledge_store_from_config(
            state.config.lock().clone(), &agent_paths::paths().home, tenant.clone(),
        )?)),
        document_extractor: None,
        tool_provider: Some(Arc::new(GatewayWorkflowToolProvider {
            ctx: crate::session::ui_store::with_tenant_scope_sync(tenant.clone(), || {
                workflow_tool_runtime_context(request)
            }),
            tenant,
        })),
        agent_provider: None,
        pause_store: None,
        model: state.model.clone(),
        temperature: state.temperature,
    })
}

struct GatewayWorkflowToolProvider {
    ctx: ToolRuntimeContext,
    tenant: TenantScope,
}

#[async_trait::async_trait]
impl WorkflowToolProvider for GatewayWorkflowToolProvider {
    async fn call(&self, request: WorkflowToolRequest) -> Result<WorkflowToolResult, String> {
        crate::session::ui_store::with_tenant_scope_sync(self.tenant.clone(), || {
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

fn workflow_tool_call_error_text(error: ToolCallError) -> String {
    match error {
        ToolCallError::Denied { message, .. } | ToolCallError::Failed(message) => message,
    }
}

fn workflow_tool_runtime_context(request: &WorkflowRunRequest) -> ToolRuntimeContext {
    let root = workflow_request_root(request);
    // Workflow inputs are data, never an approval or a session capability.
    let tool_use_context = ToolUseContext::new("workflow", root.clone()).with_channel("workflow");
    ToolRuntimeContext::new("workflow", root).with_tool_use_context(tool_use_context)
}

fn workflow_request_root(request: &WorkflowRunRequest) -> Option<String> {
    request
        .inputs
        .get(WORKFLOW_INPUT_ROOT)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}


// Keep host integration tests on the same implementation source, with crate-local
// visibility; the production library does not expose protocol internals for tests.
#[cfg(test)]
#[path = "../../../../../libs/gateway-workflow/src/server.rs"]
mod server_test_source;
#[cfg(test)]
#[path = "../../../../../libs/gateway-workflow/src/host_tests.rs"]
mod workflow_tests;
