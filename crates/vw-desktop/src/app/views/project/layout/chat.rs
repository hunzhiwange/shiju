use crate::app::{App, Message};
use crate::app::components::{chat_panel, input_panel};
pub fn chat_area(app: &App, spacing: f32, radius: f32, padding: f32) -> iced::Element<'_, Message> {
    desktop_workspace::views::project::layout::chat::chat_area(chat_panel::view(app), input_panel::view(app), app.workspace_mode == crate::app::state::WorkspaceMode::Work,
        crate::app::theme_catalog::resolve_chrome_style_from_skin(app.active_theme_skin.as_ref(), &app.app_theme), spacing, radius, padding)
}
