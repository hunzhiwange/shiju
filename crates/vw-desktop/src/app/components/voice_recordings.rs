//! The archive sits in the existing settings scroll area (4 px gutter).
use crate::app::message::voice_recordings::{Event, State};
use crate::app::{App, Message};
use iced::Element;
pub(crate) fn view(app: &App) -> Element<'_, Message> { content(&app.voice.recordings) }
pub(super) fn content(state: &State) -> Element<'_, Message> {
    desktop_voice_ui::components::voice_recordings::content(
        desktop_voice_ui::components::voice_recordings::View {
            entries: &state.entries, result: state.result.as_deref(), problem: state.problem.as_deref(), replaying: state.replay.is_some(), busy: state.busy,
        }, crate::app::voice::recordings::path(),
    ).map(map)
}
fn map(event: desktop_voice_ui::components::voice_recordings::Event) -> Message {
    use desktop_voice_ui::components::voice_recordings::Event as Ui;
    let event = match event {
        Ui::Refresh => Event::Refresh, Ui::Clear => Event::Clear, Ui::Play(id) => Event::Play(id),
        Ui::Retry(id) => Event::Retry(id), Ui::Delete(id) => Event::Delete(id), Ui::Stop => Event::Stop,
        Ui::Apply => Event::Apply, Ui::Dismiss => Event::Dismiss,
    };
    Message::Voice(crate::app::message::voice::Event::Recordings(event))
}
