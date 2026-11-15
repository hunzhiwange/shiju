use crate::app::{App, Message};
pub(super) use desktop_file_tree::find_results::*;
pub fn build_find_results_list(app: &App) -> iced::Element<'_, Message> { desktop_file_tree::find_results::build_find_results_list(super::adapter::inputs(app)).map(super::adapter::map) }
pub fn build_find_results_tabs(app: &App) -> iced::Element<'_, Message> { desktop_file_tree::find_results::build_find_results_tabs(super::adapter::inputs(app)).map(super::adapter::map) }
