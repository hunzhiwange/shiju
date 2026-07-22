use iced::Color;

use crate::app::components::git_panel::diff_view::DiffRenderCtx;
use crate::app::components::git_panel::utils::Lang;
use crate::app::state::{GitDiffContextMenuState, GitDiffSelectedLine};
use crate::app::{App, DiffTheme};

fn palette() -> (Color, Color, Color, Color, Color) {
    (
        Color::from_rgba8(0, 255, 0, 0.10),
        Color::from_rgba8(0, 255, 0, 0.25),
        Color::from_rgba8(255, 0, 0, 0.10),
        Color::from_rgba8(255, 0, 0, 0.25),
        Color::from_rgba8(255, 210, 0, 0.22),
    )
}

fn render_count(
    app: &App,
    old_index: usize,
    old_len: usize,
    new_index: usize,
    new_len: usize,
    old_lines: &[&str],
    new_lines: &[&str],
) -> usize {
    let ctx = DiffRenderCtx::new(app);
    let (add_line_bg, add_word_bg, del_line_bg, del_word_bg, hover) = palette();

    super::replaces::render_replace_ops(
        app,
        &ctx,
        "src/lib.rs",
        old_index,
        old_len,
        new_index,
        new_len,
        old_lines,
        new_lines,
        Lang::Rust,
        DiffTheme::GitHub,
        add_line_bg,
        add_word_bg,
        del_line_bg,
        del_word_bg,
        hover,
        0.22,
        hover,
        false,
        true,
    )
    .len()
}

#[test]
fn should_compute_word_diff_requires_highlight_equal_lengths_changes_and_short_lines() {
    let mut app = App::new().0;
    app.show_git_diff_highlight = true;

    assert!(super::replaces::should_compute_word_diff(&app, 1, 1, "let a = 1;", "let a = 2;"));
    assert!(super::replaces::should_compute_word_diff(&app, 1, 1, "a", "b"));
    assert!(!super::replaces::should_compute_word_diff(&app, 1, 2, "a", "b"));
    assert!(!super::replaces::should_compute_word_diff(&app, 1, 1, "same", "same"));
    assert!(!super::replaces::should_compute_word_diff(&app, 1, 1, &"a".repeat(513), "b"));

    app.show_git_diff_highlight = false;
    assert!(super::replaces::should_compute_word_diff(&app, 1, 1, "a", "b"));
}

#[test]
fn should_compute_word_diff_enforces_new_line_length_boundary() {
    let mut app = App::new().0;
    app.show_git_diff_highlight = true;
    let at_limit = "b".repeat(512);
    let over_limit = "b".repeat(513);

    assert!(super::replaces::should_compute_word_diff(&app, 1, 1, "a", &at_limit));
    assert!(!super::replaces::should_compute_word_diff(&app, 1, 1, "a", &over_limit));
}

#[test]
fn render_replace_ops_merge_view_emits_old_then_new_rows() {
    let mut app = App::new().0;
    app.merge_view = true;

    assert_eq!(
        render_count(&app, 0, 2, 0, 3, &["old a", "old b"], &["new a", "new b", "new c"]),
        5
    );
}

#[test]
fn render_replace_ops_split_view_uses_max_old_new_count() {
    let mut app = App::new().0;
    app.merge_view = false;

    assert_eq!(
        render_count(&app, 0, 2, 0, 3, &["old a", "old b"], &["new a", "new b", "new c"]),
        3
    );
    assert_eq!(render_count(&app, 0, 3, 0, 1, &["old a", "old b", "old c"], &["new a"]), 3);
}

#[test]
fn render_replace_ops_handles_equal_length_word_diff_and_missing_lines() {
    let mut app = App::new().0;
    app.show_git_diff_highlight = true;
    app.merge_view = false;

    assert_eq!(render_count(&app, 0, 2, 0, 2, &["let a = 1;", "let b = 1;"], &["let a = 2;"]), 2);
}

#[test]
fn render_replace_ops_merge_view_zero_old_len_emits_only_new_rows() {
    let mut app = App::new().0;
    app.merge_view = true;

    assert_eq!(render_count(&app, 0, 0, 0, 2, &[], &["new a", "new b"]), 2);
}

#[test]
fn render_replace_ops_merge_view_zero_new_len_emits_only_old_rows() {
    let mut app = App::new().0;
    app.merge_view = true;

    assert_eq!(render_count(&app, 0, 2, 0, 0, &["old a", "old b"], &[]), 2);
}

#[test]
fn render_replace_ops_split_view_zero_old_len_emits_only_new_panes() {
    let mut app = App::new().0;
    app.merge_view = false;

    assert_eq!(render_count(&app, 0, 0, 0, 2, &[], &["new a", "new b"]), 2);
}

#[test]
fn render_replace_ops_split_view_wraps_matching_context_menu_for_new_only_row() {
    let mut app = App::new().0;
    app.merge_view = false;
    app.git_diff_context_menu = Some(GitDiffContextMenuState {
        file: "src/lib.rs".to_string(),
        line: 2,
        is_old: false,
        x: 12.0,
        y: 24.0,
    });
    let ctx = DiffRenderCtx::new(&app);
    let (add_line_bg, add_word_bg, del_line_bg, del_word_bg, hover) = palette();

    let rows = super::replaces::render_replace_ops(
        &app,
        &ctx,
        "src/lib.rs",
        0,
        0,
        2,
        1,
        &[],
        &["new a", "new b", "new c"],
        Lang::Rust,
        DiffTheme::GitHub,
        add_line_bg,
        add_word_bg,
        del_line_bg,
        del_word_bg,
        hover,
        0.22,
        hover,
        false,
        true,
    );

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].as_widget().children().len(), 2);
}

#[test]
fn render_replace_ops_split_view_zero_new_len_emits_only_old_panes() {
    let mut app = App::new().0;
    app.merge_view = false;

    assert_eq!(render_count(&app, 0, 2, 0, 0, &["old a", "old b"], &[]), 2);
}

#[test]
fn render_replace_ops_zero_len_in_both_views_produces_no_rows() {
    let mut app = App::new().0;

    app.merge_view = true;
    assert_eq!(render_count(&app, 0, 0, 0, 0, &[] as &[&str], &[] as &[&str]), 0);

    app.merge_view = false;
    assert_eq!(render_count(&app, 0, 0, 0, 0, &[] as &[&str], &[] as &[&str]), 0);
}

#[test]
fn render_replace_ops_covers_selected_staged_hovered_and_file_staged_branches() {
    let mut app = App::new().0;
    app.staged_old_lines_selected = vec![("src/lib.rs".to_string(), 0)];
    app.staged_lines_selected = vec![("src/lib.rs".to_string(), 0)];
    app.git_diff_selected_lines = vec![
        GitDiffSelectedLine {
            file: "src/lib.rs".to_string(),
            line: 0,
            is_old: true,
            text: "old".to_string(),
        },
        GitDiffSelectedLine {
            file: "src/lib.rs".to_string(),
            line: 0,
            is_old: false,
            text: "new".to_string(),
        },
    ];
    app.git_diff_hovered_line = Some(("src/lib.rs".to_string(), 0, true));

    assert_eq!(render_count(&app, 0, 1, 0, 1, &["old"], &["new"]), 2);

    app.merge_view = true;
    app.staged_files_selected = vec!["src/lib.rs".to_string()];
    app.git_diff_hovered_line = Some(("src/lib.rs".to_string(), 0, false));
    assert_eq!(render_count(&app, 0, 1, 0, 1, &["old"], &["new"]), 2);
}
