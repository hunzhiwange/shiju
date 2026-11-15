use crate::app::{App, Message};
pub fn drag_badge_layer(app: &App) -> iced::Element<'_, Message> {
    desktop_workspace::views::project::layout::drag::drag_badge_layer(
        &app.file_tree_interaction.dragging_file_paths, app.project_path.as_deref(),
        app.cursor_position, app.window_size,
        crate::app::components::top_bar::TOP_BAR_HEIGHT + crate::app::components::tab_bar::TAB_BAR_HEIGHT,
    )
}
