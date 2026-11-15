use super::super::{AgentAppsReadError, GatewayClient};
use vw_api_external_session::*;

impl GatewayClient {
    pub async fn external_history_options(
        &self,
        request: &HistoryMaintenanceTarget,
        token: &str,
    ) -> Result<HistoryOptions, AgentAppsReadError> {
        gateway_external_session::GatewayExternalSessionClient::new(self.transport.clone())
            .external_history_options(request, token).await
    }

    pub async fn external_history_options_set(
        &self,
        request: &HistoryOptionsUpdate,
        token: &str,
    ) -> Result<HistoryOptions, AgentAppsReadError> {
        gateway_external_session::GatewayExternalSessionClient::new(self.transport.clone())
            .external_history_options_set(request, token).await
    }

    pub async fn external_history_backups(
        &self,
        request: &HistoryMaintenanceTarget,
        token: &str,
    ) -> Result<HistoryBackups, AgentAppsReadError> {
        gateway_external_session::GatewayExternalSessionClient::new(self.transport.clone())
            .external_history_backups(request, token).await
    }
}
