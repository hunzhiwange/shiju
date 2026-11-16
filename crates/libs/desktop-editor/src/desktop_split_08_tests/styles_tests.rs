use crate::components::preview_panel::styles::{
    PreviewTabsLayout, estimate_preview_tabs_viewport_width, should_show_preview_tabs_scrollbar,
};
fn layout() -> PreviewTabsLayout {
    PreviewTabsLayout {
        window_width: 1200.0,
        show_file_manager: false,
        file_manager_width: 240.0,
        show_diff: false,
        split_ratio: 0.5,
    }
}
fn assert_close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.001, "actual={actual}, expected={expected}");
}

#[test]
fn viewport_width_accounts_for_file_manager_and_diff_split() {
    let mut app = layout();
    app.window_width = 1000.0;
    app.show_file_manager = false;
    app.show_diff = false;

    assert_eq!(estimate_preview_tabs_viewport_width(app), 982.0);

    app.show_file_manager = true;
    app.file_manager_width = 240.0;
    assert_eq!(estimate_preview_tabs_viewport_width(app), 734.0);

    app.show_diff = true;
    app.split_ratio = 0.75;
    assert_close(estimate_preview_tabs_viewport_width(app), 168.0);

    app.split_ratio = 0.01;
    assert_close(estimate_preview_tabs_viewport_width(app), 577.2);
}

#[test]
fn viewport_width_clamps_small_layouts_and_diff_ratio() {
    let mut app = layout();
    app.show_file_manager = false;
    app.show_diff = false;
    app.window_width = 0.0;
    assert_eq!(estimate_preview_tabs_viewport_width(app), 1.0);

    app.window_width = 1000.0;
    app.show_file_manager = true;
    app.file_manager_width = 2000.0;
    assert_eq!(estimate_preview_tabs_viewport_width(app), 1.0);

    app.show_file_manager = false;
    app.show_diff = true;
    app.split_ratio = 2.0;
    assert_close(estimate_preview_tabs_viewport_width(app), 180.4);

    app.split_ratio = -1.0;
    assert_close(estimate_preview_tabs_viewport_width(app), 775.6);
}

#[test]
fn scrollbar_visibility_tracks_estimated_tab_widths() {
    let mut app = layout();
    let mut titles = vec![];
    app.window_width = 1200.0;
    assert!(!should_show_preview_tabs_scrollbar(app, titles.iter().copied()));

    titles = vec!["main.rs"];
    assert!(!should_show_preview_tabs_scrollbar(app, titles.iter().copied()));

    app.window_width = 160.0;
    titles = vec!["very-long-file-name-that-will-overflow-the-small-preview-tabs.rs"];
    assert!(should_show_preview_tabs_scrollbar(app, titles.iter().copied()));
}

#[test]
fn scrollbar_visibility_uses_strict_boundary_and_tab_spacing() {
    let mut app = layout();
    let mut titles;
    app.show_file_manager = false;
    app.show_diff = false;
    titles = vec![""];

    app.window_width = 116.0;
    assert!(!should_show_preview_tabs_scrollbar(app, titles.iter().copied()));
    app.window_width = 115.5;
    assert!(should_show_preview_tabs_scrollbar(app, titles.iter().copied()));

    titles = vec!["", ""];
    app.window_width = 214.0;
    assert!(!should_show_preview_tabs_scrollbar(app, titles.iter().copied()));
    app.window_width = 213.5;
    assert!(should_show_preview_tabs_scrollbar(app, titles.iter().copied()));
}
