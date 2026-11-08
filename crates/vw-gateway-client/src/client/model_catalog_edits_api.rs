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
        self.catalog_post("edits", project, input, token).await
    }

    pub async fn set_model_catalog_default(
        &self,
        project: &ModelCatalogProject,
        input: &ModelCatalogDefaultRequest,
        token: &str,
    ) -> Result<ModelCatalogWriteResponse, Error> {
        self.catalog_post("defaults", project, input, token).await
    }

    pub async fn preview_model_catalog_fill(
        &self,
        project: &ModelCatalogProject,
        input: &ModelCatalogFillPreviewRequest,
        token: &str,
    ) -> Result<ModelCatalogFillPreviewResponse, Error> {
        self.catalog_post("fill-preview", project, input, token).await
    }

    pub async fn commit_model_catalog_fill(
        &self,
        project: &ModelCatalogProject,
        input: &ModelCatalogFillCommitRequest,
        token: &str,
    ) -> Result<ModelCatalogWriteResponse, Error> {
        self.catalog_post("fill-commit", project, input, token).await
    }

    async fn catalog_post<T: serde::de::DeserializeOwned>(
        &self,
        resource: &str,
        project: &ModelCatalogProject,
        input: &impl serde::Serialize,
        token: &str,
    ) -> Result<T, Error> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (resource, project, input, token);
            Err(Error::Transport("model catalog requires native transport".into()))
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut credential = reqwest::header::HeaderValue::from_str(token)
                .map_err(|_| Error::Transport("invalid application credential".into()))?;
            credential.set_sensitive(true);
            let request = self
                .dynamic_host_client
                .post(format!("{}/v1/model-catalog/{resource}", self.endpoint.base_url()))
                .header("x-agent-apps-token", credential)
                .query(project)
                .json(input);
            let response = crate::http::apply_request_context(request, &self.endpoint)
                .send()
                .await
                .map_err(|_| Error::Transport("model catalog request failed".into()))?;
            let status = response.status();
            if !status.is_success() {
                return Err(Error::Http { status: status.as_u16(), body: serde_json::Value::Null });
            }
            response
                .json()
                .await
                .map_err(|_| Error::Decode("invalid model catalog response".into()))
        }
    }
}
