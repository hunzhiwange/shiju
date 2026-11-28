//! Desktop composition of the voice settings page and recordings domain.
use crate::app::i18n::I18n;
use crate::app::message::{SettingsMessage, settings::voice::Event};
use crate::app::{App, Message};
use iced::widget::column;
use iced::{Element, Length};

fn message(event: Event) -> Message {
    Message::Settings(SettingsMessage::Voice(event))
}

pub(crate) fn view(app: &App) -> Element<'_, Message> {
    let i18n = I18n::new(app.general_settings.desktop_locale);
    column![
        desktop_settings_voice::view::view(&app.voice.settings, i18n, &app.voice.hotkeys)
            .map(message),
        super::voice_recordings::view(app),
    ]
    .spacing(20)
    .width(Length::Fill)
    .into()
}

pub(crate) fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    desktop_settings_voice::view::modal(
        &app.voice.settings,
        I18n::new(app.general_settings.desktop_locale),
        base,
        message,
    )
}
