//! External project opening, with local reveal safety delegated to desktop-config.
pub async fn open_project(
    endpoint: gateway_endpoint::GatewayEndpoint,
    path: &str,
    target: &str,
) -> Result<(), String> {
    if target == "finder" {
        let client = vw_gateway_client::GatewayClient::new(endpoint)?;
        desktop_config::config_file_manager::reveal_gateway_path_locally(client.endpoint(), path).await
    } else {
        crate::session_gateway::gateway_external_open_async(endpoint, path, target).await
    }
}
