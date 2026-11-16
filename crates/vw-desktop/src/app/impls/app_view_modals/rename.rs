use super::{App, Message, message};
use iced::Element;
pub(crate) fn with_session_rename<'a>(app: &'a App, root: Element<'a, Message>) -> Element<'a, Message> {
    desktop_workspace::views::modals::rename::with_session_rename(app.session_rename_id.as_deref(), &app.session_rename_value, &app.sessions, &app.project_sessions, app.desktop_locale, |event| {
        use desktop_workspace::views::modals::rename::Event;
        Message::Project(match event {
            Event::SessionRenameChanged(value) => message::ProjectMessage::SessionRenameChanged(value),
            Event::SessionRenameSave => message::ProjectMessage::SessionRenameSave,
            Event::SessionRenameCancel => message::ProjectMessage::SessionRenameCancel,
        })
    }, root)
}
