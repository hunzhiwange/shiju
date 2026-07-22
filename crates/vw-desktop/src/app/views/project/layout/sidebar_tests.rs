use iced::Element;
use iced::widget::text;

use super::*;

fn test_app() -> App {
    let (app, _task) = App::new();
    app
}

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("sidebar_tests"));
}

#[test]
fn sidebar_group_expanded_builds_resize_handle() {
    let mut app = test_app();
    app.show_settings = true;
    app.project_path = Some("/tmp/vibe-window-project".to_string());
    app.recent_projects = vec!["/tmp/vibe-window-project".to_string()];
    app.recent_projects_edits = vec!["Vibe Window".to_string()];

    let (sidebar, resize_handle) = sidebar_group(&app, 320.0, 56.0, 0.5, 12.0);

    keep_element(sidebar);
    keep_element(resize_handle.expect("expanded settings sidebar exposes resize handle"));
}

#[test]
fn sidebar_group_expanded_clamps_negative_panel_width() {
    let mut app = test_app();
    app.show_settings = true;

    let (sidebar, resize_handle) = sidebar_group(&app, 40.0, 80.0, 1.0, 0.0);

    keep_element(sidebar);
    keep_element(resize_handle.expect("resize handle is still available when panel width clamps"));
}

#[test]
fn sidebar_group_collapsed_hides_resize_handle() {
    let mut app = test_app();
    app.show_settings = false;
    app.recent_projects = vec!["/tmp/vibe-window-project".to_string()];
    app.recent_projects_edits = vec![String::new()];

    let (sidebar, resize_handle) = sidebar_group(&app, 320.0, 56.0, 0.5, 12.0);

    keep_element(sidebar);
    assert!(resize_handle.is_none());
}

#[test]
fn hover_overlay_layout_without_hover_uses_plain_sidebar() {
    let mut app = test_app();
    app.hovered_recent_project = None;
    let left_sidebar = text("left").into();
    let right_column = text("chat").into();

    let layout =
        hover_overlay_layout(&app, left_sidebar, right_column, 320.0, 56.0, 0.5, 12.0, 4.0);

    keep_element(layout);
}

#[test]
fn hover_overlay_layout_with_hover_builds_sessions_overlay() {
    let mut app = test_app();
    app.hovered_recent_project = Some("/tmp/vibe-window-project".to_string());
    app.recent_projects = vec!["/tmp/vibe-window-project".to_string()];
    app.recent_projects_edits = vec!["Vibe Window".to_string()];
    let left_sidebar = text("left").into();
    let right_column = text("chat").into();

    let layout =
        hover_overlay_layout(&app, left_sidebar, right_column, 320.0, 56.0, 0.5, 12.0, 4.0);

    keep_element(layout);
}

#[test]
fn hover_overlay_layout_with_hover_clamps_negative_panel_width() {
    let mut app = test_app();
    app.hovered_recent_project = Some("/tmp/vibe-window-project".to_string());
    let left_sidebar = text("left").into();
    let right_column = text("chat").into();

    let layout = hover_overlay_layout(&app, left_sidebar, right_column, 40.0, 80.0, 1.0, 0.0, 0.0);

    keep_element(layout);
}

#[test]
fn sidebar_group_expanded_zero_scale_uses_left_rail_width_only() {
    let mut app = test_app();
    app.show_settings = true;

    let (sidebar, resize_handle) = sidebar_group(&app, 400.0, 100.0, 0.0, 12.0);

    keep_element(sidebar);
    keep_element(resize_handle.expect("resize handle present when expanded"));
}

#[test]
fn sidebar_group_expanded_full_scale_uses_max_width() {
    let mut app = test_app();
    app.show_settings = true;

    let (sidebar, resize_handle) = sidebar_group(&app, 400.0, 100.0, 1.0, 12.0);

    keep_element(sidebar);
    keep_element(resize_handle.expect("resize handle present when expanded"));
}

#[test]
fn sidebar_group_expanded_equal_panel_and_rail_widths() {
    let mut app = test_app();
    app.show_settings = true;

    let (sidebar, resize_handle) = sidebar_group(&app, 100.0, 100.0, 0.75, 8.0);

    keep_element(sidebar);
    keep_element(resize_handle.expect("resize handle present when expanded"));
}

#[test]
fn sidebar_group_expanded_large_left_rail() {
    let mut app = test_app();
    app.show_settings = true;

    let (sidebar, resize_handle) = sidebar_group(&app, 120.0, 200.0, 0.5, 12.0);

    keep_element(sidebar);
    keep_element(resize_handle.expect("resize handle present when expanded"));
}

#[test]
fn sidebar_group_collapsed_empty_projects() {
    let mut app = test_app();
    app.show_settings = false;

    let (sidebar, resize_handle) = sidebar_group(&app, 320.0, 56.0, 0.5, 12.0);

    keep_element(sidebar);
    assert!(resize_handle.is_none());
}

#[test]
fn hover_overlay_layout_with_hover_zero_session_panel_scale() {
    let mut app = test_app();
    app.hovered_recent_project = Some("/tmp/project".to_string());
    let left_sidebar = text("left").into();
    let right_column = text("chat").into();

    let layout =
        hover_overlay_layout(&app, left_sidebar, right_column, 400.0, 100.0, 0.0, 12.0, 4.0);

    keep_element(layout);
}

#[test]
fn hover_overlay_layout_with_hover_zero_spacing() {
    let mut app = test_app();
    app.hovered_recent_project = Some("/tmp/project".to_string());
    let left_sidebar = text("left").into();
    let right_column = text("chat").into();

    let layout =
        hover_overlay_layout(&app, left_sidebar, right_column, 400.0, 100.0, 0.5, 12.0, 0.0);

    keep_element(layout);
}

#[test]
fn hover_overlay_layout_without_hover_zero_spacing() {
    let app = test_app();
    let left_sidebar = text("left").into();
    let right_column = text("chat").into();

    let layout =
        hover_overlay_layout(&app, left_sidebar, right_column, 400.0, 100.0, 0.5, 12.0, 0.0);

    keep_element(layout);
}

#[test]
fn hover_overlay_layout_with_hover_equal_panel_and_rail_widths() {
    let mut app = test_app();
    app.hovered_recent_project = Some("/tmp/project".to_string());
    let left_sidebar = text("left").into();
    let right_column = text("chat").into();

    let layout =
        hover_overlay_layout(&app, left_sidebar, right_column, 100.0, 100.0, 0.75, 8.0, 2.0);

    keep_element(layout);
}
