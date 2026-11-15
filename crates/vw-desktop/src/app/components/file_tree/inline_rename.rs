pub use desktop_file_tree::inline_rename::{INPUT_ID, selection_end};
use crate::app::{App, Message};
pub(super) fn view(app: &App, depth: usize, directory: bool, expanded: bool) -> iced::Element<'_, Message> {
    desktop_file_tree::inline_rename::view(super::adapter::inputs(app), depth, directory, expanded).map(super::adapter::map)
}
#[cfg(test)]
#[path = "inline_rename_tests.rs"]
mod inline_rename_tests;
