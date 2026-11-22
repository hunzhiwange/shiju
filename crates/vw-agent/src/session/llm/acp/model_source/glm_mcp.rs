//! Host-owned HTTP peers for the managed GLM projection.
use agent_client_protocol::McpServer;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

pub(super) async fn open_selected_http_peer(
    definition: &McpServer,
    timeout: Duration,
    closed: CancellationToken,
) -> anyhow::Result<mcp_client::RunningMcpPeer> {
    anyhow::ensure!(
        matches!(definition, McpServer::Http(_)),
        "GLM HTTP projection requires a selected HTTP service; stdio requires the host's reviewed ELF projection"
    );
    tokio::select! {
        biased;
        () = closed.cancelled() => anyhow::bail!("GLM MCP selection was revoked"),
        result = crate::tools::mcp_common::open_connection_check_peer(
            definition.clone(), timeout, closed.clone(), None, None,
        ) => result,
    }
}
