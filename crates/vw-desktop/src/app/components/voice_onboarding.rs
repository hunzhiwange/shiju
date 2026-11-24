//! Explicit setup only: opening this modal never captures focus or requests permission.
use crate::app::voice::permissions::{PermissionStatus};
use crate::app::{
    App, Message,
    message::voice::{Event, OnboardingEvent},
};
use iced::Element;
fn msg(event: OnboardingEvent) -> Message {
    Message::Voice(Event::Onboarding(event))
}
pub(crate) fn with_onboarding<'a>(
    app: &'a App,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    if !app.voice.insertion.onboarding {
        return base;
    }
    modal(crate::app::i18n::I18n::new(app.general_settings.desktop_locale), base, app.voice.insertion.permissions, app.voice.insertion.pending.as_deref())
}
pub(super) fn modal<'a>(i18n: crate::app::i18n::I18n, base: Element<'a, Message>, statuses: [PermissionStatus; 3], pending: Option<&'a str>) -> Element<'a, Message> {
    desktop_voice_ui::components::voice_onboarding::modal(i18n, base, msg, statuses, pending)
}
