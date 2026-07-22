use std::collections::HashSet;

use iced::Color;
use similar::DiffOp;

use super::super::DiffRenderCtx;
use crate::app::components::git_panel::utils::FileStatus;
use crate::app::{App, DiffTheme};

fn render_file(
    app: &App,
    file: &str,
    status: FileStatus,
    contents: Option<(&str, &str)>,
    loading: bool,
) {
    let render_ctx = DiffRenderCtx::new(app);
    let _view = super::view_file(
        app,
        &render_ctx,
        file,
        contents,
        loading,
        status,
        1,
        1,
        DiffTheme::GitHub,
        Color::WHITE,
        Color::from_rgba8(0, 255, 0, 0.10),
        Color::from_rgba8(0, 255, 0, 0.25),
        Color::from_rgba8(255, 0, 0, 0.10),
        Color::from_rgba8(255, 0, 0, 0.25),
        true,
    );
}

#[test]
fn changed_diff_line_sets_returns_empty_for_equal_and_zero_length_operations() {
    let groups = vec![vec![
        DiffOp::Equal { old_index: 0, new_index: 0, len: 3 },
        DiffOp::Delete { old_index: 3, old_len: 0, new_index: 3 },
        DiffOp::Insert { old_index: 3, new_index: 3, new_len: 0 },
        DiffOp::Replace { old_index: 3, old_len: 0, new_index: 3, new_len: 0 },
    ]];

    let changed = super::changed_diff_line_sets(&groups);

    assert_eq!(changed, (HashSet::new(), HashSet::new()));
}

#[test]
fn changed_diff_line_sets_deduplicates_overlapping_ranges_across_groups() {
    let groups = vec![
        vec![
            DiffOp::Delete { old_index: 2, old_len: 3, new_index: 2 },
            DiffOp::Insert { old_index: 5, new_index: 4, new_len: 3 },
        ],
        vec![DiffOp::Replace { old_index: 4, old_len: 3, new_index: 6, new_len: 2 }],
    ];

    let (old_lines, new_lines) = super::changed_diff_line_sets(&groups);

    assert_eq!(old_lines, HashSet::from([2, 3, 4, 5, 6]));
    assert_eq!(new_lines, HashSet::from([4, 5, 6, 7]));
}

#[test]
fn file_line_selection_state_ignores_other_files_and_unchanged_lines() {
    let mut app = App::new().0;
    app.staged_files_selected.push("src/other.rs".to_string());
    app.staged_old_lines_selected =
        vec![("src/other.rs".to_string(), 3), ("src/lib.rs".to_string(), 99)];
    app.staged_lines_selected =
        vec![("src/other.rs".to_string(), 7), ("src/lib.rs".to_string(), 100)];
    let changed = (HashSet::from([3]), HashSet::from([7]));
    let render_ctx = DiffRenderCtx::new(&app);

    assert_eq!(
        super::file_line_selection_state(&app, &render_ctx, "src/lib.rs", &changed),
        super::FileLineSelectionState::None
    );
}

#[test]
fn file_line_selection_state_counts_old_and_new_sides_independently() {
    let mut app = App::new().0;
    let changed = (HashSet::from([3]), HashSet::from([3]));
    app.staged_old_lines_selected.push(("src/lib.rs".to_string(), 3));
    let render_ctx = DiffRenderCtx::new(&app);
    assert_eq!(
        super::file_line_selection_state(&app, &render_ctx, "src/lib.rs", &changed),
        super::FileLineSelectionState::Partial
    );

    app.staged_lines_selected.push(("src/lib.rs".to_string(), 3));
    let render_ctx = DiffRenderCtx::new(&app);
    assert_eq!(
        super::file_line_selection_state(&app, &render_ctx, "src/lib.rs", &changed),
        super::FileLineSelectionState::All
    );
}

#[test]
fn view_file_handles_missing_content_with_an_old_line_selection() {
    let mut app = App::new().0;
    app.expanded_files_set.insert("src/lib.rs".to_string());
    app.staged_old_lines_selected.push(("src/lib.rs".to_string(), 0));

    render_file(&app, "src/lib.rs", FileStatus::Modified, None, false);
}

#[test]
fn view_file_handles_insert_only_delete_only_and_unchanged_modified_content() {
    let mut app = App::new().0;
    app.expanded_files_set.insert("src/lib.rs".to_string());

    render_file(&app, "src/lib.rs", FileStatus::Modified, Some(("", "inserted\n")), false);
    render_file(&app, "src/lib.rs", FileStatus::Modified, Some(("deleted\n", "")), false);
    render_file(&app, "src/lib.rs", FileStatus::Modified, Some(("same\n", "same\n")), false);
}

#[test]
fn view_file_collapses_unselected_manual_hunks_and_hunks_over_the_auto_expand_budget() {
    let mut app = App::new().0;
    app.expanded_files_set.insert("src/lib.rs".to_string());
    app.expanded_hunks.push(("src/lib.rs".to_string(), 99));
    render_file(&app, "src/lib.rs", FileStatus::Modified, Some(("before\n", "after\n")), false);

    app.expanded_hunks.clear();
    let old_content = (0..600).map(|line| format!("old-{line}\n")).collect::<String>();
    let new_content = (0..600).map(|line| format!("new-{line}\n")).collect::<String>();
    render_file(
        &app,
        "src/lib.rs",
        FileStatus::Modified,
        Some((&old_content, &new_content)),
        false,
    );
}

#[test]
fn view_file_handles_empty_added_and_deleted_files() {
    let mut app = App::new().0;
    app.expanded_files_set.insert("src/empty.rs".to_string());

    render_file(&app, "src/empty.rs", FileStatus::Added, Some(("", "")), false);
    render_file(&app, "src/empty.rs", FileStatus::Deleted, Some(("", "")), false);
}

#[test]
fn diff_file_actions_menu_supports_files_without_deleted_content() {
    let _menu = super::diff_file_actions_menu("src/new.rs", None);
}
