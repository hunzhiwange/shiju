use super::code_review::{git_diff_by_file, view};
use crate::app::App;
#[test]
fn git_diff_by_file_without_project_returns_empty_collections() {
    let mut app = App::new().0;
    app.navigation_tabs.project_path = None;

    let (files, patches) = git_diff_by_file(&app);

    assert!(files.is_empty());
    assert!(patches.is_empty());
}

#[test]
fn git_diff_by_file_with_unavailable_project_records_returns_empty_collections() {
    let mut app = App::new().0;
    app.navigation_tabs.project_path = Some("/definitely/not/a/live/project".to_string());

    let (files, patches) = git_diff_by_file(&app);

    assert!(files.is_empty());
    assert!(patches.is_empty());
}

#[test]
fn view_without_project_constructs_empty_diff_state() {
    let mut app = App::new().0;
    app.navigation_tabs.project_path = None;

    let _element = view(&app);
}
