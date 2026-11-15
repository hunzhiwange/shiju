use super::super::{AgentAppsReadError, GatewayClient};
use vw_api_external_session::*;

impl GatewayClient {
    pub async fn external_session_compression(
        &self,
        request: &HistoryMaintenanceTarget,
        token: &str,
    ) -> Result<SessionCompressionState, AgentAppsReadError> {
        gateway_external_session::GatewayExternalSessionClient::new(self.transport.clone())
            .external_session_compression(request, token).await
    }

    pub async fn external_session_compression_set(
        &self,
        request: &SessionCompressionUpdate,
        token: &str,
    ) -> Result<SessionCompressionState, AgentAppsReadError> {
        gateway_external_session::GatewayExternalSessionClient::new(self.transport.clone())
            .external_session_compression_set(request, token).await
    }
}
