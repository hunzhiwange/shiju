//! 验证差异行选择、拖拽范围、悬停状态及其匹配优先级。
//! 用文件名、差异侧、反向区间和注释草稿组合约束选择判定。

use iced::Color;

use crate::app::state::{GitDiffCommentDraft, GitDiffLineRange, GitDiffSelectedLine};

#[test]
fn task_714_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("selection_tests.rs"));
}

fn app() -> crate::app::App {
    crate::app::App::new().0
}

fn range(file: &str, start: usize, end: usize, is_old: bool) -> GitDiffLineRange {
    GitDiffLineRange { file: file.to_string(), start, end, is_old }
}

#[test]
fn selected_border_uses_expected_highlight_style() {
    let border = super::selection::selected_border();

    assert_eq!(border.width, 1.0);
    assert_eq!(border.color, Color::from_rgba8(255, 210, 0, 0.95));
}

#[test]
fn mix_color_clamps_ratio_and_forces_opaque_alpha() {
    let a = Color::from_rgba(0.2, 0.4, 0.6, 0.3);
    let b = Color::from_rgba(0.8, 0.2, 0.0, 0.7);

    assert_eq!(super::selection::mix_color(a, b, -1.0), Color::from_rgba(0.2, 0.4, 0.6, 1.0));
    assert_eq!(super::selection::mix_color(a, b, 2.0), Color::from_rgba(0.8, 0.2, 0.0, 1.0));

    let mixed = super::selection::mix_color(a, b, 0.5);
    assert!((mixed.r - 0.5).abs() < f32::EPSILON);
    assert!((mixed.g - 0.3).abs() < f32::EPSILON);
    assert!((mixed.b - 0.3).abs() < f32::EPSILON);
    assert_eq!(mixed.a, 1.0);
}

#[test]
fn range_selection_checks_file_side_and_reversed_bounds() {
    let mut app = app();
    app.git_diff_selected_range = Some(range("src/lib.rs", 8, 4, false));

    assert!(super::selection::is_range_selected(&app, "src/lib.rs", 4, false));
    assert!(super::selection::is_range_selected(&app, "src/lib.rs", 6, false));
    assert!(super::selection::is_range_selected(&app, "src/lib.rs", 8, false));
    assert!(!super::selection::is_range_selected(&app, "src/lib.rs", 3, false));
    assert!(!super::selection::is_range_selected(&app, "src/lib.rs", 6, true));
    assert!(!super::selection::is_range_selected(&app, "src/main.rs", 6, false));
}

#[test]
fn range_selection_supports_forward_and_single_line_bounds() {
    let mut app = app();
    app.git_diff_selected_range = Some(range("src/lib.rs", 4, 8, false));

    assert!(super::selection::is_range_selected(&app, "src/lib.rs", 4, false));
    assert!(super::selection::is_range_selected(&app, "src/lib.rs", 8, false));

    app.git_diff_selected_range = Some(range("src/lib.rs", 6, 6, true));
    assert!(super::selection::is_range_selected(&app, "src/lib.rs", 6, true));
    assert!(!super::selection::is_range_selected(&app, "src/lib.rs", 5, true));
    assert!(!super::selection::is_range_selected(&app, "src/lib.rs", 7, true));
}

#[test]
fn range_selection_is_inactive_without_a_range() {
    let app = app();

    assert!(!super::selection::is_range_selected(&app, "src/lib.rs", 1, false));
}

#[test]
fn drag_range_has_priority_over_selected_range_and_comment_draft() {
    let mut app = app();
    app.git_diff_selected_range = Some(range("selected.rs", 1, 3, false));
    app.git_diff_comment_draft = Some(GitDiffCommentDraft {
        range: range("draft.rs", 10, 11, true),
        editor: iced::widget::text_editor::Content::new(),
    });
    app.git_diff_drag_range = Some(range("drag.rs", 20, 21, false));

    assert!(super::selection::is_range_selected(&app, "drag.rs", 20, false));
    assert!(!super::selection::is_range_selected(&app, "selected.rs", 2, false));
    assert!(!super::selection::is_range_selected(&app, "draft.rs", 10, true));

    app.git_diff_drag_range = None;
    assert!(super::selection::is_range_selected(&app, "selected.rs", 2, false));

    app.git_diff_selected_range = None;
    assert!(super::selection::is_range_selected(&app, "draft.rs", 10, true));
}

#[test]
fn diff_selected_combines_active_range_and_explicit_diff_lines() {
    let mut app = app();
    app.git_diff_selected_range = Some(range("src/lib.rs", 1, 2, false));
    app.git_diff_selected_lines.push(GitDiffSelectedLine {
        file: "src/lib.rs".to_string(),
        line: 9,
        is_old: true,
        text: "removed".to_string(),
    });

    let render_ctx = super::DiffRenderCtx::new(&app);

    assert!(super::selection::is_diff_selected(&app, &render_ctx, "src/lib.rs", 2, false));
    assert!(super::selection::is_diff_selected(&app, &render_ctx, "src/lib.rs", 9, true));
    assert!(!super::selection::is_diff_selected(&app, &render_ctx, "src/lib.rs", 9, false));
}

#[test]
fn hover_matches_file_line_and_side_only() {
    let mut app = app();
    app.git_diff_hovered_line = Some(("src/lib.rs".to_string(), 12, true));

    assert!(super::selection::is_diff_hovered(&app, "src/lib.rs", 12, true));
    assert!(!super::selection::is_diff_hovered(&app, "src/lib.rs", 12, false));
    assert!(!super::selection::is_diff_hovered(&app, "src/lib.rs", 11, true));
    assert!(!super::selection::is_diff_hovered(&app, "src/main.rs", 12, true));
}

#[test]
fn hover_is_inactive_without_a_hovered_line() {
    let app = app();

    assert!(!super::selection::is_diff_hovered(&app, "src/lib.rs", 12, true));
}
