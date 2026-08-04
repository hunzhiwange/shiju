use crate::app::App;
use crate::app::state::ToolDetailDialog;
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, layout, widget::Tree};
use iced::widget::{Id, text_editor};
use iced::{Font, Pixels, Size};

fn test_app() -> App {
    let (app, _task) = App::new();
    app
}

fn contains_full_width_bottom_panel(layout: Layout<'_>, viewport: Size, panel_height: f32) -> bool {
    let bounds = layout.bounds();
    let matches_panel = bounds.x.abs() < 0.001
        && (bounds.y + bounds.height - viewport.height).abs() < 0.001
        && (bounds.width - viewport.width).abs() < 0.001
        && (bounds.height - panel_height).abs() < 0.001;

    matches_panel
        || layout
            .children()
            .any(|child| contains_full_width_bottom_panel(child, viewport, panel_height))
}

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("layout_tests"));
}

#[test]
fn base_layout_builds_with_settings_sidebar_and_bottom_panel() {
    let mut app = test_app();
    app.show_settings = true;
    app.terminal.is_visible = true;

    let element = super::build_base_layout(&app, 476.0, 56.0, 0.6, 12.0, 0.0, 6.0, 0.0);

    std::hint::black_box(element);
}

#[tokio::test(flavor = "multi_thread")]
async fn base_layout_places_terminal_across_the_full_bottom_edge() {
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let mut app = test_app();
    app.show_settings = true;
    app.terminal.is_visible = true;
    app.terminal.tabs.clear();
    app.terminal.active_id = None;
    app.terminal.height = 173.0;

    let mut element = super::build_base_layout(&app, 476.0, 56.0, 0.6, 12.0, 0.0, 6.0, 0.0);
    let viewport = Size::new(1_200.0, 800.0);
    let limits = layout::Limits::new(Size::ZERO, viewport);
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(&mut tree, &renderer, &limits);

    assert!(
        contains_full_width_bottom_panel(Layout::new(&node), viewport, app.terminal.height),
        "terminal panel should span the complete bottom edge"
    );

    let root = Layout::new(&node);
    let mut sections = root.children();
    let workspace = sections.next().expect("layout should contain the workspace");
    let terminal = sections.next().expect("layout should contain the terminal");
    let workspace_content =
        workspace.children().next().expect("workspace container should contain its content");
    let gap = workspace.bounds().y + workspace.bounds().height
        - (workspace_content.bounds().y + workspace_content.bounds().height);
    assert!(
        (gap - super::WORKSPACE_BOTTOM_GAP).abs() < 0.001,
        "expected a {}px terminal top gap, got {gap}px",
        super::WORKSPACE_BOTTOM_GAP
    );
    assert!(
        (terminal.bounds().y - (workspace.bounds().y + workspace.bounds().height)).abs() < 0.001,
        "terminal should not add an outer top gap"
    );
}

#[test]
fn base_layout_builds_without_settings_sidebar_or_bottom_panel() {
    let mut app = test_app();
    app.show_settings = false;
    app.terminal.is_visible = false;

    let element = super::build_base_layout(&app, 476.0, 56.0, 0.6, 12.0, 0.0, 6.0, 0.0);

    std::hint::black_box(element);
}

#[test]
fn base_layout_builds_hovered_recent_project_overlay() {
    let mut app = test_app();
    app.show_settings = false;
    app.hovered_recent_project = Some("/tmp/vibe-window-hovered".to_string());

    let element = super::build_base_layout(&app, 476.0, 56.0, 0.6, 12.0, 2.0, 6.0, 0.0);

    std::hint::black_box(element);
}

#[test]
fn base_layout_clamps_sidebar_overlay_width_inputs() {
    let mut app = test_app();
    app.show_settings = false;
    app.hovered_recent_project = Some("/tmp/vibe-window-hovered".to_string());

    let element = super::build_base_layout(&app, 40.0, 80.0, 1.25, 0.0, 1.0, 0.0, 0.0);

    std::hint::black_box(element);
}

#[test]
fn base_layout_builds_chat_fullscreen_without_bottom_panel() {
    let mut app = test_app();
    app.chat_panel_fullscreen = true;
    app.terminal.is_visible = true;

    let element = super::build_base_layout(&app, 476.0, 56.0, 0.6, 12.0, 0.0, 6.0, 0.0);

    std::hint::black_box(element);
}

#[test]
fn base_layout_builds_git_fullscreen_without_bottom_panel() {
    let mut app = test_app();
    app.git_diff_fullscreen = true;
    app.terminal.is_visible = true;

    let element = super::build_base_layout(&app, 476.0, 56.0, 0.6, 12.0, 0.0, 6.0, 0.0);

    std::hint::black_box(element);
}

#[test]
fn base_layout_builds_with_tool_detail_overlay() {
    let mut app = test_app();
    app.tool_detail_dialog = Some(ToolDetailDialog {
        msg_idx: 1,
        tool_idx: 2,
        title: "Tool output".to_string(),
        content: "line 1\nline 2".to_string(),
        editor: text_editor::Content::with_text("line 1\nline 2"),
        editor_id: Id::unique(),
        context_menu_open: false,
        context_menu_pos: None,
        scroll_top_line: 0.0,
        scroll_remainder: 0.0,
        viewport_height: 100.0,
    });

    let element = super::build_base_layout(&app, 476.0, 56.0, 0.6, 12.0, 0.0, 6.0, 0.0);

    std::hint::black_box(element);
}

#[test]
fn work_layout_code_builder_still_accepts_code_panel_state() {
    let mut app = test_app();
    app.show_diff = true;
    app.show_file_manager = true;
    app.terminal.is_visible = true;

    let element = super::build_base_layout(&app, 476.0, 56.0, 0.6, 12.0, 0.0, 6.0, 0.0);

    std::hint::black_box(element);
}

#[test]
fn drag_badge_layer_builds_empty_layer() {
    let app = test_app();

    let element = super::drag_badge_layer(&app);

    std::hint::black_box(element);
}

#[test]
fn drag_badge_layer_builds_active_badge() {
    let mut app = test_app();
    app.project_path = Some("/tmp/project".to_string());
    app.dragging_file_paths =
        vec!["/tmp/project/src/main.rs".to_string(), "/tmp/project/README.md".to_string()];
    app.cursor_position = iced::Point::new(9_999.0, -20.0);
    app.window_size = (180.0, 36.0);

    let element = super::drag_badge_layer(&app);

    std::hint::black_box(element);
}
