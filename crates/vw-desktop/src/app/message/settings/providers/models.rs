//! Inject desktop configuration access and retain provider model exports.
pub use desktop_settings_providers::provider_models::*;
pub(super) fn access() -> desktop_config::ConfigAccess {
    #[cfg(not(target_arch = "wasm32"))]
    let path = desktop_config::config_gateway::project_config_override_path();
    #[cfg(target_arch = "wasm32")]
    let path = None;
    desktop_config::ConfigAccess::new(crate::app::config::gateway_client_endpoint(), path)
}
