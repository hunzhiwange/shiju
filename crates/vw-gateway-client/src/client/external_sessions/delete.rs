use super::super::{AgentAppsReadError, GatewayClient};
use vw_api_external_session::{DeleteSessionOutcome, DeleteSessionRequest};

impl GatewayClient {
    pub async fn external_session_delete(
        &self,
        request: &DeleteSessionRequest,
        token: &str,
    ) -> Result<DeleteSessionOutcome, AgentAppsReadError> {
        gateway_external_session::GatewayExternalSessionClient::new(self.transport.clone())
            .external_session_delete(request, token).await
    }

    pub async fn external_sessions_delete(
        &self,
        request: &[DeleteSessionRequest],
        token: &str,
    ) -> Result<Vec<DeleteSessionOutcome>, AgentAppsReadError> {
        gateway_external_session::GatewayExternalSessionClient::new(self.transport.clone())
            .external_sessions_delete(request, token).await
    }
}
