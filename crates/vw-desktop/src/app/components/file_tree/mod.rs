mod adapter;
pub(crate) mod inline_rename;
pub(crate) mod model;
pub(crate) mod selection;
mod tree_list;
pub(crate) use desktop_file_tree::tree_list::change_list_expand_keys;
use crate::app::{App, Message};
pub fn view(app: &App) -> iced::Element<'_, Message> {
    desktop_file_tree::view(adapter::inputs(app)).map(adapter::map)
}
pub fn view_file_manager(app: &App) -> iced::Element<'_, Message> {
    desktop_file_tree::view_file_manager(adapter::inputs(app)).map(adapter::map)
}
#[cfg(test)]
mod mod_tests;
#[cfg(test)]
mod tree_list_tests;
