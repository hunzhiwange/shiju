//! API 密钥连接弹窗；使用现有供应商消息保存或放弃草稿。

use crate::view_style::ViewStyle;
use desktop_ui_controls::system_settings_common::{
    SETTINGS_CONTROL_TEXT_SIZE, primary_action_btn_style, rounded_action_btn_style,
    settings_close_button, settings_error_banner, settings_field_label, settings_modal_header, settings_modal_layout, settings_text_input,
};
use desktop_i18n::settings_models::connection_text::text as connection_text;
use desktop_i18n::settings_models::messages::render as owned_text;
use crate::view_message::SettingsMessage;
use crate::view_message::Message;
use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Element, Length, widget::text::Wrapping};

pub fn modal<'a>(s: &'a crate::provider_state::ProviderSettingsState, style: ViewStyle<'a>) -> Option<(Message, Element<'a, Message>)> {
    let Some(modal) = s
        .connect_modal
        .as_ref()
        .filter(|modal| !crate::provider_state::is_shiju_official_provider_id(&modal.provider_id))
    else {
        return None;
    };

    let locale = style.locale;
    let close = Message::Settings(SettingsMessage::ProviderConnectClose);
    let header = settings_modal_header(
        text(owned_text(locale, "配置 %s 的密钥", &[modal.provider_name.clone()]))
            .size(18)
            .wrapping(Wrapping::WordOrGlyph),
        settings_close_button(close.clone()),
    );

    let key_label = connection_text(locale, "API 密钥");
    let mut field = column![
        style.help_label(key_label, settings_field_label(key_label), connection_text(locale, "录入并保存当前 Provider 的 API 密钥。"))
        .wrap(),
        settings_text_input(connection_text(locale, "输入 API 密钥"), &modal.api_key)
            .secure(true)
            .on_input(|value| Message::Settings(SettingsMessage::ProviderConnectApiKeyChanged(
                value
            ))),
    ]
    .spacing(8);
    if let Some(error) = &s.connect_error {
        field = field.push(settings_error_banner(error.display(locale)));
    }

    let footer = container(
        row![
            button(text(connection_text(locale, "取消")).size(SETTINGS_CONTROL_TEXT_SIZE))
                .on_press(close.clone())
                .padding([8, 14])
                .style(rounded_action_btn_style),
            button(text(connection_text(locale, "保存密钥")).size(SETTINGS_CONTROL_TEXT_SIZE))
                .on_press(Message::Settings(SettingsMessage::ProviderConnectSubmit))
                .padding([8, 14])
                .style(primary_action_btn_style),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .align_right(Length::Fill);
    let body = container(field).padding([18, 20]).width(Length::Fill);
    let card = settings_modal_layout(header, body, Some(footer.into()))
        .width(Length::Fixed(560.0))
        .height(Length::Fixed(350.0));

    Some((close, container(card).padding(12).into()))
}

#[cfg(test)]
pub(super) fn view_overlays<'a>(app: &'a crate::provider_view_test_support::Fixture, base: Element<'a, Message>) -> Element<'a, Message> {
    let Some((close, card)) = modal(&app.provider_settings, crate::provider_view_test_support::style(app)) else { return base };
    desktop_ui_controls::system_settings_common::settings_modal_overlay(Some(base), close, card)
}
