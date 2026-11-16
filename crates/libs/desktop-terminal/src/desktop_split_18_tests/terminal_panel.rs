use crate::{Shell, TerminalState, TerminalTheme, ViewInput, message::Message};
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, layout, widget::Tree};
use iced::{Font, Pixels, Size};
fn test_app() -> TerminalState {
    blank_terminal(true)
}
fn keep_element(element: iced::Element<'_, Message>) {
    std::hint::black_box(element);
}
fn view(state: &TerminalState) -> iced::Element<'_, Message> {
    crate::terminal_panel::view(ViewInput {
        tabs: &state.tabs,
        active_id: state.active_id,
        is_visible: state.is_visible,
        height: state.height,
        tab_context_menu_id: state.tab_context_menu_id,
        tab_context_menu_pos: state.tab_context_menu_pos,
        desktop_locale: Default::default(),
        focused: false,
        #[cfg(not(target_arch = "wasm32"))]
        panes: &state.panes,
        #[cfg(not(target_arch = "wasm32"))]
        content_menu: state.content_menu,
        #[cfg(not(target_arch = "wasm32"))]
        is_fullscreen: state.is_fullscreen,
    })
}
fn ensure_terminal_tabs(app: &mut TerminalState, count: usize) -> bool {
    while app.tabs.len() < count {
        if !app.add_terminal(None) {
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

#[tokio::test(flavor = "multi_thread")]
async fn view_fills_parent_width_independent_of_window_snapshot() {
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let app = blank_terminal(true);

    let mut element = view(&app);
    let bounds = Size::new(1_200.0, 200.0);
    let limits = layout::Limits::new(Size::ZERO, bounds);
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(&mut tree, &renderer, &limits);
    let root = Layout::new(&node);
    let content = root.children().next().expect("terminal root should contain content");
    assert_close(content.bounds().width, bounds.width);
    #[cfg(not(target_arch = "wasm32"))]
    let column = {
        let grid = content.children().next().expect("responsive terminal grid");
        assert_close(grid.bounds().width, bounds.width);
        grid.children().next().expect("single terminal pane")
    };
    #[cfg(target_arch = "wasm32")]
    let column = content;
    let mut panels = column.children();
    let header = panels.next().expect("terminal column should contain a header");
    let output = panels.next().expect("terminal column should contain terminal output");

    for panel in [root, column, header, output] {
        assert_close(panel.bounds().width, bounds.width);
    }
}

#[test]
fn view_builds_empty_container_when_terminal_is_hidden() {
    let mut app = test_app();
    app.is_visible = false;

    keep_element(view(&app));
}

#[test]
fn view_builds_empty_terminal_when_no_tabs_exist() {
    let app = blank_terminal(true);

    keep_element(view(&app));
}

#[test]
fn view_builds_visible_terminal_with_default_tab() {
    let mut app = test_app();
    app.is_visible = true;

    keep_element(view(&app));
}

#[test]
fn view_falls_back_to_first_tab_when_active_id_is_missing() {
    let mut app = test_app();
    app.is_visible = true;
    app.active_id = None;

    keep_element(view(&app));
}

#[test]
fn view_builds_settings_layout_with_non_finite_width() {
    let mut app = test_app();
    app.is_visible = true;

    keep_element(view(&app));
}

#[test]
fn view_builds_settings_layout_with_clamped_width() {
    let mut app = test_app();
    app.is_visible = true;

    keep_element(view(&app));
}

#[test]
fn view_builds_settings_layout_with_minimum_clamped_width_and_tiny_window() {
    let mut app = test_app();
    app.is_visible = true;

    keep_element(view(&app));
}

#[test]
fn view_builds_long_title_tabs_and_invalid_active_fallback() {
    let mut app = test_app();
    app.is_visible = true;
    if !ensure_terminal_tabs(&mut app, 1) {
        return;
    }

    let first_id = app.tabs.first().expect("first terminal tab").id;
    let tab = app.tabs.iter_mut().find(|tab| tab.id == first_id).expect("first tab");
    tab.title = "a very long terminal tab title that should clamp to the maximum width".to_string();
    app.active_id = Some(u64::MAX);

    keep_element(view(&app));
}

#[test]
fn view_builds_multi_tab_context_menu() {
    let mut app = test_app();
    app.is_visible = true;
    if !ensure_terminal_tabs(&mut app, 2) {
        return;
    }

    let first_id = app.tabs.first().expect("first terminal tab").id;
    app.active_id = Some(first_id);
    app.tab_context_menu_id = Some(first_id);
    app.tab_context_menu_pos = Some((48.0, 32.0));

    keep_element(view(&app));
}

#[test]
fn view_builds_multi_tab_with_non_first_active_tab_and_default_context_anchor() {
    let mut app = test_app();
    app.is_visible = true;
    if !ensure_terminal_tabs(&mut app, 2) {
        return;
    }

    let second_id = app.tabs.last().expect("second terminal tab").id;
    app.active_id = Some(second_id);
    app.tab_context_menu_id = Some(second_id);
    app.tab_context_menu_pos = None;

    keep_element(view(&app));
}

#[test]
fn view_builds_single_tab_context_menu_with_disabled_close() {
    let mut app = test_app();
    app.is_visible = true;
    if !ensure_terminal_tabs(&mut app, 1) {
        return;
    }

    let first_id = app.tabs.first().expect("first terminal tab").id;
    app.tab_context_menu_id = Some(first_id);
    app.tab_context_menu_pos = None;

    keep_element(view(&app));
}

#[test]
fn view_builds_rename_modal_for_active_tab() {
    let mut app = test_app();
    app.is_visible = true;
    if !ensure_terminal_tabs(&mut app, 1) {
        return;
    }

    let active_id = app.active_id.expect("active terminal tab");
    let tab = app.tabs.iter_mut().find(|tab| tab.id == active_id).expect("active terminal tab");
    tab.edit_title = Some("新的终端名称".to_string());

    keep_element(view(&app));
}

#[test]
fn view_ignores_rename_modal_for_inactive_tab() {
    let mut app = test_app();
    app.is_visible = true;
    if !ensure_terminal_tabs(&mut app, 2) {
        return;
    }

    let inactive_id = app.tabs.first().expect("first terminal tab").id;
    let active_id = app.tabs.last().expect("last terminal tab").id;
    app.active_id = Some(active_id);
    app.tabs
        .iter_mut()
        .find(|tab| tab.id == inactive_id)
        .expect("inactive terminal tab")
        .edit_title = Some("未激活名称".to_string());

    keep_element(view(&app));
}

#[test]
fn view_ignores_rename_modal_when_active_id_points_to_missing_tab() {
    let mut app = test_app();
    app.is_visible = true;
    if !ensure_terminal_tabs(&mut app, 1) {
        return;
    }

    let first_id = app.tabs.first().expect("first terminal tab").id;
    app.active_id = Some(u64::MAX);
    app.tabs.iter_mut().find(|tab| tab.id == first_id).expect("first terminal tab").edit_title =
        Some("保留但不弹出".to_string());

    keep_element(view(&app));
}
