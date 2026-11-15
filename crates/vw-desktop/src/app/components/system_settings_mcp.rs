//! MCP desktop composition, state projection and local message mapping.
use crate::app::{App, Message};
use crate::app::message::settings::{McpMessage, SettingsMessage};
use crate::app::state::McpSettingsTab;
use crate::app::components::system_settings_search::SettingsTabDescriptor;
use desktop_settings_mcp::{page, catalog};
use iced::{Element, widget::container};
#[cfg(test)]
use vw_api_mcp_inspect::McpServerInspectDto;
use desktop_ui_controls::system_settings_common::settings_modal_overlay;
#[cfg(test)]
use config_ui::DesktopLocale;
fn event(value: page::McpMessage) -> Message {
    Message::Settings(SettingsMessage::Mcp(match value {
        page::McpMessage::TabSelected(v0) => McpMessage::TabSelected(v0),
        page::McpMessage::Refresh => McpMessage::Refresh,
        page::McpMessage::InspectRefresh => McpMessage::InspectRefresh,
        page::McpMessage::EditorAction(v0) => McpMessage::EditorAction(v0),
        page::McpMessage::Format => McpMessage::Format,
        page::McpMessage::InsertTemplate => McpMessage::InsertTemplate,
        page::McpMessage::NewServerPanelToggled => McpMessage::NewServerPanelToggled,
        page::McpMessage::NewServerNameChanged(v0) => McpMessage::NewServerNameChanged(v0),
        page::McpMessage::NewServerTransportChanged(v0) => McpMessage::NewServerTransportChanged(v0),
        page::McpMessage::NewServerCommandChanged(v0) => McpMessage::NewServerCommandChanged(v0),
        page::McpMessage::NewServerArgsChanged(v0) => McpMessage::NewServerArgsChanged(v0),
        page::McpMessage::NewServerUrlChanged(v0) => McpMessage::NewServerUrlChanged(v0),
        page::McpMessage::NewServerTimeoutChanged(v0) => McpMessage::NewServerTimeoutChanged(v0),
        page::McpMessage::NewServerEnabledToggled(v0) => McpMessage::NewServerEnabledToggled(v0),
        page::McpMessage::NewServerRequiredToggled(v0) => McpMessage::NewServerRequiredToggled(v0),
        page::McpMessage::AddServer => McpMessage::AddServer,
        page::McpMessage::ServerEditRequested(v0) => McpMessage::ServerEditRequested(v0),
        page::McpMessage::ServerEditCanceled => McpMessage::ServerEditCanceled,
        page::McpMessage::SaveServerEdit => McpMessage::SaveServerEdit,
        page::McpMessage::ServerEnabledToggled(v0, v1) => McpMessage::ServerEnabledToggled(v0, v1),
        page::McpMessage::RuntimeServerEnabledToggled(v0, v1) => McpMessage::RuntimeServerEnabledToggled(v0, v1),
        page::McpMessage::PreviewExpandedToggled(v0) => McpMessage::PreviewExpandedToggled(v0),
        page::McpMessage::RequestDelete(v0) => McpMessage::RequestDelete(v0),
        page::McpMessage::ConfirmDelete(v0) => McpMessage::ConfirmDelete(v0),
        page::McpMessage::CancelDelete => McpMessage::CancelDelete,
        page::McpMessage::Save => McpMessage::Save,
    }))
}
fn help<'a>(i18n: desktop_i18n::I18n, theme: &'a iced::Theme, title: &'a str,
    label: iced::widget::Text<'a>, description: std::borrow::Cow<'a, str>) -> iced::widget::Row<'a, Message> {
    crate::app::components::system_settings_common::settings_hover_help_label(i18n, theme, title, label, description)
}
pub(super) const MCP_TABS: &[SettingsTabDescriptor<McpSettingsTab>] = &[
    SettingsTabDescriptor::new(
        page::MCP_TAB_LABELS[0].1,
        page::MCP_TAB_LABELS[0].0,
        &[
            "mcpServers",
            "server",
            "stdio",
            "http",
            "sse",
            "headers",
            "env",
            "allowedTools",
            "deniedTools",
            "timeout",
        ],
    ),
    SettingsTabDescriptor::new(
        page::MCP_TAB_LABELS[1].1,
        page::MCP_TAB_LABELS[1].0,
        &["runtime", "运行时覆盖", "inspect", "工具预览"],
    ),
    SettingsTabDescriptor::new(
        page::MCP_TAB_LABELS[2].1,
        page::MCP_TAB_LABELS[2].0,
        &["json", "原始配置", "高级编辑"],
    ),
];


fn catalog_event(value: catalog::Message) -> Message {
    use crate::app::message::settings::{mcp_management_form as form, mcp_management_import as import, mcp_management_list as list};
    match value {
        catalog::Message::Refresh => Message::Settings(SettingsMessage::Mcp(McpMessage::Catalog(list::CatalogMessage::Refresh))),
        catalog::Message::Open(record) => form::message(form::FormMessage::Open(record)),
        catalog::Message::Import => import::message(import::ImportMessage::Open),
    }
}
pub(crate) fn managed_catalog_content(result: Option<&Result<vw_api_mcp_management::McpCatalogResponse, String>>, loading: bool) -> Element<'_, Message> {
    catalog::content(result, loading).map(catalog_event)
}
pub fn view(app: &App) -> Element<'_, Message> {
    use crate::app::message::settings::mcp_management_list as list;
    page::view(event, &app.mcp_settings.page, app.desktop_locale, &app.app_theme, [
        catalog::panel(list::visible(app), list::loading(app)).map(catalog_event),
        bindings::panel(app), resync::panel(app),
        crate::app::components::agent_apps_diagnostics::mcp_panel::view(app), tools::panel(app),
    ], help)
}
pub fn view_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    if crate::app::message::settings::mcp_management_tools::visible(app).is_some() {
        return settings_modal_overlay(
            Some(dialog),
            crate::app::message::settings::mcp_management_tools::message(
                crate::app::message::settings::mcp_management_tools::ToolsMessage::Close,
            ),
            container(tools::card(app)).padding(12),
        );
    }

    if crate::app::message::settings::mcp_management_resync::visible(app).is_some_and(|s| s.open) {
        return resync::overlay(app, dialog);
    }
    if let Some(state) = crate::app::message::settings::mcp_management_import::visible(app) {
        return settings_modal_overlay(
            Some(dialog),
            crate::app::message::settings::mcp_management_import::message(
                crate::app::message::settings::mcp_management_import::ImportMessage::Close,
            ),
            import::panel(app, state),
        );
    }
    if let Some(draft) = &app.mcp_settings.connection_form {
        use crate::app::message::settings::mcp_management_form::{FormMessage, message};
        return settings_modal_overlay(
            Some(dialog),
            message(FormMessage::Close),
            container(connection_form_panel(draft)).padding(12),
        );
    }
    page::view_overlays(event, &app.mcp_settings.page, app.desktop_locale, &app.app_theme, help, dialog)
}

#[cfg(test)]
use desktop_settings_mcp::ConfiguredMcpServerRow;
#[cfg(test)]
use page::mcp_tab_labels;
#[cfg(test)]
fn new_server_panel(app: &App) -> Element<'_, Message> {
    page::new_server_panel(event, &app.mcp_settings.page, app.desktop_locale, &app.app_theme, help)
}
#[cfg(test)]
fn configured_server_panel<'a>(
    locale: DesktopLocale,
    server: ConfiguredMcpServerRow,
    inspect: Option<&'a McpServerInspectDto>,
    preview_key: String,
    preview_expanded: bool,
    controls_enabled: bool,
    preview_enabled: bool,
    previewing: bool,
) -> Element<'a, Message> {
    page::configured_server_panel(event, locale, server, inspect, preview_key, preview_expanded, controls_enabled, preview_enabled, previewing)
}
#[cfg(test)]
fn runtime_server_panel(
    locale: DesktopLocale,
    server: &McpServerInspectDto,
    preview_key: String,
    preview_expanded: bool,
    controls_enabled: bool,
    preview_enabled: bool,
    previewing: bool,
) -> Element<'_, Message> {
    page::runtime_server_panel(event, locale, server, preview_key, preview_expanded, controls_enabled, preview_enabled, previewing)
}

#[cfg(test)]
#[path = "system_settings_mcp_tests.rs"]
mod system_settings_mcp_tests;
#[path = "system_settings_mcp_connection.rs"]
mod connection;
pub(crate) use connection::connection_form_panel;

#[path = "system_settings_mcp_bindings.rs"]
mod bindings;

#[path = "system_settings_mcp_import.rs"]
pub(crate) mod import;

#[path = "system_settings_mcp_resync.rs"]
mod resync;

#[path = "system_settings_mcp_tools.rs"]
pub(crate) mod tools;
