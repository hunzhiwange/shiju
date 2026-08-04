//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。

use crate::app::state::GatewayClientServerDraft;
use crate::app::{App, Message, update_gateway_client_config};
use iced::Task;
use vw_config_types::ui::GatewayClientSystemSettingsConfig;
use vw_gateway_client::vw_api_types::id::TenantScope;

use super::messages::{GatewayClientMessage, SettingsMessage};

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

fn selected_index(app: &App) -> Option<usize> {
    app.gateway_client_settings
        .servers
        .iter()
        .position(|server| server.id == app.gateway_client_settings.selected_server_id)
}

fn sync_selected_server_from_inputs(app: &mut App) -> Result<(), String> {
    let Some(index) = selected_index(app) else {
        return Ok(());
    };
    let settings = &app.gateway_client_settings;
    let platform_id = settings.platform_id_input.trim();
    let company_id = settings.company_id_input.trim();
    TenantScope::new(platform_id, company_id).map_err(|err| format!("租户配置无效：{err}"))?;
    let draft = GatewayClientServerDraft {
        id: settings.selected_server_id.clone(),
        name: normalize_name(&settings.name_input, index),
        host: normalize_host(&settings.host_input),
        port: settings.port.clamp(1, u16::MAX),
        platform_id: platform_id.to_string(),
        company_id: company_id.to_string(),
        skey: settings.skey_input.trim().to_string(),
    };
    app.gateway_client_settings.servers[index] = draft;
    Ok(())
}

fn load_selected_server_into_inputs(app: &mut App) {
    let Some(index) = selected_index(app) else {
        return;
    };
    let draft = app.gateway_client_settings.servers[index].clone();
    app.gateway_client_settings.name_input = draft.name;
    app.gateway_client_settings.host_input = normalize_host(&draft.host);
    app.gateway_client_settings.port = draft.port.clamp(1, u16::MAX);
    app.gateway_client_settings.platform_id_input = draft.platform_id;
    app.gateway_client_settings.company_id_input = draft.company_id;
    app.gateway_client_settings.skey_input = draft.skey;
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

fn persist_gateway_client_settings(app: &mut App) -> Result<(), String> {
    sync_selected_server_from_inputs(app)?;
    let servers = app
        .gateway_client_settings
        .servers
        .iter()
        .map(GatewayClientServerDraft::to_config)
        .collect::<Vec<_>>();
    let active_server_id = app.gateway_client_settings.selected_server_id.clone();

    update_gateway_client_config(|cfg| {
        cfg.set_servers(servers, active_server_id);
    });
    Ok(())
}

fn tenant_validation_failure(app: &mut App, error: String) -> Task<Message> {
    app.gateway_client_settings.save_error = Some(error.clone());
    app.show_error_toast(error)
}

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回的 `Task` 用于继续执行异步保存、加载或通知清理；没有后续动作时返回空任务。
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    let SettingsMessage::GatewayClient(message) = message else {
        return Task::none();
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
    );

    match message {
        GatewayClientMessage::SelectServer(server_id) => {
            if let Err(error) = sync_selected_server_from_inputs(app) {
                return tenant_validation_failure(app, error);
            }
            if app.gateway_client_settings.servers.iter().any(|server| server.id == server_id) {
                app.gateway_client_settings.selected_server_id = server_id;
                load_selected_server_into_inputs(app);
            }
        }
        GatewayClientMessage::AddServer => {
            if let Err(error) = sync_selected_server_from_inputs(app) {
                return tenant_validation_failure(app, error);
            }
            let id = {
                let mut cfg = GatewayClientSystemSettingsConfig::default();
                cfg.set_servers(
                    app.gateway_client_settings
                        .servers
                        .iter()
                        .map(GatewayClientServerDraft::to_config)
                        .collect(),
                    app.gateway_client_settings.selected_server_id.clone(),
                );
                next_server_id(&cfg)
            };
            let index = app.gateway_client_settings.servers.len();
            let draft = GatewayClientServerDraft {
                id: id.clone(),
                name: format!("网关 {}", index + 1),
                host: "127.0.0.1".to_string(),
                port: vw_config_types::gateway::DEFAULT_GATEWAY_PORT,
                platform_id: "10000".to_string(),
                company_id: "100000000".to_string(),
                skey: String::new(),
            };
            app.gateway_client_settings.servers.push(draft);
            app.gateway_client_settings.selected_server_id = id;
            load_selected_server_into_inputs(app);
        }
        GatewayClientMessage::RemoveServerRequested(server_id) => {
            if app.gateway_client_settings.servers.len() > 1
                && app.gateway_client_settings.servers.iter().any(|server| server.id == server_id)
            {
                app.gateway_client_settings.pending_remove_server_id = Some(server_id);
            }
            return Task::none();
        }
        GatewayClientMessage::RemoveServerConfirmed(server_id) => {
            let pending_matches = app.gateway_client_settings.pending_remove_server_id.as_deref()
                == Some(server_id.as_str());
            app.gateway_client_settings.pending_remove_server_id = None;
            if !pending_matches {
                return Task::none();
            }
            if app.gateway_client_settings.servers.len() > 1 {
                let removed_health_key = app
                    .gateway_client_settings
                    .servers
                    .iter()
                    .find(|server| server.id == server_id)
                    .and_then(crate::app::message::gateway_health::server_health_key);
                app.gateway_client_settings.servers.retain(|server| server.id != server_id);
                if let Some(key) = removed_health_key {
                    app.gateway_client_settings.health.remove(&key);
                }
                if app.gateway_client_settings.selected_server_id == server_id
                    && let Some(first) = app.gateway_client_settings.servers.first()
                {
                    app.gateway_client_settings.selected_server_id = first.id.clone();
                }
                load_selected_server_into_inputs(app);
            }
        }
        GatewayClientMessage::RemoveServerCanceled => {
            app.gateway_client_settings.pending_remove_server_id = None;
            return Task::none();
        }
        GatewayClientMessage::NameChanged(value) => app.gateway_client_settings.name_input = value,
        GatewayClientMessage::HostChanged(value) => {
            app.gateway_client_settings.host_input = value;
        }
        GatewayClientMessage::PortChanged(value) => {
            app.gateway_client_settings.port = value.clamp(1, u16::MAX);
        }
        GatewayClientMessage::PlatformIdChanged(value) => {
            app.gateway_client_settings.platform_id_input = value;
        }
        GatewayClientMessage::CompanyIdChanged(value) => {
            app.gateway_client_settings.company_id_input = value;
        }
        GatewayClientMessage::SkeyChanged(value) => app.gateway_client_settings.skey_input = value,
        GatewayClientMessage::HelpOpen => {
            app.gateway_client_settings.show_help_modal = true;
            return Task::none();
        }
        GatewayClientMessage::HelpClose => {
            app.gateway_client_settings.show_help_modal = false;
            return Task::none();
        }
    }

    if let Err(error) = persist_gateway_client_settings(app) {
        return tenant_validation_failure(app, error);
    }
    load_selected_server_into_inputs(app);
    app.gateway_client_settings.host_input =
        normalize_host(&app.gateway_client_settings.host_input);
    app.gateway_client_settings.save_error = None;
    if refresh_health { Task::done(Message::GatewayHealthTick) } else { Task::none() }
}
