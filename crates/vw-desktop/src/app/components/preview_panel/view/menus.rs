use crate::app::{App, Message, message};
use iced::Element;
#[cfg(test)]
pub(super) fn nav_popup_item_message(parent: &str, name: &str, is_dir: bool) -> Message {
    map_message(desktop_editor::components::preview_panel::view::menus::nav_popup_item_message(parent, name, is_dir))
}
pub fn build_menu_ui(app: &App) -> Element<'_, Message> {
    let exists = app.preview_context_target.as_ref().is_some_and(|(path, ..)| app.navigation_tabs.preview_tabs.iter().any(|tab| &tab.path == path));
    desktop_editor::components::preview_panel::view::menus::build_menu_ui(
        app.show_preview_context_menu && app.preview_context_target.is_some() && app.preview_context_menu_pos.is_some(),
        exists, desktop_i18n::I18n::new(app.desktop_locale), app.preview_nav_popup.as_ref(),
    ).map(map_message)
}
fn map_message(action: desktop_editor::components::preview_panel::view::MenuMessage) -> Message {
    use desktop_editor::components::preview_panel::view::MenuMessage as Action;
    match action {
        Action::ContextMenuCopy => Message::Preview(message::PreviewMessage::ContextMenuCopy),
        Action::ContextMenuCut => Message::Preview(message::PreviewMessage::ContextMenuCut),
        Action::ContextMenuPaste => Message::Preview(message::PreviewMessage::ContextMenuPaste),
        Action::ContextMenuDelete => Message::Preview(message::PreviewMessage::ContextMenuDelete),
        Action::InsertSelected => Message::Chat(message::ChatMessage::InsertSelected),
        Action::InsertSelectionPositions => Message::Chat(message::ChatMessage::InsertSelectionPositions),
        Action::Navigate { full_path, is_dir } => Message::Batch(vec![
            Message::Preview(message::PreviewMessage::CloseNavPopup),
            if is_dir { Message::Project(message::ProjectMessage::ToggleTreeDir(full_path)) }
            else { Message::Preview(message::PreviewMessage::Open(full_path)) },
        ]),
    }
}
