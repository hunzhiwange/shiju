use crate::app::{App, Message};
use crate::app::message::voice::Event;
use desktop_chat_input::input_panel::voice_controls as domain;
fn map_event(event: domain::Event) -> Message {
 Message::Voice(match event { domain::Event::Start => Event::Start, domain::Event::Finish => Event::Finish, domain::Event::Cancel => Event::Cancel, domain::Event::Apply => Event::Apply })
}
fn input(app: &App) -> domain::Input { domain::Input { phase: app.voice.draft.phase, pending: app.voice.draft.pending.is_some(), ready: app.voice.ready() } }
pub(super) fn view(app: &App) -> iced::Element<'_, Message> { domain::view(input(app)).map(map_event) }
pub(super) fn recording_bar(app: &App) -> iced::Element<'_, Message> { domain::recording_bar(input(app), app.voice.waveform.view()).map(map_event) }
