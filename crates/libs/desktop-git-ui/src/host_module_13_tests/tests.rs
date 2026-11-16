//! 原宿主入口的领域状态与效果断言，调用真实领域路由。

use crate::message_git::messages::GitMessage;
use crate::test_support::Fixture;
use crate::test_support::update;

fn test_app() -> Fixture {
    Fixture::default()
}

#[test]
fn test_module_is_wired() {
    let module = module_path!();

    assert!(module.ends_with("tests"));
}

#[test]
fn update_routes_worktree_menu_message_to_refresh_handler() {
    let mut app = test_app();
    app.git.git_worktree_menu_open = false;

    let _task = update(&mut app, GitMessage::ToggleGitWorktreeMenu(true));

    assert!(app.git.git_worktree_menu_open);
}

#[test]
fn update_routes_line_selection_message_to_diff_handler() {
    let mut app = test_app();

    let _task = update(
        &mut app,
        GitMessage::ToggleDiffLineSelection(
            "src/lib.rs".to_string(),
            7,
            false,
            "+added".to_string(),
        ),
    );

    assert_eq!(app.git.git_diff_selected_lines.len(), 1);
    let selected = &app.git.git_diff_selected_lines[0];
    assert_eq!(selected.file, "src/lib.rs");
    assert_eq!(selected.line, 7);
    assert!(!selected.is_old);
    assert_eq!(selected.text, "+added");
}

#[test]
fn update_routes_custom_diff_title_message_to_modal_handler() {
    let mut app = test_app();

    let _task = update(&mut app, GitMessage::CustomDiffTitleChanged("Review".to_string()));

    assert_eq!(app.git.git_custom_diff_title, "Review");
}

#[test]
fn update_routes_filter_query_message_to_stage_commit_handler() {
    let mut app = test_app();

    let _task = update(&mut app, GitMessage::FilterQueryChanged("src/*.rs".to_string()));

    assert_eq!(app.git.git_filter_query, "src/*.rs");
}
