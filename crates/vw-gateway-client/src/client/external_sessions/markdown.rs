use super::super::{AgentAppsReadError, GatewayClient};
use vw_api_external_session::{SessionMarkdownRequest, SessionMarkdownResponse};

impl GatewayClient {
    pub async fn external_session_markdown(
        &self,
        request: &SessionMarkdownRequest,
        token: &str,
    ) -> Result<SessionMarkdownResponse, AgentAppsReadError> {
        gateway_external_session::GatewayExternalSessionClient::new(self.transport.clone())
            .external_session_markdown(request, token).await
    }
}
