use super::{ProjectMessage, update};
use crate::app::{App, state::FindInFolderTab};

fn find_tab(id: &str) -> FindInFolderTab {
    FindInFolderTab {
        id: id.to_string(),
        title: "查找".to_string(),
        scope_path: "/tmp/project".to_string(),
        query_input: String::new(),
        replace_input: String::new(),
        query_editor: iced::widget::text_editor::Content::new(),
        replace_editor: iced::widget::text_editor::Content::new(),
        query: String::new(),
        replace_text: String::new(),
        case_sensitive: false,
        whole_word: false,
        use_regex: false,
        running: false,
        error: None,
        limit_reached: false,
        matches: Vec::new(),
    }
}

#[test]
fn update_dispatches_basic_message() {
    let (mut app, _task) = App::new();

    let _task = update(&mut app, ProjectMessage::ProjectPathChanged("/tmp/project".to_string()));

    assert_eq!(app.project_path_input, "/tmp/project");
}

#[test]
fn update_dispatches_find_message() {
    let (mut app, _task) = App::new();
    app.find_results_tabs.push(find_tab("find-tab"));

    let _task = update(
        &mut app,
        ProjectMessage::FileTreeFindCaseSensitiveToggled("find-tab".to_string(), true),
    );

    assert!(app.find_results_tabs[0].case_sensitive);
}

#[test]
fn update_dispatches_file_tree_message() {
    let (mut app, _task) = App::new();

    let _task = update(
        &mut app,
        ProjectMessage::FileTreeRightClicked(
            "/tmp/project/src/main.rs".to_string(),
            "tree".to_string(),
            12.0,
            24.0,
        ),
    );

    assert_eq!(app.file_tree_menu_path.as_deref(), Some("/tmp/project/src/main.rs"));
    assert_eq!(app.file_tree_menu_source.as_deref(), Some("tree"));
    assert_eq!(app.file_tree_menu_anchor, Some(iced::Point::new(12.0, 24.0)));
}

#[test]
fn update_dispatches_session_message() {
    let (mut app, _task) = App::new();

    let _task =
        update(&mut app, ProjectMessage::SessionRightClicked("session-id".to_string(), 8.0, 16.0));

    assert_eq!(app.session_menu_id.as_deref(), Some("session-id"));
    assert_eq!(app.session_menu_anchor, Some(iced::Point::new(8.0, 16.0)));
}
