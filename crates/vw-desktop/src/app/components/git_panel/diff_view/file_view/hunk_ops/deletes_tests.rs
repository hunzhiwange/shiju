use iced::Color;

use crate::app::components::git_panel::diff_view::DiffRenderCtx;
use crate::app::components::git_panel::utils::Lang;
use crate::app::state::GitDiffSelectedLine;
use crate::app::{App, DiffTheme};

fn colors() -> (Color, Color, Color) {
    (
        Color::from_rgba8(255, 0, 0, 0.10),
        Color::from_rgba8(255, 0, 0, 0.25),
        Color::from_rgba8(255, 210, 0, 0.22),
    )
}

fn render_count(
    app: &App,
    file: &str,
    old_index: usize,
    old_len: usize,
    old_lines: &[&str],
) -> usize {
    let ctx = DiffRenderCtx::new(app);
    let (line_bg, word_bg, hover) = colors();

    super::deletes::render_delete_ops(
        app,
        &ctx,
        file,
        old_index,
        old_len,
        old_index,
        old_lines,
        Lang::Rust,
        DiffTheme::GitHub,
        line_bg,
        word_bg,
        hover,
        0.22,
        hover,
        false,
        true,
    )
    .len()
}

#[test]
fn render_delete_ops_returns_one_row_per_deleted_line_in_split_view() {
    let mut app = App::new().0;
    app.merge_view = false;

    assert_eq!(render_count(&app, "src/lib.rs", 0, 2, &["old one", "old two"]), 2);
}

#[test]
fn render_delete_ops_returns_one_row_per_deleted_line_in_merge_view() {
    let mut app = App::new().0;
    app.merge_view = true;

    assert_eq!(render_count(&app, "src/lib.rs", 1, 2, &["keep", "old one", "old two"]), 2);
}

#[test]
fn render_delete_ops_tolerates_missing_source_lines_and_zero_len() {
    let app = App::new().0;

    assert_eq!(render_count(&app, "src/lib.rs", 4, 2, &["only"]), 2);
    assert_eq!(render_count(&app, "src/lib.rs", 0, 0, &["only"]), 0);
}

#[test]
fn render_delete_ops_handles_selected_and_hovered_rows_independently_at_nonzero_offset() {
    let mut app = App::new().0;
    app.merge_view = true;
    app.staged_old_lines_selected = vec![("src/lib.rs".to_string(), 1)];
    app.git_diff_selected_lines = vec![GitDiffSelectedLine {
        file: "src/lib.rs".to_string(),
        line: 1,
        is_old: true,
        text: "old one".to_string(),
    }];
    app.git_diff_hovered_line = Some(("src/lib.rs".to_string(), 2, true));

    assert_eq!(render_count(&app, "src/lib.rs", 1, 2, &["keep", "old one", "old two"]), 2);
}

#[test]
fn render_delete_ops_file_staging_does_not_require_line_staging() {
    let mut app = App::new().0;
    app.staged_files_selected = vec!["src/lib.rs".to_string()];

    assert!(app.staged_old_lines_selected.is_empty());
    assert_eq!(render_count(&app, "src/lib.rs", 0, 1, &["old one"]), 1);
}

#[test]
fn render_delete_ops_emits_requested_rows_when_offset_reaches_missing_tail() {
    let app = App::new().0;

    assert_eq!(render_count(&app, "src/lib.rs", 1, 3, &["keep", "old one"]), 3);
}
