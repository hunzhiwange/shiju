use super::{AgentAppsReadError as Error, GatewayClient};
use {
    vw_api_transport::id::TaskId,
    vw_api_types::model_catalog::ModelCatalogProject,
    vw_api_types::model_catalog::ModelDiscoveryRequest,
    vw_api_types::model_catalog::ModelDiscoveryStatus,
};

impl GatewayClient {
    pub async fn start_model_discovery(
        &self,
        project: &ModelCatalogProject,
        input: &ModelDiscoveryRequest,
        token: &str,
    ) -> Result<ModelDiscoveryStatus, Error> {
        gateway_model_catalog::GatewayModelCatalogClient::new(self.transport.clone())
            .start_model_discovery(project, input, token).await
    }

    pub async fn model_discovery(
        &self,
        project: &ModelCatalogProject,
        target_host_id: &str,
        id: &TaskId,
        token: &str,
    ) -> Result<ModelDiscoveryStatus, Error> {
        gateway_model_catalog::GatewayModelCatalogClient::new(self.transport.clone())
            .model_discovery(project, target_host_id, id, token).await
    }

    pub async fn cancel_model_discovery(
        &self,
        project: &ModelCatalogProject,
        target_host_id: &str,
        id: &TaskId,
        token: &str,
    ) -> Result<ModelDiscoveryStatus, Error> {
        gateway_model_catalog::GatewayModelCatalogClient::new(self.transport.clone())
            .cancel_model_discovery(project, target_host_id, id, token).await
    }

}
