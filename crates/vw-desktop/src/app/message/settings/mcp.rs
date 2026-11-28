//! 处理 MCP 系统设置页的读取、校验和保存。

use crate::app::config::{
    load_global_config_value_async, load_mcp_inspect_async, patch_full_agent_config_async,
};
use crate::app::{App, Message};
use iced::Task;
use serde_json::{Value, json};

use super::messages::{McpMessage, SettingsMessage};

#[path = "mcp_transfer.rs"]
pub(crate) mod transfer;

#[path = "mcp_extensions.rs"]
pub(crate) mod extensions;

fn refresh_task() -> Task<Message> {
    Task::perform(load_global_config_value_async(), |result| {
        Message::Settings(SettingsMessage::Mcp(McpMessage::Loaded(result)))
    })
}

fn inspect_task(directory: Option<String>) -> Task<Message> {
    Task::perform(async move { load_mcp_inspect_async(directory.as_deref(), None).await }, |result| {
        Message::Settings(SettingsMessage::Mcp(McpMessage::Inspected(result)))
    })
}

fn refresh_all_task(directory: Option<String>) -> Task<Message> {
    Task::batch(vec![refresh_task(), inspect_task(directory)])
}

pub(super) fn refresh_configuration(app: &mut App) -> Task<Message> {
    app.mcp_settings.loading = true;
    app.mcp_settings.inspect_loading = true;
    refresh_all_task(app.navigation_tabs.project_path.clone())
}

fn save_task(mcp_servers: Value) -> Task<Message> {
    Task::perform(
        async move { patch_full_agent_config_async(json!({ "mcpServers": mcp_servers })).await },
        |result| Message::Settings(SettingsMessage::Mcp(McpMessage::Saved(result))),
    )
}

fn save_runtime_overrides_task(runtime_overrides: Vec<Value>) -> Task<Message> {
    Task::perform(
        async move {
            patch_full_agent_config_async(json!({ "mcpServerOverrides": runtime_overrides })).await
        },
        |result| Message::Settings(SettingsMessage::Mcp(McpMessage::Saved(result))),
    )
}

pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    use crate::app::state::McpSettingsTab;
    use desktop_settings_mcp::configuration_update::{Effect, Message as LocalMessage};
    let SettingsMessage::Mcp(message) = message else { return Task::none() };
    if app.mcp_settings.extensions.active.is_some()
        && matches!(
            message,
            McpMessage::Refresh
                | McpMessage::EditorAction(_)
                | McpMessage::Format
                | McpMessage::InsertTemplate
                | McpMessage::NewServerPanelToggled
                | McpMessage::AddServer
                | McpMessage::ServerEditRequested(_)
                | McpMessage::SaveServerEdit
                | McpMessage::ServerEnabledToggled(_, _)
                | McpMessage::RuntimeServerEnabledToggled(_, _)
                | McpMessage::RequestDelete(_)
                | McpMessage::ConfirmDelete(_)
                | McpMessage::Save
        )
    {
        return Task::none();
    }
    let local = match message {
        McpMessage::GlobalSyncMenu(menu) => {
            if menu.is_none()
                || (crate::app::components::system_settings_mcp::is_visible(app)
                    && !app.mcp_settings.modal_open()
                    && !app.mcp_settings.loading
                    && !app.mcp_settings.mutating())
            {
                app.mcp_settings.page.global_sync_menu = menu;
            }
            return Task::none();
        }
        McpMessage::Extension(event) => return extensions::update(app, event),
        McpMessage::ConnectionForm(value) => return super::mcp_management_form::update(app, value),
        McpMessage::Transfer(value) => return transfer::update(app, value),
        McpMessage::TabSelected(v0) => LocalMessage::TabSelected(v0),
        McpMessage::Refresh => LocalMessage::Refresh,
        McpMessage::Loaded(v0) => LocalMessage::Loaded(v0),
        McpMessage::ServerSearchChanged(query) => LocalMessage::ServerSearchChanged(query),
        McpMessage::InspectRefresh => LocalMessage::InspectRefresh,
        McpMessage::Connect(key) => LocalMessage::Connect(key),
        McpMessage::Connected(key, result) => LocalMessage::Connected(key, result),
        McpMessage::Inspected(v0) => LocalMessage::Inspected(v0),
        McpMessage::EditorAction(v0) => LocalMessage::EditorAction(v0),
        McpMessage::Format => LocalMessage::Format,
        McpMessage::InsertTemplate => LocalMessage::InsertTemplate,
        McpMessage::NewServerPanelToggled => LocalMessage::NewServerPanelToggled,
        McpMessage::NewServerNameChanged(v0) => LocalMessage::NewServerNameChanged(v0),
        McpMessage::NewServerTransportChanged(v0) => LocalMessage::NewServerTransportChanged(v0),
        McpMessage::NewServerCommandChanged(v0) => LocalMessage::NewServerCommandChanged(v0),
        McpMessage::NewServerArgsChanged(v0) => LocalMessage::NewServerArgsChanged(v0),
        McpMessage::NewServerUrlChanged(v0) => LocalMessage::NewServerUrlChanged(v0),
        McpMessage::NewServerTimeoutChanged(v0) => LocalMessage::NewServerTimeoutChanged(v0),
        McpMessage::NewServerEnabledToggled(v0) => LocalMessage::NewServerEnabledToggled(v0),
        McpMessage::NewServerRequiredToggled(v0) => LocalMessage::NewServerRequiredToggled(v0),
        McpMessage::AddServer => LocalMessage::AddServer,
        McpMessage::ServerEditRequested(v0) => LocalMessage::ServerEditRequested(v0),
        McpMessage::ServerEditCanceled => LocalMessage::ServerEditCanceled,
        McpMessage::SaveServerEdit => LocalMessage::SaveServerEdit,
        McpMessage::ServerEnabledToggled(v0, v1) => LocalMessage::ServerEnabledToggled(v0, v1),
        McpMessage::RuntimeServerEnabledToggled(v0, v1) => {
            LocalMessage::RuntimeServerEnabledToggled(v0, v1)
        }
        McpMessage::PreviewExpandedToggled(v0) => LocalMessage::PreviewExpandedToggled(v0),
        McpMessage::RequestDelete(v0) => LocalMessage::RequestDelete(v0),
        McpMessage::ConfirmDelete(v0) => LocalMessage::ConfirmDelete(v0),
        McpMessage::CancelDelete => LocalMessage::CancelDelete,
        McpMessage::Save => LocalMessage::Save,
        McpMessage::Saved(v0) => LocalMessage::Saved(v0),
    };
    let show_extensions = matches!(&local, LocalMessage::TabSelected(McpSettingsTab::Extensions));
    let directory = app.navigation_tabs.project_path.clone();
    match desktop_settings_mcp::configuration_update::update(&mut app.mcp_settings.page, local) {
        Effect::None if show_extensions => extensions::refresh_statuses(app),
        Effect::None => Task::none(),
        Effect::Inspect => inspect_task(directory),
        Effect::InspectServer { key, name } => Task::perform(
            async move { (key, load_mcp_inspect_async(directory.as_deref(), Some(&name)).await) },
            |(key, result)| Message::Settings(SettingsMessage::Mcp(McpMessage::Connected(key, result))),
        ),
        Effect::RefreshAll => {
            let extension_status = if app.mcp_settings.active_tab == McpSettingsTab::Extensions {
                extensions::refresh_statuses(app)
            } else {
                Task::none()
            };
            Task::batch([refresh_all_task(directory), extension_status])
        }
        Effect::Save(value) => save_task(value),
        Effect::SaveOverrides(value) => save_runtime_overrides_task(value),
    }
}
#[cfg(test)]
#[path = "mcp_tests.rs"]
mod mcp_tests;
