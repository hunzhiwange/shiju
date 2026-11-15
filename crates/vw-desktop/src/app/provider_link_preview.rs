//! Read-only handoff from the launch inbox. Never fetch config URLs or persist credentials.
use crate::app::{Message, message::settings::{GeneralSettingsMessage, SettingsMessage}};
use iced::{Element, Length};
use iced::widget::text;

use desktop_platform::provider_link_preview::{parse, pending};

pub(crate) fn overlay<'a>(base: Element<'a, Message>) -> Element<'a, Message> {
    use crate::app::components::system_settings_common::{
        settings_close_button, settings_modal_header, settings_modal_layout, settings_modal_overlay,
    };
    let Some(link) = pending() else { return base };
    let close = Message::Settings(SettingsMessage::General(
        GeneralSettingsMessage::ProviderLinkRejected(link.id),
    ));
    let parsed = parse(&link.url);
    let body = desktop_platform::provider_link_preview::body(parsed).map(|_| unreachable!());
    let card = settings_modal_layout(
        settings_modal_header(text("供应商链接预览"), settings_close_button(close.clone())),
        body, None,
    ).width(Length::Fixed(560.0)).height(Length::Fixed(340.0));
    settings_modal_overlay(Some(base), close, card)
}
