#[cfg(test)]
pub fn build_file_tree_list(app: &App) -> iced::Element<'_, Message> {
    desktop_file_tree::tree_list::build_file_tree_list(super::adapter::inputs(app)).map(super::adapter::map)
}
#[cfg(test)]
pub fn build_changes_list(app: &App) -> iced::Element<'_, Message> {
    desktop_file_tree::tree_list::build_changes_list(super::adapter::inputs(app)).map(super::adapter::map)
}

#[cfg(test)]
use crate::app::App;
#[cfg(test)]
use crate::app::Message;
