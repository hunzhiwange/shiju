//! Configuration route composition and host integration tests.
use crate::app::agent::gateway::AppState;
use axum::{
    Json, Router,
    extract::{Extension, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
#[cfg(test)]
use axum::extract::Query;
use serde_json::Value;
use vw_api_transport::id::TenantScope;

use crate::app::agent::config;

use crate::app::agent::gateway::ApiError;
use gateway_config::host::InstanceQuery;

use crate::app::agent::provider;

#[derive(Clone)]
struct ConfigHost;

impl gateway_config::host::ConfigHost for ConfigHost {
    type State = AppState;
    type HookCandidate = crate::hooks::HookRuntimeCandidate;
    fn config(state: &AppState) -> &parking_lot::Mutex<config::Config> { &state.config }
    fn pairing(state: &AppState) -> &crate::security::pairing::PairingGuard { &state.pairing }
    fn require_auth(state: &AppState, headers: &HeaderMap) -> Result<(), (StatusCode, Json<Value>)> {
        agent_gateway::api::auth::require_pairing_auth(&state.pairing, headers)
    }
    fn mask_sensitive_fields(config: &config::Config) -> config::Config {
        super::super::secrets::mask_sensitive_fields(config)
    }
    fn hydrate_config_for_save(incoming: config::Config, current: &config::Config) -> config::Config {
        super::super::secrets::hydrate_config_for_save(incoming, current)
    }
    fn resolve_directory(query: &InstanceQuery, headers: &HeaderMap) -> String {
        crate::gateway::instance::resolve_directory(
            &crate::gateway::instance::InstanceQuery { directory: query.directory.clone() }, headers)
    }
    fn with_instance<T: Send + 'static>(directory: String,
        action: impl FnOnce() -> gateway_config::host::HostFuture<Result<T, ApiError>> + Send + 'static,
    ) -> gateway_config::host::HostFuture<Result<T, ApiError>> {
        Box::pin(crate::gateway::instance::with_instance(directory, action))
    }
    fn list_providers() -> gateway_config::host::HostFuture<Result<std::collections::HashMap<String, provider::provider::Info>, agent_provider::provider::CatalogError>> {
        Box::pin(provider::provider::list())
    }
    fn build_hook_candidate(state: &AppState, config: &config::Config) -> Result<Option<Self::HookCandidate>, String> {
        state.hook_runtime.as_ref().map(|runtime| runtime.build_candidate(config))
            .transpose().map_err(|error| error.to_string())
    }
    fn publish_hook_candidate(state: &AppState, candidate: Option<Self::HookCandidate>) {
        if let (Some(runtime), Some(candidate)) = (&state.hook_runtime, candidate) {
            runtime.replace(candidate);
        }
    }
    async fn invalidate_session_query_engines(state: &AppState) {
        crate::gateway::chat::invalidate_all_session_query_engines(state).await;
    }
}

pub(crate) fn router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    gateway_config::handlers::router::<S, ConfigHost>()
}
pub(crate) fn stateful_router() -> Router<AppState> {
    gateway_config::handlers::stateful_router::<ConfigHost>()
}

pub async fn handle_api_config_get(
    State(state): State<AppState>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
) -> impl IntoResponse {
    gateway_config::handlers::handle_api_config_get::<ConfigHost>(State(state), Extension(tenant), headers).await
}

pub async fn handle_api_config_put(
    State(state): State<AppState>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    body: String,
) -> impl IntoResponse {
    gateway_config::handlers::handle_api_config_put::<ConfigHost>(State(state), Extension(tenant), headers, body).await
}

#[cfg(test)]
async fn config_providers(
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<gateway_config::handlers::ConfigProvidersResponse>, ApiError> {
    gateway_config::handlers::config_providers::<ConfigHost>(Query(query), headers).await
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod config_tests;
