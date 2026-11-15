//! Host actions used by the configuration routes; policies remain with their owners.
use std::{collections::HashMap, future::Future, pin::Pin};
use agent_config::runtime::Config;
use axum::{Json, http::{HeaderMap, StatusCode}};
use gateway_types::ApiError;
use serde::Deserialize;

pub type HostFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

#[derive(Debug, Deserialize)]
pub struct InstanceQuery {
    pub directory: Option<String>,
}

pub trait ConfigHost: Clone + Send + Sync + 'static {
    type State: Clone + Send + Sync + 'static;
    type HookCandidate: Send;
    fn config(state: &Self::State) -> &parking_lot::Mutex<Config>;
    fn pairing(state: &Self::State) -> &gateway_pairing::pairing::PairingGuard;
    fn require_auth(state: &Self::State, headers: &HeaderMap) -> Result<(), (StatusCode, Json<serde_json::Value>)>;
    fn mask_sensitive_fields(config: &Config) -> Config;
    fn hydrate_config_for_save(incoming: Config, current: &Config) -> Config;
    fn resolve_directory(query: &InstanceQuery, headers: &HeaderMap) -> String;
    fn with_instance<T: Send + 'static>(directory: String,
        action: impl FnOnce() -> HostFuture<Result<T, ApiError>> + Send + 'static,
    ) -> HostFuture<Result<T, ApiError>>;
    fn list_providers() -> HostFuture<Result<HashMap<String, agent_provider::provider::Info>, agent_provider::provider::CatalogError>>;
    fn build_hook_candidate(state: &Self::State, config: &Config) -> Result<Option<Self::HookCandidate>, String>;
    fn publish_hook_candidate(state: &Self::State, candidate: Option<Self::HookCandidate>);
    fn invalidate_session_query_engines(state: &Self::State) -> impl Future<Output = ()> + Send;
}
