//! Dispatch configured OS shortcuts through the existing voice lifecycle.
use super::{
    voice::{self, Event},
    voice_draft::Phase,
};
use crate::app::voice::hotkeys::Action;
use crate::app::{App, Message};
use iced::Task;
pub(super) fn tick(app: &mut App) -> Task<Message> {
    if app.shutdown_requested {
        return Task::none();
    }
    if app.voice.settings.busy || app.voice.settings.edit.is_some() {
        let active = app.voice.hotkeys.machine.active;
        app.voice.hotkeys.reload();
        return if active { voice::update(app, Event::Cancel) } else { Task::none() };
    }
    app.voice.insertion.poll();
    let previous_error = app.voice.hotkeys.error.clone();
    let actions = app.voice.hotkeys.poll();
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
                if matches!(app.voice.draft.phase, Phase::Recording | Phase::Transcribing)
                    || app.voice.conversation.is_some()
                {
                    app.voice.hotkeys.machine.reset();
                    // The same poll may also contain this rejected press's release.
                    // It must not finish a capture started by the foreground button.
                    break;
                }
                if app.voice.insertion.pending.is_some() {
                    app.voice.hotkeys.machine.reset();
                    tasks.push(app.show_error_toast("请先复制或丢弃上次未插入的听写结果。"));
                    break;
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
    if app.voice.hotkeys.machine.active && app.voice.draft.phase != Phase::Recording {
        app.voice.hotkeys.machine.reset();
    }
    Task::batch(tasks)
}
