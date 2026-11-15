use super::super::{AgentAppsReadError, GatewayClient};
use vw_api_external_session::{SessionContentRequest, SessionContentResponse};

impl GatewayClient {
    pub async fn external_session_content(
        &self,
        request: &SessionContentRequest,
        token: &str,
    ) -> Result<SessionContentResponse, AgentAppsReadError> {
        gateway_external_session::GatewayExternalSessionClient::new(self.transport.clone())
            .external_session_content(request, token).await
    }
}
