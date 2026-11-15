use super::super::{AgentAppsReadError, GatewayClient};
use vw_api_external_session::{
    HistoryMaintenanceProgress, HistoryMaintenanceQuery, HistoryMaintenanceRequest,
};

impl GatewayClient {
    pub async fn external_history_maintenance_submit(
        &self,
        request: &HistoryMaintenanceRequest,
        token: &str,
    ) -> Result<HistoryMaintenanceProgress, AgentAppsReadError> {
        gateway_external_session::GatewayExternalSessionClient::new(self.transport.clone())
            .external_history_maintenance_submit(request, token).await
    }

    pub async fn external_history_maintenance_query(
        &self,
        request: &HistoryMaintenanceQuery,
        token: &str,
    ) -> Result<HistoryMaintenanceProgress, AgentAppsReadError> {
        gateway_external_session::GatewayExternalSessionClient::new(self.transport.clone())
            .external_history_maintenance_query(request, token).await
    }

    pub async fn external_history_maintenance_cancel(
        &self,
        request: &HistoryMaintenanceQuery,
        token: &str,
    ) -> Result<HistoryMaintenanceProgress, AgentAppsReadError> {
        gateway_external_session::GatewayExternalSessionClient::new(self.transport.clone())
            .external_history_maintenance_cancel(request, token).await
    }
}
