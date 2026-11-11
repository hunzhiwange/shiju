//! 系统设置中 MCP 配置页面的界面拼装与交互消息转换。

use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, primary_action_btn_style,
    rounded_action_btn_style, settings_checkbox_style, settings_close_button, settings_divider,
    settings_error_banner, settings_field_label, settings_group_card, settings_hover_help_label,
    settings_modal_card_style, settings_modal_header, settings_modal_layout,
    settings_modal_overlay, settings_muted_text_style, settings_page_heading_with_help,
    settings_panel, settings_select, settings_success_banner, settings_tab_button,
    settings_tab_row, settings_text_editor_style, settings_text_input, settings_value_badge,
};
use crate::app::components::system_settings_search::SettingsTabDescriptor;
use crate::app::i18n::{I18n, settings_extensions::text as tr};
use crate::app::message::settings::{McpMessage, SettingsMessage};
use crate::app::state::{McpNewServerTransport, McpSettingsState, McpSettingsTab};
use crate::app::{App, Message};
use iced::widget::{Space, button, checkbox, column, container, row, text, text_editor};
use iced::{Alignment, Background, Border, Color, Element, Length};
use serde_json::Value;
use vw_config_types::ui::DesktopLocale;
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
    column![settings_field_label(label), control.into()].spacing(6).width(Length::Fill).into()
}

const MCP_TAB_LABELS: &[(&str, McpSettingsTab)] = &[
    ("全局服务器", McpSettingsTab::Servers),
    ("运行时 MCP", McpSettingsTab::Runtime),
    ("高级 JSON", McpSettingsTab::Json),
];
pub(super) const MCP_TABS: &[SettingsTabDescriptor<McpSettingsTab>] = &[
    SettingsTabDescriptor::new(
        MCP_TAB_LABELS[0].1,
        MCP_TAB_LABELS[0].0,
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
        MCP_TAB_LABELS[1].1,
        MCP_TAB_LABELS[1].0,
        &["runtime", "运行时覆盖", "inspect", "工具预览"],
    ),
    SettingsTabDescriptor::new(
        MCP_TAB_LABELS[2].1,
        MCP_TAB_LABELS[2].0,
        &["json", "原始配置", "高级编辑"],
    ),
];

fn mcp_tab_labels() -> &'static [(&'static str, McpSettingsTab)] {
    MCP_TAB_LABELS
}

fn mcp_tab_button(
    locale: DesktopLocale,
    label: &'static str,
    tab: McpSettingsTab,
    active_tab: McpSettingsTab,
) -> Element<'static, Message> {
    settings_tab_button(
        tr(locale, label),
        tab == active_tab,
        Some(Message::Settings(SettingsMessage::Mcp(McpMessage::TabSelected(tab)))),
    )
    .into()
}

fn compact_text(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let truncated = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() { format!("{truncated}...") } else { truncated }
}

fn configured_mcp_server_rows(
    locale: DesktopLocale,
    editor_text: &str,
) -> Result<Vec<ConfiguredMcpServerRow>, String> {
    let trimmed = editor_text.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let value =
        serde_json::from_str::<serde_json::Value>(trimmed).map_err(|err| err.to_string())?;
    let servers = value.as_array().ok_or_else(|| tr(locale, "MCP 配置必须是数组").to_string())?;
    let mut rows = Vec::with_capacity(servers.len());
    for (index, server) in servers.iter().enumerate() {
        let object = server.as_object().ok_or_else(|| {
            crate::__!(crate::app::i18n::I18n::new(locale), "mcpServers[%u] 必须是对象", index)
                .expect("valid MCP row error template")
        })?;
        let name = object
            .get("name")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(tr(locale, "未命名"))
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

fn status_badge(
    locale: DesktopLocale,
    inspect: Option<&McpServerInspectDto>,
    enabled: bool,
) -> Element<'static, Message> {
    let (label, tone) = connection_status_label_and_tone(inspect, enabled);
    container(
        row![
            text("●").size(11).style(move |theme: &iced::Theme| {
                iced::widget::text::Style { color: Some(status_dot_color(theme, tone)) }
            }),
            text(tr(locale, label)).size(13),
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
    locale: DesktopLocale,
    title: &'static str,
    count: usize,
    error: Option<&str>,
) -> Element<'static, Message> {
    let label = match error {
        Some(_) => crate::__!(crate::app::i18n::I18n::new(locale), "%s · 错误", title)
            .expect("valid MCP template"),
        None => crate::__!(crate::app::i18n::I18n::new(locale), "%s · %u", title, count)
            .expect("valid MCP template"),
    };
    text(label).size(12).style(settings_muted_text_style).into()
}

fn tools_preview_section(
    locale: DesktopLocale,
    inspect: &McpServerInspectDto,
) -> Element<'static, Message> {
    let mut content = column![preview_section_header(
        locale,
        tr(locale, "工具"),
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
        return content
            .push(text(tr(locale, "未发现工具")).size(11).style(settings_muted_text_style))
            .into();
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
            text(
                crate::__!(
                    crate::app::i18n::I18n::new(locale),
                    "另有 %u 个工具未展开",
                    hidden_count
                )
                .expect("valid MCP template"),
            )
            .size(11)
            .style(settings_muted_text_style),
        );
    }

    content.into()
}

fn value_preview_section(
    locale: DesktopLocale,
    title: &'static str,
    items: &[Value],
    error: Option<&str>,
    primary_keys: &[&str],
    secondary_keys: &[&str],
) -> Element<'static, Message> {
    let mut content = column![preview_section_header(locale, title, items.len(), error)]
        .spacing(8)
        .width(Length::Fill);

    if let Some(error) = error {
        return content
            .push(text(compact_text(error, 130)).size(11).style(settings_muted_text_style))
            .into();
    }

    if items.is_empty() {
        return content
            .push(text(tr(locale, "未发现条目")).size(11).style(settings_muted_text_style))
            .into();
    }

    for item in items.iter().take(6) {
        let primary =
            value_field(item, primary_keys).unwrap_or_else(|| tr(locale, "未命名").to_string());
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
            text(
                crate::__!(
                    crate::app::i18n::I18n::new(locale),
                    "另有 %u 个条目未展开",
                    hidden_count
                )
                .expect("valid MCP template"),
            )
            .size(11)
            .style(settings_muted_text_style),
        );
    }

    content.into()
}

fn preview_action_message(enabled: bool) -> Option<Message> {
    enabled.then(|| Message::Settings(SettingsMessage::Mcp(McpMessage::InspectRefresh)))
}

fn empty_preview(
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
        toolbar_button(label, preview_action_message(preview_enabled)),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .into()
}

fn capability_preview(
    locale: DesktopLocale,
    inspect: Option<&McpServerInspectDto>,
    preview_enabled: bool,
    previewing: bool,
) -> Element<'static, Message> {
    let Some(inspect) = inspect else {
        return empty_preview(locale, preview_enabled, previewing);
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
    let checkbox_label = if server.enabled { tr(locale, "启用") } else { tr(locale, "禁用") };
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
        status_badge(locale, inspect, server.enabled),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    if server.required {
        badges = badges.push(settings_value_badge(tr(locale, "必需")));
    }
    if let Some(count) = inspect.and_then(|entry| entry.tool_count) {
        badges = badges.push(settings_value_badge(
            crate::__!(crate::app::i18n::I18n::new(locale), "%u 个工具", count)
                .expect("valid MCP template"),
        ));
    }
    if let Some(count) = inspect.and_then(|entry| entry.resource_count) {
        badges = badges.push(settings_value_badge(
            crate::__!(crate::app::i18n::I18n::new(locale), "%u 个资源", count)
                .expect("valid MCP template"),
        ));
    }
    if let Some(count) = inspect.and_then(|entry| entry.prompt_count) {
        badges = badges.push(settings_value_badge(
            crate::__!(crate::app::i18n::I18n::new(locale), "%u 个 prompts", count)
                .expect("valid MCP template"),
        ));
    }

    let preview_label = if previewing { tr(locale, "预览中...") } else { tr(locale, "预览") };
    let preview_toggle_label =
        if preview_expanded { tr(locale, "收起") } else { tr(locale, "展开") };
    let preview_toggle_message = Some(Message::Settings(SettingsMessage::Mcp(
        McpMessage::PreviewExpandedToggled(preview_key),
    )));
    let edit_message = controls_enabled.then(|| {
        Message::Settings(SettingsMessage::Mcp(McpMessage::ServerEditRequested(server_index)))
    });
    let delete_message = controls_enabled
        .then(|| Message::Settings(SettingsMessage::Mcp(McpMessage::RequestDelete(server_index))));

    let mut content = column![
        column![
            text(server.name).size(16),
            text(format!("mcpServers[{}]", server.index)).size(12).style(settings_muted_text_style),
        ]
        .spacing(4),
        badges.wrap(),
        row![
            enabled_checkbox,
            toolbar_button(tr(locale, "编辑"), edit_message),
            toolbar_button(tr(locale, "删除"), delete_message),
            toolbar_button(preview_label, preview_action_message(preview_enabled)),
            toolbar_button(preview_toggle_label, preview_toggle_message),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .wrap(),
    ]
    .spacing(12);
    if preview_expanded {
        content = content.push(settings_divider()).push(capability_preview(
            locale,
            inspect,
            preview_enabled,
            previewing,
        ));
    }

    settings_panel(content).into()
}

fn server_form_heading(locale: DesktopLocale, editing: bool) -> (&'static str, &'static str) {
    if editing {
        (
            tr(locale, "编辑 MCP 服务器"),
            tr(locale, "修改当前服务器；高级 JSON 字段会在传输类型不变时保留。"),
        )
    } else {
        (
            tr(locale, "新增 MCP 服务器"),
            tr(locale, "按服务器逐项添加；复杂字段可继续在高级 JSON 中编辑。"),
        )
    }
}

fn server_form_toggle() -> Message {
    Message::Settings(SettingsMessage::Mcp(McpMessage::NewServerPanelToggled))
}

fn new_server_panel(app: &App) -> Element<'_, Message> {
    let locale = app.desktop_locale;
    let i18n = I18n::new(locale);
    let s = &app.mcp_settings;
    let editing = s.editing_server_index.is_some();
    let (title, description) = server_form_heading(locale, editing);
    let controls_enabled = !s.loading && !s.saving;
    let input = |placeholder, value: &str, on_input: fn(String) -> McpMessage| {
        settings_text_input(placeholder, value)
            .on_input(move |next| Message::Settings(SettingsMessage::Mcp(on_input(next))))
    };

    let name_input = input(
        tr(locale, "例如 local-filesystem"),
        &s.new_server.name_input,
        McpMessage::NewServerNameChanged,
    );
    let timeout_input =
        input("30", &s.new_server.timeout_input, McpMessage::NewServerTimeoutChanged);
    let transport =
        settings_select(McpNewServerTransport::ALL, Some(s.new_server.transport), |value| {
            Message::Settings(SettingsMessage::Mcp(McpMessage::NewServerTransportChanged(value)))
        })
        .width(Length::Fill);
    let transport_label = tr(locale, "类型");
    let mut transport_field = column![
        settings_hover_help_label(
            i18n,
            &app.app_theme,
            transport_label,
            settings_field_label(transport_label),
            format!("stdio · {} / {}\nHTTP / SSE · URL", tr(locale, "命令"), tr(locale, "参数")),
        )
        .wrap(),
        transport,
    ]
    .spacing(6)
    .width(Length::Fill);
    if editing {
        transport_field = transport_field.push(
            text(tr(locale, "更改类型并保存会清除原有的 env 或 headers。"))
                .size(12)
                .style(settings_muted_text_style),
        );
    }
    let mut fields =
        column![form_field(tr(locale, "名称"), name_input), transport_field].spacing(12);
    match s.new_server.transport {
        McpNewServerTransport::Stdio => {
            fields = fields
                .push(form_field(
                    tr(locale, "命令"),
                    input(
                        "npx / uvx / python",
                        &s.new_server.command_input,
                        McpMessage::NewServerCommandChanged,
                    ),
                ))
                .push(form_field(
                    tr(locale, "参数"),
                    input(
                        "-y, @modelcontextprotocol/server-filesystem, /tmp",
                        &s.new_server.args_input,
                        McpMessage::NewServerArgsChanged,
                    ),
                ));
        }
        McpNewServerTransport::Http | McpNewServerTransport::Sse => {
            fields = fields.push(form_field(
                "URL",
                input(
                    "https://example.com/mcp",
                    &s.new_server.url_input,
                    McpMessage::NewServerUrlChanged,
                ),
            ));
        }
    }
    fields = fields.push(form_field(tr(locale, "超时"), timeout_input)).push(
        row![
            checkbox(s.new_server.enabled)
                .label(tr(locale, "启用"))
                .on_toggle(|value| {
                    Message::Settings(SettingsMessage::Mcp(McpMessage::NewServerEnabledToggled(
                        value,
                    )))
                })
                .style(settings_checkbox_style),
            checkbox(s.new_server.required)
                .label(tr(locale, "必需"))
                .on_toggle(|value| {
                    Message::Settings(SettingsMessage::Mcp(McpMessage::NewServerRequiredToggled(
                        value,
                    )))
                })
                .style(settings_checkbox_style),
        ]
        .spacing(16)
        .align_y(Alignment::Center)
        .wrap(),
    );
    let mut body = column![].spacing(12);
    if let Some(error) = server_save_error(locale, s) {
        body = body.push(settings_error_banner(error));
    }
    body = body.push(settings_panel(fields));

    let heading =
        settings_hover_help_label(i18n, &app.app_theme, title, text(title).size(20), description)
            .wrap();
    let header = settings_modal_header(heading, settings_close_button(server_form_toggle()));
    let submit_label =
        if editing { tr(locale, "保存修改") } else { tr(locale, "新增服务器") };
    let submit_message = if editing { McpMessage::SaveServerEdit } else { McpMessage::AddServer };
    let mut footer = row![Space::new().width(Length::Fill)];
    if editing {
        footer = footer.push(toolbar_button(
            tr(locale, "取消编辑"),
            controls_enabled
                .then(|| Message::Settings(SettingsMessage::Mcp(McpMessage::ServerEditCanceled))),
        ));
    }
    let footer = footer
        .push(toolbar_button(tr(locale, "收起"), Some(server_form_toggle())))
        .push(
            button(text(submit_label).size(13))
                .padding([8, 16])
                .style(primary_action_btn_style)
                .on_press_maybe(
                    controls_enabled
                        .then(|| Message::Settings(SettingsMessage::Mcp(submit_message))),
                ),
        )
        .spacing(8)
        .align_y(Alignment::Center);
    settings_modal_layout(header, body, Some(footer.into()))
        .style(|theme| {
            let mut style = settings_modal_card_style(theme);
            if let Some(iced::Background::Color(color)) = &mut style.background {
                color.a = 1.0;
            }
            style
        })
        .width(640)
        .height(590)
        .into()
}

fn runtime_server_panel(
    locale: DesktopLocale,
    server: &McpServerInspectDto,
    preview_key: String,
    preview_expanded: bool,
    controls_enabled: bool,
    preview_enabled: bool,
    previewing: bool,
) -> Element<'_, Message> {
    let checkbox_label = if server.enabled { tr(locale, "启用") } else { tr(locale, "禁用") };
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
        status_badge(locale, Some(server), server.enabled),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    if server.required {
        badges = badges.push(settings_value_badge(tr(locale, "必需")));
    }
    if let Some(count) = server.tool_count {
        badges = badges.push(settings_value_badge(
            crate::__!(crate::app::i18n::I18n::new(locale), "%u 个工具", count)
                .expect("valid MCP template"),
        ));
    }
    if let Some(count) = server.resource_count {
        badges = badges.push(settings_value_badge(
            crate::__!(crate::app::i18n::I18n::new(locale), "%u 个资源", count)
                .expect("valid MCP template"),
        ));
    }
    if let Some(count) = server.prompt_count {
        badges = badges.push(settings_value_badge(
            crate::__!(crate::app::i18n::I18n::new(locale), "%u 个 prompts", count)
                .expect("valid MCP template"),
        ));
    }

    let preview_label = if previewing { tr(locale, "预览中...") } else { tr(locale, "预览") };
    let preview_toggle_label =
        if preview_expanded { tr(locale, "收起") } else { tr(locale, "展开") };
    let preview_toggle_message = Some(Message::Settings(SettingsMessage::Mcp(
        McpMessage::PreviewExpandedToggled(preview_key),
    )));

    let mut content = column![
        column![
            text(server.name.as_str()).size(16),
            text(tr(locale, "插件或项目级 MCP")).size(12).style(settings_muted_text_style),
        ]
        .spacing(4),
        badges.wrap(),
        row![
            enabled_checkbox,
            toolbar_button(preview_label, preview_action_message(preview_enabled)),
            toolbar_button(preview_toggle_label, preview_toggle_message),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .wrap(),
    ]
    .spacing(12);
    if preview_expanded {
        content = content.push(settings_divider()).push(capability_preview(
            locale,
            Some(server),
            preview_enabled,
            previewing,
        ));
    }

    settings_panel(content).into()
}

fn server_save_error(locale: DesktopLocale, s: &McpSettingsState) -> Option<String> {
    s.save_error.as_ref().map(|error| {
        s.save_error_translation
            .as_ref()
            .filter(|message| {
                message
                    .render(crate::app::i18n::I18n::new(DesktopLocale::SimplifiedChinese))
                    .is_ok_and(|source| source == *error)
            })
            .map(|message| {
                message
                    .render(crate::app::i18n::I18n::new(locale))
                    .expect("valid MCP error template")
            })
            .unwrap_or_else(|| crate::app::i18n::I18n::new(locale).__(error).to_owned())
    })
}

/// 构建 MCP 系统设置页。
pub fn view(app: &App) -> Element<'_, Message> {
    let locale = app.desktop_locale;
    let s = &app.mcp_settings;
    let i18n = I18n::new(locale);
    let section = |title, description| {
        settings_group_card(
            settings_hover_help_label(
                i18n,
                &app.app_theme,
                title,
                text(title).size(14),
                description,
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
            .push(toolbar_button(tr(locale, "格式化"), format_message))
            .push(toolbar_button(tr(locale, "插入模板"), template_message));
    }
    action_row = action_row.push(save_button);

    let mut content = column![
        settings_page_heading_with_help(
            i18n,
            &app.app_theme,
            tr(locale, "MCP 配置"),
            tr(locale, "配置全局 mcpServers；插件声明的 MCP 会随插件启用状态在运行时合并。"),
        ),
        managed_catalog_panel(app),
        bindings::panel(app),
        crate::app::components::agent_apps_diagnostics::mcp_panel::view(app),
        action_row.wrap(),
        row![
            settings_value_badge(
                crate::__!(crate::app::i18n::I18n::new(locale), "%u 个服务器", s.server_count)
                    .expect("valid server count template")
            ),
            settings_value_badge(tr(locale, "全局配置")),
            settings_value_badge(
                crate::__!(
                    crate::app::i18n::I18n::new(locale),
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
                .map(|(label, tab)| { mcp_tab_button(locale, label, *tab, s.active_tab) })
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
                crate::__!(
                    crate::app::i18n::I18n::new(locale),
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
            content = content.push(toolbar_button(title, Some(server_form_toggle())));
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
                    content = content.push(configured_server_panel(
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
                        content = content.push(runtime_server_panel(
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
                        Message::Settings(SettingsMessage::Mcp(McpMessage::EditorAction(action)))
                    })
                    .padding(SETTINGS_CONTROL_PADDING)
                    .size(SETTINGS_CONTROL_TEXT_SIZE)
                    .height(Length::Fixed(360.0))
                    .style(settings_text_editor_style),
            ));
        }
    }

    container(content).padding([0, 7]).width(Length::Fill).into()
}

/// 在 MCP 设置页上叠加服务器表单或删除确认。
pub fn view_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
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
    let locale = app.desktop_locale;
    if app.mcp_settings.active_tab != McpSettingsTab::Servers {
        return dialog;
    }

    let s = &app.mcp_settings;
    let Some(index) = s.delete_pending else {
        return if s.new_server_expanded {
            settings_modal_overlay(
                Some(container(dialog).width(Length::Fill).height(Length::Fill).into()),
                server_form_toggle(),
                container(new_server_panel(app)).padding(12),
            )
        } else {
            dialog
        };
    };
    let server_name = configured_mcp_server_rows(locale, &app.mcp_settings.editor.text())
        .ok()
        .and_then(|rows| rows.get(index).map(|row| row.name.clone()))
        .unwrap_or_else(|| {
            crate::__!(crate::app::i18n::I18n::new(locale), "mcpServers[%u]", index)
                .expect("valid MCP template")
        });
    let confirm = Message::Settings(SettingsMessage::Mcp(McpMessage::ConfirmDelete(index)));
    let cancel = Message::Settings(SettingsMessage::Mcp(McpMessage::CancelDelete));
    let confirm_dialog = crate::app::components::toast::confirm_dialog(
        tr(locale, "确认删除 MCP 服务器"),
        crate::__!(
            crate::app::i18n::I18n::new(locale),
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

#[cfg(test)]
#[path = "system_settings_mcp_tests.rs"]
mod system_settings_mcp_tests;

/// Managed catalog and connection editing; the surrounding page owns scrolling.
fn managed_catalog_panel(app: &App) -> Element<'_, Message> {
    use crate::app::message::settings::mcp_management_form::{FormMessage, message};
    use crate::app::message::settings::mcp_management_list as catalog;
    let mut body = column![
        managed_catalog_content(catalog::visible(app), catalog::loading(app)),
        toolbar_button("新增受管连接", Some(message(FormMessage::Open(None)))),
        toolbar_button(
            "从原生应用导入",
            Some(crate::app::message::settings::mcp_management_import::message(
                crate::app::message::settings::mcp_management_import::ImportMessage::Open
            ))
        ),
    ]
    .spacing(8);
    if let Some(Ok(response)) = catalog::visible(app) {
        for record in &response.records {
            body = body.push(
                row![
                    text(record.name.clone()),
                    toolbar_button(
                        "替换连接定义",
                        Some(message(FormMessage::Open(Some(record.clone()))))
                    )
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }
    }
    body.into()
}

pub(crate) fn managed_catalog_content(
    result: Option<
        &Result<vw_gateway_client::vw_api_types::mcp_management::McpCatalogResponse, String>,
    >,
    loading: bool,
) -> Element<'_, Message> {
    use crate::app::message::settings::mcp_management_list as catalog;
    let mut body = column![
        text("受管 MCP 目录").size(16),
        toolbar_button(
            if loading { "刷新中..." } else { "刷新目录" },
            Some(Message::Settings(SettingsMessage::Mcp(McpMessage::Catalog(
                catalog::CatalogMessage::Refresh
            ))))
        ),
        text("原生应用：独立配置分发；识矩运行时：项目选择；ACP：会话挂载。三种使用范围独立。")
            .size(12)
            .style(settings_muted_text_style),
    ]
    .spacing(10)
    .width(Length::Fill);
    match result {
        Some(Ok(response)) => {
            if response.records.is_empty() {
                body = body.push(text("当前目录为空").size(12));
            }
            for label in catalog::rows(response) {
                body = body.push(text(label).size(13));
            }
            body = body.push(
                text("原生状态为上次观察；刷新目录不执行连接或重新核验原生文件。")
                    .size(12)
                    .style(settings_muted_text_style),
            );
        }
        Some(Err(error)) => {
            body = body.push(settings_error_banner(error.clone()));
        }
        None => {
            body = body.push(
                text(if loading {
                    "正在加载目录"
                } else {
                    "尚未加载，请先在应用管理连接目标主机后刷新目录"
                })
                .size(12)
                .style(settings_muted_text_style),
            );
        }
    }
    settings_panel(body).into()
}

#[path = "system_settings_mcp_connection.rs"]
mod connection;
pub(crate) use connection::connection_form_panel;

#[path = "system_settings_mcp_bindings.rs"]
mod bindings;

#[path = "system_settings_mcp_import.rs"]
pub(crate) mod import;
