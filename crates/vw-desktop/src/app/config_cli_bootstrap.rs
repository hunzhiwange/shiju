//! Desktop bootstrap cache and path injection.
pub async fn bootstrap_cli_service_async() -> Result<(), String> {
    desktop_config::config_cli_bootstrap::bootstrap_cli_service_async(
        super::system_settings::load_gateway_client_bootstrap_config(),
        super::system_settings::gateway_client_bootstrap_cache_path(),
    ).await
}
