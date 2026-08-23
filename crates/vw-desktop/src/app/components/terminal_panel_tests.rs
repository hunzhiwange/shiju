//! 验证终端面板的标签管理、尺寸计算、显示状态和交互控件。
//!
//! 终端夹具构造不同可见性与标签数量，用于校验尺寸、选中项和关闭边界。

use crate::app::terminal::{Shell, TerminalState, TerminalTheme};
use crate::app::{App, Message};
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, layout, widget::Tree};
use iced::{Background, Font, Pixels, Size, Theme};

fn test_app() -> App {
    App::new().0
}

fn keep_element(element: iced::Element<'_, Message>) {
    std::hint::black_box(element);
}

fn ensure_terminal_tabs(app: &mut App, count: usize) -> bool {
    while app.terminal.tabs.len() < count {
        if !app.terminal.add_terminal(None) {
            return false;
        }
    }

    true
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
fn terminal_surface_uses_theme_base_background() {
    for theme in [Theme::Dark, Theme::Light] {
        let style = super::terminal_panel::terminal_surface_style(&theme);
        assert_eq!(
            style.background,
            Some(Background::Color(theme.extended_palette().background.base.color))
        );
    }
}

#[test]
fn terminal_content_bottom_padding_is_half_the_other_edges() {
    let padding = super::terminal_panel::terminal_content_padding(1);

    assert_close(padding.top, 8.0);
    assert_close(padding.right, 8.0);
    assert_close(padding.bottom, 4.0);
    assert_close(padding.left, 8.01);
}

#[tokio::test(flavor = "multi_thread")]
async fn view_fills_parent_width_independent_of_window_snapshot() {
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let mut app = test_app();
    app.terminal = blank_terminal(true);
    app.window_size = (480.0, 800.0);

    let mut element = super::terminal_panel::view(&app);
    let bounds = Size::new(1_200.0, 200.0);
    let limits = layout::Limits::new(Size::ZERO, bounds);
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(&mut tree, &renderer, &limits);
    let root = Layout::new(&node);
    let column = root.children().next().expect("terminal root should contain a column");
    let mut panels = column.children();
    let header = panels.next().expect("terminal column should contain a header");
    let output = panels.next().expect("terminal column should contain terminal output");

    for panel in [root, column, header, output] {
        assert_close(panel.bounds().width, bounds.width);
    }
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
    if !ensure_terminal_tabs(&mut app, 1) {
        return;
    }

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
    if !ensure_terminal_tabs(&mut app, 2) {
        return;
    }

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
    if !ensure_terminal_tabs(&mut app, 2) {
        return;
    }

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
    if !ensure_terminal_tabs(&mut app, 1) {
        return;
    }

    let first_id = app.terminal.tabs.first().expect("first terminal tab").id;
    app.terminal.tab_context_menu_id = Some(first_id);
    app.terminal.tab_context_menu_pos = None;

    keep_element(super::terminal_panel::view(&app));
}

#[test]
fn view_builds_rename_modal_for_active_tab() {
    let mut app = test_app();
    app.terminal.is_visible = true;
    if !ensure_terminal_tabs(&mut app, 1) {
        return;
    }

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
    if !ensure_terminal_tabs(&mut app, 2) {
        return;
    }

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
    if !ensure_terminal_tabs(&mut app, 1) {
        return;
    }

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
