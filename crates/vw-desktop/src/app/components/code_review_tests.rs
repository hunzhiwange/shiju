use super::code_review::{build_code_review_view_from_diff, git_diff_by_file, view};
use crate::app::App;
use std::collections::HashMap;
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
