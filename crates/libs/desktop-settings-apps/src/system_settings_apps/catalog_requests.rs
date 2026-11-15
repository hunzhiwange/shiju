//! Catalog request error mapping; cross-page navigation stays in the desktop shell.
use super::catalog::{self, CatalogRow};
use vw_gateway_client::GatewayClient;
pub async fn load(client: GatewayClient, host: String, token: String) -> Result<Vec<CatalogRow>, String> {
    let local = client.endpoint().is_loopback() && host == local_app_auth::HOST_ID;
    catalog::load(client, host, token).await.map_err(|error| {
        if local && matches!(error, gateway_http::AgentAppsReadError::Http { status: 503, .. }) {
            "本机应用服务尚未就绪，请重启新版网关后重试".into()
        } else {
            catalog::error_label(&error).into()
        }
    })
}
