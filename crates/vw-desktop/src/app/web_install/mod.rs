//! Desktop mapping for local Web installation events.
pub use desktop_web_install::{Event, State};

use crate::app::message::ViewMessage;
use crate::app::{App, Message};
use iced::Task;

pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    if matches!(&event, Event::Open) {
        app.navigation_panels.active_menu = None;
    }
    let (task, error) = desktop_web_install::update(
        &mut app.web_install,
        event,
        app.desktop_update.source,
        app.shutdown_requested,
    );
    let task = task.map(|event| Message::View(ViewMessage::WebInstall(event)));
    match error {
        Some(error) => Task::batch([task, app.show_error_toast(error)]),
        None => task,
    }
}
