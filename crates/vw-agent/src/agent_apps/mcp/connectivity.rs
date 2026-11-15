//! Native transport adapter; catalog policy and revision checks belong to the domain.
pub use agent_app_services::mcp::connectivity::*;
use agent_app_services::mcp::connection_runtime::{CheckPeer, ConnectionCheckRuntime};
use crate::tools::mcp_common::{RunningMcpPeer, open_connection_check_peer};
use mcp_client::ProbeChild;
pub(crate) struct NativeConnectionCheckRuntime;
pub(crate) struct NativeCheckPeer(RunningMcpPeer);
impl CheckPeer for NativeCheckPeer {
    fn has_tools(&self) -> bool {
        self.0.peer_info().is_some_and(|info| info.capabilities.tools.is_some())
    }
    async fn tool_count(&self) -> anyhow::Result<u32> {
        Ok(crate::plugins::list_mcp_catalog_tools(self.0.peer()).await?.len() as u32)
    }
    async fn cancel(self) { let _ = RunningMcpPeer::cancel(self.0).await; }
}
impl ConnectionCheckRuntime for NativeConnectionCheckRuntime {
    type Peer = NativeCheckPeer;
    async fn open(
        &self,
        server: agent_client_protocol::McpServer,
        timeout: std::time::Duration,
        http_io: tokio_util::sync::CancellationToken,
        child: Option<&mut Option<ProbeChild>>,
    ) -> anyhow::Result<NativeCheckPeer> {
        open_connection_check_peer(server, timeout, http_io, child, None).await.map(NativeCheckPeer)
    }
}
