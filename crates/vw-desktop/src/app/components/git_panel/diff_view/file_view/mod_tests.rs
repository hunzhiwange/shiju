//! 验证差异文件视图的行选择、折叠预算、加载状态与文件类型分支。
//! 大型修改块受自动展开预算限制，手动展开和菜单状态仍应覆盖默认折叠决策。

use std::collections::HashSet;

use iced::Color;
use similar::DiffOp;

use crate::app::components::git_panel::diff_view::DiffRenderCtx;
use crate::app::components::git_panel::utils::FileStatus;
use crate::app::state::GitDiffFileMenuState;
use crate::app::{App, DiffTheme};

fn colors() -> (Color, Color, Color, Color, Color) {
    (
        Color::WHITE,
        Color::from_rgba8(0, 255, 0, 0.10),
        Color::from_rgba8(0, 255, 0, 0.25),
        Color::from_rgba8(255, 0, 0, 0.10),
        Color::from_rgba8(255, 0, 0, 0.25),
    )
}

fn render_file(app: &App, status: FileStatus, contents: Option<(&str, &str)>, loading: bool) {
    let ctx = DiffRenderCtx::new(app);
    let (bg, add_line, add_word, del_line, del_word) = colors();
    let _view = super::view_file(
        app,
        &ctx,
        "src/lib.rs",
        contents,
        loading,
        status,
        2,
        1,
        DiffTheme::GitHub,
        bg,
        add_line,
        add_word,
        del_line,
        del_word,
        true,
    );
}

#[test]
fn changed_diff_line_sets_collects_only_changed_lines() {
    let groups = vec![vec![
        DiffOp::Equal { old_index: 0, new_index: 0, len: 1 },
        DiffOp::Delete { old_index: 1, old_len: 2, new_index: 1 },
        DiffOp::Insert { old_index: 3, new_index: 1, new_len: 2 },
        DiffOp::Replace { old_index: 3, old_len: 1, new_index: 3, new_len: 2 },
    ]];

    let (old_lines, new_lines) = super::changed_diff_line_sets(&groups);

    assert_eq!(old_lines, HashSet::from([1, 2, 3]));
    assert_eq!(new_lines, HashSet::from([1, 2, 3, 4]));
}

#[test]
fn file_line_selection_state_distinguishes_none_partial_all_and_file_stage() {
    let mut app = App::new().0;
    let changed = (HashSet::from([0, 1]), HashSet::from([0, 1]));
    let ctx = DiffRenderCtx::new(&app);
    assert_eq!(
        super::file_line_selection_state(&app, &ctx, "src/lib.rs", &changed),
        super::FileLineSelectionState::None
    );

    app.staged_lines_selected = vec![("src/lib.rs".to_string(), 0)];
    let ctx = DiffRenderCtx::new(&app);
    assert_eq!(
        super::file_line_selection_state(&app, &ctx, "src/lib.rs", &changed),
        super::FileLineSelectionState::Partial
    );

    app.staged_old_lines_selected =
        vec![("src/lib.rs".to_string(), 0), ("src/lib.rs".to_string(), 1)];
    app.staged_lines_selected = vec![("src/lib.rs".to_string(), 0), ("src/lib.rs".to_string(), 1)];
    let ctx = DiffRenderCtx::new(&app);
    assert_eq!(
        super::file_line_selection_state(&app, &ctx, "src/lib.rs", &changed),
        super::FileLineSelectionState::All
    );

    app.staged_old_lines_selected.clear();
    app.staged_lines_selected.clear();
    app.staged_files_selected = vec!["src/lib.rs".to_string()];
    let ctx = DiffRenderCtx::new(&app);
    assert_eq!(
        super::file_line_selection_state(&app, &ctx, "src/lib.rs", &changed),
        super::FileLineSelectionState::All
    );

    let empty = (HashSet::new(), HashSet::new());
    assert_eq!(
        super::file_line_selection_state(&app, &ctx, "src/lib.rs", &empty),
        super::FileLineSelectionState::None
    );
}

#[test]
fn diff_line_select_controls_and_file_menu_build() {
    let _selected = super::diff_line_select_button(
        true,
        crate::app::Message::Git(crate::app::message::GitMessage::SelectAllFileLines(
            "src/lib.rs".to_string(),
        )),
    );
    let _unselected = super::diff_line_select_button(
        false,
        crate::app::Message::Git(crate::app::message::GitMessage::ClearAllFileLines(
            "src/lib.rs".to_string(),
        )),
    );
    let _spacer = super::diff_line_select_spacer();
    let _menu = super::diff_file_actions_menu("src/lib.rs", Some("deleted".to_string()));
}

#[test]
fn view_file_covers_collapsed_loading_missing_and_file_status_branches() {
    let mut app = App::new().0;
    render_file(&app, FileStatus::Modified, Some(("old\n", "new\n")), false);

    app.expanded_files_set.insert("src/lib.rs".to_string());
    render_file(&app, FileStatus::Modified, None, true);
    render_file(&app, FileStatus::Modified, None, false);
    render_file(&app, FileStatus::Added, Some(("", "new\nline\n")), false);
    render_file(&app, FileStatus::Untracked, Some(("", "new\nline\n")), false);
    render_file(&app, FileStatus::Deleted, Some(("old\nline\n", "")), false);
    render_file(&app, FileStatus::Unknown, Some(("old\n", "new\n")), false);
}

#[test]
fn view_file_missing_contents_builds_header_with_existing_line_selection() {
    let mut app = App::new().0;
    app.staged_old_lines_selected = vec![("src/lib.rs".to_string(), 4)];

    render_file(&app, FileStatus::Modified, None, false);

    app.staged_old_lines_selected.clear();
    app.staged_lines_selected = vec![("src/lib.rs".to_string(), 7)];
    render_file(&app, FileStatus::Modified, None, false);
}

#[test]
fn view_file_covers_modified_renamed_hunk_menu_and_manual_expansion_branches() {
    let mut app = App::new().0;
    app.expanded_files_set.insert("src/lib.rs".to_string());
    app.git_diff_file_menu = Some(GitDiffFileMenuState { file: "src/lib.rs".to_string() });
    app.staged_lines_selected = vec![("src/lib.rs".to_string(), 1)];
    render_file(&app, FileStatus::Modified, Some(("same\nold\nend\n", "same\nnew\nend\n")), false);

    app.expanded_hunks = vec![("src/lib.rs".to_string(), 0)];
    app.staged_files_selected = vec!["src/lib.rs".to_string()];
    render_file(&app, FileStatus::Renamed, Some(("a\nb\nc\n", "a\nB\nc\n")), false);
}

#[test]
fn view_file_collapses_large_modified_hunk_when_auto_expand_budget_is_exceeded() {
    let mut app = App::new().0;
    app.expanded_files_set.insert("src/lib.rs".to_string());
    let old_content = (0..520).map(|idx| format!("old-{idx}\n")).collect::<String>();
    let new_content = (0..520).map(|idx| format!("new-{idx}\n")).collect::<String>();

    render_file(&app, FileStatus::Modified, Some((&old_content, &new_content)), false);
}
