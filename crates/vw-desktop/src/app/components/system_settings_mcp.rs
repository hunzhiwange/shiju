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
        page::McpMessage::Connect(key) => McpMessage::Connect(key),
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
fn configuration_transfer_panel(app: &App) -> Element<'_, Message> {
    use crate::app::message::settings::mcp::transfer::message;
    use desktop_settings_mcp::{configuration_transfer::Message as Event, toolbar_button};
    use desktop_ui_controls::system_settings_common::settings_muted_text_style;
    use iced::widget::{column, row, text};

    let state = &app.mcp_settings.page;
    let ready = !state.loading && !state.saving && !state.transfer_pending;
    let mut actions = row![].spacing(8);
    if cfg!(not(target_arch = "wasm32")) {
        actions = actions
            .push(toolbar_button("导入配置文件", ready.then(|| message(Event::Import))))
            .push(toolbar_button("导出 JSON 备份", ready.then(|| message(Event::Export))));
    }
    actions = actions
        .push(toolbar_button("粘贴配置 JSON", ready.then(|| message(Event::Paste))))
        .push(toolbar_button("复制 AI 修改提示词", Some(message(Event::CopyPrompt))));
    let mut panel = column![actions.wrap()].spacing(6);
    if cfg!(not(target_arch = "wasm32")) {
        panel = panel.push(
            text("支持 Codex config.toml、Claude Code .mcp.json、Cursor mcp.json 和识矩全局 MCP 备份。导入后检查草稿并保存。备份包含编辑器中的全局服务器配置，可能含有凭据。")
                .size(12)
                .style(settings_muted_text_style),
        );
    }
    panel.into()
}

#[cfg(not(target_arch = "wasm32"))]
fn rustcodegraph_install_panel(app: &App) -> Element<'_, Message> {
    use desktop_i18n::settings_extensions::text as tr;
    use desktop_ui_controls::system_settings_common::{
        settings_group_card, settings_muted_text_style,
    };
    use iced::widget::{column, text};

    if !crate::app::config::gateway_client_endpoint().is_loopback() {
        return iced::widget::Space::new().into();
    }
    let state = &app.mcp_settings;
    let locale = app.general_settings.desktop_locale;
    let label = if state.installing_rustcodegraph {
        "正在安装 RustCodeGraph…"
    } else {
        "一键安装 RustCodeGraph MCP"
    };
    let ready = !state.loading && !state.saving && !state.transfer_pending
        && !state.new_server_expanded;
    let action = ready.then_some(Message::Settings(SettingsMessage::Mcp(
        McpMessage::InstallRustCodeGraph,
    )));
    settings_group_card(column![
        desktop_settings_mcp::toolbar_button(tr(locale, label), action),
        text(tr(locale, "从 GitHub 下载并注册到本机网关。使用服务器配置中的命令路径，在项目目录运行 init -i 创建索引。"))
            .size(12)
            .style(settings_muted_text_style),
    ].spacing(8))
    .padding([14, 16])
    .into()
}

#[cfg(not(target_arch = "wasm32"))]
fn chrome_devtools_install_panel(app: &App) -> Element<'_, Message> {
    use desktop_i18n::settings_extensions::text as tr;
    use desktop_ui_controls::system_settings_common::{
        settings_group_card, settings_muted_text_style,
    };
    use iced::widget::{column, text};

    if !crate::app::config::gateway_client_endpoint().is_loopback() {
        return iced::widget::Space::new().into();
    }
    let state = &app.mcp_settings;
    let locale = app.general_settings.desktop_locale;
    let label = if state.installing_chrome_devtools {
        "正在安装 Chrome DevTools MCP…"
    } else {
        "一键安装 Chrome DevTools MCP"
    };
    let ready = !state.loading
        && !state.saving
        && !state.transfer_pending
        && !state.new_server_expanded;
    let action = ready.then_some(Message::Settings(SettingsMessage::Mcp(
        McpMessage::InstallChromeDevTools,
    )));
    settings_group_card(
        column![
            desktop_settings_mcp::toolbar_button(tr(locale, label), action),
            text(tr(locale, "通过 npm 下载官方包。需要 Node.js LTS、npm 和最新版稳定版 Chrome。"))
                .size(12)
                .style(settings_muted_text_style),
        ]
        .spacing(8),
    )
    .padding([14, 16])
    .into()
}

pub fn view(app: &App) -> Element<'_, Message> {
    page::view(
        event,
        &app.mcp_settings.page,
        app.general_settings.desktop_locale,
        &app.appearance.app_theme,
        std::array::from_fn(|index| match index {
            0 => configuration_transfer_panel(app),
            #[cfg(not(target_arch = "wasm32"))]
            1 => rustcodegraph_install_panel(app),
            #[cfg(not(target_arch = "wasm32"))]
            2 => chrome_devtools_install_panel(app),
            _ => iced::widget::Space::new().into(),
        }),
        help,
    )
}
/// Managed-directory controls belong exclusively to the third-party workspace.
pub(crate) fn third_party_panel(app: &App) -> Element<'_, Message> {
    use crate::app::message::settings::mcp_management_list as list;
    iced::widget::column![
        catalog::panel(list::visible(app), list::loading(app)).map(catalog_event),
        resync::panel(app),
        crate::app::components::agent_apps_diagnostics::mcp_panel::view(app),
        tools::panel(app),
    ].spacing(12).into()
}

pub(crate) fn third_party_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
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
    dialog
}

pub fn view_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    page::view_overlays(
        event,
        &app.mcp_settings.page,
        app.general_settings.desktop_locale,
        &app.appearance.app_theme,
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
pub(crate) mod resync;

#[path = "system_settings_mcp_tools.rs"]
pub(crate) mod tools;
