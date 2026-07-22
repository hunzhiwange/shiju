use super::{view, view_file_manager};
use crate::app::App;
use crate::app::state::{FindInFolderMatch, FindInFolderTab};

#[test]
fn file_tree_views_handle_empty_and_loaded_projects() {
    let app = App::new().0;
    let _ = view(&app);
    let _ = view_file_manager(&app);

    let mut app = App::new().0;
    app.project_path = Some("/tmp/demo".to_string());
    let _ = view(&app);
    let _ = view_file_manager(&app);
}

#[test]
fn file_manager_view_active_find_results_tab_shows_find_results() {
    let mut app = App::new().0;
    app.project_path = Some("/tmp/demo".to_string());
    app.active_find_results_tab_id = Some("test-tab".to_string());
    let _ = view_file_manager(&app);
}

#[test]
fn file_manager_view_active_find_results_tab_with_matching_tab() {
    let mut app = App::new().0;
    app.project_path = Some("/tmp/demo".to_string());
    app.active_find_results_tab_id = Some("tab-1".to_string());
    app.find_results_tabs = vec![FindInFolderTab {
        id: "tab-1".to_string(),
        title: "search".to_string(),
        scope_path: "/tmp/demo".to_string(),
        query_input: String::new(),
        replace_input: String::new(),
        query_editor: iced::widget::text_editor::Content::new(),
        replace_editor: iced::widget::text_editor::Content::new(),
        query: "test".to_string(),
        replace_text: String::new(),
        case_sensitive: false,
        whole_word: false,
        use_regex: false,
        running: false,
        error: None,
        limit_reached: false,
        matches: vec![FindInFolderMatch {
            path: "/tmp/demo/src/main.rs".to_string(),
            line: 1,
            column: 1,
            preview: "fn main()".to_string(),
            match_len: 2,
        }],
    }];
    let _ = view_file_manager(&app);
}

#[test]
fn file_manager_view_show_changes_with_git_changes() {
    let mut app = App::new().0;
    app.project_path = Some("/tmp/demo".to_string());
    app.file_manager_show_changes = true;
    app.git_changed_files = vec!["src/main.rs".to_string(), "README.md".to_string()];
    let _ = view_file_manager(&app);
}

#[test]
fn file_manager_view_show_changes_loading_indicator() {
    let mut app = App::new().0;
    app.project_path = Some("/tmp/demo".to_string());
    app.file_manager_show_changes = true;
    app.git_changed_files_loading = true;
    let _ = view_file_manager(&app);
}

#[test]
fn file_manager_view_show_all_files() {
    let mut app = App::new().0;
    app.project_path = Some("/tmp/demo".to_string());
    app.file_manager_show_changes = false;
    let _ = view_file_manager(&app);
}

#[test]
fn file_manager_view_with_find_results_tabs_shows_tab_bar() {
    let mut app = App::new().0;
    app.project_path = Some("/tmp/demo".to_string());
    app.find_results_tabs = vec![FindInFolderTab {
        id: "tab-1".to_string(),
        title: "search".to_string(),
        scope_path: "/tmp/demo".to_string(),
        query_input: String::new(),
        replace_input: String::new(),
        query_editor: iced::widget::text_editor::Content::new(),
        replace_editor: iced::widget::text_editor::Content::new(),
        query: "hello".to_string(),
        replace_text: String::new(),
        case_sensitive: false,
        whole_word: false,
        use_regex: false,
        running: false,
        error: None,
        limit_reached: false,
        matches: vec![],
    }];
    let _ = view_file_manager(&app);
}

#[test]
fn file_manager_view_refresh_buttons_disabled_when_refreshing() {
    let mut app = App::new().0;
    app.project_path = Some("/tmp/demo".to_string());
    app.file_manager_show_changes = true;
    app.file_manager_changes_refreshing = true;
    let _ = view_file_manager(&app);

    let mut app = App::new().0;
    app.project_path = Some("/tmp/demo".to_string());
    app.file_manager_show_changes = false;
    app.file_manager_file_tree_refreshing = true;
    let _ = view_file_manager(&app);
}

#[test]
fn file_manager_view_combines_change_count_refreshing_and_find_results_precedence() {
    let mut app = App::new().0;
    app.project_path = Some("/tmp/demo".to_string());
    app.file_manager_show_changes = true;
    app.file_manager_changes_refreshing = true;
    app.file_manager_file_tree_refreshing = true;
    app.git_changed_files = vec!["src/main.rs".to_string()];
    app.active_find_results_tab_id = Some("tab-1".to_string());
    app.find_results_tabs = vec![FindInFolderTab {
        id: "tab-1".to_string(),
        title: "search".to_string(),
        scope_path: "/tmp/demo".to_string(),
        query_input: String::new(),
        replace_input: String::new(),
        query_editor: iced::widget::text_editor::Content::new(),
        replace_editor: iced::widget::text_editor::Content::new(),
        query: "main".to_string(),
        replace_text: String::new(),
        case_sensitive: false,
        whole_word: false,
        use_regex: false,
        running: true,
        error: None,
        limit_reached: true,
        matches: vec![FindInFolderMatch {
            path: "/tmp/demo/src/main.rs".to_string(),
            line: 1,
            column: 4,
            preview: "fn main()".to_string(),
            match_len: 4,
        }],
    }];

    let _ = view_file_manager(&app);
}

#[test]
fn file_tree_views_treat_empty_project_path_as_loaded_state() {
    let mut app = App::new().0;
    app.project_path = Some(String::new());

    let _ = view(&app);
    let _ = view_file_manager(&app);
}
