use super::LayoutMode;

#[test]
fn mobile_layout_main_window_round_trip_preserves_business_and_panel_state() {
    use crate::app::message::{ViewMessage, view::layout};
    use crate::app::state::WorkspaceMode;
    let mut app = crate::app::App::new().0;
    let main = iced::window::Id::unique();
    app.main_window_id = Some(main);
    app.startup_resize_checked = true;
    app.active_session_id = Some("layout-session".into());
    app.layout_drag.split_ratio = 0.63;
    app.layout_drag.work_sidebar_width = 321.0;
    app.navigation_workspace.work_split_ratio = 0.57;
    app.layout_drag.settings_panel_width = 411.0;
    app.layout_drag.file_manager_width = 299.0;
    for mode in [WorkspaceMode::Work, WorkspaceMode::Code] {
        app.navigation_mode.workspace_mode = mode;
        for width in [390.0, 1200.0] {
            let _ = layout::update(&mut app, ViewMessage::WindowResized(main, width, 800.0));
            assert_eq!(app.mobile_layout.mode(), if width < 768.0 {
                LayoutMode::Mobile
            } else { LayoutMode::Desktop });
            assert_eq!(app.navigation_mode.workspace_mode, mode);
            assert_eq!(app.active_session_id.as_deref(), Some("layout-session"));
            assert_eq!((app.layout_drag.split_ratio, app.layout_drag.work_sidebar_width, app.navigation_workspace.work_split_ratio),
                (0.63, 321.0, 0.57));
            assert_eq!((app.layout_drag.settings_panel_width, app.layout_drag.file_manager_width), (411.0, 299.0));
        }
    }
    let auxiliary = iced::window::Id::unique();
    app.task_pet_window_id = Some(auxiliary);
    let _ = layout::update(&mut app, ViewMessage::WindowResized(auxiliary, 200.0, 200.0));
    assert_eq!(app.mobile_layout.mode(), LayoutMode::Desktop);
    assert_eq!(app.window_size, (1200.0, 800.0));
}
