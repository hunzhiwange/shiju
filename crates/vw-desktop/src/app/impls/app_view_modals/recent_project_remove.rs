use super::{App, Message, message};
use iced::Element;
pub(crate) fn with_recent_project_remove_confirm<'a>(app: &'a App, root: Element<'a, Message>) -> Element<'a, Message> {
    desktop_workspace::views::modals::recent_project_remove::with_recent_project_remove_confirm(app.workspace_state.recent_project_remove_confirm_path.as_ref(), &app.workspace_state.recent_projects, &app.workspace_state.recent_projects_edits, &app.workspace_state.recent_projects_meta, app.general_settings.desktop_locale, |event| {
        use desktop_workspace::views::modals::recent_project_remove::Event;
        Message::Project(match event {
            Event::RecentRemoveConfirmed(path) => message::ProjectMessage::RecentRemoveConfirmed(path),
            Event::RecentRemoveCanceled => message::ProjectMessage::RecentRemoveCanceled,
        })
    }, root)
}
