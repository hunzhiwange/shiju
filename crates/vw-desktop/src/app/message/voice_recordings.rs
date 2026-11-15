//! Desktop routing for archive actions.
use crate::app::{App, Message};
use iced::Task;
pub use desktop_voice_ui::message::voice_recordings::{Event, State};
use desktop_voice_ui::message::voice_recordings as domain;
fn message(event: Event) -> Message { Message::Voice(super::voice::Event::Recordings(event)) }
pub(crate) fn invalidate(app: &mut App) {
    let scope = super::voice::scope(app);
    let visible = app.show_system_settings && app.system_settings_tab == crate::app::components::system_settings::SystemTab::Voice;
    domain::invalidate(&mut app.voice.recordings, scope, &crate::app::gateway_client_endpoint(), visible, crate::app::voice::recordings::path());
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    invalidate(app);
    let active = app.voice.chat_active();
    let busy = matches!(app.voice.draft.phase, super::voice_draft::Phase::Recording | super::voice_draft::Phase::Transcribing);
    let scope = super::voice::scope(app);
    match domain::update(&mut app.voice.recordings, event, active, busy, app.voice.settings.preview.is_some(), scope, crate::app::gateway_client_endpoint(), crate::app::voice::recordings::path(), crate::app::voice::preferences::path()) {
        Ok(domain::Outcome::None) => Task::none(),
        Ok(domain::Outcome::Task(task)) => task.map(message),
        Ok(domain::Outcome::Apply(text)) => { super::voice::paste(app, text, false); app.voice.recordings.stop(); Task::none() }
        Err(error) => app.show_error_toast(error),
    }
}
#[cfg(test)]
#[path = "voice_recordings_tests.rs"]
mod tests;
