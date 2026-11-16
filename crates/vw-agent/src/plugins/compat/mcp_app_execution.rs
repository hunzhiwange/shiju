//! Application execution and approval bridge; bindings belong to plugin-mcp.
use crate::tools::{ToolRuntimeContext, ToolCallResult};
use anyhow::Result;
use serde_json::Value;
use std::{sync::{Arc, Mutex}, path::Path};
use tokio_util::sync::CancellationToken;
use vw_api_transport::TenantScope;
struct AppContext {
    context: ToolRuntimeContext,
    prompts: Option<Arc<Mutex<tokio::sync::mpsc::UnboundedReceiver<tool_context::approval::NonCliApprovalPrompt>>>>,
}
impl plugin_mcp::mcp_app_execution::AppContext for AppContext {
    fn session(&self) -> &str { &self.context.session }
    fn root(&self) -> Option<&str> { self.context.root.as_deref() }
    fn abort_token(&self) -> Option<CancellationToken> { self.context.tool_use_context().abort_token().cloned() }
    fn tool_id(&self, server: &str, name: &str) -> Result<String> { crate::tools::mcp_tool::mcp_tool_id(server, name) }
    fn contains_tool(&self, id: &str) -> bool { self.context.session_tools().tools().iter().any(|tool| tool.name() == id) }
    fn prepare_approval(&self, cancelled: CancellationToken) -> Arc<dyn plugin_mcp::mcp_app_execution::AppContext> {
        // Pages cannot inherit the model turn's full-access or approval bypass.
        let (prompt_tx, prompt_rx) = tokio::sync::mpsc::unbounded_channel();
        let approval = (*self.context.tool_use_context()).clone()
            .with_full_access_enabled(false).with_bypass_non_cli_approval_for_turn(false)
            .with_abort_token(cancelled).with_channel("desktop")
            .with_non_cli_approval_context(tool_context::approval::NonCliApprovalContext {
                sender: "desktop".into(), reply_target: self.context.session.clone(), prompt_tx,
            });
        Arc::new(Self { context: self.context.clone().with_tool_use_context(approval), prompts: Some(Arc::new(Mutex::new(prompt_rx))) })
    }
    fn drain_prompts(&self) {
        if let Some(prompts) = &self.prompts {
            while prompts.lock().unwrap_or_else(std::sync::PoisonError::into_inner).try_recv().is_ok() {}
        }
    }
    fn execute(&self, id: &str, input: &str) -> Result<ToolCallResult> {
        Ok(crate::tools::execute_bound_app_tool(id, input, &self.context)?)
    }
}
pub(crate) fn register(context: &ToolRuntimeContext, requested: &str, result: &mut ToolCallResult) -> Result<()> {
    plugin_mcp::mcp_app_execution::register(Arc::new(AppContext { context: context.clone(), prompts: None }), crate::session::ui_store::current_tenant_scope(), requested, result)
}
pub(crate) fn revoke(session: &str, root: &Path) {
    revoke_for_tenant(&crate::session::ui_store::current_tenant_scope(), session, root);
}
pub(crate) fn revoke_for_tenant(tenant: &TenantScope, session: &str, root: &Path) {
    plugin_mcp::mcp_app_execution::revoke_for_tenant(tenant, session, root);
}
pub(crate) fn call(session: &str, server: &str, handle: &str, name: &str, arguments: Value) -> Result<Value> {
    plugin_mcp::mcp_app_execution::call(&crate::session::ui_store::current_tenant_scope(), session, server, handle, name, arguments)
}
