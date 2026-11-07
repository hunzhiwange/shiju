//! Mode status and decisions never expose the saved client configuration.
use super::super::agent_apps_read_api::AgentAppsReadError as Error;
use super::GatewayClient;
use vw_api_types::{agent_apps::AppId, app_providers::mode_state::*};
impl GatewayClient {
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
            let mut credential = reqwest::header::HeaderValue::from_str(token)
                .map_err(|_| Error::Transport("invalid application credential".into()))?;
            credential.set_sensitive(true);
            let app = serde_json::to_value(app)
                .map_err(|_| Error::Transport("invalid application".into()))?;
            let app = app.as_str().ok_or_else(|| Error::Transport("invalid application".into()))?;
            let request = self
                .dynamic_host_client
                .get(format!("{}/v1/app-providers/{app}/mode/state", self.endpoint.base_url()))
                .header("x-agent-apps-token", credential)
                .query(query);
            let response = crate::http::apply_request_context(request, &self.endpoint)
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
