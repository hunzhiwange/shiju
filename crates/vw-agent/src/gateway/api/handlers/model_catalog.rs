//! Host authorization composition for the model catalog routes.
use axum::Router;
use std::path::PathBuf;

pub(crate) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}

pub(super) fn router_with_policy(path: Option<PathBuf>) -> Router {
    gateway_model::router_with_policy(path, gateway_model::Authorization {
        provider_config_home: super::agent_apps::provider_config_home,
        authorize_provider_config_access: super::agent_apps::authorize_provider_config_access,
        #[cfg(target_os = "macos")]
        provider_discovery_host: super::agent_apps::provider_discovery_host,
    })
}
