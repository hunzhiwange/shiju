//! Keep rejected initialization distinct from a stalled handshake.

use rmcp::{
    RoleClient, ServiceExt,
    model::{ClientJsonRpcMessage, ServerJsonRpcMessage},
    transport::Transport,
};

use super::{NoopClient, RunningMcpPeer};

pub(super) async fn serve<T: Transport<RoleClient> + 'static>(
    client: NoopClient,
    transport: T,
    connection_check: bool,
) -> anyhow::Result<RunningMcpPeer> {
    if connection_check {
        client.serve(CheckTransport { transport, awaiting_initialize: true }).await
    } else {
        client.serve(transport).await
    }
    .map_err(anyhow::Error::from)
}

struct CheckTransport<T> {
    transport: T,
    awaiting_initialize: bool,
}

impl<T: Transport<RoleClient>> Transport<RoleClient> for CheckTransport<T> {
    type Error = T::Error;

    fn send(
        &mut self,
        message: ClientJsonRpcMessage,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send + 'static {
        self.transport.send(message)
    }

    async fn receive(&mut self) -> Option<ServerJsonRpcMessage> {
        let message = self.transport.receive().await?;
        if self.awaiting_initialize {
            match message {
                // rmcp 0.8 ignores JSON-RPC errors while waiting for initialize.
                // End the handshake immediately without forwarding the error payload.
                ServerJsonRpcMessage::Error(_) => return None,
                ServerJsonRpcMessage::Response(_) => self.awaiting_initialize = false,
                _ => {}
            }
        }
        Some(message)
    }

    async fn close(&mut self) -> Result<(), Self::Error> {
        self.transport.close().await
    }
}
