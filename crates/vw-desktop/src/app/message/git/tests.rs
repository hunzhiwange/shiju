use super::{GitMessage, update};
use crate::app::App;

fn test_app() -> App {
    App::new().0
}

#[test]
fn test_module_is_wired() {
    let module = module_path!();

    assert!(module.ends_with("tests"));
}

#[test]
fn expand_direction_variants_are_distinct() {
    assert_ne!(super::ExpandDirection::Up, super::ExpandDirection::Down);
    assert_eq!(super::ExpandDirection::All, super::ExpandDirection::All);
}

#[test]
fn update_routes_worktree_menu_message_to_refresh_handler() {
    let mut app = test_app();
    app.git_worktree_menu_open = false;

    let _task = update(&mut app, GitMessage::ToggleGitWorktreeMenu(true));

    assert!(app.git_worktree_menu_open);
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

    assert_eq!(app.git_diff_selected_lines.len(), 1);
    let selected = &app.git_diff_selected_lines[0];
    assert_eq!(selected.file, "src/lib.rs");
    assert_eq!(selected.line, 7);
    assert!(!selected.is_old);
    assert_eq!(selected.text, "+added");
}

#[test]
fn update_routes_custom_diff_title_message_to_modal_handler() {
    let mut app = test_app();

    let _task = update(&mut app, GitMessage::CustomDiffTitleChanged("Review".to_string()));

    assert_eq!(app.git_custom_diff_title, "Review");
}

#[test]
fn update_routes_filter_query_message_to_stage_commit_handler() {
    let mut app = test_app();

    let _task = update(&mut app, GitMessage::FilterQueryChanged("src/*.rs".to_string()));

    assert_eq!(app.git_filter_query, "src/*.rs");
}
