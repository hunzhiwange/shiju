//! Provider composition adapter; domain state and algorithms live in agent-provider.
#[cfg(test)]
use crate::config;
#[cfg(test)]
use std::sync::Arc;
#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use serde_json::Value;
#[cfg(test)]
use crate::project::instance;
pub use agent_provider::provider::*;
pub(crate) fn ensure_host() {
    install_host(Host {
        current_tenant_scope: crate::session::ui_store::current_tenant_scope,
        read_catalogs: |config| Box::pin(async move { super::model_catalog::read(&config).await }),
    });
}
pub async fn list() -> Result<std::collections::HashMap<String, Info>, CatalogError> {
    ensure_host();
    agent_provider::provider::list().await
}
pub async fn list_for_settings() -> Result<std::collections::HashMap<String, Info>, CatalogError> {
    ensure_host();
    agent_provider::provider::list_for_settings().await
}
pub async fn settings_provider_list() -> Result<(std::collections::HashMap<String, Info>, std::collections::HashMap<String, String>), CatalogError> {
    ensure_host();
    agent_provider::provider::settings_provider_list().await
}
pub async fn invalidate_cache() {
    ensure_host();
    agent_provider::provider::invalidate_cache().await
}
pub async fn get_provider(provider_id: &str) -> Result<Option<Info>, CatalogError> {
    ensure_host();
    agent_provider::provider::get_provider(provider_id).await
}
pub async fn get_model(provider_id: &str, model_id: &str) -> Result<Model, CatalogError> {
    ensure_host();
    agent_provider::provider::get_model(provider_id, model_id).await
}
pub async fn default_model() -> Result<ParsedModelRef, String> {
    ensure_host();
    agent_provider::provider::default_model().await
}
pub async fn get_small_model(provider_id: &str) -> Result<Option<Model>, CatalogError> {
    ensure_host();
    agent_provider::provider::get_small_model(provider_id).await
}
pub fn init() { ensure_host(); agent_provider::provider::init(); }
#[cfg(test)]
async fn load_state_impl(filter: bool) -> State {
    ensure_host();
    agent_provider::provider::load_state_impl(filter).await
}
#[cfg(test)]
#[path = "provider_tests.rs"]
mod provider_tests;
