use crate::app::{App, Message, message::ViewMessage};
use crate::app::components::{file_tree, preview_panel};
use desktop_workspace::views::project::layout::main::{self, MainLayout};
pub fn main_area(app: &App, spacing: f32, content_pad: f32, chat_pad: f32, radius: f32) -> iced::Element<'_, Message> {
    main::main_area(MainLayout {
        chat_panel_fullscreen: app.chat_panel_fullscreen,
        chat_panel_half_fullscreen: app.chat_panel_half_fullscreen,
        git_diff_fullscreen: app.git.git_diff_fullscreen,
        git_diff_half_fullscreen: app.git.git_diff_half_fullscreen,
        file_manager_width: app.layout_drag.file_manager_width,
        split_ratio: app.layout_drag.split_ratio,
        show_diff: app.navigation_panels.show_diff,
        show_file_manager: app.file_manager.show_file_manager,
    }, super::chat::chat_area(app, spacing, radius, chat_pad), preview_panel::view(app), || file_tree::view_file_manager(app), Message::View(ViewMessage::SplitDragStarted), Message::View(ViewMessage::FileManagerDragStarted), spacing, content_pad)
}
