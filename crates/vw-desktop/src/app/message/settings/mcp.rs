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

#[cfg(not(target_arch = "wasm32"))]
#[path = "mcp_rustcodegraph.rs"]
mod rustcodegraph;

#[cfg(not(target_arch = "wasm32"))]
#[path = "mcp_chrome_devtools.rs"]
mod chrome_devtools;

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
    use desktop_settings_mcp::configuration_update::{Effect, Message as LocalMessage};
    let SettingsMessage::Mcp(message) = message else { return Task::none() };
    if (app.mcp_settings.installing_rustcodegraph || app.mcp_settings.installing_chrome_devtools)
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
        McpMessage::InstallChromeDevTools => {
            #[cfg(not(target_arch = "wasm32"))]
            if !app.mcp_settings.loading
                && !app.mcp_settings.saving
                && !app.mcp_settings.transfer_pending
                && !app.mcp_settings.new_server_expanded
            {
                let endpoint = crate::app::config::gateway_client_endpoint();
                app.mcp_settings.installing_chrome_devtools = true;
                app.mcp_settings.saving = true;
                return Task::perform(chrome_devtools::install(endpoint), |result| {
                    Message::Settings(SettingsMessage::Mcp(McpMessage::ChromeDevToolsInstalled(
                        result,
                    )))
                });
            }
            return Task::none();
        }
        McpMessage::ChromeDevToolsInstalled(result) => {
            if !app.mcp_settings.installing_chrome_devtools {
                return Task::none();
            }
            app.mcp_settings.installing_chrome_devtools = false;
            app.mcp_settings.saving = false;
            return match result {
                Ok(()) => {
                    app.mcp_settings.loading = true;
                    app.mcp_settings.inspect_loading = true;
                    let locale = app.general_settings.desktop_locale;
                    Task::batch([
                        refresh_all_task(app.navigation_tabs.project_path.clone()),
                        app.show_success_toast(desktop_i18n::settings_extensions::text(
                            locale,
                            "Chrome DevTools MCP 已安装",
                        )),
                    ])
                }
                Err(error) => app.show_error_toast(error),
            };
        }
        McpMessage::InstallRustCodeGraph => {
            #[cfg(not(target_arch = "wasm32"))]
            if !app.mcp_settings.loading
                && !app.mcp_settings.saving
                && !app.mcp_settings.page.transfer_pending
                && !app.mcp_settings.page.new_server_expanded
            {
                let endpoint = crate::app::config::gateway_client_endpoint();
                let editor = app.mcp_settings.editor.text();
                app.mcp_settings.installing_rustcodegraph = true;
                app.mcp_settings.saving = true;
                return Task::perform(rustcodegraph::install(endpoint, editor), |result| {
                    Message::Settings(SettingsMessage::Mcp(McpMessage::RustCodeGraphInstalled(result)))
                });
            }
            return Task::none();
        }
        McpMessage::RustCodeGraphInstalled(result) => {
            if !app.mcp_settings.installing_rustcodegraph {
                return Task::none();
            }
            app.mcp_settings.installing_rustcodegraph = false;
            app.mcp_settings.saving = false;
            return match result {
                Ok(()) => {
                    app.mcp_settings.loading = true;
                    app.mcp_settings.inspect_loading = true;
                    let locale = app.general_settings.desktop_locale;
                    Task::batch([
                        refresh_all_task(app.navigation_tabs.project_path.clone()),
                        app.show_success_toast(desktop_i18n::settings_extensions::text(
                            locale,
                            "RustCodeGraph MCP 已安装",
                        )),
                    ])
                }
                Err(error) => {
                    app.mcp_settings.loading = false;
                    app.show_error_toast(error)
                }
            };
        }
        McpMessage::Tools(value) => return super::mcp_management_tools::update(app, value),
        McpMessage::Diagnostic(value) => {
            return crate::app::message::agent_apps_diagnostics::update(app, value);
        }
        McpMessage::Catalog(value) => return super::mcp_management_list::update(app, value),
        McpMessage::ConnectionForm(value) => return super::mcp_management_form::update(app, value),
        McpMessage::Bindings(value) => return super::mcp_management_bindings::update(app, value),
        McpMessage::Resync(value) => return super::mcp_management_resync::update(app, value),
        McpMessage::Transfer(value) => return transfer::update(app, value),
        McpMessage::Import(value) => return super::mcp_management_import::update(app, value),
        McpMessage::TabSelected(v0) => LocalMessage::TabSelected(v0),
        McpMessage::Refresh => LocalMessage::Refresh,
        McpMessage::Loaded(v0) => LocalMessage::Loaded(v0),
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
    let directory = app.navigation_tabs.project_path.clone();
    match desktop_settings_mcp::configuration_update::update(&mut app.mcp_settings.page, local) {
        Effect::None => Task::none(),
        Effect::Inspect => inspect_task(directory),
        Effect::InspectServer { key, name } => Task::perform(
            async move { (key, load_mcp_inspect_async(directory.as_deref(), Some(&name)).await) },
            |(key, result)| Message::Settings(SettingsMessage::Mcp(McpMessage::Connected(key, result))),
        ),
        Effect::RefreshAll => refresh_all_task(directory),
        Effect::RefreshAllAndCatalog => Task::batch([
            refresh_all_task(directory),
            super::mcp_management_list::update(
                app,
                super::mcp_management_list::CatalogMessage::Refresh,
            ),
        ]),
        Effect::Save(value) => save_task(value),
        Effect::SaveOverrides(value) => save_runtime_overrides_task(value),
    }
}
#[cfg(test)]
#[path = "mcp_tests.rs"]
mod mcp_tests;
