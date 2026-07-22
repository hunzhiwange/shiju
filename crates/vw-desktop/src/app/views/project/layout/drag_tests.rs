use iced::{Color, Point, Theme};

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("drag_tests"));
}

#[test]
fn display_drag_path_uses_project_relative_path() {
    let path = "/workspace/app/src/main.rs";

    let display = super::display_drag_path(path, Some("/workspace/app"));

    assert_eq!(display, "src/main.rs");
}

#[test]
fn display_drag_path_handles_project_root_itself() {
    let path = "/workspace/app";

    let display = super::display_drag_path(path, Some("/workspace/app"));

    assert_eq!(display, "");
}

#[test]
fn display_drag_path_keeps_absolute_path_when_outside_project() {
    let path = "/tmp/notes.md";

    let display = super::display_drag_path(path, Some("/workspace/app"));

    assert_eq!(display, "/tmp/notes.md");
}

#[test]
fn display_drag_path_does_not_strip_partial_component_prefix() {
    let path = "/workspace/application/src/main.rs";

    let display = super::display_drag_path(path, Some("/workspace/app"));

    assert_eq!(display, "/workspace/application/src/main.rs");
}

#[test]
fn display_drag_path_normalizes_windows_separators_without_project_root() {
    let path = r"C:\workspace\app\src\main.rs";

    let display = super::display_drag_path(path, None);

    assert_eq!(display, "C:/workspace/app/src/main.rs");
}

#[test]
fn display_drag_path_normalizes_windows_separators_after_project_strip() {
    let path = "/workspace/app/src\\main.rs";

    let display = super::display_drag_path(path, Some("/workspace/app"));

    assert_eq!(display, "src/main.rs");
}

#[test]
fn clamp_badge_position_offsets_top_chrome() {
    let cursor = Point::new(320.0, 180.0);

    let (x, y) = super::clamp_badge_position(cursor, (1200.0, 800.0));

    assert_eq!(x, 320.0);
    assert_eq!(
        y,
        cursor.y
            - crate::app::components::top_bar::TOP_BAR_HEIGHT
            - crate::app::components::tab_bar::TAB_BAR_HEIGHT
    );
}

#[test]
fn clamp_badge_position_never_goes_negative() {
    let cursor = Point::new(-20.0, -30.0);

    let (x, y) = super::clamp_badge_position(cursor, (1200.0, 800.0));

    assert_eq!((x, y), (0.0, 0.0));
}

#[test]
fn clamp_badge_position_respects_badge_bounds() {
    let cursor = Point::new(2_000.0, 2_000.0);

    let (x, y) = super::clamp_badge_position(cursor, (500.0, 120.0));

    assert_eq!((x, y), (240.0, 78.0));
}

#[test]
fn clamp_badge_position_handles_tiny_window() {
    let cursor = Point::new(20.0, 80.0);

    let (x, y) = super::clamp_badge_position(cursor, (120.0, 30.0));

    assert_eq!((x, y), (0.0, 0.0));
}

#[test]
fn clamp_badge_position_allows_origin_when_window_matches_badge_size() {
    let cursor = Point::new(30.0, 160.0);

    let (x, y) = super::clamp_badge_position(cursor, (260.0, 42.0));

    assert_eq!((x, y), (0.0, 0.0));
}

#[test]
fn drag_extra_count_text_is_empty_for_zero_or_one_path() {
    assert_eq!(super::drag_extra_count_text(0), "");
    assert_eq!(super::drag_extra_count_text(1), "");
}

#[test]
fn drag_extra_count_text_shows_remaining_path_count() {
    assert_eq!(super::drag_extra_count_text(3), "+2");
}

#[test]
fn badge_marker_text_style_uses_theme_primary_color() {
    let theme = Theme::Dark;
    let style = super::badge_marker_text_style(&theme);

    assert_eq!(style.color, Some(theme.palette().primary));
}

#[test]
fn badge_path_text_style_uses_theme_text_color() {
    let theme = Theme::Dark;
    let style = super::badge_path_text_style(&theme);

    assert_eq!(style.color, Some(theme.palette().text));
}

#[test]
fn badge_extra_text_style_uses_secondary_color() {
    let theme = Theme::Dark;
    let style = super::badge_extra_text_style(&theme);

    assert_eq!(style.color, Some(theme.extended_palette().secondary.base.color));
}

#[test]
fn badge_container_style_uses_dark_theme_surface_and_shadow() {
    let theme = Theme::Dark;
    let palette = theme.extended_palette();

    let style = super::badge_container_style(&theme);

    assert_eq!(style.background, Some(palette.background.base.color.into()));
    assert_eq!(style.border.color, palette.background.strong.color);
    assert_eq!(style.border.width, 1.0);
    assert_eq!(style.shadow.color, Color::BLACK.scale_alpha(0.12));
}

#[test]
fn drag_badge_layer_builds_empty_layer_without_paths() {
    let (app, _task) = crate::app::App::new();

    let _element = super::drag_badge_layer(&app);
}

#[test]
fn drag_badge_layer_builds_badge_for_dragging_paths() {
    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some("/workspace/app".to_string());
    app.dragging_file_paths =
        vec!["/workspace/app/src/main.rs".to_string(), "/workspace/app/README.md".to_string()];
    app.cursor_position = Point::new(360.0, 220.0);
    app.window_size = (960.0, 640.0);

    let _element = super::drag_badge_layer(&app);
}
