//! Workflow 路由组合与宿主工具适配。
use std::sync::Arc;
use axum::Router;
// Workflow value conversion is owned by gateway-workflow.

use vw_api_transport::TenantScope;
use vw_api_workflow::WorkflowRunRequest;
use crate::app::agent::gateway::ApiError;
use crate::app::agent::gateway::state::AppState;
use crate::app::agent::tools::ToolRuntimeContext;
use workflow_engine::WorkflowRuntime;

pub(crate) fn router() -> Router<AppState> {
    gateway_workflow::server::router(gateway_workflow::server::RuntimeFactory(workflow_runtime))
}
pub(crate) use gateway_workflow::server::applications_router;
// Workflow input policy is owned by gateway-workflow::tool_provider.
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

type GatewayWorkflowToolProvider = gateway_workflow::tool_provider::GatewayWorkflowToolProvider<crate::tools::context::AgentContextHost>;
#[cfg(test)]
use gateway_workflow::tool_provider::{workflow_tool_call_error_text, workflow_request_root};
fn workflow_tool_runtime_context(request: &WorkflowRunRequest) -> ToolRuntimeContext {
    gateway_workflow::tool_provider::workflow_tool_runtime_context::<crate::tools::context::AgentContextHost>(request)
}

// Keep host integration tests on the same implementation source, with crate-local
// visibility; the production library does not expose protocol internals for tests.
#[cfg(test)]
#[path = "../../../../../libs/gateway-workflow/src/server.rs"]
mod server_test_source;
#[cfg(test)]
#[path = "../../../../../libs/gateway-workflow/src/host_tests.rs"]
mod workflow_tests;
