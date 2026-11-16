// 保留宿主契约：Screen、文件管理器 changes 开关选择 git_panel，以及活动路径在多标签中的选择；内容类型分支已迁至 desktop-editor。
//! 验证预览内容视图测试模块的装配入口。
//! 内容类型的具体渲染分支由同目录视图测试承担，此处只守住模块接线。

#[test]
fn content_tests_are_wired() {
    assert!(module_path!().contains("content_tests"));
}

use super::content::build_content_base;
use crate::app::components::editor::Editor;
use crate::app::{App, PreviewTab, Screen};
use iced::advanced::widget::Tree;
use iced::widget::Id;

fn app() -> App {
    App::new().0
}

fn tab(path: &str, content: &str) -> PreviewTab {
    PreviewTab {
        path: path.to_string(),
        title: path.rsplit('/').next().unwrap_or(path).to_string(),
        content: content.to_string(),
        is_dirty: false,
        truncated: false,
        read_only: false,
        auto_save_revision: 0,
        editor: Editor::new(content, "txt"),
        scroll_id: Id::unique(),
        #[cfg(not(target_arch = "wasm32"))]
        lsp_server_key: None,
        #[cfg(not(target_arch = "wasm32"))]
        lsp_uri: None,
        #[cfg(not(target_arch = "wasm32"))]
        lsp_language_id: None,
    }
}

fn content_tree(app: &App) -> Tree {
    let element = build_content_base(app);
    Tree::new(element.as_widget())
}

#[test]
fn content_shows_empty_selection_message_in_preview_screen() {
    let mut app = app();
    app.screen = Screen::Preview;
    app.active_preview_path = None;

    let tree = content_tree(&app);

    assert!(tree.children.is_empty());
}

#[test]
fn content_selects_the_tab_matching_the_active_path() {
    let mut editor_app = app();
    editor_app.active_preview_path = Some("/tmp/notes.txt".to_string());
    editor_app.preview_tabs = vec![tab("/tmp/notes.txt", "notes")];
    let editor_tag = content_tree(&editor_app).tag;

    let mut app = app();
    app.active_preview_path = Some("/tmp/selected.png".to_string());
    app.preview_tabs = vec![
        tab("/tmp/first.txt", "first"),
        tab("/tmp/selected.png", ""),
        tab("/tmp/last.rs", "last"),
    ];

    assert_ne!(content_tree(&app).tag, editor_tag);
}

#[test]
fn content_shows_empty_selection_message_when_project_changes_are_hidden() {
    let mut app = app();
    app.screen = Screen::Project;
    app.active_preview_path = None;
    app.file_manager.file_manager_show_changes = false;

    let tree = content_tree(&app);

    assert!(tree.children.is_empty());
}

#[test]
fn content_uses_git_panel_when_changes_view_is_enabled() {
    let mut app = app();
    app.active_preview_path = None;
    app.file_manager.file_manager_show_changes = true;
    app.screen = Screen::Project;

    let tree = content_tree(&app);

    assert_eq!(tree.children.len(), 1);
}
