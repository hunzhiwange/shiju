//! Host lifecycle composition; plugin-hooks owns the single state container.
use crate::tools::{ToolUseContext, context::AgentContextHost};
use std::{path::Path, sync::Arc};
pub use plugin_hooks::hook_session::validate;
pub(crate) async fn start(context: Arc<ToolUseContext>, resumed: bool) -> anyhow::Result<()> {
    plugin_hooks::hook_session::start::<AgentContextHost>(context, resumed).await
}
pub(crate) async fn close(session: &str, root: &Path) {
    plugin_hooks::hook_session::close::<AgentContextHost>(session, root).await;
}

impl plugin_hooks::host::HookHost for AgentContextHost {
    fn sessions() -> &'static plugin_hooks::hook_session::HookSessions<Self> {
        static SESSIONS: std::sync::OnceLock<plugin_hooks::hook_session::HookSessions<AgentContextHost>> = std::sync::OnceLock::new();
        SESSIONS.get_or_init(Default::default)
    }
    fn directory() -> String { crate::project::instance::directory() }
    async fn execution_runner(root: &Path) -> anyhow::Result<Option<Arc<agent_hooks::HookRunner>>> {
        let config = crate::config::get_project_for_tenant(&crate::session::ui_store::current_tenant_scope(), root).await?;
        crate::hooks::execution_hook_runner(&config)
    }
    async fn cleanup(context: &Arc<ToolUseContext>) {
        #[cfg(unix)]
        if let Some(runtime) = crate::tools::computer_control::NativeControlRuntime::shared() {
            runtime.stop_context(context).await;
        }
        #[cfg(not(unix))]
        let _ = context;
    }
    fn new_runner() -> agent_hooks::HookRunner { crate::hooks::new_hook_runner() }
    fn connections(session: &str, workspace: &Path) -> Arc<Self::Connections> {
        crate::plugins::mcp_session_connections(session, workspace)
    }
    fn hook_source_matches(server: &agent_client_protocol::McpServerHttp, root: &Path, name: &str) -> anyhow::Result<bool> {
        super::mcp_auth::hook_source_matches(server, root, name)
    }
    async fn with_peer<T: Send, F>(connections: &Self::Connections, server: &agent_client_protocol::McpServer, timeout: Option<std::time::Duration>, f: F) -> anyhow::Result<T>
    where F: for<'a> FnOnce(&'a rmcp::service::Peer<rmcp::service::RoleClient>) -> tool_mcp::ClientFuture<'a, T> + Send {
        connections.with_peer(server, timeout, f).await
    }
}
