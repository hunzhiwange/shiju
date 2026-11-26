use crate::app::components::{chat_panel, input_panel};
use crate::app::{App, Message};
use desktop_chat_view::panel::PanelPhase;

pub fn chat_area(app: &App, radius: f32, padding: f32) -> iced::Element<'_, Message> {
    desktop_workspace::views::project::layout::chat::chat_area(
        chat_panel::view(app),
        input_panel::view(app),
        app.navigation_mode.workspace_mode == crate::app::state::WorkspaceMode::Work,
        crate::app::theme_catalog::resolve_chrome_style_from_skin(
            app.appearance.active_theme_skin.as_ref(),
            &app.appearance.app_theme,
        ),
        chat_panel::phase(app) == PanelPhase::Empty,
        radius,
        padding,
    )
}
