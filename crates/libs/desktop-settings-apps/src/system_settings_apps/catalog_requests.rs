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

/// Query fixed metadata and let the authorized host validate upgrade support.
pub async fn check_updates(client: GatewayClient, host: String, token: String, rows: Vec<CatalogRow>) -> Vec<desktop_apps_ui::catalog_state::UpdateCandidate> {
    use iced::futures::{stream, StreamExt};
    use vw_api_app_operations::{AppActionPreviewRequest, AppActionTarget, AppLifecycleAction};
    let http = reqwest::Client::new();
    let lifecycle = gateway_app_lifecycle::GatewayAppLifecycleClient::new(client.transport().clone());
    stream::iter(rows).filter_map(|row| async move { super::presentation::preferred(&row).cloned() })
        .map(|installation| {
            let (http, lifecycle, host, token) = (&http, &lifecycle, &host, &token);
            async move {
                let version = app_versions::latest::query_latest(http, installation.app_id, installation.version.as_deref()).await;
                let supported = if version.update_available {
                    lifecycle.agent_apps_action_preview_authorized(host, &AppActionPreviewRequest {
                        app_id: installation.app_id, action: AppLifecycleAction::Update,
                        target: AppActionTarget::Installation { installation_id: installation.id.clone() },
                    }, token).await.is_ok()
                } else { false };
                let status = match version.unknown_reason {
                    Some(app_versions::latest::UnknownReason::Network) => "检查失败，请重试",
                    Some(app_versions::latest::UnknownReason::Unsupported) => "暂不支持版本检查",
                    Some(app_versions::latest::UnknownReason::LocalAhead) => "本地版本领先",
                    Some(_) => "版本不可比较",
                    None if version.update_available && !supported => "升级预检未通过",
                    None if version.update_available => "可升级",
                    None => "已是最新版本",
                }.to_owned();
                desktop_apps_ui::catalog_state::UpdateCandidate { installation, latest: version.latest_version, supported, status }
            }
        }).buffered(6).collect().await
}
