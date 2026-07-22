use iced::Color;

use crate::app::components::git_panel::diff_view::DiffRenderCtx;
use crate::app::components::git_panel::utils::Lang;
use crate::app::state::{GitDiffLineRange, GitDiffSelectedLine};
use crate::app::{App, DiffTheme};

fn colors() -> (Color, Color, Color) {
    (
        Color::from_rgba8(0, 255, 0, 0.10),
        Color::from_rgba8(0, 255, 0, 0.25),
        Color::from_rgba8(255, 210, 0, 0.22),
    )
}

fn render_count(
    app: &App,
    file: &str,
    new_index: usize,
    new_len: usize,
    new_lines: &[&str],
) -> usize {
    let ctx = DiffRenderCtx::new(app);
    let (line_bg, word_bg, hover) = colors();

    super::inserts::render_insert_ops(
        app,
        &ctx,
        file,
        new_index,
        new_len,
        new_lines,
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
fn render_insert_ops_returns_one_row_per_inserted_line_in_split_view() {
    let mut app = App::new().0;
    app.merge_view = false;

    assert_eq!(render_count(&app, "src/lib.rs", 0, 2, &["new one", "new two"]), 2);
}

#[test]
fn render_insert_ops_returns_one_row_per_inserted_line_in_merge_view() {
    let mut app = App::new().0;
    app.merge_view = true;

    assert_eq!(render_count(&app, "src/lib.rs", 1, 2, &["keep", "new one", "new two"]), 2);
}

#[test]
fn render_insert_ops_tolerates_missing_source_lines_and_zero_len() {
    let app = App::new().0;

    assert_eq!(render_count(&app, "src/lib.rs", 3, 2, &["only"]), 2);
    assert_eq!(render_count(&app, "src/lib.rs", 0, 0, &["only"]), 0);
}

#[test]
fn render_insert_ops_covers_explicit_selection_and_line_staging() {
    let mut app = App::new().0;
    app.staged_lines_selected = vec![("src/lib.rs".to_string(), 0)];
    app.git_diff_selected_lines = vec![GitDiffSelectedLine {
        file: "src/lib.rs".to_string(),
        line: 0,
        is_old: false,
        text: "new one".to_string(),
    }];

    assert_eq!(render_count(&app, "src/lib.rs", 0, 1, &["new one"]), 1);
}

#[test]
fn render_insert_ops_covers_hover_and_file_staging_without_selection() {
    let mut app = App::new().0;
    app.git_diff_hovered_line = Some(("src/lib.rs".to_string(), 0, false));
    app.staged_files_selected = vec!["src/lib.rs".to_string()];

    assert_eq!(render_count(&app, "src/lib.rs", 0, 1, &["new one"]), 1);
}

#[test]
fn render_insert_ops_handles_range_selection_empty_and_unicode_content() {
    let mut app = App::new().0;
    app.git_diff_selected_range =
        Some(GitDiffLineRange { file: "src/lib.rs".to_string(), start: 1, end: 2, is_old: false });

    assert_eq!(render_count(&app, "src/lib.rs", 1, 2, &["keep", "", "新增行"]), 2);
}
