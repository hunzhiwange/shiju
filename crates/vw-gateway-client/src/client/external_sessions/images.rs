use super::super::{AgentAppsReadError, GatewayClient};
use vw_api_external_session::{SessionImageRequest, SessionImageResponse};

impl GatewayClient {
    pub async fn external_session_image(
        &self,
        request: &SessionImageRequest,
        token: &str,
    ) -> Result<SessionImageResponse, AgentAppsReadError> {
        gateway_external_session::GatewayExternalSessionClient::new(self.transport.clone())
            .external_session_image(request, token).await
    }
}
