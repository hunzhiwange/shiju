//! Read the native model catalog through an authenticated gateway.

use super::{AgentAppsReadError as Error, GatewayClient};
use serde::Serialize;
use vw_api_types::{
    agent_apps::AppId,
    common::{PaginatedResponse, PaginationRequest},
    id::ModelId,
    model_catalog::{
        ModelCatalogCapabilities, ModelCatalogIdentity, ModelCatalogPreset, ModelCatalogProject,
        ModelCatalogQuery, ModelCatalogReadResponse, ModelCatalogTemplates,
    },
};

impl GatewayClient {
    pub async fn model_catalog(
        &self,
        project: &ModelCatalogProject,
        query: &ModelCatalogQuery,
        token: &str,
    ) -> Result<ModelCatalogReadResponse, Error> {
        self.catalog_read(
            "",
            project,
            &query.deployment,
            query.search.as_deref(),
            query.page.as_ref(),
            token,
        )
        .await
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
        self.catalog_read(
            "/presets",
            project,
            &CatalogTarget { target_host_id, app_id },
            search,
            page,
            token,
        )
        .await
    }

    pub async fn model_catalog_templates(
        &self,
        project: &ModelCatalogProject,
        target_host_id: &str,
        app_id: AppId,
        token: &str,
    ) -> Result<ModelCatalogTemplates, Error> {
        self.catalog_read(
            "/templates",
            project,
            &CatalogTarget { target_host_id, app_id },
            None,
            None,
            token,
        )
        .await
    }

    pub async fn model_catalog_capabilities(
        &self,
        project: &ModelCatalogProject,
        deployment: &ModelCatalogIdentity,
        model_id: &ModelId,
        token: &str,
    ) -> Result<ModelCatalogCapabilities, Error> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Model<'a> {
            #[serde(flatten)]
            deployment: &'a ModelCatalogIdentity,
            model_id: &'a ModelId,
        }
        self.catalog_read(
            "/capabilities",
            project,
            &Model { deployment, model_id },
            None,
            None,
            token,
        )
        .await
    }

    async fn catalog_read<T: serde::de::DeserializeOwned>(
        &self,
        resource: &str,
        project: &ModelCatalogProject,
        target: &impl Serialize,
        search: Option<&str>,
        page: Option<&PaginationRequest>,
        token: &str,
    ) -> Result<T, Error> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (resource, project, target, search, page, token);
            Err(Error::Transport("model catalog requires native transport".into()))
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut credential = reqwest::header::HeaderValue::from_str(token)
                .map_err(|_| Error::Transport("invalid application credential".into()))?;
            credential.set_sensitive(true);
            let mut request = self
                .dynamic_host_client
                .get(format!("{}/v1/model-catalog{resource}", self.endpoint.base_url()))
                .header("x-agent-apps-token", credential)
                .query(project)
                .query(target);
            if let Some(search) = search {
                request = request.query(&[("search", search)]);
            }
            if let Some(page) = page {
                request = request.query(page);
            }
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CatalogTarget<'a> {
    target_host_id: &'a str,
    app_id: AppId,
}
