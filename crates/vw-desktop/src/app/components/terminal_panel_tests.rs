use crate::app::terminal::{Shell, TerminalState, TerminalTheme};
use crate::app::{App, Message};

fn test_app() -> App {
    App::new().0
}

fn keep_element(element: iced::Element<'_, Message>) {
    std::hint::black_box(element);
}

fn blank_terminal(is_visible: bool) -> TerminalState {
    TerminalState::blank_with_settings(
        is_visible,
        Shell::Bash,
        TerminalTheme::System,
        "JetBrains Mono".to_string(),
        13.0,
        200.0,
    )
}

fn assert_close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.001, "expected {expected}, got {actual}");
}

#[test]
fn available_terminal_width_uses_only_left_rail_when_settings_are_hidden() {
    assert_close(super::terminal_panel::available_terminal_width(1200.0, false, 370.0), 1144.0);
    assert_close(super::terminal_panel::available_terminal_width(-100.0, false, 370.0), 0.0);
}

#[test]
fn available_terminal_width_clamps_finite_settings_width() {
    assert_close(super::terminal_panel::available_terminal_width(1000.0, true, 20.0), 849.6);
    assert_close(super::terminal_panel::available_terminal_width(2000.0, true, 1200.0), 1489.6);
}

#[test]
fn available_terminal_width_defaults_non_finite_settings_width() {
    for settings_width in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert_close(
            super::terminal_panel::available_terminal_width(1000.0, true, settings_width),
            747.6,
        );
    }
}

#[test]
fn available_terminal_width_handles_non_finite_window_width_without_underflow() {
    assert_close(super::terminal_panel::available_terminal_width(f32::NAN, false, 370.0), 0.0);
    assert!(
        super::terminal_panel::available_terminal_width(f32::INFINITY, true, 370.0).is_infinite()
    );
}

#[test]
fn terminal_tab_width_counts_unicode_scalars_and_clamps_bounds() {
    assert_close(super::terminal_panel::terminal_tab_width(""), 92.0);
    assert_close(super::terminal_panel::terminal_tab_width("终端标签名称一二"), 129.2);
    assert_close(super::terminal_panel::terminal_tab_width(&"界".repeat(40)), 260.0);
}

#[test]
fn terminal_tab_width_counts_combining_marks_and_emoji_as_scalars() {
    assert_close(super::terminal_panel::terminal_tab_width("e\u{301}"), 92.0);
    assert_close(super::terminal_panel::terminal_tab_width("😀😀😀😀"), 95.6);
}

#[test]
fn terminal_tab_active_state_defaults_to_first_or_matches_requested_id() {
    assert!(super::terminal_panel::terminal_tab_is_active(None, 7, 0));
    assert!(!super::terminal_panel::terminal_tab_is_active(None, 8, 1));
    assert!(!super::terminal_panel::terminal_tab_is_active(Some(8), 7, 0));
    assert!(super::terminal_panel::terminal_tab_is_active(Some(8), 8, 1));
}

#[test]
fn active_terminal_tab_index_prefers_match_then_falls_back_to_first() {
    assert_eq!(
        super::terminal_panel::active_terminal_tab_index(None, std::iter::empty::<u64>()),
        None,
    );
    assert_eq!(super::terminal_panel::active_terminal_tab_index(None, [7, 8]), Some(0));
    assert_eq!(super::terminal_panel::active_terminal_tab_index(Some(8), [7, 8]), Some(1));
    assert_eq!(super::terminal_panel::active_terminal_tab_index(Some(9), [7, 8]), Some(0));
}

#[test]
fn active_terminal_tab_index_returns_first_duplicate_match() {
    assert_eq!(super::terminal_panel::active_terminal_tab_index(Some(7), [7, 8, 7]), Some(0),);
}

#[test]
fn view_builds_empty_container_when_terminal_is_hidden() {
    let mut app = test_app();
    app.terminal.is_visible = false;

    keep_element(super::terminal_panel::view(&app));
}

#[test]
fn view_builds_empty_terminal_when_no_tabs_exist() {
    let mut app = test_app();
    app.terminal = blank_terminal(true);

    keep_element(super::terminal_panel::view(&app));
}

#[test]
fn view_builds_visible_terminal_with_default_tab() {
    let mut app = test_app();
    app.terminal.is_visible = true;
    app.window_size = (1200.0, 800.0);
    app.show_settings = false;

    keep_element(super::terminal_panel::view(&app));
}

#[test]
fn view_falls_back_to_first_tab_when_active_id_is_missing() {
    let mut app = test_app();
    app.terminal.is_visible = true;
    app.terminal.active_id = None;
    app.window_size = (0.0, 800.0);

    keep_element(super::terminal_panel::view(&app));
}

#[test]
fn view_builds_settings_layout_with_non_finite_width() {
    let mut app = test_app();
    app.terminal.is_visible = true;
    app.show_settings = true;
    app.settings_panel_width = f32::NAN;
    app.window_size = (420.0, 700.0);

    keep_element(super::terminal_panel::view(&app));
}

#[test]
fn view_builds_settings_layout_with_clamped_width() {
    let mut app = test_app();
    app.terminal.is_visible = true;
    app.show_settings = true;
    app.settings_panel_width = 1200.0;
    app.window_size = (980.0, 700.0);

    keep_element(super::terminal_panel::view(&app));
}

#[test]
fn view_builds_settings_layout_with_minimum_clamped_width_and_tiny_window() {
    let mut app = test_app();
    app.terminal.is_visible = true;
    app.show_settings = true;
    app.settings_panel_width = 20.0;
    app.window_size = (24.0, 700.0);

    keep_element(super::terminal_panel::view(&app));
}

#[test]
fn view_builds_long_title_tabs_and_invalid_active_fallback() {
    let mut app = test_app();
    app.terminal.is_visible = true;

    let first_id = app.terminal.tabs.first().expect("first terminal tab").id;
    let tab = app.terminal.tabs.iter_mut().find(|tab| tab.id == first_id).expect("first tab");
    tab.title = "a very long terminal tab title that should clamp to the maximum width".to_string();
    app.terminal.active_id = Some(u64::MAX);

    keep_element(super::terminal_panel::view(&app));
}

#[test]
fn view_builds_multi_tab_context_menu() {
    let mut app = test_app();
    app.terminal.is_visible = true;

    let added = app.terminal.add_terminal(None);
    assert!(added, "expected test terminal tab to be created");

    let first_id = app.terminal.tabs.first().expect("first terminal tab").id;
    app.terminal.active_id = Some(first_id);
    app.terminal.tab_context_menu_id = Some(first_id);
    app.terminal.tab_context_menu_pos = Some((48.0, 32.0));

    keep_element(super::terminal_panel::view(&app));
}

#[test]
fn view_builds_multi_tab_with_non_first_active_tab_and_default_context_anchor() {
    let mut app = test_app();
    app.terminal.is_visible = true;

    let added = app.terminal.add_terminal(None);
    assert!(added, "expected test terminal tab to be created");

    let second_id = app.terminal.tabs.last().expect("second terminal tab").id;
    app.terminal.active_id = Some(second_id);
    app.terminal.tab_context_menu_id = Some(second_id);
    app.terminal.tab_context_menu_pos = None;

    keep_element(super::terminal_panel::view(&app));
}

#[test]
fn view_builds_single_tab_context_menu_with_disabled_close() {
    let mut app = test_app();
    app.terminal.is_visible = true;

    let first_id = app.terminal.tabs.first().expect("first terminal tab").id;
    app.terminal.tab_context_menu_id = Some(first_id);
    app.terminal.tab_context_menu_pos = None;

    keep_element(super::terminal_panel::view(&app));
}

#[test]
fn view_builds_rename_modal_for_active_tab() {
    let mut app = test_app();
    app.terminal.is_visible = true;

    let active_id = app.terminal.active_id.expect("active terminal tab");
    let tab =
        app.terminal.tabs.iter_mut().find(|tab| tab.id == active_id).expect("active terminal tab");
    tab.edit_title = Some("新的终端名称".to_string());

    keep_element(super::terminal_panel::view(&app));
}

#[test]
fn view_ignores_rename_modal_for_inactive_tab() {
    let mut app = test_app();
    app.terminal.is_visible = true;

    let added = app.terminal.add_terminal(None);
    assert!(added, "expected test terminal tab to be created");

    let inactive_id = app.terminal.tabs.first().expect("first terminal tab").id;
    let active_id = app.terminal.tabs.last().expect("last terminal tab").id;
    app.terminal.active_id = Some(active_id);
    app.terminal
        .tabs
        .iter_mut()
        .find(|tab| tab.id == inactive_id)
        .expect("inactive terminal tab")
        .edit_title = Some("未激活名称".to_string());

    keep_element(super::terminal_panel::view(&app));
}

#[test]
fn view_ignores_rename_modal_when_active_id_points_to_missing_tab() {
    let mut app = test_app();
    app.terminal.is_visible = true;

    let first_id = app.terminal.tabs.first().expect("first terminal tab").id;
    app.terminal.active_id = Some(u64::MAX);
    app.terminal
        .tabs
        .iter_mut()
        .find(|tab| tab.id == first_id)
        .expect("first terminal tab")
        .edit_title = Some("保留但不弹出".to_string());

    keep_element(super::terminal_panel::view(&app));
}
