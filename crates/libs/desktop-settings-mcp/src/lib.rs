//! MCP settings views, state, and authorized gateway operations.
use config_ui::DesktopLocale;
use desktop_i18n::settings_extensions::text as tr;
use desktop_ui_controls::system_settings_common::settings_muted_text_style;
use iced::{Alignment, Background, Border, Color, Element, Length};
use iced::widget::{column, container, row, text};
use serde_json::Value;
use vw_api_mcp_inspect::{McpConnectionStatusDto, McpInspectResponse, McpServerInspectDto};

#[derive(Debug, Clone)]
pub struct ConfiguredMcpServerRow {
    pub index: usize,
    pub name: String,
    pub transport: String,
    pub enabled: bool,
    pub required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum McpStatusTone {
    Connected,
    Error,
    Disabled,
    Pending,
}

pub(crate) fn compact_text(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let truncated = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() { format!("{truncated}...") } else { truncated }
}

pub(crate) fn configured_mcp_server_rows(
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
            desktop_i18n::__!(desktop_i18n::I18n::new(locale), "mcpServers[%u] 必须是对象", index)
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

pub(crate) fn inspect_for_server<'a>(
    inspect: Option<&'a McpInspectResponse>,
    name: &str,
) -> Option<&'a McpServerInspectDto> {
    inspect?.servers.iter().find(|server| server.name == name)
}

pub(crate) fn connection_status_label_and_tone(
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

pub(crate) fn status_dot_color(theme: &iced::Theme, tone: McpStatusTone) -> Color {
    let palette = theme.extended_palette();
    match tone {
        McpStatusTone::Connected => palette.success.base.color,
        McpStatusTone::Error => palette.danger.base.color,
        McpStatusTone::Disabled => palette.background.strong.color,
        McpStatusTone::Pending => palette.warning.base.color,
    }
}

pub(crate) fn status_badge_style(theme: &iced::Theme) -> iced::widget::container::Style {
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

pub(crate) fn status_badge<Message: Clone + 'static>(
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

pub(crate) fn value_field(value: &Value, keys: &[&str]) -> Option<String> {
    let object = value.as_object()?;
    keys.iter()
        .filter_map(|key| object.get(*key).and_then(Value::as_str))
        .map(str::trim)
        .find(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn preview_section_header<Message: Clone + 'static>(
    locale: DesktopLocale,
    title: &'static str,
    count: usize,
    error: Option<&str>,
) -> Element<'static, Message> {
    let label = match error {
        Some(_) => desktop_i18n::__!(desktop_i18n::I18n::new(locale), "%s · 错误", title)
            .expect("valid MCP template"),
        None => desktop_i18n::__!(desktop_i18n::I18n::new(locale), "%s · %u", title, count)
            .expect("valid MCP template"),
    };
    text(label).size(12).style(settings_muted_text_style).into()
}

pub(crate) fn tool_summary<Message: Clone + 'static>(
    tool: &vw_api_tools::ToolSpecDto,
) -> Element<'static, Message> {
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
    .align_y(Alignment::Center)
    .into()
}

pub(crate) fn tools_preview_section<Message: Clone + 'static>(
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
        content = content.push(tool_summary(tool));
    }

    let hidden_count = inspect.tools.len().saturating_sub(8);
    if hidden_count > 0 {
        content = content.push(
            text(
                desktop_i18n::__!(
                    desktop_i18n::I18n::new(locale),
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

pub(crate) fn value_preview_section<Message: Clone + 'static>(
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
                desktop_i18n::__!(
                    desktop_i18n::I18n::new(locale),
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

#[cfg(test)]
mod tests;

pub mod connection;
pub mod tools;

pub(crate) fn toolbar_button<'a, Message: Clone + 'a>(
    label: &'static str,
    message: Option<Message>,
) -> iced::widget::Button<'a, Message> {
    let mut control = iced::widget::button(text(label).size(13))
        .padding([7, 14])
        .style(desktop_ui_controls::system_settings_common::rounded_action_btn_style);
    if let Some(message) = message { control = control.on_press(message); }
    control
}

pub(crate) fn form_field<'a, Message: Clone + 'a>(
    label: &'static str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    column![desktop_ui_controls::system_settings_common::settings_field_label(label), control.into()]
        .spacing(6).width(Length::Fill).into()
}
pub mod import;
pub mod resync;
pub mod distribution;
pub mod bindings;
pub mod page;
pub mod state;
pub mod catalog;

pub mod configuration;
pub mod configuration_update;
pub mod management_context;
pub mod management_form;
pub mod management_bindings;
pub mod management_import;

pub mod diagnostic_context;
pub mod management_list;
pub mod management_probe;
pub mod management_resync;
pub mod management_tools;
pub mod management_effect;

pub mod settings_message;

#[cfg(test)]
mod desktop_split_02_tests;

#[cfg(test)]
mod desktop_split_14_tests;

#[cfg(test)]
mod desktop_split_29_tests;

pub mod management_state;
pub mod management_update;
pub mod management_form_update;
pub mod management_import_update;
pub mod management_bindings_update;

#[cfg(test)]
mod host_module_24_tests;

#[cfg(test)]
mod host_module_39_infrastructure_tests;
