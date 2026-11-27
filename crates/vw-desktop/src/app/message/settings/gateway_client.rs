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
    let user_changed = old_endpoint.user_id() != endpoint.user_id();
    let connection_changed = old_endpoint.base_url() != endpoint.base_url()
        || old_endpoint.tenant_scope() != endpoint.tenant_scope()
        || old_endpoint.auth.as_ref().and_then(|auth| auth.skey.as_ref())
            != endpoint.auth.as_ref().and_then(|auth| auth.skey.as_ref());
    let preferences = if connection_changed || user_changed {
        let archived = super::archived_chats::gateway_changed(app);
        app.session_state.last_llm_full_access = None;
        app.reset_draft_full_access();
        let worktrees = super::worktrees::gateway_changed(app);
        Task::batch([
            archived,
            worktrees,
            app.reload_project_chat_preferences(true),
            #[cfg(not(target_arch = "wasm32"))]
            crate::app::workbench_projects::ensure_project_catalog(app),
        ])
    } else {
        Task::none()
    };
    let named_user = endpoint.user_id().as_ref() != vw_api_transport::DEFAULT_USER_ID;
    let sessions = if user_changed || (named_user && connection_changed) {
        app.session_state.gateway_user_generation = app.session_state.gateway_user_generation.wrapping_add(1);
        app.session_state.agent_stream_id = app.session_state.agent_stream_id.wrapping_add(1);
        app.session_state.active_session_id = None;
        app.session_state.session_view_cache.gateway_changed();
        app.session_state.session_chat_cache.clear();
        app.session_state.session_chat_message_id_cache.clear();
        app.session_state.session_runtime_states.clear();
        app.session_state.session_history_reset_boundaries = Default::default();
        app.session_state.session_reasoning_efforts.clear();
        app.session_state.permission_modal_requests.clear();
        app.session_state.paged_chat_timeline = None;
        app.close_session_transient_ui();
        app.workspace_state.project_sessions.clear();
        app.workspace_state.project_sessions_loading.clear();
        app.workspace_state.project_session_load_counts.clear();
        app.workspace_state.session_tree_expanded.clear();
        app.reload_sessions_for_project(app.navigation_tabs.project_path.clone())
    } else {
        Task::none()
    };
    #[cfg(not(target_arch = "wasm32"))]
    let employee_scope = if connection_changed || user_changed {
        crate::app::digital_employees::chat::check_scope(app, None)
    } else {
        Task::none()
    };
    #[cfg(target_arch = "wasm32")]
    let employee_scope = Task::none();
    #[cfg(not(target_arch = "wasm32"))]
    let media = if connection_changed || user_changed {
        Task::batch([
            crate::app::message::chat::media_jobs::gateway_changed(app),
            super::multimodal::gateway_changed(app),
        ])
    } else {
        Task::none()
    };
    #[cfg(target_arch = "wasm32")]
    let media = Task::none();
    Task::batch([save, preferences, sessions, employee_scope, media, Task::done(Message::GatewayHealthTick)])
}

#[cfg(test)]
#[path = "gateway_client_tests.rs"]
mod tests;
