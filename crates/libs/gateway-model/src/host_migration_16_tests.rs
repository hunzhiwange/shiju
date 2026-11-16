//! Catalog routes with real authorization, storage and tenant middleware.
mod model_catalog {
    pub(super) fn router_with_policy(path: Option<std::path::PathBuf>) -> axum::Router {
        use gateway_agent_apps::host_context;
        crate::router_with_policy(path, crate::Authorization {
            provider_config_home: host_context::provider_config_home,
            authorize_provider_config_access: host_context::authorize_provider_config_access,
            provider_discovery_host: host_context::provider_discovery_host,
        })
    }
}
mod model_catalog_tests;
mod model_discovery_tests;

mod catalog_acceptance_tests;
