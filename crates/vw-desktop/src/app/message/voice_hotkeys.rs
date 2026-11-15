//! Dispatch configured OS shortcuts through the existing voice lifecycle.
use super::{
    voice::{self, Event},
    voice_draft::Phase,
};
use crate::app::voice::hotkeys::Action;
use crate::app::{App, Message};
use iced::Task;
pub(super) fn tick(app: &mut App) -> Task<Message> {
    use desktop_voice_ui::message::voice_hotkeys::{self as policy, Poll};
    let (actions, previous_error) = match policy::poll(&mut app.voice, app.shutdown_requested, crate::app::voice::preferences::load) {
        Poll::Skip => return Task::none(),
        Poll::Cancel => return voice::update(app, Event::Cancel),
        Poll::Actions { actions, previous_error } => (actions, previous_error),
    };
    let mut tasks = vec![dispatch(app, actions)];
    if app.voice.hotkeys.error != previous_error
        && let Some(error) = app.voice.hotkeys.error.clone()
    {
        tasks.push(app.show_error_toast(error));
    }
    Task::batch(tasks)
}
pub(super) fn dispatch(app: &mut App, actions: Vec<Action>) -> Task<Message> {
    let mut tasks = Vec::new();
    for action in actions {
        let mut external = None;
        let event = match action {
            Action::Start => {
                match desktop_voice_ui::message::voice_hotkeys::accept_start(&mut app.voice) {
                    Ok(true) => {}, Ok(false) => break,
                    Err(error) => { tasks.push(app.show_error_toast(error)); break; }
                }
                external = match crate::app::voice::insertion::capture_external() {
                    Ok(None) => None,
                    Ok(Some(target)) => Some(Ok(target)),
                    Err(error) => Some(Err(error)),
                };
                Event::Start
            }
            Action::Finish => Event::Finish,
            Action::Cancel => Event::Cancel,
            Action::Chat => Event::ChatStart,
        };
        tasks.push(voice::update(app, event));
        if external.is_some() && app.voice.draft.phase == Phase::Recording {
            app.voice.insertion.target = external;
        }
    }
    desktop_voice_ui::message::voice_hotkeys::finish_poll(&mut app.voice);
    Task::batch(tasks)
}
