use crate::GatewayClient;
use vw_api_app_operations::data_management::*;
use gateway_http::AgentAppsReadError;
use vw_api_transport::ProjectScope;

impl GatewayClient {
    pub async fn data_backup_inventory(&self, project: &ProjectScope, host: &str)
        -> Result<Vec<DataBackupSummary>, AgentAppsReadError> {
        gateway_app_data::GatewayAppDataClient::new(self.transport.clone())
            .data_backup_inventory(project, host).await
    }
}
