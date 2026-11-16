use crate::project_message::find::{FindMessage, FindState, handle};
use desktop_file_tree::state::FindInFolderMatch;
fn find_tab(id: &str) -> desktop_file_tree::state::FindInFolderTab {
    desktop_file_tree::state::FindInFolderTab {
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
        localized_error: None,
        results_page: 0,
    }
}

fn find_match(preview: &str) -> desktop_file_tree::state::FindInFolderMatch {
    desktop_file_tree::state::FindInFolderMatch {
        path: "/tmp/project/a.rs".to_string(),
        line: 1,
        column: 2,
        preview: preview.to_string(),
        match_len: preview.len(),
    }
}

#[test]
fn find_in_project_without_project_path_does_not_create_tabs() {
    let mut app = FindState::default();
    let project_path: Option<String> = None;

    let _ =
        handle(&mut app, project_path.as_deref(), Vec::new(), FindMessage::FileTreeFindInProject);

    assert!(app.find_results_tabs.is_empty());
    assert!(app.active_find_results_tab_id.is_none());
}

#[test]
fn find_scope_changed_resolves_project_relative_input_and_rejects_escape() {
    let mut app = FindState::default();
    let project_path: Option<String> = Some("/tmp/project".to_string());
    app.find_results_tabs.push(find_tab("a"));

    let _ = handle(
        &mut app,
        project_path.as_deref(),
        Vec::new(),
        FindMessage::FileTreeFindScopeChanged("a".to_string(), "src/nested".to_string()),
    );
    assert_eq!(app.find_results_tabs[0].scope_path, "/tmp/project/src/nested");
    assert!(app.find_results_tabs[0].error.is_none());

    let _ = handle(
        &mut app,
        project_path.as_deref(),
        Vec::new(),
        FindMessage::FileTreeFindScopeChanged("a".to_string(), String::new()),
    );
    assert_eq!(app.find_results_tabs[0].scope_path, "/tmp/project");

    for invalid in ["/tmp/outside", "../outside", "src/../../outside"] {
        let _ = handle(
            &mut app,
            project_path.as_deref(),
            Vec::new(),
            FindMessage::FileTreeFindScopeChanged("a".to_string(), invalid.to_string()),
        );
        assert_eq!(app.find_results_tabs[0].scope_path, "/tmp/project");
        assert_eq!(app.find_results_tabs[0].error.as_deref(), Some("作用域必须是项目内的相对路径"));
    }
}

#[test]
fn find_completed_updates_existing_tab_without_replacing_matches_on_error() {
    let mut app = FindState::default();
    let project_path: Option<String> = None;
    let mut tab = find_tab("a");
    tab.matches.push(FindInFolderMatch {
        path: "/tmp/project/a.rs".to_string(),
        line: 1,
        column: 2,
        preview: "old".to_string(),
        match_len: 3,
    });
    app.find_results_tabs.push(tab);

    let _ = handle(
        &mut app,
        project_path.as_deref(),
        Vec::new(),
        FindMessage::FileTreeFindCompleted {
            tab_id: "a".to_string(),
            title: "查找: main".to_string(),
            scope_path: "/tmp/project".to_string(),
            query: "main".to_string(),
            replace_text: "lib".to_string(),
            case_sensitive: true,
            whole_word: true,
            use_regex: false,
            matches: Vec::new(),
            error: Some("bad regex".to_string()),
            localized_error: None,
            limit_reached: true,
        },
    );

    let tab = &app.find_results_tabs[0];
    assert_eq!(tab.title, "查找: main");
    assert_eq!(tab.query, "main");
    assert_eq!(tab.replace_text, "lib");
    assert!(tab.case_sensitive);
    assert!(tab.whole_word);
    assert!(!tab.running);
    assert_eq!(tab.error.as_deref(), Some("bad regex"));
    assert!(tab.limit_reached);
    assert_eq!(tab.matches.len(), 1);
}

#[test]
fn find_completed_creates_missing_tab_with_matches() {
    let mut app = FindState::default();
    let project_path: Option<String> = None;
    let matches = vec![FindInFolderMatch {
        path: "/tmp/project/a.rs".to_string(),
        line: 4,
        column: 5,
        preview: "fn main()".to_string(),
        match_len: 4,
    }];

    let _ = handle(
        &mut app,
        project_path.as_deref(),
        Vec::new(),
        FindMessage::FileTreeFindCompleted {
            tab_id: "new".to_string(),
            title: "查找: main".to_string(),
            scope_path: "/tmp/project".to_string(),
            query: "main".to_string(),
            replace_text: String::new(),
            case_sensitive: false,
            whole_word: false,
            use_regex: false,
            matches,
            error: None,
            localized_error: None,
            limit_reached: false,
        },
    );

    assert_eq!(app.find_results_tabs.len(), 1);
    assert_eq!(app.find_results_tabs[0].query_input, "main");
    assert_eq!(app.find_results_tabs[0].matches.len(), 1);
    assert_eq!(app.active_find_results_tab_id.as_deref(), Some("new"));
}

#[test]
fn find_completed_success_replaces_existing_matches_and_clears_running() {
    let mut app = FindState::default();
    let project_path: Option<String> = None;
    let mut tab = find_tab("a");
    tab.running = true;
    tab.error = Some("old error".to_string());
    tab.matches.push(find_match("old"));
    app.find_results_tabs.push(tab);

    let _ = handle(
        &mut app,
        project_path.as_deref(),
        Vec::new(),
        FindMessage::FileTreeFindCompleted {
            tab_id: "a".to_string(),
            title: "查找: new".to_string(),
            scope_path: "/tmp/project/src".to_string(),
            query: "new".to_string(),
            replace_text: "updated".to_string(),
            case_sensitive: false,
            whole_word: true,
            use_regex: true,
            matches: vec![find_match("new")],
            error: None,
            localized_error: None,
            limit_reached: false,
        },
    );

    let tab = &app.find_results_tabs[0];
    assert_eq!(tab.scope_path, "/tmp/project/src");
    assert_eq!(tab.matches.len(), 1);
    assert_eq!(tab.matches[0].preview, "new");
    assert!(!tab.running);
    assert!(tab.error.is_none());
    assert!(tab.whole_word);
    assert!(tab.use_regex);
}

#[test]
fn tab_selected_empty_clears_active_find_tab() {
    let mut app = FindState::default();
    let project_path: Option<String> = None;
    app.active_find_results_tab_id = Some("a".to_string());

    let _ = handle(
        &mut app,
        project_path.as_deref(),
        Vec::new(),
        FindMessage::FileTreeFindTabSelected(String::new()),
    );

    assert!(app.active_find_results_tab_id.is_none());
}

#[test]
fn tab_selected_while_search_is_running_keeps_the_active_find_tab() {
    let mut app = FindState::default();
    let project_path: Option<String> = None;
    let mut running = find_tab("running");
    running.running = true;
    app.find_results_tabs.push(running);
    app.find_results_tabs.push(find_tab("other"));
    app.active_find_results_tab_id = Some("running".to_string());

    let _ = handle(
        &mut app,
        project_path.as_deref(),
        Vec::new(),
        FindMessage::FileTreeFindTabSelected("other".to_string()),
    );

    assert_eq!(app.active_find_results_tab_id.as_deref(), Some("running"));
}

#[test]
fn find_tab_right_click_records_menu_target_and_close_dismisses_it() {
    let mut app = FindState::default();
    let project_path: Option<String> = None;
    app.find_results_tabs.push(find_tab("a"));

    let _ = handle(
        &mut app,
        project_path.as_deref(),
        Vec::new(),
        FindMessage::FileTreeFindTabRightClicked("a".to_string(), 12.0, 24.0),
    );

    assert_eq!(app.find_results_tab_menu_id.as_deref(), Some("a"));
    assert_eq!(app.find_results_tab_menu_pos, Some(iced::Point::new(12.0, 24.0)));

    let _ = handle(
        &mut app,
        project_path.as_deref(),
        Vec::new(),
        FindMessage::FileTreeFindTabMenuClose,
    );

    assert!(app.find_results_tab_menu_id.is_none());
    assert!(app.find_results_tab_menu_pos.is_none());
}
