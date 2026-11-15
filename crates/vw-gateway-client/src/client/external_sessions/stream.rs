use super::super::{AgentAppsReadError, GatewayClient};
use vw_api_external_session::SessionStreamRequest;
#[cfg(not(target_arch = "wasm32"))]
pub use gateway_external_session::ExternalSessionStream;

impl GatewayClient {
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn external_session_stream(
        &self,
        request: &SessionStreamRequest,
        token: &str,
    ) -> Result<ExternalSessionStream, AgentAppsReadError> {
        gateway_external_session::GatewayExternalSessionClient::new(self.transport.clone())
            .external_session_stream(request, token).await
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn cancel_external_session_stream(
        &self,
        request: &SessionStreamRequest,
        token: &str,
    ) -> Result<(), AgentAppsReadError> {
        gateway_external_session::GatewayExternalSessionClient::new(self.transport.clone())
            .cancel_external_session_stream(request, token).await
    }
}
