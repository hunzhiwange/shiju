//! MCP desktop composition, state projection and local message mapping.
use crate::app::components::system_settings_search::SettingsTabDescriptor;
use crate::app::message::settings::{McpMessage, SettingsMessage};
use crate::app::state::McpSettingsTab;
use crate::app::{App, Message};
use desktop_settings_mcp::{catalog, page};
use desktop_ui_controls::system_settings_common::settings_modal_overlay;
use iced::{Element, widget::container};
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
        page::McpMessage::NewServerTransportChanged(v0) => {
            McpMessage::NewServerTransportChanged(v0)
        }
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
        page::McpMessage::RuntimeServerEnabledToggled(v0, v1) => {
            McpMessage::RuntimeServerEnabledToggled(v0, v1)
        }
        page::McpMessage::PreviewExpandedToggled(v0) => McpMessage::PreviewExpandedToggled(v0),
        page::McpMessage::RequestDelete(v0) => McpMessage::RequestDelete(v0),
        page::McpMessage::ConfirmDelete(v0) => McpMessage::ConfirmDelete(v0),
        page::McpMessage::CancelDelete => McpMessage::CancelDelete,
        page::McpMessage::Save => McpMessage::Save,
    }))
}
fn help<'a>(
    i18n: desktop_i18n::I18n,
    theme: &'a iced::Theme,
    title: &'a str,
    label: iced::widget::Text<'a>,
    description: std::borrow::Cow<'a, str>,
) -> iced::widget::Row<'a, Message> {
    crate::app::components::system_settings_common::settings_hover_help_label(
        i18n,
        theme,
        title,
        label,
        description,
    )
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
    use crate::app::message::settings::{
        mcp_management_form as form, mcp_management_import as import, mcp_management_list as list,
    };
    match value {
        catalog::Message::Refresh => Message::Settings(SettingsMessage::Mcp(McpMessage::Catalog(
            list::CatalogMessage::Refresh,
        ))),
        catalog::Message::Open(record) => form::message(form::FormMessage::Open(record)),
        catalog::Message::Import => import::message(import::ImportMessage::Open),
    }
}
pub fn view(app: &App) -> Element<'_, Message> {
    use crate::app::message::settings::mcp_management_list as list;
    page::view(
        event,
        &app.mcp_settings.page,
        app.desktop_locale,
        &app.app_theme,
        [
            catalog::panel(list::visible(app), list::loading(app)).map(catalog_event),
            bindings::panel(app),
            resync::panel(app),
            crate::app::components::agent_apps_diagnostics::mcp_panel::view(app),
            tools::panel(app),
        ],
        help,
    )
}
pub fn view_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    use desktop_settings_mcp::management_state::Overlay;
    let current = crate::app::message::settings::mcp_management_list::context(app);
    let diagnostics = crate::app::message::agent_apps_diagnostics::context(app);
    let overlay = app.mcp_settings.overlay(current.as_ref(), diagnostics.as_ref());
    if overlay == Overlay::Tools {
        return settings_modal_overlay(
            Some(dialog),
            crate::app::message::settings::mcp_management_tools::message(
                crate::app::message::settings::mcp_management_tools::ToolsMessage::Close,
            ),
            container(tools::card(app)).padding(12),
        );
    }

    if overlay == Overlay::Resync {
        return resync::overlay(app, dialog);
    }
    if overlay == Overlay::Import
        && let Some(state) = &app.mcp_settings.import
    {
        return settings_modal_overlay(
            Some(dialog),
            crate::app::message::settings::mcp_management_import::message(
                crate::app::message::settings::mcp_management_import::ImportMessage::Close,
            ),
            import::panel(app, state),
        );
    }
    if overlay == Overlay::Form
        && let Some(draft) = &app.mcp_settings.connection_form
    {
        use crate::app::message::settings::mcp_management_form::{FormMessage, message};
        return settings_modal_overlay(
            Some(dialog),
            message(FormMessage::Close),
            container(connection_form_panel(draft)).padding(12),
        );
    }
    page::view_overlays(
        event,
        &app.mcp_settings.page,
        app.desktop_locale,
        &app.app_theme,
        help,
        dialog,
    )
}

#[path = "system_settings_mcp_connection.rs"]
mod connection;
#[cfg(test)]
#[path = "system_settings_mcp_tests.rs"]
mod system_settings_mcp_tests;
pub(crate) use connection::connection_form_panel;

#[path = "system_settings_mcp_bindings.rs"]
mod bindings;

#[path = "system_settings_mcp_import.rs"]
pub(crate) mod import;

#[path = "system_settings_mcp_resync.rs"]
mod resync;

#[path = "system_settings_mcp_tools.rs"]
pub(crate) mod tools;
