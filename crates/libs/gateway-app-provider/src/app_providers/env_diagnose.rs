use super::{GatewayAppProviderClient, provider_collection_path};
use vw_api_app_identity::{AppId};
impl GatewayAppProviderClient {
    /// Native transport only; token authorizes the selected execution host independently.
    pub async fn provider_env_diagnose(
        &self,
        app: AppId,
        request_body: &vw_api_app_provider_environment::env_diagnose::EnvDiagnoseRequest,
        token: &str,
    ) -> Result<
        vw_api_app_provider_environment::env_diagnose::EnvDiagnosis,
        gateway_app_read::AgentAppsReadError,
    > {
        use gateway_app_read::AgentAppsReadError as Error;
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (app, request_body, token);
            Err(Error::Transport("authorized application reads require native transport".into()))
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let base = format!("{}/environment/diagnose", provider_collection_path(app));
            let mut credential = reqwest::header::HeaderValue::from_str(token)
                .map_err(|_| Error::Transport("invalid application credential".into()))?;
            credential.set_sensitive(true);
            let request = self.transport.dynamic_host_client()
                .post(format!("{}{}", self.transport.endpoint().base_url(), base))
                .json(request_body)
                .header("x-agent-apps-token", credential);
            let response = gateway_http::apply_request_context(request, self.transport.endpoint())
                .send()
                .await
                .map_err(|_| Error::Transport("application provider read failed".into()))?;
            let status = response.status();
            if !status.is_success() {
                // Configuration errors and permission denials must not expose raw payloads.
                return Err(Error::Http { status: status.as_u16(), body: serde_json::Value::Null });
            }
            response.json().await.map_err(|_| Error::Decode("invalid environment diagnosis".into()))
        }
    }
}
