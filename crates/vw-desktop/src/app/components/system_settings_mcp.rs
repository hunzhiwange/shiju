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
        page::McpMessage::ServerSearchChanged(query) => McpMessage::ServerSearchChanged(query),
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
    SettingsTabDescriptor::new(
        page::MCP_TAB_LABELS[3].1,
        page::MCP_TAB_LABELS[3].0,
        &["一键安装", "install", "RustCodeGraph", "Chrome DevTools"],
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
    let i18n = desktop_i18n::I18n::new(app.general_settings.desktop_locale);
    use crate::app::message::settings::mcp::transfer::message;
    use desktop_settings_mcp::{configuration_transfer::Message as Event, toolbar_button};
    use desktop_ui_controls::system_settings_common::settings_muted_text_style;
    use iced::widget::{column, row, text};

    let state = &app.mcp_settings.page;
    let ready = !state.loading && !state.saving && !state.transfer_pending;
    let mut actions = row![].spacing(8);
    if cfg!(not(target_arch = "wasm32")) {
        actions = actions
            .push(toolbar_button(i18n.__("导入配置文件"), ready.then(|| message(Event::Import))))
            .push(toolbar_button(i18n.__("导出 JSON 备份"), ready.then(|| message(Event::Export))));
    }
    actions = actions.push(toolbar_button(
        i18n.__("粘贴配置 JSON"),
        ready.then(|| message(Event::Paste)),
    ));
    let mut panel = column![actions.wrap()].spacing(6);
    if cfg!(not(target_arch = "wasm32")) {
        panel = panel.push(
            text(i18n.__("支持 Codex config.toml、Claude Code .mcp.json、Cursor mcp.json 和识矩全局 MCP 备份。导入后检查草稿并保存。备份包含编辑器中的全局服务器配置，可能含有凭据。"))
                .size(12)
                .style(settings_muted_text_style),
        );
    }
    panel.into()
}

#[cfg(not(target_arch = "wasm32"))]
fn extension_row(
    app: &App,
    extension: gateway_desktop_settings::McpExtension,
    title: &'static str,
    description: &'static str,
) -> Element<'static, Message> {
    use crate::app::message::settings::mcp::extensions::message;
    use desktop_i18n::settings_extensions::text as tr;
    use desktop_settings_mcp::{extensions::Message as Event, toolbar_button};
    use desktop_ui_controls::system_settings_common::settings_muted_text_style;
    use iced::widget::{column, container, progress_bar, row, text, text::Wrapping};
    use iced::{Alignment, Length};

    let locale = app.general_settings.desktop_locale;
    let i18n = desktop_i18n::I18n::new(locale);
    let feedback = |value: &str| desktop_i18n::settings_extensions::feedback(i18n, value);
    let state = &app.mcp_settings;
    let card = state.extensions.cards.get(&extension);
    let expanded = card.is_some_and(|card| card.expanded);
    let active = state.extensions.active.as_ref().filter(|active| active.extension == extension);
    let can_install = !state.loading
        && !state.saving
        && !state.transfer_pending
        && !state.new_server_expanded
        && state.extensions.active.is_none();
    let muted = |value: String| {
        text(value)
            .size(12)
            .style(settings_muted_text_style)
            .width(Length::Fill)
            .wrapping(Wrapping::WordOrGlyph)
    };
    let actions = row![
        toolbar_button(
            tr(locale, if active.is_some() { "安装中…" } else { "安装或升级" }),
            can_install.then(|| message(Event::Install(extension))),
        ),
        toolbar_button(
            tr(locale, if expanded { "收起" } else { "安装详情" }),
            Some(message(Event::Toggle(extension))),
        ),
    ]
    .spacing(8)
    .wrap()
    .align_x(iced::alignment::Horizontal::Right);
    let mut content = column![
        row![
            column![
                text(title).size(15).wrapping(Wrapping::WordOrGlyph),
                muted(description.to_owned()),
            ]
            .spacing(5)
            .width(Length::FillPortion(3)),
            container(actions).align_right(Length::FillPortion(2)),
        ]
        .spacing(16)
        .align_y(Alignment::Center)
    ]
    .spacing(10)
    .width(Length::Fill);

    if !expanded {
        return container(content).width(Length::Fill).padding([14, 0]).into();
    }
    if let Some(active) = active {
        let stage = active
            .progress
            .as_ref()
            .map(|progress| progress.stage.as_str())
            .filter(|stage| !stage.is_empty())
            .unwrap_or("安装中…");
        content = content.push(muted(feedback(stage)));
        if let Some(download) =
            active.progress.as_ref().and_then(|progress| progress.download.as_ref())
        {
            let mib = |bytes: u64| bytes as f64 / 1_048_576.0;
            let rate = download.bytes_per_second as f64 / 1024.0;
            let amount = if let Some(total) = download.total.filter(|total| *total > 0) {
                let fraction = (download.downloaded as f64 / total as f64).clamp(0.0, 1.0);
                content = content.push(progress_bar(0.0..=1.0, fraction as f32).girth(8));
                format!("{:.1} / {:.1} MiB", mib(download.downloaded), mib(total))
            } else {
                format!("{:.1} MiB", mib(download.downloaded))
            };
            content = content
                .push(muted(format!("{} · {amount} · {rate:.1} KiB/s", tr(locale, "下载中"))));
        }
    }
    if let Some(report) = card.and_then(|card| card.report.as_ref()) {
        let transcript = report
            .steps
            .iter()
            .map(|step| feedback(step))
            .chain(std::iter::once(feedback(&report.message)))
            .collect::<Vec<_>>()
            .join("\n\n");
        content = content.push(extension_output(
            tr(locale, if report.success { "安装完成" } else { "安装失败" }).to_owned(),
            tr(locale, "复制安装输出"),
            transcript,
        ));
    }
    match card.and_then(|card| card.instructions.as_ref()) {
        Some(Ok(instructions)) => {
            content = content
                .push(muted(tr(locale, "在网关主机的终端执行，完成后刷新并测试连接。").to_owned()));
            for command in &instructions.commands {
                content = content.push(extension_output(
                    format!(
                        "{} · {} · {}",
                        tr(locale, "手动安装命令"),
                        instructions.shell,
                        instructions.version
                    ),
                    tr(locale, "复制安装命令"),
                    command.clone(),
                ));
            }
            content = content
                .push(muted(
                    tr(
                        locale,
                        "手动安装后，在高级 JSON 中新增此配置，或在同名服务器中更新命令和参数。",
                    )
                    .to_owned(),
                ))
                .push(extension_output(
                    tr(locale, "MCP 配置").to_owned(),
                    tr(locale, "复制 MCP 配置"),
                    instructions.configuration.clone(),
                ));
        }
        Some(Err(error)) => {
            content =
                content.push(muted(format!("{} · {error}", tr(locale, "无法读取网关安装命令"))));
        }
        None => {
            content = content.push(muted(tr(locale, "正在读取网关安装命令…").to_owned()));
        }
    }
    container(content).width(Length::Fill).padding([14, 0]).into()
}

#[cfg(not(target_arch = "wasm32"))]
fn extension_output(
    label: String,
    copy_label: &'static str,
    output: String,
) -> Element<'static, Message> {
    use crate::app::message::settings::mcp::extensions::message;
    use desktop_settings_mcp::{extensions::Message as Event, toolbar_button};
    use desktop_ui_controls::system_settings_common::{settings_muted_text_style, settings_panel};
    use iced::Length;
    use iced::widget::{column, row, scrollable, text, text::Wrapping};

    settings_panel(
        column![
            row![
                text(label).size(13).width(Length::Fill).wrapping(Wrapping::WordOrGlyph),
                toolbar_button(copy_label, Some(message(Event::Copy(output.clone())))),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            scrollable(
                text(output)
                    .size(12)
                    .style(settings_muted_text_style)
                    .width(Length::Fill)
                    .wrapping(Wrapping::WordOrGlyph),
            )
            .direction(iced::widget::scrollable::Direction::Vertical(
                iced::widget::scrollable::Scrollbar::new().width(4).scroller_width(4),
            ))
            .height(Length::Shrink),
        ]
        .spacing(8),
    )
    .padding([10, 12])
    .max_height(220)
    .into()
}

fn extensions_panel(app: &App) -> Element<'_, Message> {
    use desktop_i18n::settings_extensions::text as tr;
    use desktop_ui_controls::system_settings_common::{
        settings_group_card, settings_muted_text_style,
    };
    use iced::widget::{container, text};

    let locale = app.general_settings.desktop_locale;
    #[cfg(not(target_arch = "wasm32"))]
    if crate::app::config::gateway_client_endpoint().is_loopback() {
        use desktop_ui_controls::system_settings_common::settings_divider;
        use gateway_desktop_settings::McpExtension;
        return settings_group_card(iced::widget::column![
            extension_row(
                app,
                McpExtension::RustCodeGraph,
                "RustCodeGraph MCP",
                tr(locale, "从 GitHub 下载并注册到本机网关。使用服务器配置中的命令路径，在项目目录运行 init -i 创建索引。"),
            ),
            settings_divider(),
            extension_row(
                app,
                McpExtension::ChromeDevTools,
                "Chrome DevTools MCP",
                tr(locale, "通过 npm 下载官方包。需要 Node.js LTS、npm 和最新版稳定版 Chrome。"),
            ),
        ])
        .into();
    }

    settings_group_card(
        container(
            text(tr(locale, "一键安装仅支持连接本机网关的桌面客户端。"))
                .size(13)
                .style(settings_muted_text_style),
        )
        .padding([16, 0]),
    )
    .into()
}

pub fn view(app: &App) -> Element<'_, Message> {
    page::view(
        event,
        &app.mcp_settings.page,
        app.general_settings.desktop_locale,
        configuration_transfer_panel(app),
        extensions_panel(app),
    )
}
/// Managed-directory controls belong exclusively to the third-party workspace.
pub(crate) fn third_party_panel(app: &App) -> Element<'_, Message> {
    use crate::app::message::settings::mcp_management_list as list;
    iced::widget::column![
        catalog::panel(
            desktop_i18n::I18n::new(app.general_settings.desktop_locale),
            list::visible(app),
            list::loading(app)
        )
        .map(catalog_event),
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
