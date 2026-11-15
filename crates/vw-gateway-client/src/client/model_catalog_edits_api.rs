//! Edit saved deployments through the authenticated model catalog API.

use super::{AgentAppsReadError as Error, GatewayClient};
use vw_api_types::model_catalog::{
    ModelCatalogDefaultRequest, ModelCatalogEditRequest, ModelCatalogFillCommitRequest,
    ModelCatalogFillPreviewRequest, ModelCatalogFillPreviewResponse, ModelCatalogProject,
    ModelCatalogWriteResponse,
};

impl GatewayClient {
    pub async fn edit_model_catalog(
        &self,
        project: &ModelCatalogProject,
        input: &ModelCatalogEditRequest,
        token: &str,
    ) -> Result<ModelCatalogWriteResponse, Error> {
        gateway_model_catalog::GatewayModelCatalogClient::new(self.transport.clone())
            .edit_model_catalog(project, input, token).await
    }

    pub async fn set_model_catalog_default(
        &self,
        project: &ModelCatalogProject,
        input: &ModelCatalogDefaultRequest,
        token: &str,
    ) -> Result<ModelCatalogWriteResponse, Error> {
        gateway_model_catalog::GatewayModelCatalogClient::new(self.transport.clone())
            .set_model_catalog_default(project, input, token).await
    }

    pub async fn preview_model_catalog_fill(
        &self,
        project: &ModelCatalogProject,
        input: &ModelCatalogFillPreviewRequest,
        token: &str,
    ) -> Result<ModelCatalogFillPreviewResponse, Error> {
        gateway_model_catalog::GatewayModelCatalogClient::new(self.transport.clone())
            .preview_model_catalog_fill(project, input, token).await
    }

    pub async fn commit_model_catalog_fill(
        &self,
        project: &ModelCatalogProject,
        input: &ModelCatalogFillCommitRequest,
        token: &str,
    ) -> Result<ModelCatalogWriteResponse, Error> {
        gateway_model_catalog::GatewayModelCatalogClient::new(self.transport.clone())
            .commit_model_catalog_fill(project, input, token).await
    }

}
