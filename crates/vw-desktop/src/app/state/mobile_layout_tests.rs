use super::{LayoutMode, LayoutPreference, MobileLayoutState};

#[test]
fn mobile_layout_auto_uses_strict_logical_width_breakpoint() {
    for (width, expected) in [
        (360.0, LayoutMode::Mobile),
        (767.99, LayoutMode::Mobile),
        (768.0, LayoutMode::Desktop),
        (1200.0, LayoutMode::Desktop),
    ] {
        assert_eq!(MobileLayoutState::new(width, 800.0).mode(), expected);
    }
}

#[test]
fn mobile_layout_explicit_choices_override_width_and_rotation() {
    let mut state = MobileLayoutState::new(390.0, 844.0);
    state.select(LayoutPreference::Desktop);
    assert_eq!(state.mode(), LayoutMode::Desktop);
    state.select(LayoutPreference::Mobile);
    state.update_size(844.0, 390.0);
    assert_eq!(state.mode(), LayoutMode::Mobile);
    assert_eq!(state.preference(), LayoutPreference::Mobile);
}

#[test]
fn mobile_layout_returning_to_auto_uses_latest_width() {
    let mut state = MobileLayoutState::new(390.0, 844.0);
    state.select(LayoutPreference::Mobile);
    state.update_size(1024.0, 768.0);
    state.select(LayoutPreference::Auto);
    assert_eq!(state.mode(), LayoutMode::Desktop);
}

#[test]
fn mobile_layout_invalid_initial_sizes_fall_back_to_desktop() {
    for (w, h) in [(0.0, 0.0), (-1.0, 800.0), (f32::NAN, 800.0),
        (f32::INFINITY, 800.0), (390.0, 0.0), (390.0, f32::NAN)] {
        let state = MobileLayoutState::new(w, h);
        assert_eq!(state.preference(), LayoutPreference::Auto);
        assert_eq!(state.mode(), LayoutMode::Desktop);
    }
}

#[test]
fn mobile_layout_invalid_resize_preserves_last_valid_width() {
    let mut state = MobileLayoutState::new(390.0, 844.0);
    for (w, h) in [(0.0, 800.0), (f32::NAN, 800.0), (1200.0, -1.0)] {
        state.update_size(w, h);
        assert_eq!(state.mode(), LayoutMode::Mobile);
    }
}

#[test]
fn mobile_layout_keyboard_height_changes_preserve_selection() {
    let mut state = MobileLayoutState::new(390.0, 844.0);
    for height in [300.0, 844.0] {
        state.update_size(390.0, height);
        assert_eq!(state.mode(), LayoutMode::Mobile);
        assert_eq!(state.preference(), LayoutPreference::Auto);
    }
}

#[test]
fn mobile_layout_uses_scaled_logical_width_without_scaling_again() {
    // A 1170 physical pixel viewport at scale 3 arrives from Iced as 390 logical pixels.
    let state = MobileLayoutState::new(1170.0 / 3.0, 2532.0 / 3.0);
    assert_eq!(state.mode(), LayoutMode::Mobile);
}

#[test]
fn mobile_layout_main_window_round_trip_preserves_business_and_panel_state() {
    use crate::app::message::{ViewMessage, view::layout};
    use crate::app::state::WorkspaceMode;
    let mut app = crate::app::App::new().0;
    let main = iced::window::Id::unique();
    app.main_window_id = Some(main);
    app.startup_resize_checked = true;
    app.active_session_id = Some("layout-session".into());
    app.split_ratio = 0.63;
    app.work_sidebar_width = 321.0;
    app.navigation_workspace.work_split_ratio = 0.57;
    app.settings_panel_width = 411.0;
    app.file_manager_width = 299.0;
    for mode in [WorkspaceMode::Work, WorkspaceMode::Code] {
        app.navigation_mode.workspace_mode = mode;
        for width in [390.0, 1200.0] {
            let _ = layout::update(&mut app, ViewMessage::WindowResized(main, width, 800.0));
            assert_eq!(app.mobile_layout.mode(), if width < 768.0 {
                LayoutMode::Mobile
            } else { LayoutMode::Desktop });
            assert_eq!(app.navigation_mode.workspace_mode, mode);
            assert_eq!(app.active_session_id.as_deref(), Some("layout-session"));
            assert_eq!((app.split_ratio, app.work_sidebar_width, app.navigation_workspace.work_split_ratio),
                (0.63, 321.0, 0.57));
            assert_eq!((app.settings_panel_width, app.file_manager_width), (411.0, 299.0));
        }
    }
    let auxiliary = iced::window::Id::unique();
    app.task_pet_window_id = Some(auxiliary);
    let _ = layout::update(&mut app, ViewMessage::WindowResized(auxiliary, 200.0, 200.0));
    assert_eq!(app.mobile_layout.mode(), LayoutMode::Desktop);
    assert_eq!(app.window_size, (1200.0, 800.0));
}
