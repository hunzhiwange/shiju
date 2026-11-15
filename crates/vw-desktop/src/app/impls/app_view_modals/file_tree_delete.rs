use super::{App, Message, message};
use iced::Element;
pub(crate) fn with_file_tree_delete_confirm<'a>(app: &'a App, root: Element<'a, Message>) -> Element<'a, Message> {
    desktop_workspace::views::modals::file_tree_delete::with_file_tree_delete_confirm(app.file_tree_interaction.file_tree_pending_delete.as_ref().map(|(_, paths)| paths.as_slice()), app.desktop_locale, |event| {
        use desktop_workspace::views::modals::file_tree_delete::Event;
        Message::Project(match event {
            Event::FileTreeDeleteCanceled => message::ProjectMessage::FileTreeDeleteCanceled,
            Event::FileTreeDeleteConfirmed => message::ProjectMessage::FileTreeDeleteConfirmed,
        })
    }, root)
}
