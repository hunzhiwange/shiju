//! MCP page presentation over domain state and explicit UI events.
use crate::*;
use crate::state::{State, McpNewServerTransport, McpSettingsTab};
use desktop_i18n::{I18n, settings_extensions::text as tr};
use desktop_ui_controls::system_settings_common::*;
use config_ui::DesktopLocale;
use iced::{Alignment, Element, Length, widget::{Space, button, checkbox, column, container, row, text}};
use vw_api_mcp_inspect::{McpConnectionStatusDto, McpServerInspectDto};
use std::borrow::Cow;
// The shell supplies its existing selectable help renderer; no application state crosses this boundary.
pub type Help<Message> = for<'a> fn(I18n, &'a iced::Theme, &'a str, iced::widget::Text<'a>, Cow<'a, str>) -> iced::widget::Row<'a, Message>;
#[derive(Debug, Clone)]
pub enum McpMessage {
    TabSelected(McpSettingsTab),
    ServerSearchChanged(String),
    Refresh,
    InspectRefresh,
    Connect(String),
    EditorAction(iced::widget::text_editor::Action),
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
    ("MCP 服务器", McpSettingsTab::Servers),
    ("运行时 MCP", McpSettingsTab::Runtime),
    ("高级 JSON", McpSettingsTab::Json),
    ("推荐 MCP", McpSettingsTab::Extensions),
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

pub(crate) fn empty_preview<'a, Message: Clone + 'static>(
    locale: DesktopLocale,
    previewing: bool,
) -> Element<'static, Message> {
    let label = if previewing { "正在连接 MCP…" } else { "连接服务器后可查看预览。" };
    text(tr(locale, label)).size(12).style(settings_muted_text_style).into()
}

pub(crate) fn capability_preview<'a, Message: Clone + 'static>(
    locale: DesktopLocale,
    inspect: Option<&McpServerInspectDto>,
    previewing: bool,
) -> Element<'static, Message> {
    if previewing {
        return empty_preview(locale, true);
    }
    let Some(inspect) = inspect else {
        return empty_preview(locale, false);
    };

    if let Some(error) = &inspect.error {
        return text(compact_text(error, 140)).size(12).style(settings_muted_text_style).into();
    }

    if inspect.connection_status == McpConnectionStatusDto::Disabled {
        return text(tr(locale, "服务器已禁用")).size(12).style(settings_muted_text_style).into();
    }

    let mut content = column![tools_preview_section(locale, inspect)]
        .spacing(12)
        .width(Length::Fill);
    let sections = [
        value_preview_section(
            locale,
            tr(locale, "资源"),
            &inspect.resources,
            inspect.resources_error.as_deref(),
            &["name", "title", "uri"],
            &["uri", "description", "mimeType"],
        ),
        value_preview_section(
            locale,
            tr(locale, "资源模板"),
            &inspect.resource_templates,
            inspect.resource_templates_error.as_deref(),
            &["name", "title", "uriTemplate"],
            &["uriTemplate", "description", "mimeType"],
        ),
        value_preview_section(
            locale,
            tr(locale, "提示词"),
            &inspect.prompts,
            inspect.prompts_error.as_deref(),
            &["name", "title"],
            &["description"],
        ),
    ];
    for section in sections.into_iter().flatten() {
        content = content.push(settings_divider()).push(section);
    }
    content.into()
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
            .unwrap_or_else(|| desktop_i18n::settings_extensions::feedback(I18n::new(locale), error))
    })
}

fn search_toolbar<'a, Message: Clone + 'a>(
    search: Element<'a, Message>,
    actions: Element<'a, Message>,
) -> Element<'a, Message> {
    settings_group_card(
        row![
            container(container(search).max_width(340)).width(Length::FillPortion(2)),
            container(actions).align_right(Length::FillPortion(3)),
        ]
        .width(Length::Fill)
        .spacing(12)
        .align_y(Alignment::Center),
    )
    .padding([12, 0])
    .into()
}

/// 构建 MCP 系统设置页。
pub fn view<'a, Message: Clone + 'static>(
    event: fn(McpMessage) -> Message,
    s: &'a State,
    locale: DesktopLocale,
    transfer_actions: Vec<Element<'a, Message>>,
    extensions_panel: Element<'a, Message>,
) -> Element<'a, Message> {
    view_with_management(event, s, locale, transfer_actions, extensions_panel, None, |index| {
        event(McpMessage::ServerEditRequested(index))
    })
}

pub fn view_with_management<'a, Message: Clone + 'static>(
    event: fn(McpMessage) -> Message,
    s: &'a State,
    locale: DesktopLocale,
    transfer_actions: Vec<Element<'a, Message>>,
    extensions_panel: Element<'a, Message>,
    management: Option<Element<'a, Message>>,
    sync_global: impl Fn(usize) -> Message,
) -> Element<'a, Message> {
    let i18n = I18n::new(locale);
    let mut content = column![
        settings_page_heading(
            tr(locale, "MCP 配置"),
            tr(locale, "管理 MCP 服务器、运行时连接和第三方同步。"),
        ),
        settings_tab_row(mcp_tab_labels().iter().map(|(label, tab)| mcp_tab_button(
            event,
            locale,
            label,
            *tab,
            s.active_tab
        )))
        .wrap(),
        settings_divider(),
    ]
    .spacing(14)
    .width(Length::Fill);

    if let Some(message) = &s.status_message {
        content = content.push(settings_success_banner(
            desktop_i18n::settings_extensions::feedback(i18n, message),
        ));
    }
    if !(s.active_tab == McpSettingsTab::Servers && s.new_server_expanded)
        && let Some(error) = server_save_error(locale, s)
    {
        content = content.push(settings_error_banner(error));
    }
    if matches!(s.active_tab, McpSettingsTab::Servers | McpSettingsTab::Runtime)
        && let Some(error) = &s.inspect_error
    {
        content = content.push(settings_panel(
            text(
                desktop_i18n::__!(I18n::new(locale), "MCP 预览失败：%s", compact_text(error, 180))
                    .expect("valid preview error template"),
            )
            .size(12)
            .style(settings_muted_text_style),
        ));
    }

    let connecting = s.connecting.is_some();
    let controls_enabled = !s.loading && !s.saving && !connecting && !s.transfer_pending;
    let refresh_label = if s.loading { "刷新中..." } else { "刷新" };
    let refresh = toolbar_button(
        tr(locale, refresh_label),
        controls_enabled.then(|| event(McpMessage::Refresh)),
    );
    let inspect_label = if s.inspect_loading { "预览中..." } else { "刷新预览" };
    let inspect = toolbar_button(
        tr(locale, inspect_label),
        preview_action_message(event, !s.inspect_loading && !s.saving && !connecting),
    );
    let save_label = if s.saving { "保存中..." } else { "保存" };
    let save = controls_enabled.then(|| event(McpMessage::Save));
    let search = settings_text_input(tr(locale, "按名称搜索服务器"), &s.server_query)
        .width(Length::Fill)
        .on_input(move |query| event(McpMessage::ServerSearchChanged(query)));
    let query = s.server_query.trim().to_lowercase();
    let matches_name = |name: &str| query.is_empty() || name.to_lowercase().contains(&query);

    match s.active_tab {
        McpSettingsTab::Servers => {
            let add = button(
                row![text("+").size(14), text(tr(locale, "新增服务器")).size(13)]
                    .align_y(Alignment::Center)
                    .spacing(6),
            )
            .padding([9, 14])
            .style(primary_action_btn_style)
            .on_press_maybe(controls_enabled.then(|| server_form_toggle(event)));
            let actions = row![refresh, toolbar_button(tr(locale, save_label), save), add]
                .align_y(Alignment::Center)
                .spacing(8)
                .wrap()
                .align_x(iced::alignment::Horizontal::Right);
            let servers = configured_mcp_server_rows(locale, &s.editor.text()).unwrap_or_default();
            let empty = if servers.is_empty() && s.loading {
                "正在加载…"
            } else if servers.is_empty() {
                "当前没有全局 MCP 服务器。"
            } else {
                "没有匹配的结果"
            };
            let show_management = management.is_some();
            let entries =
                servers.into_iter().filter(|server| matches_name(&server.name)).map(|server| {
                    let preview_key = format!("global:{}", server.name);
                    let preview_expanded = s.expanded_previews.contains(&preview_key);
                    let previewing = s.connecting.as_ref() == Some(&preview_key);
                    let inspected = inspect_for_server(s.inspect.as_ref(), &server.name);
                    let sync = (show_management
                        && controls_enabled
                        && !config_tools::is_builtin_mcp_server_id(&server.name))
                        .then(|| sync_global(server.index));
                    server_panels::configured_server_panel_with_sync(
                        event,
                        locale,
                        server,
                        inspected,
                        preview_key,
                        preview_expanded,
                        controls_enabled,
                        previewing,
                        sync,
                    )
                });
            content = content.push(search_toolbar(search.into(), actions.into()));
            if let Some(management) = management {
                content = content
                    .push(settings_group_header(text(tr(locale, "全局 MCP 服务器")).size(16)))
                    .push(server_list(locale, entries, empty))
                    .push(management);
            } else {
                content = content.push(server_list(locale, entries, empty));
            }
        }
        McpSettingsTab::Runtime => {
            let servers = s.inspect.as_ref().map_or(&[][..], |report| report.servers.as_slice());
            let empty = if servers.is_empty() && s.inspect_loading {
                "预览中..."
            } else if s.inspect.is_none() {
                "尚未加载 MCP 预览。"
            } else if servers.is_empty() {
                "当前运行时没有 MCP 服务器。"
            } else {
                "没有匹配的结果"
            };
            let entries =
                servers.iter().filter(|server| matches_name(&server.name)).map(|server| {
                    let preview_key = format!("runtime:{}", server.name);
                    let preview_expanded = s.expanded_previews.contains(&preview_key);
                    let previewing = s.connecting.as_ref() == Some(&preview_key);
                    runtime_server_panel(
                        event,
                        locale,
                        server,
                        preview_key,
                        preview_expanded,
                        controls_enabled,
                        previewing,
                    )
                });
            content = content
                .push(search_toolbar(search.into(), inspect.into()))
                .push(server_list(locale, entries, empty));
        }
        McpSettingsTab::Json => {
            let edit_enabled = !s.loading && !s.saving && !s.transfer_pending;
            content = content.push(
                settings_group_card(
                    column![
                        row![refresh]
                            .extend(transfer_actions)
                            .spacing(8)
                            .align_y(Alignment::Center)
                            .wrap(),
                        settings_text_editor(&s.editor)
                            .on_action(move |action| event(McpMessage::EditorAction(action)))
                            .height(Length::Fixed(360.0)),
                        container(
                            row![
                                toolbar_button(
                                    tr(locale, "格式化"),
                                    edit_enabled.then(|| event(McpMessage::Format)),
                                ),
                                toolbar_button(
                                    tr(locale, "插入模板"),
                                    edit_enabled.then(|| event(McpMessage::InsertTemplate)),
                                ),
                                toolbar_button(tr(locale, save_label), save)
                                    .style(primary_action_btn_style),
                            ]
                            .spacing(8)
                            .align_y(Alignment::Center)
                            .wrap()
                            .align_x(iced::alignment::Horizontal::Right),
                        )
                        .align_right(Length::Fill),
                    ]
                    .spacing(12)
                    .width(Length::Fill),
                )
                .padding([14, 0]),
            );
        }
        McpSettingsTab::Extensions => {
            content = content.push(extensions_panel);
        }
    }

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
use server_form::server_form_toggle;
use server_panels::server_list;
