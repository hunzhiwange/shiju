use std::collections::HashSet;

use crate::app::App;
use crate::app::state::GitDiffSelectedLine;

#[test]
fn diff_render_ctx_snapshots_line_file_and_diff_selection() {
    let mut app = App::new().0;
    app.staged_files_selected = vec!["src/lib.rs".to_string()];
    app.staged_lines_selected = vec![("src/lib.rs".to_string(), 2)];
    app.staged_old_lines_selected = vec![("src/lib.rs".to_string(), 1)];
    app.git_diff_selected_lines = vec![GitDiffSelectedLine {
        file: "src/lib.rs".to_string(),
        line: 3,
        is_old: false,
        text: "selected".to_string(),
    }];

    let ctx = super::DiffRenderCtx::new(&app);

    assert!(ctx.is_file_staged("src/lib.rs"));
    assert!(ctx.is_new_line_staged("src/lib.rs", 2));
    assert!(ctx.is_old_line_staged("src/lib.rs", 1));
    assert!(ctx.is_diff_line_selected("src/lib.rs", 3, false));
    assert!(ctx.is_new_line_staged("src/lib.rs", 99));
    assert!(ctx.is_old_line_staged("src/lib.rs", 99));
    assert!(!ctx.is_file_staged("src/main.rs"));
    assert!(!ctx.is_diff_line_selected("src/lib.rs", 3, true));
}

#[test]
fn diff_render_ctx_empty_app_has_empty_sets() {
    let app = App::new().0;
    let ctx = super::DiffRenderCtx::new(&app);

    assert_eq!(ctx.selected_new_lines, HashSet::new());
    assert_eq!(ctx.selected_old_lines, HashSet::new());
    assert_eq!(ctx.selected_diff_lines, HashSet::new());
    assert_eq!(ctx.selected_files, HashSet::new());
}

#[test]
fn diff_render_ctx_matches_line_selection_without_file_stage() {
    let mut app = App::new().0;
    app.staged_lines_selected = vec![("src/lib.rs".to_string(), 2)];
    app.staged_old_lines_selected = vec![("src/lib.rs".to_string(), 1)];
    app.git_diff_selected_lines = vec![
        GitDiffSelectedLine {
            file: "src/lib.rs".to_string(),
            line: 1,
            is_old: true,
            text: "-old".to_string(),
        },
        GitDiffSelectedLine {
            file: "src/lib.rs".to_string(),
            line: 2,
            is_old: false,
            text: "+new".to_string(),
        },
    ];

    let ctx = super::DiffRenderCtx::new(&app);

    assert!(!ctx.is_file_staged("src/lib.rs"));
    assert!(ctx.is_new_line_staged("src/lib.rs", 2));
    assert!(ctx.is_old_line_staged("src/lib.rs", 1));
    assert!(!ctx.is_new_line_staged("src/lib.rs", 1));
    assert!(!ctx.is_old_line_staged("src/lib.rs", 2));
    assert!(!ctx.is_new_line_staged("src/main.rs", 2));
    assert!(!ctx.is_old_line_staged("src/main.rs", 1));
    assert!(ctx.is_diff_line_selected("src/lib.rs", 1, true));
    assert!(ctx.is_diff_line_selected("src/lib.rs", 2, false));
    assert!(!ctx.is_diff_line_selected("src/lib.rs", 1, false));
}

#[test]
fn diff_render_ctx_deduplicates_inputs_into_sets() {
    let mut app = App::new().0;
    app.staged_files_selected = vec!["src/lib.rs".to_string(), "src/lib.rs".to_string()];
    app.staged_lines_selected = vec![("src/lib.rs".to_string(), 2), ("src/lib.rs".to_string(), 2)];
    app.staged_old_lines_selected =
        vec![("src/lib.rs".to_string(), 1), ("src/lib.rs".to_string(), 1)];
    app.git_diff_selected_lines = vec![
        GitDiffSelectedLine {
            file: "src/lib.rs".to_string(),
            line: 3,
            is_old: false,
            text: "+first".to_string(),
        },
        GitDiffSelectedLine {
            file: "src/lib.rs".to_string(),
            line: 3,
            is_old: false,
            text: "+second".to_string(),
        },
    ];

    let ctx = super::DiffRenderCtx::new(&app);

    assert_eq!(ctx.selected_files.len(), 1);
    assert_eq!(ctx.selected_new_lines.len(), 1);
    assert_eq!(ctx.selected_old_lines.len(), 1);
    assert_eq!(ctx.selected_diff_lines.len(), 1);
    assert!(ctx.is_diff_line_selected("src/lib.rs", 3, false));
}

#[test]
fn diff_view_layout_constants_are_stable() {
    assert_eq!(super::DIFF_MARKER_WIDTH, 8.0);
    assert_eq!(super::DIFF_LINE_NUMBER_WIDTH, 46.0);
    assert_eq!(super::DIFF_SPLIT_DIVIDER_WIDTH, 8.0);
}
