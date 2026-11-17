#[cfg(test)]
pub fn right_column_chrome(content: iced::Element<'_, Message>, radius: f32, mode: WorkspaceMode) -> iced::Element<'_, Message> {
    desktop_workspace::views::project::layout::chrome::right_column_chrome(content, radius, mode == WorkspaceMode::Work)
}

#[cfg(test)]
use crate::app::Message;
#[cfg(test)]
use crate::app::state::WorkspaceMode;
#[cfg(test)]
pub use desktop_workspace::views::project::layout::chrome::overlay_divider;
