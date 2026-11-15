//! Host transport operations used only by explicit authorized catalog checks.
use std::{future::Future, time::Duration};
use agent_client_protocol::McpServer;
use tokio_util::sync::CancellationToken;

pub trait CheckPeer: Send {
    fn has_tools(&self) -> bool;
    fn tool_count(&self) -> impl Future<Output = anyhow::Result<u32>> + Send;
    fn cancel(self) -> impl Future<Output = ()> + Send;
}

pub trait ConnectionCheckRuntime: Sync {
    type Peer: CheckPeer;
    fn open(
        &self,
        server: McpServer,
        timeout: Duration,
        http_io: CancellationToken,
        child: Option<&mut Option<mcp_client::ProbeChild>>,
    ) -> impl Future<Output = anyhow::Result<Self::Peer>> + Send;
}
