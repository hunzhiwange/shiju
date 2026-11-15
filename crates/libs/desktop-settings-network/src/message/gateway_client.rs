//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。


use config_ui::GatewayClientSystemSettingsConfig;
use vw_api_transport::id::TenantScope;

use crate::messages::GatewayClientMessage;
use crate::Message as SettingsMessage;
use crate::state::{GatewayClientSettingsState, GatewayClientServerDraft};
use desktop_i18n::settings_models::messages::Feedback;

fn normalize_host(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() { "127.0.0.1".to_string() } else { trimmed.to_string() }
}

fn normalize_name(raw: &str, index: usize) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() { format!("网关 {}", index + 1) } else { trimmed.to_string() }
}

#[cfg(test)]
#[path = "gateway_client_tests.rs"]
mod gateway_client_tests;

fn selected_index(app: &GatewayClientSettingsState) -> Option<usize> {
    app
        .servers
        .iter()
        .position(|server| server.id == app.selected_server_id)
}

fn sync_selected_server_from_inputs(app: &mut GatewayClientSettingsState) -> Result<(), Feedback> {
    let Some(index) = selected_index(app) else {
        return Ok(());
    };
    let settings = &app;
    let platform_id = settings.platform_id_input.trim();
    let company_id = settings.company_id_input.trim();
    TenantScope::new(platform_id, company_id)
        .map_err(|err| Feedback::owned("租户配置无效：%s", vec![err.to_string()]))?;
    let draft = GatewayClientServerDraft {
        id: settings.selected_server_id.clone(),
        name: normalize_name(&settings.name_input, index),
        host: normalize_host(&settings.host_input),
        port: settings.port.clamp(1, u16::MAX),
        platform_id: platform_id.to_string(),
        company_id: company_id.to_string(),
        skey: settings.skey_input.trim().to_string(),
    };
    app.servers[index] = draft;
    Ok(())
}

fn load_selected_server_into_inputs(app: &mut GatewayClientSettingsState) {
    let Some(index) = selected_index(app) else {
        return;
    };
    let draft = app.servers[index].clone();
    app.name_input = draft.name;
    app.host_input = normalize_host(&draft.host);
    app.port = draft.port.clamp(1, u16::MAX);
    app.platform_id_input = draft.platform_id;
    app.company_id_input = draft.company_id;
    app.skey_input = draft.skey;
}

fn next_server_id(config: &GatewayClientSystemSettingsConfig) -> String {
    let existing = config
        .normalized_servers()
        .into_iter()
        .map(|server| server.id)
        .collect::<std::collections::HashSet<_>>();
    for index in 1..=10_000 {
        let candidate = if index == 1 { "local".to_string() } else { format!("gateway-{index}") };
        if !existing.contains(&candidate) {
            return candidate;
        }
    }
    "gateway-new".to_string()
}

fn show_save_error(app: &mut GatewayClientSettingsState, error: Feedback) -> Effect {
    app.save_error = Some(error.clone());
    Effect::Error(error)
}

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回领域效果，由宿主执行持久化或平台动作。
pub fn update(app: &mut GatewayClientSettingsState, message: SettingsMessage) -> Effect {
    let SettingsMessage::GatewayClient(message) = message else {
        return Effect::None;
    };
    let refresh_health = matches!(
        &message,
        GatewayClientMessage::SelectServer(_)
            | GatewayClientMessage::AddServer
            | GatewayClientMessage::RemoveServerConfirmed(_)
            | GatewayClientMessage::HostChanged(_)
            | GatewayClientMessage::PortChanged(_)
            | GatewayClientMessage::PlatformIdChanged(_)
            | GatewayClientMessage::CompanyIdChanged(_)
            | GatewayClientMessage::SkeyChanged(_)
    );
    let changes_selection = matches!(
        &message,
        GatewayClientMessage::SelectServer(_)
            | GatewayClientMessage::AddServer
            | GatewayClientMessage::RemoveServerConfirmed(_)
    );
    let previous = app.clone();

    match message {
        GatewayClientMessage::SelectServer(server_id) => {
            if let Err(error) = sync_selected_server_from_inputs(app) {
                return show_save_error(app, error);
            }
            if app.servers.iter().any(|server| server.id == server_id) {
                app.selected_server_id = server_id;
                load_selected_server_into_inputs(app);
            }
        }
        GatewayClientMessage::AddServer => {
            if let Err(error) = sync_selected_server_from_inputs(app) {
                return show_save_error(app, error);
            }
            let id = {
                let mut cfg = GatewayClientSystemSettingsConfig::default();
                cfg.set_servers(
                    app
                        .servers
                        .iter()
                        .map(GatewayClientServerDraft::to_config)
                        .collect(),
                    app.selected_server_id.clone(),
                );
                next_server_id(&cfg)
            };
            let index = app.servers.len();
            let draft = GatewayClientServerDraft {
                id: id.clone(),
                name: format!("网关 {}", index + 1),
                host: "127.0.0.1".to_string(),
                port: config_gateway::DEFAULT_GATEWAY_PORT,
                platform_id: "10000".to_string(),
                company_id: "100000000".to_string(),
                skey: String::new(),
            };
            app.servers.push(draft);
            app.selected_server_id = id;
            load_selected_server_into_inputs(app);
        }
        GatewayClientMessage::RemoveServerRequested(server_id) => {
            if app.servers.len() > 1
                && app.servers.iter().any(|server| server.id == server_id)
            {
                app.pending_remove_server_id = Some(server_id);
            }
            return Effect::None;
        }
        GatewayClientMessage::RemoveServerConfirmed(server_id) => {
            let pending_matches = app.pending_remove_server_id.as_deref()
                == Some(server_id.as_str());
            app.pending_remove_server_id = None;
            if !pending_matches {
                return Effect::None;
            }
            if app.servers.len() > 1 {
                let removed_health_key = app
                    .servers
                    .iter()
                    .find(|server| server.id == server_id)
                    .and_then(crate::server_health_key);
                app.servers.retain(|server| server.id != server_id);
                if let Some(key) = removed_health_key {
                    app.health.remove(&key);
                }
                if app.selected_server_id == server_id
                    && let Some(first) = app.servers.first()
                {
                    app.selected_server_id = first.id.clone();
                }
                load_selected_server_into_inputs(app);
            }
        }
        GatewayClientMessage::RemoveServerCanceled => {
            app.pending_remove_server_id = None;
            return Effect::None;
        }
        GatewayClientMessage::NameChanged(value) => app.name_input = value,
        GatewayClientMessage::HostChanged(value) => {
            app.host_input = value;
        }
        GatewayClientMessage::PortChanged(value) => {
            app.port = value.clamp(1, u16::MAX);
        }
        GatewayClientMessage::PlatformIdChanged(value) => {
            app.platform_id_input = value;
        }
        GatewayClientMessage::CompanyIdChanged(value) => {
            app.company_id_input = value;
        }
        GatewayClientMessage::SkeyChanged(value) => app.skey_input = value,
        GatewayClientMessage::HelpOpen => {
            app.show_help_modal = true;
            return Effect::None;
        }
        GatewayClientMessage::HelpClose => {
            app.show_help_modal = false;
            return Effect::None;
        }
    }

    if let Err(error) = sync_selected_server_from_inputs(app) {
        return show_save_error(app, error);
    }
    let mut config = GatewayClientSystemSettingsConfig::default();
    config.set_servers(app.servers.iter().map(GatewayClientServerDraft::to_config).collect(), app.selected_server_id.clone());
    Effect::Save { config, previous, changes_selection, refresh_health }
}
pub enum Effect {
    None,
    Error(Feedback),
    Save { config: GatewayClientSystemSettingsConfig, previous: GatewayClientSettingsState, changes_selection: bool, refresh_health: bool },
}
pub fn save_failed(app: &mut GatewayClientSettingsState, previous: GatewayClientSettingsState, changes_selection: bool) -> Feedback {
    if changes_selection { *app = previous; } else { app.servers = previous.servers; }
    let error = Feedback::owned("无法保存本机网关连接。", vec![]);
    app.save_error = Some(error.clone());
    error
}
pub fn save_completed(app: &mut GatewayClientSettingsState) {
    load_selected_server_into_inputs(app);
    app.save_error = None;
}
