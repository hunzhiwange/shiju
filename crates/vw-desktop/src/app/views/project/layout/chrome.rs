use crate::app::{Message, state::WorkspaceMode};
pub use desktop_workspace::views::project::layout::chrome::overlay_divider;
pub fn right_column_chrome(content: iced::Element<'_, Message>, radius: f32, mode: WorkspaceMode) -> iced::Element<'_, Message> {
    desktop_workspace::views::project::layout::chrome::right_column_chrome(content, radius, mode == WorkspaceMode::Work)
}
