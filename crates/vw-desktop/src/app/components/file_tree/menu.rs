use crate::app::{App, Message};
pub(super) use desktop_file_tree::menu::*;
pub fn build_file_tree_menu(app: &App, include_find_in_folder: bool) -> iced::Element<'_, Message> { desktop_file_tree::menu::build_file_tree_menu(super::adapter::inputs(app), include_find_in_folder).map(super::adapter::map) }
pub(super) fn file_tree_menu_overlay<'a>(app: &'a App, row: iced::Element<'a, Message>, include_find_in_folder: bool) -> iced::Element<'a, Message> {
    desktop_ui_controls::overlays::PointBelowOverlay::new(row, build_file_tree_menu(app, include_find_in_folder))
        .show(true).anchor(app.file_tree_interaction.file_tree_menu_anchor.unwrap_or(iced::Point::ORIGIN))
        .on_close(Message::Project(crate::app::message::ProjectMessage::FileTreeMenuClose))
        .on_key_press(|key, modifiers| file_tree_menu_shortcut(key, modifiers, app.file_tree_interaction.file_tree_clipboard.is_some()))
        .capture_keyboard().into()
}
pub(super) fn file_tree_menu_shortcut(key: &iced::keyboard::Key, modifiers: iced::keyboard::Modifiers, can_paste: bool) -> Option<Message> {
    desktop_file_tree::menu::file_tree_menu_shortcut(key, modifiers, can_paste).map(super::adapter::map)
}
