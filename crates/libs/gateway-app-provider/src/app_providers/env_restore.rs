use super::{GatewayAppProviderClient, provider_collection_path};
use vw_api_app_identity::{AppId};
impl GatewayAppProviderClient {
    /// Native transport only; token authorizes the selected execution host independently.
    pub async fn provider_env_restore(
        &self,
        app: AppId,
        request_body: &vw_api_app_provider_environment::env_restore::EnvRestoreRequest,
        token: &str,
    ) -> Result<
        vw_api_app_provider_environment::env_restore::EnvRestoreResult,
        gateway_app_read::AgentAppsReadError,
    > {
        use gateway_app_read::AgentAppsReadError as Error;
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (app, request_body, token);
            Err(Error::Transport("authorized application writes require native transport".into()))
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let base = format!("{}/environment/restore", provider_collection_path(app));
            let request = self.transport.dynamic_host_client()
                .post(format!("{}{}", self.transport.endpoint().base_url(), base))
                .json(request_body);
            let response = gateway_http::apply_request_context(request, self.transport.endpoint())
                .send()
                .await
                .map_err(|_| Error::Transport("application environment restore failed".into()))?;
            let status = response.status();
            if !status.is_success() {
                // Configuration errors and permission denials must not expose raw payloads.
                return Err(Error::Http { status: status.as_u16(), body: serde_json::Value::Null });
            }
            response
                .json()
                .await
                .map_err(|_| Error::Decode("invalid environment restore result".into()))
        }
    }
}
