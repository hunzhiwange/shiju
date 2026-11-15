//! MCP page presentation over domain state and explicit UI events.
use crate::*;
use crate::state::{State, McpNewServerTransport, McpSettingsTab};
use desktop_i18n::{I18n, settings_extensions::text as tr};
use desktop_ui_controls::system_settings_common::*;
use config_ui::DesktopLocale;
use iced::{Alignment, Element, Length, widget::{Space, button, checkbox, column, container, row, text, text_editor}};
use vw_api_mcp_inspect::{McpConnectionStatusDto, McpServerInspectDto};
use std::borrow::Cow;
// The shell supplies its existing selectable help renderer; no application state crosses this boundary.
pub type Help<Message> = for<'a> fn(I18n, &'a iced::Theme, &'a str, iced::widget::Text<'a>, Cow<'a, str>) -> iced::widget::Row<'a, Message>;
#[derive(Debug, Clone)]
pub enum McpMessage {
    TabSelected(McpSettingsTab),
    Refresh,
    InspectRefresh,
    EditorAction(text_editor::Action),
    Format,
    InsertTemplate,
    NewServerPanelToggled,
    NewServerNameChanged(String),
    NewServerTransportChanged(McpNewServerTransport),
    NewServerCommandChanged(String),
    NewServerArgsChanged(String),
    NewServerUrlChanged(String),
    NewServerTimeoutChanged(String),
    NewServerEnabledToggled(bool),
    NewServerRequiredToggled(bool),
    AddServer,
    ServerEditRequested(usize),
    ServerEditCanceled,
    SaveServerEdit,
    ServerEnabledToggled(usize, bool),
    RuntimeServerEnabledToggled(String, bool),
    PreviewExpandedToggled(String),
    /// 请求删除指定索引的 MCP 服务器（弹出确认）。
    RequestDelete(usize),
    /// 确认删除指定的 MCP 服务器；处理时仍需匹配当前待确认索引。
    ConfirmDelete(usize),
    /// 取消删除操作。
    CancelDelete,
    Save,
}
use crate::{toolbar_button, form_field};
pub const MCP_TAB_LABELS: &[(&str, McpSettingsTab)] = &[
    ("全局服务器", McpSettingsTab::Servers),
    ("运行时 MCP", McpSettingsTab::Runtime),
    ("高级 JSON", McpSettingsTab::Json),
];
pub fn mcp_tab_labels() -> &'static [(&'static str, McpSettingsTab)] {
    MCP_TAB_LABELS
}

pub(crate) fn mcp_tab_button<'a, Message: Clone + 'static>(event: fn(McpMessage) -> Message,
    locale: DesktopLocale,
    label: &'static str,
    tab: McpSettingsTab,
    active_tab: McpSettingsTab,
) -> Element<'static, Message> {
    settings_tab_button(
        tr(locale, label),
        tab == active_tab,
        Some(event(McpMessage::TabSelected(tab))),
    )
    .into()
}

pub(crate) fn preview_action_message<'a, Message: Clone + 'static>(event: fn(McpMessage) -> Message, enabled: bool) -> Option<Message> {
    enabled.then(|| event(McpMessage::InspectRefresh))
}

pub(crate) fn empty_preview<'a, Message: Clone + 'static>(event: fn(McpMessage) -> Message,
    locale: DesktopLocale,
    preview_enabled: bool,
    previewing: bool,
) -> Element<'static, Message> {
    let label = if previewing { tr(locale, "预览中...") } else { tr(locale, "预览") };
    row![
        text(tr(locale, "暂无预览结果"))
            .size(12)
            .style(settings_muted_text_style)
            .width(Length::Fill),
        toolbar_button(label, preview_action_message(event, preview_enabled)),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .into()
}

pub(crate) fn capability_preview<'a, Message: Clone + 'static>(event: fn(McpMessage) -> Message,
    locale: DesktopLocale,
    inspect: Option<&McpServerInspectDto>,
    preview_enabled: bool,
    previewing: bool,
) -> Element<'static, Message> {
    let Some(inspect) = inspect else {
        return empty_preview(event, locale, preview_enabled, previewing);
    };

    if let Some(error) = &inspect.error {
        return text(compact_text(error, 140)).size(12).style(settings_muted_text_style).into();
    }

    if inspect.connection_status == McpConnectionStatusDto::Disabled {
        return text(tr(locale, "服务器已禁用")).size(12).style(settings_muted_text_style).into();
    }

    column![
        tools_preview_section(locale, inspect),
        settings_divider(),
        value_preview_section(
            locale,
            tr(locale, "资源"),
            &inspect.resources,
            inspect.resources_error.as_deref(),
            &["name", "title", "uri"],
            &["uri", "description", "mimeType"]
        ),
        settings_divider(),
        value_preview_section(
            locale,
            tr(locale, "资源模板"),
            &inspect.resource_templates,
            inspect.resource_templates_error.as_deref(),
            &["name", "title", "uriTemplate"],
            &["uriTemplate", "description", "mimeType"]
        ),
        settings_divider(),
        value_preview_section(
            locale,
            "Prompts",
            &inspect.prompts,
            inspect.prompts_error.as_deref(),
            &["name", "title"],
            &["description"]
        ),
    ]
    .spacing(12)
    .width(Length::Fill)
    .into()
}

pub(crate) fn server_save_error(locale: DesktopLocale, s: &State) -> Option<String> {
    s.save_error.as_ref().map(|error| {
        s.save_error_translation
            .as_ref()
            .filter(|message| {
                message
                    .render(I18n::new(DesktopLocale::SimplifiedChinese))
                    .is_ok_and(|source| source == *error)
            })
            .map(|message| {
                message
                    .render(I18n::new(locale))
                    .expect("valid MCP error template")
            })
            .unwrap_or_else(|| I18n::new(locale).__(error).to_owned())
    })
}

/// 构建 MCP 系统设置页。
pub fn view<'a, Message: Clone + 'static>(event: fn(McpMessage) -> Message, s: &'a State, locale: DesktopLocale, theme: &'a iced::Theme, panels: [Element<'a, Message>; 5], help: Help<Message>) -> Element<'a, Message> {
    let [catalog_panel, bindings_panel, resync_panel, diagnostics_panel, tools_panel] = panels;
    let i18n = I18n::new(locale);
    let section = |title: &'static str, description: &'static str| {
        settings_group_card(
            help(
                i18n,
                theme,
                title,
                text(title).size(14),
                description.into(),
            )
            .wrap(),
        )
        .padding([14, 16])
    };
    let refresh_label = if s.loading { tr(locale, "刷新中...") } else { tr(locale, "刷新") };
    let inspect_label =
        if s.inspect_loading { tr(locale, "预览中...") } else { tr(locale, "刷新预览") };
    let save_label = if s.saving { tr(locale, "保存中...") } else { tr(locale, "保存") };

    let refresh_message = (!s.loading && !s.saving)
        .then(|| event(McpMessage::Refresh));
    let inspect_message = (!s.inspect_loading && !s.saving)
        .then(|| event(McpMessage::InspectRefresh));
    let format_message = (!s.loading && !s.saving)
        .then(|| event(McpMessage::Format));
    let template_message = (!s.loading && !s.saving)
        .then(|| event(McpMessage::InsertTemplate));
    let save_message = (!s.loading && !s.saving)
        .then(|| event(McpMessage::Save));

    let mut save_button =
        button(text(save_label).size(13)).padding([7, 16]).style(primary_action_btn_style);
    if let Some(message) = save_message {
        save_button = save_button.on_press(message);
    }

    let mut action_row = row![
        toolbar_button(refresh_label, refresh_message),
        toolbar_button(inspect_label, inspect_message),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    if s.active_tab == McpSettingsTab::Json {
        action_row = action_row
            .push(toolbar_button(tr(locale, "格式化"), format_message))
            .push(toolbar_button(tr(locale, "插入模板"), template_message));
    }
    action_row = action_row.push(save_button);

    let mut content = column![
        help(i18n, theme, tr(locale, "MCP 配置"), text(tr(locale, "MCP 配置")).size(28), tr(locale, "配置全局 mcpServers；插件声明的 MCP 会随插件启用状态在运行时合并。").into()).wrap(),
        catalog_panel,
        bindings_panel,
        resync_panel,
        diagnostics_panel,
        action_row.wrap(),
        row![
            settings_value_badge(
                desktop_i18n::__!(I18n::new(locale), "%u 个服务器", s.server_count)
                    .expect("valid server count template")
            ),
            settings_value_badge(tr(locale, "全局配置")),
            settings_value_badge(
                desktop_i18n::__!(
                    I18n::new(locale),
                    "%u 个运行时服务器",
                    s.inspect.as_ref().map(|inspect| inspect.servers.len()).unwrap_or(0)
                )
                .expect("valid runtime count template")
            ),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        settings_tab_row(
            mcp_tab_labels()
                .iter()
                .map(|(label, tab)| { mcp_tab_button(event, locale, label, *tab, s.active_tab) })
        )
        .wrap(),
    ]
    .spacing(16)
    .width(Length::Fill);

    if let Some(message) = &s.status_message {
        content = content.push(settings_success_banner(message));
    }
    if !(s.active_tab == McpSettingsTab::Servers && s.new_server_expanded)
        && let Some(error) = server_save_error(locale, s)
    {
        content = content.push(settings_error_banner(error));
    }
    if let Some(error) = &s.inspect_error {
        content = content.push(settings_panel(
            text(
                desktop_i18n::__!(
                    I18n::new(locale),
                    "MCP 预览失败：%s",
                    compact_text(error, 180)
                )
                .expect("valid preview error template"),
            )
            .size(12)
            .style(settings_muted_text_style),
        ));
    }

    let configured_rows = configured_mcp_server_rows(locale, &s.editor.text()).unwrap_or_default();
    match s.active_tab {
        McpSettingsTab::Servers => {
            content = content.push(section(
                tr(locale, "全局 MCP 服务器"),
                tr(locale, "逐条启用、禁用和预览全局 mcpServers。"),
            ));
            let (title, _) = server_form_heading(locale, s.editing_server_index.is_some());
            content = content.push(toolbar_button(title, Some(server_form_toggle(event))));
            if configured_rows.is_empty() {
                content = content.push(settings_panel(
                    text(tr(locale, "当前没有全局 MCP 服务器。"))
                        .size(12)
                        .style(settings_muted_text_style),
                ));
            } else {
                let controls_enabled = !s.loading && !s.saving;
                let preview_enabled = !s.loading && !s.saving && !s.inspect_loading;
                let inspect = s.inspect.as_ref();
                for server in configured_rows.iter().cloned() {
                    let preview_key = format!("global:{}", server.name);
                    let preview_expanded = s.expanded_previews.contains(&preview_key);
                    let inspect_entry = inspect_for_server(inspect, &server.name);
                    content = content.push(configured_server_panel(event,
                        locale,
                        server,
                        inspect_entry,
                        preview_key,
                        preview_expanded,
                        controls_enabled,
                        preview_enabled,
                        s.inspect_loading,
                    ));
                }
            }
        }
        McpSettingsTab::Runtime => {
            content = content.push(section(
                tr(locale, "运行时 MCP"),
                tr(locale, "展示项目配置和插件声明合并后的 MCP 连接、工具、资源与 prompts。"),
            ));
            if let Some(inspect) = s.inspect.as_ref() {
                if inspect.servers.is_empty() {
                    content = content.push(settings_panel(
                        text(tr(locale, "当前运行时没有 MCP 服务器。"))
                            .size(12)
                            .style(settings_muted_text_style),
                    ));
                } else {
                    let controls_enabled = !s.loading && !s.saving;
                    let preview_enabled = !s.loading && !s.saving && !s.inspect_loading;
                    for server in &inspect.servers {
                        let preview_key = format!("runtime:{}", server.name);
                        let preview_expanded = s.expanded_previews.contains(&preview_key);
                        content = content.push(runtime_server_panel(event,
                            locale,
                            server,
                            preview_key,
                            preview_expanded,
                            controls_enabled,
                            preview_enabled,
                            s.inspect_loading,
                        ));
                    }
                }
            } else {
                content = content.push(settings_panel(
                    text(tr(locale, "尚未加载 MCP 预览。"))
                        .size(12)
                        .style(settings_muted_text_style),
                ));
            }
        }
        McpSettingsTab::Json => {
            content = content.push(section(
                tr(locale, "高级 JSON"),
                tr(locale, "支持 type=stdio/http/sse；可配置 headers、env、enabled、required、timeout、allowedTools、deniedTools。"),
            ));
            content = content.push(settings_panel(
                text_editor(&s.editor)
                    .placeholder(
                        r#"[
  {
    "name": "local-demo",
    "type": "stdio",
    "command": "npx",
    "args": ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
  }
]"#,
                    )
                    .on_action(|action| {
                        event(McpMessage::EditorAction(action))
                    })
                    .padding(SETTINGS_CONTROL_PADDING)
                    .size(SETTINGS_CONTROL_TEXT_SIZE)
                    .height(Length::Fixed(360.0))
                    .style(settings_text_editor_style),
            ));
        }
    }

    content = content.push(tools_panel);

    container(content).padding([0, 7]).width(Length::Fill).into()
}

/// 在 MCP 设置页上叠加服务器表单或删除确认。
pub fn view_overlays<'a, Message: Clone + 'static>(event: fn(McpMessage) -> Message, s: &'a State, locale: DesktopLocale, theme: &'a iced::Theme, help: Help<Message>, dialog: Element<'a, Message>) -> Element<'a, Message> {
    if s.active_tab != McpSettingsTab::Servers {
        return dialog;
    }

    let Some(index) = s.delete_pending else {
        return if s.new_server_expanded {
            settings_modal_overlay(
                Some(container(dialog).width(Length::Fill).height(Length::Fill).into()),
                server_form_toggle(event),
                container(new_server_panel(event, s, locale, theme, help)).padding(12),
            )
        } else {
            dialog
        };
    };
    let server_name = configured_mcp_server_rows(locale, &s.editor.text())
        .ok()
        .and_then(|rows| rows.get(index).map(|row| row.name.clone()))
        .unwrap_or_else(|| {
            desktop_i18n::__!(I18n::new(locale), "mcpServers[%u]", index)
                .expect("valid MCP template")
        });
    let confirm = event(McpMessage::ConfirmDelete(index));
    let cancel = event(McpMessage::CancelDelete);
    let confirm_dialog = desktop_ui_controls::toast::confirm_dialog(
        tr(locale, "确认删除 MCP 服务器"),
        desktop_i18n::__!(
            I18n::new(locale),
            "将删除 MCP 服务器「%s」。此操作不可撤销。",
            server_name.as_str()
        )
        .expect("valid MCP template"),
        tr(locale, "确认删除"),
        tr(locale, "取消"),
        confirm,
        cancel,
    );

    iced::widget::stack![dialog, confirm_dialog].into()
}


mod server_panels;
pub use server_panels::{configured_server_panel, runtime_server_panel};
mod server_form;
pub use server_form::new_server_panel;
use server_form::{server_form_heading, server_form_toggle};
