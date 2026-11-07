use super::super::agent_apps_read_api::AgentAppsReadError as Error;
use super::{GatewayClient, provider_collection_path};
use vw_api_types::{agent_apps::AppId, app_providers::paths::*};

enum Request {
    Inspect,
    Read,
    Save,
}

impl GatewayClient {
    /// Native transport only; token authorizes the selected execution host independently.
    pub async fn app_provider_paths(
        &self,
        app: AppId,
        request_body: &ProviderPathsRequest,
        token: &str,
    ) -> Result<ProviderPaths, Error> {
        self.config_directory_request(app, Request::Inspect, request_body, token).await
    }

    pub async fn app_provider_directory(
        &self,
        app: AppId,
        request: &ProviderDirectoryQuery,
        token: &str,
    ) -> Result<ProviderDirectoryStatus, Error> {
        self.config_directory_request(app, Request::Read, request, token).await
    }

    pub async fn app_provider_directory_save(
        &self,
        app: AppId,
        request: &ProviderDirectoryUpdate,
        token: &str,
    ) -> Result<ProviderDirectoryStatus, Error> {
        self.config_directory_request(app, Request::Save, request, token).await
    }

    async fn config_directory_request<B: serde::Serialize, R: serde::de::DeserializeOwned>(
        &self,
        app: AppId,
        request: Request,
        body: &B,
        token: &str,
    ) -> Result<R, Error> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (app, request, body, token);
            Err(Error::Transport("configuration paths require native transport".into()))
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let base =
                format!("{}{}/config", self.endpoint.base_url(), provider_collection_path(app));
            let mut credential = reqwest::header::HeaderValue::from_str(token)
                .map_err(|_| Error::Transport("invalid application credential".into()))?;
            credential.set_sensitive(true);
            let client = &self.dynamic_host_client;
            let request = match request {
                Request::Inspect => client.post(format!("{base}/paths")).json(body),
                Request::Read => client.get(format!("{base}/directory")).query(body),
                Request::Save => client.put(format!("{base}/directory")).json(body),
            }
            .header("x-agent-apps-token", credential);
            let response = crate::http::apply_request_context(request, &self.endpoint)
                .send()
                .await
                .map_err(|_| Error::Transport("configuration path request failed".into()))?;
            let status = response.status();
            if !status.is_success() {
                // Configuration errors and permission denials must not expose raw payloads.
                return Err(Error::Http { status: status.as_u16(), body: serde_json::Value::Null });
            }
            response
                .json()
                .await
                .map_err(|_| Error::Decode("invalid application provider paths".into()))
        }
    }
}
