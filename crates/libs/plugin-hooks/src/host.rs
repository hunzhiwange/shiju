//! Concrete I/O and composition ports used by the existing hook call paths.
use std::{path::Path, sync::Arc, time::Duration};
use tool_context::context::ToolUseContext;
use agent_client_protocol::{McpServer, McpServerHttp};
use crate::hook_session::HookSessions;

pub trait HookHost: tool_mcp::McpToolHost<HookRunner = agent_hooks::HookRunner, AsyncHooks = crate::hook_async::AsyncHooks> {
    fn sessions() -> &'static HookSessions<Self>;
    fn directory() -> String;
    fn execution_runner(root: &Path) -> impl Future<Output = anyhow::Result<Option<Arc<agent_hooks::HookRunner>>>> + Send;
    fn cleanup(context: &Arc<ToolUseContext<Self>>) -> impl Future<Output = ()> + Send;
    fn new_runner() -> agent_hooks::HookRunner;
    fn connections(session: &str, workspace: &Path) -> Arc<Self::Connections>;
    fn hook_source_matches(server: &McpServerHttp, root: &Path, name: &str) -> anyhow::Result<bool>;
    fn with_peer<T: Send, F>(connections: &Self::Connections, server: &McpServer, timeout: Option<Duration>, f: F) -> impl Future<Output = anyhow::Result<T>> + Send
    where F: for<'a> FnOnce(&'a rmcp::service::Peer<rmcp::service::RoleClient>) -> tool_mcp::ClientFuture<'a, T> + Send;
}
