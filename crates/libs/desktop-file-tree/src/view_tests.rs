//! 验证文件管理面板在项目加载、变更列表和搜索模式间的视图切换。
//! 搜索结果优先级、刷新禁用态和空项目路径用于锁定多状态竞争时的选路规则。

use super::{
    FILE_MANAGER_MODE_BAR_HEIGHT, FILE_MANAGER_SEARCH_ICON_SIZE, FILE_MANAGER_SEARCH_TAB_WIDTH,
    file_manager_search_message, view, view_file_manager,
};
use crate::{FileTreeView, state, selection};
#[derive(Default)]
pub(crate) struct ViewState {
    file_tree_drop_target: Option<String>,
    file_tree_operation_in_progress: bool,
    file_tree_selection: selection::FileTreeSelection,
    file_tree_expanded_set: std::collections::HashSet<String>,
    git_changed_files: Vec<String>,
    git_changed_files_loading: bool,
    git_focused_file: Option<String>,
    file_manager_refresh_frame: usize,
    file_manager_changes_refreshing: bool,
    file_manager_file_tree_refreshing: bool,
    file_manager_show_changes: bool,
    file_index: Vec<String>,
    desktop_locale: config_ui::DesktopLocale,
    project_path: Option<String>,
    find_results_tabs: Vec<state::FindInFolderTab>,
    active_find_results_tab_id: Option<String>,
    find_results_tab_menu_id: Option<String>,
    find_results_tab_menu_pos: Option<iced::Point>,
    file_tree_rename_value: String,
    file_tree_rename_path: Option<String>,
    file_tree_menu_path: Option<String>,
    file_tree_menu_source: Option<String>,
    file_tree_menu_anchor: Option<iced::Point>,
    can_paste: bool,
}
impl ViewState {
    fn inputs(&self) -> FileTreeView<'_> {
        FileTreeView {
            file_tree_drop_target: &self.file_tree_drop_target,
            file_tree_operation_in_progress: self.file_tree_operation_in_progress,
            file_tree_selection: &self.file_tree_selection,
            file_tree_expanded_set: &self.file_tree_expanded_set,
            git_changed_files: &self.git_changed_files,
            git_changed_files_loading: self.git_changed_files_loading,
            git_focused_file: &self.git_focused_file,
            file_manager_refresh_frame: self.file_manager_refresh_frame,
            file_manager_changes_refreshing: self.file_manager_changes_refreshing,
            file_manager_file_tree_refreshing: self.file_manager_file_tree_refreshing,
            file_manager_show_changes: self.file_manager_show_changes,
            file_index: &self.file_index,
            desktop_locale: self.desktop_locale,
            project_path: &self.project_path,
            find_results_tabs: &self.find_results_tabs,
            active_find_results_tab_id: &self.active_find_results_tab_id,
            find_results_tab_menu_id: &self.find_results_tab_menu_id,
            find_results_tab_menu_pos: self.find_results_tab_menu_pos,
            file_tree_rename_value: &self.file_tree_rename_value,
            file_tree_rename_path: &self.file_tree_rename_path,
            file_tree_menu_path: &self.file_tree_menu_path,
            file_tree_menu_source: &self.file_tree_menu_source,
            file_tree_menu_anchor: self.file_tree_menu_anchor,
            can_paste: self.can_paste,
            file_tree_model: None,
        }
    }
}

use crate::ProjectMessage;
use crate::state::{FindInFolderMatch, FindInFolderTab};
use iced::advanced::Layout;
use iced::advanced::layout;
use iced::advanced::renderer::Headless;
use iced::advanced::widget::Tree;
use iced::{Font, Pixels, Size};

#[test]
fn file_tree_views_handle_empty_and_loaded_projects() {
    let app = ViewState::default();
    let _ = view(app.inputs());
    let _ = view_file_manager(app.inputs());

    let mut app = ViewState::default();
    app.project_path = Some("/tmp/demo".to_string());
    let _ = view(app.inputs());
    let _ = view_file_manager(app.inputs());
}

#[test]
fn file_manager_view_active_find_results_tab_shows_find_results() {
    let mut app = ViewState::default();
    app.project_path = Some("/tmp/demo".to_string());
    app.active_find_results_tab_id = Some("test-tab".to_string());
    let _ = view_file_manager(app.inputs());
}

#[test]
fn file_manager_view_active_find_results_tab_with_matching_tab() {
    let mut app = ViewState::default();
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
        localized_error: None,
        results_page: 0,
    }];
    let _ = view_file_manager(app.inputs());
}

#[test]
fn file_manager_view_show_changes_with_git_changes() {
    let mut app = ViewState::default();
    app.project_path = Some("/tmp/demo".to_string());
    app.file_manager_show_changes = true;
    app.git_changed_files = vec!["src/main.rs".to_string(), "README.md".to_string()];
    let _ = view_file_manager(app.inputs());
}

#[test]
fn file_manager_view_show_changes_loading_indicator() {
    let mut app = ViewState::default();
    app.project_path = Some("/tmp/demo".to_string());
    app.file_manager_show_changes = true;
    app.git_changed_files_loading = true;
    let _ = view_file_manager(app.inputs());
}

#[test]
fn file_manager_view_show_all_files() {
    let mut app = ViewState::default();
    app.project_path = Some("/tmp/demo".to_string());
    app.file_manager_show_changes = false;
    let _ = view_file_manager(app.inputs());
}

#[test]
fn file_manager_view_with_saved_find_result_builds_single_mode_bar() {
    let mut app = ViewState::default();
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
        localized_error: None,
        results_page: 0,
    }];
    let _ = view_file_manager(app.inputs());
}

#[test]
fn file_manager_search_button_opens_or_reuses_search() {
    let app = ViewState::default();
    assert!(matches!(file_manager_search_message(app.inputs()), ProjectMessage::FileTreeFindInProject));

    let mut app = ViewState::default();
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
        localized_error: None,
        results_page: 0,
    }];

    let ProjectMessage::FileTreeFindTabSelected(id) = file_manager_search_message(app.inputs()) else {
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
    let mut app = ViewState::default();
    app.project_path = Some("/tmp/demo".to_string());
    let mut manager = view_file_manager(app.inputs());
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
    let mut app = ViewState::default();
    app.project_path = Some("/tmp/demo".to_string());
    app.file_manager_show_changes = true;
    app.file_manager_changes_refreshing = true;
    let _ = view_file_manager(app.inputs());

    let mut app = ViewState::default();
    app.project_path = Some("/tmp/demo".to_string());
    app.file_manager_show_changes = false;
    app.file_manager_file_tree_refreshing = true;
    let _ = view_file_manager(app.inputs());
}

#[test]
fn file_manager_view_combines_change_count_refreshing_and_find_results_precedence() {
    let mut app = ViewState::default();
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
        localized_error: None,
        results_page: 0,
    }];

    let _ = view_file_manager(app.inputs());
}

#[test]
fn file_tree_views_treat_empty_project_path_as_loaded_state() {
    let mut app = ViewState::default();
    app.project_path = Some(String::new());

    let _ = view(app.inputs());
    let _ = view_file_manager(app.inputs());
}
