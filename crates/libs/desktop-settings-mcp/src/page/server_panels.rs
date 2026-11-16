//! Configured and runtime server cards.
use super::*;
use iced::widget::column;
pub fn configured_server_panel<'a, Message: Clone + 'static>(event: fn(McpMessage) -> Message,
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
            event(McpMessage::ServerEnabledToggled(
                server_index,
                value,
            ))
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
            desktop_i18n::__!(I18n::new(locale), "%u 个工具", count)
                .expect("valid MCP template"),
        ));
    }
    if let Some(count) = inspect.and_then(|entry| entry.resource_count) {
        badges = badges.push(settings_value_badge(
            desktop_i18n::__!(I18n::new(locale), "%u 个资源", count)
                .expect("valid MCP template"),
        ));
    }
    if let Some(count) = inspect.and_then(|entry| entry.prompt_count) {
        badges = badges.push(settings_value_badge(
            desktop_i18n::__!(I18n::new(locale), "%u 个 prompts", count)
                .expect("valid MCP template"),
        ));
    }

    let preview_label = if previewing { tr(locale, "预览中...") } else { tr(locale, "预览") };
    let preview_toggle_label =
        if preview_expanded { tr(locale, "收起") } else { tr(locale, "展开") };
    let preview_toggle_message = Some(event(
        McpMessage::PreviewExpandedToggled(preview_key),
    ));
    let edit_message = controls_enabled.then(|| {
        event(McpMessage::ServerEditRequested(server_index))
    });
    let delete_message = controls_enabled
        .then(|| event(McpMessage::RequestDelete(server_index)));

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
            toolbar_button(preview_label, preview_action_message(event, preview_enabled)),
            toolbar_button(preview_toggle_label, preview_toggle_message),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .wrap(),
    ]
    .spacing(12);
    if preview_expanded {
        content = content.push(settings_divider()).push(capability_preview(event,
            locale,
            inspect,
            preview_enabled,
            previewing,
        ));
    }

    settings_panel(content).into()
}

pub fn runtime_server_panel<'a, Message: Clone + 'static>(event: fn(McpMessage) -> Message,
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
            event(McpMessage::RuntimeServerEnabledToggled(
                server_name.clone(),
                value,
            ))
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
            desktop_i18n::__!(I18n::new(locale), "%u 个工具", count)
                .expect("valid MCP template"),
        ));
    }
    if let Some(count) = server.resource_count {
        badges = badges.push(settings_value_badge(
            desktop_i18n::__!(I18n::new(locale), "%u 个资源", count)
                .expect("valid MCP template"),
        ));
    }
    if let Some(count) = server.prompt_count {
        badges = badges.push(settings_value_badge(
            desktop_i18n::__!(I18n::new(locale), "%u 个 prompts", count)
                .expect("valid MCP template"),
        ));
    }

    let preview_label = if previewing { tr(locale, "预览中...") } else { tr(locale, "预览") };
    let preview_toggle_label =
        if preview_expanded { tr(locale, "收起") } else { tr(locale, "展开") };
    let preview_toggle_message = Some(event(
        McpMessage::PreviewExpandedToggled(preview_key),
    ));

    let mut content = column![
        column![
            text(server.name.as_str()).size(16),
            text(tr(locale, "插件或项目级 MCP")).size(12).style(settings_muted_text_style),
        ]
        .spacing(4),
        badges.wrap(),
        row![
            enabled_checkbox,
            toolbar_button(preview_label, preview_action_message(event, preview_enabled)),
            toolbar_button(preview_toggle_label, preview_toggle_message),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .wrap(),
    ]
    .spacing(12);
    if preview_expanded {
        content = content.push(settings_divider()).push(capability_preview(event,
            locale,
            Some(server),
            preview_enabled,
            previewing,
        ));
    }

    settings_panel(content).into()
}
