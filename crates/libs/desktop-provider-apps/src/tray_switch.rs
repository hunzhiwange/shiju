//! Provider switching from the native tray.
use crate::Scope;
use vw_gateway_client::GatewayClient;
use vw_api_types::agent_apps::AppId;
use vw_api_app_provider_edit::catalog::ProviderSelection;
use vw_api_app_provider_base::claude::ClaudeLiveResult;
use vw_api_app_provider_base::claude::ClaudeTarget;
use vw_api_app_provider_profiles::claude_desktop::ClaudeDesktopTarget;
use vw_api_app_provider_profiles::codex_catalog::CodexTarget;
use vw_api_app_provider_profiles::gemini::GeminiTarget;
use vw_api_app_provider_extended::grok::GrokTarget;

pub fn supports(app: AppId) -> bool {
    matches!(
        app,
        AppId::Claude | AppId::ClaudeDesktop | AppId::Codex | AppId::Gemini | AppId::GrokBuild
    )
}

pub async fn switch(client: GatewayClient, scope: Scope, id: String, token: String) -> Result<(), String> {
    let host = scope.host;
    let result = match scope.app {
        AppId::Claude => {
            gateway_app_provider::GatewayAppProviderClient::new(client.transport().clone())
                .claude_provider_switch(
                    &id,
                    &ClaudeTarget { target_host_id: host },
                    &token,
                )
                .await
        }
        AppId::ClaudeDesktop => {
            gateway_app_provider::GatewayAppProviderClient::new(client.transport().clone())
                .claude_desktop_provider_switch(
                    &id,
                    &ClaudeDesktopTarget { target_host_id: host },
                    &token,
                )
                .await
        }
        AppId::Codex => {
            gateway_app_provider::GatewayAppProviderClient::new(client.transport().clone()).codex_provider_switch(
                    &id,
                    &CodexTarget { target_host_id: host },
                    &token,
                )
                .await
        }
        AppId::Gemini => {
            gateway_app_provider::GatewayAppProviderClient::new(client.transport().clone())
                .gemini_provider_switch(
                    &id,
                    &GeminiTarget { target_host_id: host },
                    &token,
                )
                .await
        }
        AppId::GrokBuild => {
            gateway_app_provider::GatewayAppProviderClient::new(client.transport().clone())
                .grok_provider_switch(
                    &id,
                    &GrokTarget { target_host_id: host },
                    &token,
                )
                .await
        }
        _ => return Err("当前应用没有托盘切换入口".into()),
    }
    .map_err(|_| "供应商切换请求失败，请检查授权与连接".to_string())?;
    match result.live {
        ClaudeLiveResult::Applied => Ok(()),
        ClaudeLiveResult::RecoveryRequired => {
            Err("配置事务需要恢复，请在供应商页面处理".into())
        }
        ClaudeLiveResult::NotAttempted | ClaudeLiveResult::Failed => {
            Err("供应商配置未成功应用，请刷新检查".into())
        }
    }
}

pub fn menu(
    scope: Option<&Scope>, generation: u64, snapshot: Option<&crate::state::list::Snapshot>,
    shutting_down: bool, loading: bool, switching: bool,
) -> (String, bool, Option<Vec<(String, String)>>) {
    let enabled = !shutting_down && !loading && !switching;
    let snapshot = scope.filter(|s| supports(s.app)).and(snapshot);
    let signature = format!("{:?}:{:?}:{:?}:{enabled}", scope, generation, snapshot);
    let rows = snapshot.map(|snapshot| snapshot.catalog.rows.iter().map(|row| {
        let checked = matches!(&snapshot.catalog.in_use, ProviderSelection::Selected { id } if id == &row.provider.id);
        (row.provider.id.clone(), format!("{}{}", if checked { "✓ " } else { "" }, row.provider.name))
    }).collect());
    (signature, enabled, rows)
}

pub fn can_start(scope: Option<&Scope>, snapshot: Option<&crate::state::list::Snapshot>, id: &str,
    shutting_down: bool, loading: bool, switching: bool) -> bool {
    !shutting_down && !loading && !switching
        && scope.is_some_and(|scope| supports(scope.app))
        && snapshot.is_some_and(|snapshot| snapshot.catalog.rows.iter().any(|row| row.provider.id == id)
            && !matches!(&snapshot.catalog.in_use, ProviderSelection::Selected { id: selected } if selected == id))
}

pub fn apply_result(snapshot: Option<&mut crate::state::list::Snapshot>, app: AppId,
    id: String, result: Result<(), String>) -> Result<&'static str, String> {
    result?;
    if let Some(snapshot) = snapshot {
        snapshot.catalog.in_use = ProviderSelection::Selected { id: id.clone() };
        snapshot.catalog.direct = ProviderSelection::Selected { id };
    }
    Ok(if app == AppId::ClaudeDesktop {
        "配置已切换，请完全退出并重启 Claude Desktop"
    } else { "供应商已切换" })
}
