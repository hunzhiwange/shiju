use super::super::{AgentAppsReadError, GatewayClient};
use vw_api_external_session::{SessionResumeResponse, SessionResumeRequest};

impl GatewayClient {
    pub async fn external_session_resume(
        &self,
        request: &SessionResumeRequest,
        token: &str,
    ) -> Result<SessionResumeResponse, AgentAppsReadError> {
        gateway_external_session::GatewayExternalSessionClient::new(self.transport.clone())
            .external_session_resume(request, token).await
    }

    pub async fn external_session_resume_command(
        &self,
        request: &SessionResumeRequest,
        token: &str,
    ) -> Result<vw_api_external_session::SessionResumeCommandResponse, AgentAppsReadError> {
        gateway_external_session::GatewayExternalSessionClient::new(self.transport.clone())
            .external_session_resume_command(request, token).await
    }
}
