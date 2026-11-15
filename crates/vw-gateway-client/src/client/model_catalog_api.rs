//! Read the native model catalog through an authenticated gateway.

use super::{AgentAppsReadError as Error, GatewayClient};
use {
    vw_api_app_identity::AppId,
    vw_api_transport::common::PaginatedResponse,
    vw_api_transport::common::PaginationRequest,
    vw_api_transport::id::ModelId,
    vw_api_types::model_catalog::ModelCatalogCapabilities,
    vw_api_types::model_catalog::ModelCatalogIdentity,
    vw_api_types::model_catalog::ModelCatalogPreset,
    vw_api_types::model_catalog::ModelCatalogProject,
    vw_api_types::model_catalog::ModelCatalogQuery,
    vw_api_types::model_catalog::ModelCatalogReadResponse,
    vw_api_types::model_catalog::ModelCatalogTemplates,
};

impl GatewayClient {
    pub async fn model_catalog(
        &self,
        project: &ModelCatalogProject,
        query: &ModelCatalogQuery,
        token: &str,
    ) -> Result<ModelCatalogReadResponse, Error> {
        gateway_model_catalog::GatewayModelCatalogClient::new(self.transport.clone())
            .model_catalog(project, query, token).await
    }

    pub async fn model_catalog_presets(
        &self,
        project: &ModelCatalogProject,
        target_host_id: &str,
        app_id: AppId,
        search: Option<&str>,
        page: Option<&PaginationRequest>,
        token: &str,
    ) -> Result<PaginatedResponse<ModelCatalogPreset>, Error> {
        gateway_model_catalog::GatewayModelCatalogClient::new(self.transport.clone())
            .model_catalog_presets(project, target_host_id, app_id, search, page, token).await
    }

    pub async fn model_catalog_templates(
        &self,
        project: &ModelCatalogProject,
        target_host_id: &str,
        app_id: AppId,
        token: &str,
    ) -> Result<ModelCatalogTemplates, Error> {
        gateway_model_catalog::GatewayModelCatalogClient::new(self.transport.clone())
            .model_catalog_templates(project, target_host_id, app_id, token).await
    }

    pub async fn model_catalog_capabilities(
        &self,
        project: &ModelCatalogProject,
        deployment: &ModelCatalogIdentity,
        model_id: &ModelId,
        token: &str,
    ) -> Result<ModelCatalogCapabilities, Error> {
        gateway_model_catalog::GatewayModelCatalogClient::new(self.transport.clone())
            .model_catalog_capabilities(project, deployment, model_id, token).await
    }

}
