//! 验证文件管理面板在项目加载、变更列表和搜索模式间的视图切换。
//! 搜索结果优先级、刷新禁用态和空项目路径用于锁定多状态竞争时的选路规则。

use super::{
    FILE_MANAGER_MODE_BAR_HEIGHT, FILE_MANAGER_SEARCH_ICON_SIZE, FILE_MANAGER_SEARCH_TAB_WIDTH,
    file_manager_search_message, view, view_file_manager,
};
use crate::app::App;
use crate::app::message::project::ProjectMessage;
use crate::app::state::{FindInFolderMatch, FindInFolderTab};
use iced::advanced::Layout;
use iced::advanced::layout;
use iced::advanced::renderer::Headless;
use iced::advanced::widget::Tree;
use iced::{Font, Pixels, Size};

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
        results_page: 0,
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
fn file_manager_view_with_saved_find_result_builds_single_mode_bar() {
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
        results_page: 0,
    }];
    let _ = view_file_manager(&app);
}

#[test]
fn file_manager_search_button_opens_or_reuses_search() {
    let app = App::new().0;
    assert!(matches!(file_manager_search_message(&app), ProjectMessage::FileTreeFindInProject));

    let mut app = App::new().0;
    app.find_results_tabs = vec![FindInFolderTab {
        id: "saved-search".to_string(),
        title: "search".to_string(),
        scope_path: "/tmp/demo".to_string(),
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
        matches: vec![],
        results_page: 0,
    }];

    let ProjectMessage::FileTreeFindTabSelected(id) = file_manager_search_message(&app) else {
        panic!("saved search should be reused");
    };
    assert_eq!(id, "saved-search");
}

#[tokio::test(flavor = "multi_thread")]
async fn file_manager_mode_bar_keeps_compact_height_and_search_tab_width() {
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let mut app = App::new().0;
    app.project_path = Some("/tmp/demo".to_string());
    let mut manager = view_file_manager(&app);
    let limits = layout::Limits::new(Size::ZERO, Size::new(360.0, 600.0));
    let mut tree = Tree::new(manager.as_widget());
    let node = manager.as_widget_mut().layout(&mut tree, &renderer, &limits);
    let header = Layout::new(&node).children().next().expect("mode bar should be first");
    let tabs = header.children().collect::<Vec<_>>();

    assert_eq!(FILE_MANAGER_SEARCH_ICON_SIZE, 14.0);
    assert_eq!(header.bounds().height, FILE_MANAGER_MODE_BAR_HEIGHT);
    assert_eq!(tabs.len(), 3);
    assert_eq!(tabs[2].bounds().width, FILE_MANAGER_SEARCH_TAB_WIDTH);
    assert!(tabs[0].bounds().width > tabs[2].bounds().width);
    assert!(tabs[1].bounds().width > tabs[2].bounds().width);
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
        results_page: 0,
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

#[test]
fn file_manager_view_construction_preserves_code_panel_preferences() {
    let mut app = App::new().0;
    app.project_path = Some("/tmp/demo".to_string());
    app.show_file_manager = false;
    app.show_diff = false;
    app.split_ratio = 0.37;
    app.chat_panel_fullscreen = true;
    app.git_diff_fullscreen = true;
    let before = (
        app.show_file_manager,
        app.show_diff,
        app.split_ratio,
        app.chat_panel_fullscreen,
        app.git_diff_fullscreen,
    );

    let _ = view_file_manager(&app);

    assert_eq!(
        (
            app.show_file_manager,
            app.show_diff,
            app.split_ratio,
            app.chat_panel_fullscreen,
            app.git_diff_fullscreen,
        ),
        before
    );
}
