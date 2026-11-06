//! Automatic local provisioning and credential restoration; remote endpoints cannot provision this machine.
use vw_gateway_client::GatewayClient;

pub(crate) fn available() -> bool {
    cfg!(unix) && crate::app::config::gateway_client().is_ok_and(|c| c.endpoint().is_loopback())
}

pub(crate) async fn connect(
    client: GatewayClient,
) -> Result<vw_shared::local_apps::Credentials, String> {
    if !cfg!(unix) || !client.endpoint().is_loopback() {
        return Err("请先在网关连接中切换到本机网关，再添加本机应用".into());
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let tenant = client.endpoint().tenant_scope().clone();
        tokio::task::spawn_blocking(move || {
            let base =
                directories::BaseDirs::new().ok_or_else(|| "无法确定本机用户目录".to_string())?;
            let result = vw_shared::local_apps::initialize(
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
    Err("请在桌面应用中添加本机应用".into())
}
