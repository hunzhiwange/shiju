//! 系统设置 - Composio 集成配置视图模块
//!
//! 本模块提供 Composio 工具集成的系统设置界面，用于配置：
//! - 是否启用 Composio 集成
//! - Composio API 密钥
//! - 默认实体 ID

use crate::app::components::system_settings_common::{
    SETTINGS_LABEL_WIDTH, settings_checkbox_style, settings_divider, settings_muted_text_style,
    settings_page_intro, settings_panel, settings_section_card, settings_text_input_style,
};
use crate::app::{App, Message, message};
use iced::widget::{checkbox, column, container, row, text, text_input};
use iced::{Alignment, Element, Length};

/// Composio 设置界面内部消息。
#[derive(Debug, Clone)]
pub enum ComposioMessage {
    EnabledToggled(bool),
    ApiKeyChanged(String),
    EntityIdChanged(String),
}

impl From<ComposioMessage> for message::SettingsMessage {
    fn from(value: ComposioMessage) -> Self {
        match value {
            ComposioMessage::EnabledToggled(enabled) => Self::ComposioEnabledToggled(enabled),
            ComposioMessage::ApiKeyChanged(api_key) => Self::ComposioApiKeyChanged(api_key),
            ComposioMessage::EntityIdChanged(entity_id) => Self::ComposioEntityIdChanged(entity_id),
        }
    }
}

fn field_row<'a>(
    label: &'static str,
    description: &'static str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    container(
        row![
            column![
                text(label).size(13),
                text(description).size(11).style(settings_muted_text_style),
            ]
            .spacing(4)
            .width(Length::Fixed(SETTINGS_LABEL_WIDTH)),
            container(control.into()).width(Length::Fill),
        ]
        .spacing(22)
        .align_y(Alignment::Center),
    )
    .padding([14, 0])
    .width(Length::Fill)
    .into()
}

fn text_row<'a>(
    label: &'static str,
    description: &'static str,
    placeholder: &'static str,
    value: &'a str,
    hint: &'static str,
    secure: bool,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    let input = text_input(placeholder, value)
        .secure(secure)
        .on_input(on_input)
        .padding([10, 12])
        .size(13)
        .style(settings_text_input_style)
        .width(Length::Fill);

    field_row(
        label,
        description,
        column![input, text(hint).size(12).style(settings_muted_text_style).width(Length::Fill),]
            .spacing(6)
            .width(Length::Fill),
    )
}

/// 渲染 Composio 配置设置视图。
pub fn view(app: &App) -> Element<'_, Message> {
    let s = &app.composio_settings;

    let enabled_row = field_row(
        "启用",
        "控制是否注册 Composio OAuth 工具集成。",
        checkbox(s.enabled)
            .label("启用 Composio OAuth 工具集成")
            .on_toggle(|value| Message::Settings(ComposioMessage::EnabledToggled(value).into()))
            .style(settings_checkbox_style),
    );

    let api_key_row = text_row(
        "API 密钥",
        "Composio 平台颁发的 API Key。",
        "cmp_...",
        &s.api_key_input,
        "留空时不写入配置，运行时将不会注册 Composio 工具。",
        true,
        |value| Message::Settings(ComposioMessage::ApiKeyChanged(value).into()),
    );

    let entity_id_row = text_row(
        "实体 ID",
        "用于区分不同用户或实体的默认标识。",
        "default",
        &s.entity_id_input,
        "为空时会自动回退为 default，用于多用户或多实体场景。",
        false,
        |value| Message::Settings(ComposioMessage::EntityIdChanged(value).into()),
    );

    let content = column![
        settings_page_intro("Composio", "配置 OAuth 工具集成开关、API Key 和默认实体 ID。"),
        settings_section_card("基础行为", "控制集成启用状态与认证信息。"),
        settings_panel(column![enabled_row, settings_divider(), api_key_row].spacing(0)),
        settings_section_card("默认实体", "配置多用户或多实体场景下的默认 entity。"),
        settings_panel(column![entity_id_row].spacing(0)),
    ]
    .spacing(16)
    .width(Length::Fill);

    content.into()
}
#[cfg(test)]
#[path = "system_settings_composio_tests.rs"]
mod system_settings_composio_tests;
