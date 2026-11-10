//! API 密钥连接弹窗；使用现有供应商消息保存或放弃草稿。

use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_TEXT_SIZE, primary_action_btn_style, rounded_action_btn_style,
    settings_close_button, settings_error_banner, settings_field_label, settings_hover_help_label,
    settings_modal_header, settings_modal_layout, settings_modal_overlay, settings_text_input,
};
use crate::app::i18n::I18n;
use crate::app::i18n::settings_models::connection_text::text as connection_text;
use crate::app::i18n::settings_models::messages::render as owned_text;
use crate::app::message::SettingsMessage;
use crate::app::{App, Message};
use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Element, Length, widget::text::Wrapping};

pub fn view_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    let s = &app.provider_settings;
    let Some(modal) = s
        .connect_modal
        .as_ref()
        .filter(|modal| !crate::app::state::is_shiju_official_provider_id(&modal.provider_id))
    else {
        return dialog;
    };

    let locale = app.desktop_locale;
    let close = Message::Settings(SettingsMessage::ProviderConnectClose);
    let header = settings_modal_header(
        text(owned_text(locale, "配置 %s 的密钥", &[modal.provider_name.clone()]))
            .size(18)
            .wrapping(Wrapping::WordOrGlyph),
        settings_close_button(close.clone()),
    );

    let key_label = connection_text(locale, "API 密钥");
    let mut field = column![
        settings_hover_help_label(
            I18n::new(locale),
            &app.app_theme,
            key_label,
            settings_field_label(key_label),
            connection_text(locale, "录入并保存当前 Provider 的 API 密钥。"),
        )
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

    settings_modal_overlay(Some(dialog), close, container(card).padding(12))
}
