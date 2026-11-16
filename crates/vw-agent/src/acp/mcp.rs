//! ACP 会话级 MCP 配置的验证、启动探测与动态工具发现。

use std::sync::Arc;

use agent_client_protocol::McpServer;

use crate::agent_apps::mcp::acp_mount::AcpMountLeases;
use crate::session::runtime::SessionToolset;
use crate::tools::catalog::ToolCatalogSnapshot;
use acp_server::server::BackendError;


/// 宿主只注入真实连接权限和挂载租约。
pub(crate) async fn prepare_session_tools(
    servers: &[McpServer],
    managed: Option<AcpMountLeases>,
) -> Result<SessionToolset, BackendError> {
    agent_acp::mcp::prepare_session_tools(servers, Arc::new(
        crate::plugins::McpSessionConnections::new(crate::plugins::compat::mcp_connections::host())
            .with_acp_mounts(crate::plugins::compat::mcp_connections::mount_leases(managed)),
    )).await
}

/// A new prompt starts from the bound mount snapshot, so prune even sources
/// already retired from the shared pool's refresh registry in a previous turn.
pub(super) fn prune_revoked_mounts(tools: SessionToolset) -> Result<SessionToolset, BackendError> {
    let Some(connections) = tools.mcp_connections() else { return Ok(tools) };
    let Some(leases) = &connections.acp_mounts else { return Ok(tools) };
    let mut current = Vec::new();
    for tool in tools.tools() {
        if let Some(name) = tool.spec().identity.namespace.as_deref()
            && !leases.is_current(name).map_err(|error| BackendError::Internal(error.to_string()))? {
            continue;
        }
        current.push(tool.clone());
    }
    Ok(tools.with_catalog(Arc::new(ToolCatalogSnapshot::from_runtime_tools(current))))
}
