use super::super::{AgentAppsReadError, GatewayClient};
use vw_api_external_session::{SessionMessagesRequest, SessionMessagesResponse};

impl GatewayClient {
    pub async fn external_session_messages(
        &self,
        request: &SessionMessagesRequest,
        token: &str,
    ) -> Result<SessionMessagesResponse, AgentAppsReadError> {
        gateway_external_session::GatewayExternalSessionClient::new(self.transport.clone())
            .external_session_messages(request, token).await
    }
}
