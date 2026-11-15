use agent_client_protocol::McpServer;
use mcp_client::{RunningMcpPeer, ToolCatalogBinding};
use std::{future::Future, path::Path, pin::Pin, sync::Arc, time::Duration};
use serde_json::Value;

pub type ClientFuture<'a, T> = Pin<Box<dyn Future<Output = anyhow::Result<T>> + Send + 'a>>;

/// Existing scoped configuration and session actions, implemented by the application.
/// Implementations retain tenant identity, approval, revocation and cancellation checks.
pub trait McpToolHost: tool_context::host::ContextHost + Send + Sync {
    type Connections: Send + Sync + 'static;
    fn new_connections() -> Self::Connections;
    fn load_config(workspace: Option<&Path>) -> impl Future<Output = anyhow::Result<acp_config::ResolvedAcpxConfig>> + Send;
    fn connection_required(server: &McpServer) -> anyhow::Result<bool>;
    fn missing_executable(server: &McpServer) -> anyhow::Result<Option<(String, String)>>;
    fn tool_allowed(server: &McpServer, name: &str) -> anyhow::Result<bool>;
    fn tool_binding(connections: &Self::Connections, server: &McpServer) -> Option<ToolCatalogBinding>;
    fn with_client<T: Send, F>(server: &McpServer, timeout: Option<Duration>, f: F) -> impl Future<Output = anyhow::Result<T>> + Send
    where F: for<'a> FnOnce(&'a RunningMcpPeer) -> ClientFuture<'a, T> + Send;
    fn with_connection<T: Send, F>(connections: &Self::Connections, server: &McpServer, timeout: Option<Duration>, f: F) -> impl Future<Output = anyhow::Result<T>> + Send
    where F: for<'a> FnOnce(&'a RunningMcpPeer) -> ClientFuture<'a, T> + Send;
    fn app_resource<'a>(client: &'a RunningMcpPeer, binding: &'a mut Value) -> ClientFuture<'a, ()>;
    fn call_resource<'a>(config: &'a acp_config::ResolvedAcpxConfig, connections: &'a Arc<Self::Connections>, input: Value) -> ClientFuture<'a, tool_types::ToolCallResult>;
}
