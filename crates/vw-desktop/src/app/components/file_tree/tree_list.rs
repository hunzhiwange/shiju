use crate::app::{App, Message};
pub fn build_file_tree_list(app: &App) -> iced::Element<'_, Message> {
    desktop_file_tree::tree_list::build_file_tree_list(super::adapter::inputs(app)).map(super::adapter::map)
}
pub fn build_changes_list(app: &App) -> iced::Element<'_, Message> {
    desktop_file_tree::tree_list::build_changes_list(super::adapter::inputs(app)).map(super::adapter::map)
}
