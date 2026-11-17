//! Mode status and decisions never expose the saved client configuration.
use gateway_app_read::AgentAppsReadError as Error;
use super::GatewayAppProviderClient;
use vw_api_app_provider_environment::mode_state::*;
use vw_api_app_identity::{AppId};
impl GatewayAppProviderClient {
    pub async fn provider_mode_state(
        &self,
        app: AppId,
        query: &ModeStateQuery,
        token: &str,
    ) -> Result<ProviderModeStatus, Error> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (app, query, token);
            Err(Error::Transport("Provider mode requires native transport".into()))
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let app = serde_json::to_value(app)
                .map_err(|_| Error::Transport("invalid application".into()))?;
            let app = app.as_str().ok_or_else(|| Error::Transport("invalid application".into()))?;
            let request = self.transport.dynamic_host_client()
                .get(format!("{}/v1/app-providers/{app}/mode/state", self.transport.endpoint().base_url()))

                .query(query);
            let response = gateway_http::apply_request_context(request, self.transport.endpoint())
                .send()
                .await
                .map_err(|_| Error::Transport("Provider mode request failed".into()))?;
            if !response.status().is_success() {
                return Err(Error::Http {
                    status: response.status().as_u16(),
                    body: serde_json::Value::Null,
                });
            }
            response
                .json()
                .await
                .map_err(|_| Error::Decode("invalid Provider mode response".into()))
        }
    }
}
