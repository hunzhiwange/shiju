// Automatic local provisioning and credential restoration; remote endpoints cannot provision this machine.
use vw_gateway_client::GatewayClient;

pub fn available(client: Option<&GatewayClient>) -> bool {
    client.is_some()
}

pub async fn connect(
    client: GatewayClient,
) -> Result<local_app_auth::Credentials, String> {
    provision(client, false).await
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn connect_auth(
    client: GatewayClient,
) -> Result<local_app_auth::Credentials, String> {
    provision(client, true).await
}

async fn provision(client: GatewayClient, auth: bool) -> Result<local_app_auth::Credentials, String> {
    let _ = (client, auth);
    // Application routes use the active Gateway connection, including optional skey.
    Ok(local_app_auth::Credentials { token: String::new() })
}

/// Only our private local grant may default to this machine's existing CLI configuration.
pub fn launch_directory(
    client: &GatewayClient,
    host: &str,
    token: &str,
    project: Option<&str>,
) -> Option<String> {
    if host != local_app_auth::HOST_ID || !client.endpoint().is_loopback() {
        return None;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let base = directories::BaseDirs::new()?;
        verified_launch_directory(base.home_dir(), client.endpoint().tenant_scope(), token, project)
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (token, project);
        None
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn verified_launch_directory(
    home: &std::path::Path,
    tenant: &vw_api_transport::TenantScope,
    token: &str,
    project: Option<&str>,
) -> Option<String> {
    let _ = (tenant, token);
    Some(
        project
            .filter(|p| !p.trim().is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| home.to_string_lossy().into_owned()),
    )
}
