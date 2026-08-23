//! 验证代码审查差异行着色、补丁渲染与文件折叠视图的边界状态。
//! 三重标记不得误判为普通增删行，无项目或不可用记录则安全退化为空集合。

use super::code_review::{
    build_code_review_view_from_diff, git_diff_by_file, render_unified_line,
    render_unified_patch_for_test, unified_style_for_line, view,
};
use crate::app::App;
use iced::{Color, Length, Size};
use std::collections::HashMap;

fn default_colors() -> (Color, Color) {
    (Color::from_rgb8(0x0D, 0x11, 0x17), Color::from_rgb8(0xC9, 0xD1, 0xD9))
}

#[test]
fn unified_style_for_line_uses_default_colors_for_each_metadata_prefix() {
    for line in [
        "diff --git a/src/lib.rs b/src/lib.rs",
        "index 1234567..89abcde 100644",
        "--- a/src/lib.rs",
        "+++ b/src/lib.rs",
    ] {
        assert_eq!(unified_style_for_line(line), default_colors(), "metadata line: {line}");
    }
}

#[test]
fn unified_style_for_line_uses_hunk_colors() {
    assert_eq!(
        unified_style_for_line("@@ -10,2 +10,3 @@ fn example()"),
        (Color::from_rgb8(0x17, 0x1B, 0x22), Color::from_rgb8(0x79, 0xC0, 0xFF))
    );
}

#[test]
fn unified_style_for_line_distinguishes_changes_from_triple_marker_lines() {
    let added = (Color::from_rgb8(0x10, 0x3B, 0x1A), Color::from_rgb8(0xC0, 0xF2, 0xD0));
    let deleted = (Color::from_rgb8(0x4C, 0x1F, 0x1F), Color::from_rgb8(0xFF, 0xC2, 0xC2));

    assert_eq!(unified_style_for_line("+added"), added);
    assert_eq!(unified_style_for_line("++also added"), added);
    assert_eq!(unified_style_for_line("-deleted"), deleted);
    assert_eq!(unified_style_for_line("--also deleted"), deleted);
    assert_eq!(unified_style_for_line("+++not-a-change"), default_colors());
    assert_eq!(unified_style_for_line("---not-a-change"), default_colors());
}

#[test]
fn unified_style_for_line_defaults_empty_context_and_unicode_lines() {
    for line in ["", " context", "context", " 上下文"] {
        assert_eq!(unified_style_for_line(line), default_colors(), "context line: {line}");
    }
}

#[test]
fn render_unified_line_constructs_every_supported_line_kind() {
    for line in ["diff --git a/a.rs b/a.rs", "@@ -1 +1 @@", "+新增", "-删除", " context"] {
        let _element = render_unified_line(line.to_string());
    }
}

#[test]
fn render_unified_patch_empty_input_builds_fill_width_empty_column() {
    let element = render_unified_patch_for_test(String::new());

    assert_eq!(element.as_widget().size(), Size::new(Length::Fill, Length::Shrink));
    let container_children = element.as_widget().children();
    assert!(container_children.is_empty());
}

#[test]
fn render_unified_patch_renders_one_row_for_each_nonempty_patch_line() {
    let patch = [
        "diff --git a/src/lib.rs b/src/lib.rs",
        "@@ -1,2 +1,2 @@",
        "-let old = true;",
        "+let 新值 = true;",
        " context",
    ]
    .join("\n");

    let element = render_unified_patch_for_test(patch);
    let container_children = element.as_widget().children();

    assert_eq!(container_children.len(), 5);
}

#[test]
fn git_diff_by_file_without_project_returns_empty_collections() {
    let mut app = App::new().0;
    app.project_path = None;

    let (files, patches) = git_diff_by_file(&app);

    assert!(files.is_empty());
    assert!(patches.is_empty());
}

#[test]
fn git_diff_by_file_with_unavailable_project_records_returns_empty_collections() {
    let mut app = App::new().0;
    app.project_path = Some("/definitely/not/a/live/project".to_string());

    let (files, patches) = git_diff_by_file(&app);

    assert!(files.is_empty());
    assert!(patches.is_empty());
}

#[test]
fn build_code_review_view_from_diff_renders_collapsed_expanded_and_missing_patch_files() {
    let mut app = App::new().0;
    app.ensure_diff_file_expanded("src/expanded.rs".to_string());
    app.ensure_diff_file_expanded("src/missing_patch.rs".to_string());

    let mut patches = HashMap::new();
    patches.insert("src/expanded.rs".to_string(), "+added\n-context".to_string());
    let element = build_code_review_view_from_diff(
        &app,
        vec![
            "src/collapsed.rs".to_string(),
            "src/expanded.rs".to_string(),
            "src/missing_patch.rs".to_string(),
        ],
        patches,
    );

    let scrollable_children = element.as_widget().children();

    assert_eq!(scrollable_children.len(), 1);
}

#[test]
fn view_without_project_constructs_empty_diff_state() {
    let mut app = App::new().0;
    app.project_path = None;

    let _element = view(&app);
}
