//! 网关客户端持久化及跨域连接切换编排。
use crate::app::{App, Message, update_gateway_client_config};
use iced::Task;
use super::messages::{SettingsMessage};
use desktop_settings_network::message::gateway_client::{self, Effect};
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    let SettingsMessage::GatewayClient(message) = message else { return Task::none(); };
    let (draft, previous, changes_selection, refresh_health) = match gateway_client::update(&mut app.gateway_client_settings, desktop_settings_network::Message::GatewayClient(message)) {
        Effect::None => return Task::none(),
        Effect::Error(error) => return app.show_error_toast(error.display(app.general_settings.desktop_locale)),
        Effect::Save { config, previous, changes_selection, refresh_health } => (config, previous, changes_selection, refresh_health),
    };
    let old_endpoint = refresh_health.then(crate::app::gateway_client_endpoint);
    let config = match update_gateway_client_config(move |cfg| cfg.set_servers(draft.servers, draft.active_server_id)) {
        Ok(config) => config,
        Err(_) => {
            let error = gateway_client::save_failed(&mut app.gateway_client_settings, previous, changes_selection);
            return app.show_error_toast(error.display(app.general_settings.desktop_locale));
        }
    };
    gateway_client::save_completed(&mut app.gateway_client_settings);
    let (endpoint, save) = app.queue_gateway_client_snapshot(config);
    #[cfg(not(target_arch = "wasm32"))]
    app.startup_configuration.select_endpoint(&endpoint);
    let Some(old_endpoint) = old_endpoint else {
        return save;
    };
    let preferences = if old_endpoint.base_url() != endpoint.base_url()
        || old_endpoint.tenant_scope() != endpoint.tenant_scope()
        || old_endpoint.auth.and_then(|auth| auth.skey) != endpoint.auth.and_then(|auth| auth.skey)
    {
        let archived = super::archived_chats::gateway_changed(app);
        app.session_state.last_llm_full_access = None;
        app.reset_draft_full_access();
        let worktrees = super::worktrees::gateway_changed(app);
        Task::batch([archived, worktrees, app.reload_project_chat_preferences(true)])
    } else {
        Task::none()
    };
    Task::batch([save, preferences, Task::done(Message::GatewayHealthTick)])
}

#[cfg(test)]
#[path = "gateway_client_tests.rs"]
mod tests;
