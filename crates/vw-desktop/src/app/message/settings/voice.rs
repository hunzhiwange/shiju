//! Desktop routing for the voice settings domain.
pub use desktop_settings_voice::{Event, Loaded, LocalResources, State, local_resources};
use crate::app::{App, Message};
use iced::Task;
fn message(event: Event) -> Message {
    Message::Settings(super::SettingsMessage::Voice(event))
}
pub(crate) fn invalidate(app: &mut App) {
    let visible = app.show_system_settings
        && app.system_settings_tab == crate::app::components::system_settings::SystemTab::Voice;
    desktop_settings_voice::invalidate(
        &mut app.voice.settings, &crate::app::gateway_client_endpoint(), visible,
    );
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    invalidate(app);
    match event {
        Event::Feedback(true, text) => return app.show_toast(crate::app::state::ToastKind::Success, text),
        Event::Feedback(false, text) => return app.show_error_toast(text),
        Event::Onboarding => return super::super::voice::update(
            app, super::super::voice::Event::Onboarding(super::super::voice::OnboardingEvent::Open),
        ),
        _ => {}
    }
    if desktop_settings_voice::cancels_activity(&app.voice.settings, app.voice.hotkeys.machine.active, &event) {
        super::super::voice::cancel(app);
    }
    let blocked = app.voice.chat_active() || app.voice.recordings.replay.is_some()
        || matches!(app.voice.draft.phase, super::super::voice_draft::Phase::Recording | super::super::voice_draft::Phase::Transcribing);
    desktop_settings_voice::update(
        &mut app.voice.settings, &mut app.voice.hotkeys, blocked,
        crate::app::gateway_client_endpoint(), crate::app::voice::preferences::path(), event,
    ).map(message)
}
#[cfg(all(test, target_os = "macos"))]
#[path = "voice_tests.rs"]
mod tests;
