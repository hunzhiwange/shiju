use gateway_app_read::AgentAppsReadError as Error;
use super::{GatewayAppProviderClient, provider_collection_path};
use vw_api_app_provider_environment::paths::*;
use vw_api_app_identity::{AppId};

enum Request {
    Inspect,
    Read,
    Save,
}

impl GatewayAppProviderClient {
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
                format!("{}{}/config", self.transport.endpoint().base_url(), provider_collection_path(app));
            let client = &self.transport.dynamic_host_client();
            let request = match request {
                Request::Inspect => client.post(format!("{base}/paths")).json(body),
                Request::Read => client.get(format!("{base}/directory")).query(body),
                Request::Save => client.put(format!("{base}/directory")).json(body),
            };
            let response = gateway_http::apply_request_context(request, self.transport.endpoint())
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
