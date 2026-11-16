mod adapter;
mod find_results;
mod icons;
pub(crate) mod inline_rename;
mod menu;
pub(crate) mod model;
#[cfg(target_os = "macos")]
mod native_drop;
mod row_layout;
pub(crate) mod selection;
mod tree_gutter;
mod tree_list;
mod widgets;
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
