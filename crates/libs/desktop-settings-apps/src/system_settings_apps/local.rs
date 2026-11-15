// Automatic local provisioning and credential restoration; remote endpoints cannot provision this machine.
use vw_gateway_client::GatewayClient;

pub fn available(client: Option<&GatewayClient>) -> bool {
    cfg!(unix) && client.is_some_and(|c| c.endpoint().is_loopback())
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

async fn provision(
    client: GatewayClient,
    auth: bool,
) -> Result<local_app_auth::Credentials, String> {
    if !cfg!(unix) || !client.endpoint().is_loopback() {
        return Err("请先在网关连接中切换到本机网关，再添加本机应用".into());
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let tenant = client.endpoint().tenant_scope().clone();
        tokio::task::spawn_blocking(move || {
            let base =
                directories::BaseDirs::new().ok_or_else(|| "无法确定本机用户目录".to_string())?;
            #[cfg(unix)]
            let initialize = if auth {
                local_app_auth::initialize_auth
            } else {
                local_app_auth::initialize
            };
            #[cfg(not(unix))]
            let initialize = {
                let _ = auth;
                local_app_auth::initialize
            };
            let result = initialize(
                base.home_dir(),
                tenant.platform_id().as_ref(),
                tenant.company_id().as_ref(),
            );
            result.map_err(|e| e.to_string())
        })
        .await
        .map_err(|_| "本机应用初始化任务失败".to_string())?
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = auth;
        Err("请在桌面应用中添加本机应用".into())
    }
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
    let credentials = local_app_auth::restore(
        home,
        tenant.platform_id().as_ref(),
        tenant.company_id().as_ref(),
    )
    .ok()??;
    if credentials.token != token {
        return None;
    }
    Some(
        project
            .filter(|p| !p.trim().is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| home.to_string_lossy().into_owned()),
    )
}
