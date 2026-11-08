use super::{AgentAppsReadError as Error, GatewayClient};
use reqwest::Method;
use vw_api_types::{
    id::TaskId,
    model_catalog::{ModelCatalogProject, ModelDiscoveryRequest, ModelDiscoveryStatus},
};

impl GatewayClient {
    pub async fn start_model_discovery(
        &self,
        project: &ModelCatalogProject,
        input: &ModelDiscoveryRequest,
        token: &str,
    ) -> Result<ModelDiscoveryStatus, Error> {
        self.discovery_request(
            Method::POST,
            project,
            &input.deployment.target_host_id,
            None,
            Some(input),
            token,
        )
        .await
    }

    pub async fn model_discovery(
        &self,
        project: &ModelCatalogProject,
        target_host_id: &str,
        id: &TaskId,
        token: &str,
    ) -> Result<ModelDiscoveryStatus, Error> {
        self.discovery_request(Method::GET, project, target_host_id, Some(id), None, token).await
    }

    pub async fn cancel_model_discovery(
        &self,
        project: &ModelCatalogProject,
        target_host_id: &str,
        id: &TaskId,
        token: &str,
    ) -> Result<ModelDiscoveryStatus, Error> {
        self.discovery_request(Method::DELETE, project, target_host_id, Some(id), None, token).await
    }

    async fn discovery_request(
        &self,
        method: Method,
        project: &ModelCatalogProject,
        target_host_id: &str,
        id: Option<&TaskId>,
        input: Option<&ModelDiscoveryRequest>,
        token: &str,
    ) -> Result<ModelDiscoveryStatus, Error> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (method, project, target_host_id, id, input, token);
            Err(Error::Transport("model discovery requires native transport".into()))
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let invalid_url = || Error::Transport("invalid model discovery URL".into());
            let mut url = reqwest::Url::parse(&format!(
                "{}/v1/model-catalog/discoveries",
                self.endpoint.base_url()
            ))
            .map_err(|_| invalid_url())?;
            if let Some(id) = id {
                url.path_segments_mut().map_err(|_| invalid_url())?.push(id.as_ref());
            }
            let mut credential = reqwest::header::HeaderValue::from_str(token)
                .map_err(|_| Error::Transport("invalid application credential".into()))?;
            credential.set_sensitive(true);
            let mut request = self
                .dynamic_host_client
                .request(method, url)
                .header("x-agent-apps-token", credential)
                .query(project)
                .query(&[("targetHostId", target_host_id)]);
            if let Some(input) = input {
                request = request.json(input);
            }
            let response = crate::http::apply_request_context(request, &self.endpoint)
                .send()
                .await
                .map_err(|_| Error::Transport("model discovery request failed".into()))?;
            let status = response.status();
            if !status.is_success() {
                return Err(Error::Http { status: status.as_u16(), body: serde_json::Value::Null });
            }
            response
                .json()
                .await
                .map_err(|_| Error::Decode("invalid model discovery response".into()))
        }
    }
}
