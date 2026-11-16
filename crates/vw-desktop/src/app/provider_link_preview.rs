//! Read-only handoff from the launch inbox. Never fetch config URLs or persist credentials.
use crate::app::{
    Message,
    message::settings::{GeneralSettingsMessage, SettingsMessage},
};
use iced::Element;

use desktop_platform::provider_link_preview::pending;

pub(crate) fn overlay<'a>(base: Element<'a, Message>) -> Element<'a, Message> {
    let Some(link) = pending() else { return base };
    let close = Message::Settings(SettingsMessage::General(
        GeneralSettingsMessage::ProviderLinkRejected(link.id),
    ));
    desktop_navigation::view_layout::link_preview::provider(base, &link.url, close)
}
