//! Integration route composition and real host integration tests.
use crate::app::agent::gateway::AppState;
use crate::app::agent::config::Config;
use axum::{extract::{Path, State}, http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json as JsonResponse}};
use gateway_integrations::IntegrationCredentialsUpdateBody;

struct IntegrationsHost;
impl gateway_integrations::IntegrationsHost for IntegrationsHost {
    type State = AppState;
    type Settings = super::super::types::IntegrationSettingsPayload;
    fn config(state: &AppState) -> &parking_lot::Mutex<Config> { &state.config }
    fn require_auth(state: &AppState, headers: &HeaderMap)
        -> Result<(), (StatusCode, JsonResponse<serde_json::Value>)> {
        super::super::auth::require_auth(state, headers)
    }
    fn build_integration_settings_payload(config: &Config) -> Self::Settings {
        super::super::integrations::build_integration_settings_payload(config)
    }
    fn config_revision(config: &Config) -> String {
        super::super::integrations::config_revision(config)
    }
    fn apply_integration_credentials_update(config: &Config, id: &str,
        fields: &std::collections::BTreeMap<String, String>) -> Result<Config, String> {
        super::super::integrations::apply_integration_credentials_update(config, id, fields)
    }
}

pub async fn handle_api_integrations(State(state): State<AppState>, headers: HeaderMap)
    -> impl IntoResponse {
    gateway_integrations::handlers::handle_api_integrations::<IntegrationsHost>(State(state), headers).await
}
pub async fn handle_api_integrations_settings(State(state): State<AppState>, headers: HeaderMap)
    -> impl IntoResponse {
    gateway_integrations::handlers::handle_api_integrations_settings::<IntegrationsHost>(State(state), headers).await
}
pub async fn handle_api_integration_credentials_put(State(state): State<AppState>, headers: HeaderMap,
    Path(id): Path<String>, JsonResponse(body): JsonResponse<IntegrationCredentialsUpdateBody>)
    -> impl IntoResponse {
    gateway_integrations::handlers::handle_api_integration_credentials_put::<IntegrationsHost>(
        State(state), headers, Path(id), JsonResponse(body)).await
}
