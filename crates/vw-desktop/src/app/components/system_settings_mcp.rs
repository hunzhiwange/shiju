//! 系统设置中 MCP 配置页面的界面拼装与交互消息转换。

use crate::app::components::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style, settings_checkbox_style, settings_divider,
    settings_error_banner, settings_muted_text_style, settings_page_intro, settings_panel,
    settings_pick_list_menu_style, settings_pick_list_style, settings_section_card,
    settings_segment_button_style, settings_success_banner, settings_text_editor_style,
    settings_text_input_style, settings_value_badge,
};
use crate::app::message::settings::{McpMessage, SettingsMessage};
use crate::app::state::{McpNewServerTransport, McpSettingsState, McpSettingsTab};
use crate::app::{App, Message};
use iced::widget::{
    button, checkbox, column, container, pick_list, row, text, text_editor, text_input,
};
use iced::{Alignment, Background, Border, Color, Element, Length};
use serde_json::Value;
use vw_gateway_client::vw_api_types::mcp::{
    McpConnectionStatusDto, McpInspectResponse, McpServerInspectDto,
};

#[derive(Debug, Clone)]
struct ConfiguredMcpServerRow {
    index: usize,
    name: String,
    transport: String,
    enabled: bool,
    required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum McpStatusTone {
    Connected,
    Error,
    Disabled,
    Pending,
}

fn toolbar_button<'a>(
    label: &'static str,
    message: Option<Message>,
) -> iced::widget::Button<'a, Message> {
    let mut button = button(text(label).size(13)).padding([7, 14]).style(rounded_action_btn_style);
    if let Some(message) = message {
        button = button.on_press(message);
    }
    button
}

fn form_field<'a>(
    label: &'static str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    column![text(label).size(11).style(settings_muted_text_style), control.into()]
        .spacing(6)
        .width(Length::Fill)
        .into()
}

fn mcp_tab_labels() -> &'static [(&'static str, McpSettingsTab)] {
    &[
        ("全局服务器", McpSettingsTab::Servers),
        ("运行时 MCP", McpSettingsTab::Runtime),
        ("高级 JSON", McpSettingsTab::Json),
    ]
}

fn mcp_tab_button(
    label: &'static str,
    tab: McpSettingsTab,
    active_tab: McpSettingsTab,
) -> Element<'static, Message> {
    let is_active = tab == active_tab;
    button(text(label).size(13))
        .padding([8, 14])
        .on_press(Message::Settings(SettingsMessage::Mcp(McpMessage::TabSelected(tab))))
        .style(move |theme: &iced::Theme, status| {
            settings_segment_button_style(theme, status, is_active)
        })
        .into()
}

fn compact_text(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let truncated = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() { format!("{truncated}...") } else { truncated }
}

fn configured_mcp_server_rows(editor_text: &str) -> Result<Vec<ConfiguredMcpServerRow>, String> {
    let trimmed = editor_text.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let value =
        serde_json::from_str::<serde_json::Value>(trimmed).map_err(|err| err.to_string())?;
    let servers = value.as_array().ok_or_else(|| "MCP 配置必须是数组".to_string())?;
    let mut rows = Vec::with_capacity(servers.len());
    for (index, server) in servers.iter().enumerate() {
        let object = server.as_object().ok_or_else(|| format!("mcpServers[{index}] 必须是对象"))?;
        let name = object
            .get("name")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("未命名")
            .to_string();
        let transport = object
            .get("type")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("stdio")
            .to_string();
        let enabled = object.get("enabled").and_then(serde_json::Value::as_bool).unwrap_or(true);
        let required = object.get("required").and_then(serde_json::Value::as_bool).unwrap_or(false);
        rows.push(ConfiguredMcpServerRow { index, name, transport, enabled, required });
    }
    Ok(rows)
}

fn inspect_for_server<'a>(
    inspect: Option<&'a McpInspectResponse>,
    name: &str,
) -> Option<&'a McpServerInspectDto> {
    inspect?.servers.iter().find(|server| server.name == name)
}

fn connection_status_label_and_tone(
    inspect: Option<&McpServerInspectDto>,
    enabled: bool,
) -> (&'static str, McpStatusTone) {
    match inspect.map(|entry| &entry.connection_status) {
        Some(McpConnectionStatusDto::Connected) => ("已连通", McpStatusTone::Connected),
        Some(McpConnectionStatusDto::Error) => ("连接错误", McpStatusTone::Error),
        Some(McpConnectionStatusDto::Disabled) => ("已禁用", McpStatusTone::Disabled),
        None if enabled => ("未预览", McpStatusTone::Pending),
        None => ("已禁用", McpStatusTone::Disabled),
    }
}

fn status_dot_color(theme: &iced::Theme, tone: McpStatusTone) -> Color {
    let palette = theme.extended_palette();
    match tone {
        McpStatusTone::Connected => palette.success.base.color,
        McpStatusTone::Error => palette.danger.base.color,
        McpStatusTone::Disabled => palette.background.strong.color,
        McpStatusTone::Pending => palette.warning.base.color,
    }
}

fn status_badge_style(theme: &iced::Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    iced::widget::container::Style {
        text_color: Some(theme.palette().text),
        background: Some(Background::Color(palette.background.weak.color.scale_alpha(0.36))),
        border: Border {
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(0.84),
            radius: 999.0.into(),
        },
        ..Default::default()
    }
}

fn status_badge(inspect: Option<&McpServerInspectDto>, enabled: bool) -> Element<'static, Message> {
    let (label, tone) = connection_status_label_and_tone(inspect, enabled);
    container(
        row![
            text("●").size(11).style(move |theme: &iced::Theme| {
                iced::widget::text::Style { color: Some(status_dot_color(theme, tone)) }
            }),
            text(label).size(13),
        ]
        .spacing(7)
        .align_y(Alignment::Center),
    )
    .padding([7, 12])
    .style(status_badge_style)
    .into()
}

fn value_field(value: &Value, keys: &[&str]) -> Option<String> {
    let object = value.as_object()?;
    keys.iter()
        .filter_map(|key| object.get(*key).and_then(Value::as_str))
        .map(str::trim)
        .find(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn preview_section_header(
    title: &'static str,
    count: usize,
    error: Option<&str>,
) -> Element<'static, Message> {
    let label = match error {
        Some(_) => format!("{title} · 错误"),
        None => format!("{title} · {count}"),
    };
    text(label).size(12).style(settings_muted_text_style).into()
}

fn tools_preview_section(inspect: &McpServerInspectDto) -> Element<'static, Message> {
    let mut content = column![preview_section_header(
        "工具",
        inspect.tools.len(),
        inspect.tools_error.as_deref()
    )]
    .spacing(8)
    .width(Length::Fill);

    if let Some(error) = &inspect.tools_error {
        return content
            .push(text(compact_text(error, 130)).size(11).style(settings_muted_text_style))
            .into();
    }

    if inspect.tools.is_empty() {
        return content.push(text("未发现工具").size(11).style(settings_muted_text_style)).into();
    }

    for tool in inspect.tools.iter().take(8) {
        content = content.push(
            row![
                text(compact_text(&tool.display_name, 42)).size(12).width(Length::Fixed(180.0)),
                text(compact_text(tool.id.as_ref(), 52))
                    .size(11)
                    .style(settings_muted_text_style)
                    .width(Length::Fixed(220.0)),
                text(compact_text(&tool.description, 90))
                    .size(11)
                    .style(settings_muted_text_style)
                    .width(Length::Fill),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        );
    }

    let hidden_count = inspect.tools.len().saturating_sub(8);
    if hidden_count > 0 {
        content = content.push(
            text(format!("另有 {hidden_count} 个工具未展开"))
                .size(11)
                .style(settings_muted_text_style),
        );
    }

    content.into()
}

fn value_preview_section(
    title: &'static str,
    items: &[Value],
    error: Option<&str>,
    primary_keys: &[&str],
    secondary_keys: &[&str],
) -> Element<'static, Message> {
    let mut content =
        column![preview_section_header(title, items.len(), error)].spacing(8).width(Length::Fill);

    if let Some(error) = error {
        return content
            .push(text(compact_text(error, 130)).size(11).style(settings_muted_text_style))
            .into();
    }

    if items.is_empty() {
        return content.push(text("未发现条目").size(11).style(settings_muted_text_style)).into();
    }

    for item in items.iter().take(6) {
        let primary = value_field(item, primary_keys).unwrap_or_else(|| "未命名".to_string());
        let secondary = value_field(item, secondary_keys).unwrap_or_default();
        content = content.push(
            row![
                text(compact_text(&primary, 48)).size(12).width(Length::Fixed(240.0)),
                text(compact_text(&secondary, 92))
                    .size(11)
                    .style(settings_muted_text_style)
                    .width(Length::Fill),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        );
    }

    let hidden_count = items.len().saturating_sub(6);
    if hidden_count > 0 {
        content = content.push(
            text(format!("另有 {hidden_count} 个条目未展开"))
                .size(11)
                .style(settings_muted_text_style),
        );
    }

    content.into()
}

fn preview_action_message(enabled: bool) -> Option<Message> {
    enabled.then(|| Message::Settings(SettingsMessage::Mcp(McpMessage::InspectRefresh)))
}

fn empty_preview(preview_enabled: bool, previewing: bool) -> Element<'static, Message> {
    let label = if previewing { "预览中..." } else { "预览" };
    row![
        text("暂无预览结果").size(12).style(settings_muted_text_style).width(Length::Fill),
        toolbar_button(label, preview_action_message(preview_enabled)),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .into()
}

fn capability_preview(
    inspect: Option<&McpServerInspectDto>,
    preview_enabled: bool,
    previewing: bool,
) -> Element<'static, Message> {
    let Some(inspect) = inspect else {
        return empty_preview(preview_enabled, previewing);
    };

    if let Some(error) = &inspect.error {
        return text(compact_text(error, 140)).size(12).style(settings_muted_text_style).into();
    }

    if inspect.connection_status == McpConnectionStatusDto::Disabled {
        return text("服务器已禁用").size(12).style(settings_muted_text_style).into();
    }

    column![
        tools_preview_section(inspect),
        settings_divider(),
        value_preview_section(
            "资源",
            &inspect.resources,
            inspect.resources_error.as_deref(),
            &["name", "title", "uri"],
            &["uri", "description", "mimeType"]
        ),
        settings_divider(),
        value_preview_section(
            "资源模板",
            &inspect.resource_templates,
            inspect.resource_templates_error.as_deref(),
            &["name", "title", "uriTemplate"],
            &["uriTemplate", "description", "mimeType"]
        ),
        settings_divider(),
        value_preview_section(
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

fn configured_server_panel<'a>(
    server: ConfiguredMcpServerRow,
    inspect: Option<&'a McpServerInspectDto>,
    preview_key: String,
    preview_expanded: bool,
    controls_enabled: bool,
    preview_enabled: bool,
    previewing: bool,
) -> Element<'a, Message> {
    let checkbox_label = if server.enabled { "启用" } else { "禁用" };
    let server_index = server.index;
    let enabled_checkbox = checkbox(server.enabled)
        .label(checkbox_label)
        .style(settings_checkbox_style)
        .width(Length::Fixed(86.0));
    let enabled_checkbox = if controls_enabled {
        enabled_checkbox.on_toggle(move |value| {
            Message::Settings(SettingsMessage::Mcp(McpMessage::ServerEnabledToggled(
                server_index,
                value,
            )))
        })
    } else {
        enabled_checkbox
    };

    let mut badges = row![
        settings_value_badge(server.transport.clone()),
        status_badge(inspect, server.enabled),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    if server.required {
        badges = badges.push(settings_value_badge("必需"));
    }
    if let Some(count) = inspect.and_then(|entry| entry.tool_count) {
        badges = badges.push(settings_value_badge(format!("{count} 个工具")));
    }
    if let Some(count) = inspect.and_then(|entry| entry.resource_count) {
        badges = badges.push(settings_value_badge(format!("{count} 个资源")));
    }
    if let Some(count) = inspect.and_then(|entry| entry.prompt_count) {
        badges = badges.push(settings_value_badge(format!("{count} 个 prompts")));
    }

    let preview_label = if previewing { "预览中..." } else { "预览" };
    let preview_toggle_label = if preview_expanded { "收起" } else { "展开" };
    let preview_toggle_message = Some(Message::Settings(SettingsMessage::Mcp(
        McpMessage::PreviewExpandedToggled(preview_key),
    )));
    let edit_message = controls_enabled.then(|| {
        Message::Settings(SettingsMessage::Mcp(McpMessage::ServerEditRequested(server_index)))
    });
    let delete_message = controls_enabled.then(|| {
        Message::Settings(SettingsMessage::Mcp(McpMessage::RequestDelete(server_index)))
    });

    let mut content = column![
        row![
            enabled_checkbox,
            column![
                text(server.name.clone()).size(14),
                text(format!("mcpServers[{}]", server.index))
                    .size(11)
                    .style(settings_muted_text_style),
            ]
            .spacing(4)
            .width(Length::Fill),
            badges,
            toolbar_button("编辑", edit_message),
            toolbar_button("删除", delete_message),
            toolbar_button(preview_label, preview_action_message(preview_enabled)),
            toolbar_button(preview_toggle_label, preview_toggle_message),
        ]
        .spacing(16)
        .align_y(Alignment::Center)
    ]
    .spacing(12);
    if preview_expanded {
        content = content.push(settings_divider()).push(capability_preview(
            inspect,
            preview_enabled,
            previewing,
        ));
    }

    settings_panel(content).into()
}

fn new_server_panel_header(expanded: bool, editing: bool) -> Element<'static, Message> {
    let toggle_label = if expanded { "收起" } else { "展开" };
    let title = if editing { "编辑 MCP 服务器" } else { "新增 MCP 服务器" };
    let description = if editing {
        "修改当前服务器；高级 JSON 字段会在传输类型不变时保留。"
    } else {
        "按服务器逐项添加；复杂字段可继续在高级 JSON 中编辑。"
    };
    row![
        column![text(title).size(14), text(description).size(11).style(settings_muted_text_style),]
            .spacing(4)
            .width(Length::Fill),
        settings_value_badge("全局配置"),
        toolbar_button(
            toggle_label,
            Some(Message::Settings(SettingsMessage::Mcp(McpMessage::NewServerPanelToggled,))),
        ),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .into()
}

fn new_server_panel(s: &McpSettingsState, controls_enabled: bool) -> Element<'_, Message> {
    let editing = s.editing_server_index.is_some();
    if !s.new_server_expanded {
        return settings_panel(column![new_server_panel_header(false, editing)].spacing(12)).into();
    }

    let input_style = |placeholder, value: &String, on_input: fn(String) -> McpMessage| {
        text_input(placeholder, value)
            .on_input(move |next| Message::Settings(SettingsMessage::Mcp(on_input(next))))
            .padding([10, 12])
            .size(13)
            .style(settings_text_input_style)
            .width(Length::Fill)
    };

    let name_input = input_style("例如 local-filesystem", &s.new_server.name_input, |value| {
        McpMessage::NewServerNameChanged(value)
    });
    let timeout_input = input_style("30", &s.new_server.timeout_input, |value| {
        McpMessage::NewServerTimeoutChanged(value)
    });

    let transport_pick =
        pick_list(McpNewServerTransport::ALL, Some(s.new_server.transport), |value| {
            Message::Settings(SettingsMessage::Mcp(McpMessage::NewServerTransportChanged(value)))
        })
        .padding([10, 14])
        .text_size(13)
        .style(settings_pick_list_style)
        .menu_style(settings_pick_list_menu_style)
        .width(Length::Fill);

    let detail_fields: Element<'_, Message> = match s.new_server.transport {
        McpNewServerTransport::Stdio => {
            let command_input =
                input_style("npx / uvx / python", &s.new_server.command_input, |value| {
                    McpMessage::NewServerCommandChanged(value)
                });
            let args_input = input_style(
                "-y, @modelcontextprotocol/server-filesystem, /tmp",
                &s.new_server.args_input,
                |value| McpMessage::NewServerArgsChanged(value),
            );
            row![
                container(form_field("命令", command_input)).width(Length::FillPortion(1)),
                container(form_field("参数", args_input)).width(Length::FillPortion(2)),
            ]
            .spacing(12)
            .into()
        }
        McpNewServerTransport::Http | McpNewServerTransport::Sse => {
            let url_input =
                input_style("https://example.com/mcp", &s.new_server.url_input, |value| {
                    McpMessage::NewServerUrlChanged(value)
                });
            row![container(form_field("URL", url_input)).width(Length::Fill)].into()
        }
    };

    let submit_label = if editing { "保存修改" } else { "新增服务器" };
    let submit_message = if editing { McpMessage::SaveServerEdit } else { McpMessage::AddServer };
    let mut add_button =
        button(text(submit_label).size(13)).padding([8, 16]).style(primary_action_btn_style);
    if controls_enabled {
        add_button = add_button.on_press(Message::Settings(SettingsMessage::Mcp(submit_message)));
    }
    let cancel_button = editing.then(|| {
        toolbar_button(
            "取消编辑",
            controls_enabled
                .then(|| Message::Settings(SettingsMessage::Mcp(McpMessage::ServerEditCanceled))),
        )
    });

    let mut actions = row![
        checkbox(s.new_server.enabled)
            .label("启用")
            .on_toggle(|value| {
                Message::Settings(SettingsMessage::Mcp(McpMessage::NewServerEnabledToggled(value)))
            })
            .style(settings_checkbox_style),
        checkbox(s.new_server.required)
            .label("必需")
            .on_toggle(|value| {
                Message::Settings(SettingsMessage::Mcp(McpMessage::NewServerRequiredToggled(value)))
            })
            .style(settings_checkbox_style),
        add_button,
    ]
    .spacing(16)
    .align_y(Alignment::Center);
    if let Some(cancel_button) = cancel_button {
        actions = actions.push(cancel_button);
    }

    settings_panel(
        column![
            new_server_panel_header(true, editing),
            settings_divider(),
            row![
                container(form_field("名称", name_input)).width(Length::FillPortion(2)),
                container(form_field("类型", transport_pick)).width(Length::FillPortion(1)),
                container(form_field("超时", timeout_input)).width(Length::FillPortion(1)),
            ]
            .spacing(12),
            detail_fields,
            actions,
        ]
        .spacing(12),
    )
    .into()
}

fn runtime_server_panel(
    server: &McpServerInspectDto,
    preview_key: String,
    preview_expanded: bool,
    controls_enabled: bool,
    preview_enabled: bool,
    previewing: bool,
) -> Element<'_, Message> {
    let checkbox_label = if server.enabled { "启用" } else { "禁用" };
    let server_name = server.name.clone();
    let enabled_checkbox = checkbox(server.enabled)
        .label(checkbox_label)
        .style(settings_checkbox_style)
        .width(Length::Fixed(86.0));
    let enabled_checkbox = if controls_enabled {
        enabled_checkbox.on_toggle(move |value| {
            Message::Settings(SettingsMessage::Mcp(McpMessage::RuntimeServerEnabledToggled(
                server_name.clone(),
                value,
            )))
        })
    } else {
        enabled_checkbox
    };

    let mut badges = row![
        settings_value_badge(server.transport.clone()),
        status_badge(Some(server), server.enabled),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    if server.required {
        badges = badges.push(settings_value_badge("必需"));
    }
    if let Some(count) = server.tool_count {
        badges = badges.push(settings_value_badge(format!("{count} 个工具")));
    }
    if let Some(count) = server.resource_count {
        badges = badges.push(settings_value_badge(format!("{count} 个资源")));
    }
    if let Some(count) = server.prompt_count {
        badges = badges.push(settings_value_badge(format!("{count} 个 prompts")));
    }

    let preview_label = if previewing { "预览中..." } else { "预览" };
    let preview_toggle_label = if preview_expanded { "收起" } else { "展开" };
    let preview_toggle_message = Some(Message::Settings(SettingsMessage::Mcp(
        McpMessage::PreviewExpandedToggled(preview_key),
    )));

    let mut content = column![
        row![
            enabled_checkbox,
            column![
                text(server.name.as_str()).size(14),
                text("插件或项目级 MCP").size(11).style(settings_muted_text_style),
            ]
            .spacing(4)
            .width(Length::Fill),
            badges,
            toolbar_button(preview_label, preview_action_message(preview_enabled)),
            toolbar_button(preview_toggle_label, preview_toggle_message),
        ]
        .spacing(16)
        .align_y(Alignment::Center)
    ]
    .spacing(12);
    if preview_expanded {
        content = content.push(settings_divider()).push(capability_preview(
            Some(server),
            preview_enabled,
            previewing,
        ));
    }

    settings_panel(content).into()
}

/// 构建 MCP 系统设置页。
pub fn view(app: &App) -> Element<'_, Message> {
    let s = &app.mcp_settings;
    let refresh_label = if s.loading { "刷新中..." } else { "刷新" };
    let inspect_label = if s.inspect_loading { "预览中..." } else { "刷新预览" };
    let save_label = if s.saving { "保存中..." } else { "保存" };

    let refresh_message = (!s.loading && !s.saving)
        .then(|| Message::Settings(SettingsMessage::Mcp(McpMessage::Refresh)));
    let inspect_message = (!s.inspect_loading && !s.saving)
        .then(|| Message::Settings(SettingsMessage::Mcp(McpMessage::InspectRefresh)));
    let format_message = (!s.loading && !s.saving)
        .then(|| Message::Settings(SettingsMessage::Mcp(McpMessage::Format)));
    let template_message = (!s.loading && !s.saving)
        .then(|| Message::Settings(SettingsMessage::Mcp(McpMessage::InsertTemplate)));
    let save_message = (!s.loading && !s.saving)
        .then(|| Message::Settings(SettingsMessage::Mcp(McpMessage::Save)));

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
            .push(toolbar_button("格式化", format_message))
            .push(toolbar_button("插入模板", template_message));
    }
    action_row = action_row.push(save_button);

    let mut content = column![
        row![
            container(settings_page_intro(
                "MCP 配置",
                "配置全局 mcpServers；插件声明的 MCP 会随插件启用状态在运行时合并。"
            ))
            .width(Length::Fill),
            action_row,
        ]
        .align_y(Alignment::Start)
        .spacing(12),
        row![
            settings_value_badge(format!("{} 个服务器", s.server_count)),
            settings_value_badge("全局配置"),
            settings_value_badge(format!(
                "{} 个运行时服务器",
                s.inspect.as_ref().map(|inspect| inspect.servers.len()).unwrap_or(0)
            )),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        mcp_tab_labels()
            .iter()
            .fold(row![].spacing(8).align_y(Alignment::Center), |row, (label, tab)| {
                row.push(mcp_tab_button(label, *tab, s.active_tab))
            }),
    ]
    .spacing(16)
    .width(Length::Fill);

    if let Some(message) = &s.status_message {
        content = content.push(settings_success_banner(message));
    }
    if let Some(error) = &s.save_error {
        content = content.push(settings_error_banner(error));
    }
    if let Some(error) = &s.inspect_error {
        content = content.push(settings_panel(
            text(format!("MCP 预览失败: {}", compact_text(error, 180)))
                .size(12)
                .style(settings_muted_text_style),
        ));
    }

    let configured_rows = configured_mcp_server_rows(&s.editor.text()).unwrap_or_default();
    match s.active_tab {
        McpSettingsTab::Servers => {
            content = content.push(settings_section_card(
                "全局 MCP 服务器",
                "逐条启用、禁用和预览全局 mcpServers。",
            ));
            content = content.push(new_server_panel(s, !s.loading && !s.saving));
            if configured_rows.is_empty() {
                content = content.push(settings_panel(
                    text("当前没有全局 MCP 服务器。").size(12).style(settings_muted_text_style),
                ));
            } else {
                let controls_enabled = !s.loading && !s.saving;
                let preview_enabled = !s.loading && !s.saving && !s.inspect_loading;
                let inspect = s.inspect.as_ref();
                for server in configured_rows.iter().cloned() {
                    let preview_key = format!("global:{}", server.name);
                    let preview_expanded = s.expanded_previews.contains(&preview_key);
                    let inspect_entry = inspect_for_server(inspect, &server.name);
                    content = content.push(configured_server_panel(
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
            content = content.push(settings_section_card(
                "运行时 MCP",
                "展示项目配置和插件声明合并后的 MCP 连接、工具、资源与 prompts。",
            ));
            if let Some(inspect) = s.inspect.as_ref() {
                if inspect.servers.is_empty() {
                    content = content.push(settings_panel(
                        text("当前运行时没有 MCP 服务器。")
                            .size(12)
                            .style(settings_muted_text_style),
                    ));
                } else {
                    let controls_enabled = !s.loading && !s.saving;
                    let preview_enabled = !s.loading && !s.saving && !s.inspect_loading;
                    for server in &inspect.servers {
                        let preview_key = format!("runtime:{}", server.name);
                        let preview_expanded = s.expanded_previews.contains(&preview_key);
                        content = content.push(runtime_server_panel(
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
                    text("尚未加载 MCP 预览。").size(12).style(settings_muted_text_style),
                ));
            }
        }
        McpSettingsTab::Json => {
            content = content.push(settings_section_card(
                "高级 JSON",
                "新增服务器或编辑 headers、env、allowedTools、deniedTools 等字段。",
            ));
            content = content.push(settings_panel(
                column![
                    text_editor(&s.editor)
                        .placeholder(
                            r#"[
  {
    "name": "local-demo",
    "type": "stdio",
    "command": "npx",
    "args": ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
  }
]"#
                        )
                        .on_action(|action| {
                            Message::Settings(SettingsMessage::Mcp(McpMessage::EditorAction(
                                action,
                            )))
                        })
                        .padding([10, 12])
                        .height(Length::Fixed(360.0))
                        .style(settings_text_editor_style),
                    text(
                        "支持 type=stdio/http/sse；可配置 headers、env、enabled、required、timeout、allowedTools、deniedTools。"
                    )
                    .size(11)
                    .style(settings_muted_text_style),
                ]
                .spacing(10),
            ));
        }
    }

    content.into()
}

/// 在 MCP 设置页上叠加统一的删除确认模态框。
pub fn view_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    if app.mcp_settings.active_tab != McpSettingsTab::Servers {
        return dialog;
    }

    let Some(index) = app.mcp_settings.delete_pending else {
        return dialog;
    };
    let server_name = configured_mcp_server_rows(&app.mcp_settings.editor.text())
        .ok()
        .and_then(|rows| rows.get(index).map(|row| row.name.clone()))
        .unwrap_or_else(|| format!("mcpServers[{index}]"));
    let confirm = Message::Settings(SettingsMessage::Mcp(McpMessage::ConfirmDelete(index)));
    let cancel = Message::Settings(SettingsMessage::Mcp(McpMessage::CancelDelete));
    let confirm_dialog = crate::app::components::toast::confirm_dialog(
        "确认删除 MCP 服务器",
        format!("将删除 MCP 服务器「{server_name}」。此操作不可撤销。"),
        "确认删除",
        "取消",
        confirm,
        cancel,
    );

    iced::widget::stack![dialog, confirm_dialog].into()
}

#[cfg(test)]
#[path = "system_settings_mcp_tests.rs"]
mod system_settings_mcp_tests;
