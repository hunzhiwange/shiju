//! Tray writes use the same scoped SDK as provider forms.
use crate::app::message::{GeneralSettingsMessage, SettingsMessage};
use crate::app::state::app_providers::list::Scope;
use crate::app::{App, Message};
use iced::Task;
use vw_gateway_client::vw_api_types::{
    agent_apps::AppId,
    app_providers::{
        catalog::ProviderSelection,
        claude::{ClaudeLiveResult, ClaudeTarget},
        claude_desktop::ClaudeDesktopTarget,
        codex_catalog::CodexTarget,
        gemini::GeminiTarget,
        grok::GrokTarget,
    },
};

pub(super) fn supports(app: AppId) -> bool {
    matches!(
        app,
        AppId::Claude | AppId::ClaudeDesktop | AppId::Codex | AppId::Gemini | AppId::GrokBuild
    )
}

pub(crate) fn start(app: &mut App, id: String) -> Task<Message> {
    let state = &app.app_providers;
    if app.shutdown_requested || state.loading || state.tray_switch.is_some() {
        return Task::none();
    }
    let Some(scope) = state.scope.clone().filter(|s| supports(s.app)) else {
        return Task::none();
    };
    let Some(snapshot) = state.snapshot.as_ref() else { return Task::none() };
    if !snapshot.catalog.rows.iter().any(|row| row.provider.id == id)
        || matches!(&snapshot.catalog.in_use, ProviderSelection::Selected { id: selected } if selected == &id)
    {
        return Task::none();
    }
    let (client, current) = match crate::app::message::app_providers::list::current(app, scope.app)
    {
        Ok(value) => value,
        Err(error) => return app.show_error_toast(error),
    };
    if current != scope {
        return Task::none();
    }
    let generation = state.generation;
    let token = app.apps_catalog.token.clone();
    app.app_providers.tray_switch = Some(id.clone());
    let worker_scope = scope.clone();
    let worker_id = id.clone();
    Task::perform(
        async move {
            let host = worker_scope.host;
            let result = match worker_scope.app {
                AppId::Claude => {
                    client
                        .claude_provider_switch(
                            &worker_id,
                            &ClaudeTarget { target_host_id: host },
                            &token,
                        )
                        .await
                }
                AppId::ClaudeDesktop => {
                    client
                        .claude_desktop_provider_switch(
                            &worker_id,
                            &ClaudeDesktopTarget { target_host_id: host },
                            &token,
                        )
                        .await
                }
                AppId::Codex => {
                    client
                        .codex_provider_switch(
                            &worker_id,
                            &CodexTarget { target_host_id: host },
                            &token,
                        )
                        .await
                }
                AppId::Gemini => {
                    client
                        .gemini_provider_switch(
                            &worker_id,
                            &GeminiTarget { target_host_id: host },
                            &token,
                        )
                        .await
                }
                AppId::GrokBuild => {
                    client
                        .grok_provider_switch(
                            &worker_id,
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
        },
        move |result| {
            Message::Settings(SettingsMessage::General(
                GeneralSettingsMessage::TrayProviderSwitched {
                    scope: scope.clone(),
                    generation,
                    id: id.clone(),
                    result,
                },
            ))
        },
    )
}

pub(crate) fn finish(
    app: &mut App,
    scope: Scope,
    generation: u64,
    id: String,
    result: Result<(), String>,
) -> Task<Message> {
    if app.app_providers.scope.as_ref() != Some(&scope)
        || app.app_providers.generation != generation
        || app.app_providers.tray_switch.as_ref() != Some(&id)
    {
        return Task::none();
    }
    app.app_providers.tray_switch = None;
    if crate::app::message::app_providers::list::current(app, scope.app)
        .map_or(true, |(_, current)| current != scope)
    {
        return Task::none();
    }
    match result {
        Ok(()) => {
            if let Some(snapshot) = app.app_providers.snapshot.as_mut() {
                snapshot.catalog.in_use = ProviderSelection::Selected { id: id.clone() };
                snapshot.catalog.direct = ProviderSelection::Selected { id };
            }
            app.show_success_toast(if scope.app == AppId::ClaudeDesktop {
                "配置已切换，请完全退出并重启 Claude Desktop"
            } else {
                "供应商已切换"
            })
        }
        Err(error) => app.show_error_toast(error),
    }
}
