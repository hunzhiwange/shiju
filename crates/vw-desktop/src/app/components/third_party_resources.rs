//! Thin resource-page composition. Navigation and refresh dispatch remain with the host.
use crate::app::{App, Message};
use iced::{Element, Length, widget::column};

/// The catalog retains its own host, request generation, reader and maintenance overlays.
/// Session identities carry their application identity; this catalog spans applications.
pub(crate) fn sessions(app: &App) -> Element<'_, Message> {
    column![
        super::external_sessions::view(
            &app.apps_catalog.external_sessions,
            crate::app::i18n::I18n::new(app.general_settings.desktop_locale),
        ),
    ]
    .spacing(16)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// Keep account login, binding and confirmation messages with the authorization owner.
/// The host must also compose system_settings_auth::overlay for this page.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn authorization(app: &App) -> Element<'_, Message> {
    super::system_settings_auth::view(app)
}
