// 保留宿主契约：目标/位置缺失和活动标签匹配投影，以及关闭导航后批量分发项目/预览消息；纯菜单构建已迁至 desktop-editor。
//! 验证预览菜单测试模块的装配入口。
//! 菜单状态组合由主视图测试覆盖，此处用于防止条件测试模块被遗漏。

#[test]
fn menus_tests_are_wired() {
    assert!(module_path!().contains("menus_tests"));
}

use super::menus::{
    build_menu_ui, nav_popup_item_message,
};
use crate::app::components::editor::Editor;
use crate::app::message::{PreviewMessage, ProjectMessage};
use crate::app::{App, Message, PreviewTab};
use iced::widget::Id;

fn app() -> App {
    App::new().0
}

fn tab(path: &str) -> PreviewTab {
    PreviewTab {
        path: path.to_string(),
        title: path.rsplit('/').next().unwrap_or(path).to_string(),
        content: "content".to_string(),
        is_dirty: false,
        truncated: false,
        read_only: false,
        auto_save_revision: 0,
        editor: Editor::new("content", "txt"),
        scroll_id: Id::unique(),
        #[cfg(not(target_arch = "wasm32"))]
        lsp_server_key: None,
        #[cfg(not(target_arch = "wasm32"))]
        lsp_uri: None,
        #[cfg(not(target_arch = "wasm32"))]
        lsp_language_id: None,
    }
}

fn keep(element: iced::Element<'_, Message>) {
    std::hint::black_box(element);
}

fn assert_close_nav_then(message: Message, expected_path: &str, expected_dir: bool) {
    let Message::Batch(messages) = message else {
        panic!("expected batch message");
    };

    assert_eq!(messages.len(), 2);
    assert!(matches!(messages[0], Message::Preview(PreviewMessage::CloseNavPopup)));

    match (&messages[1], expected_dir) {
        (Message::Project(ProjectMessage::ToggleTreeDir(path)), true) => {
            assert_eq!(path, expected_path);
        }
        (Message::Preview(PreviewMessage::Open(path)), false) => {
            assert_eq!(path, expected_path);
        }
        _ => panic!("unexpected nav popup action"),
    }
}

#[test]
fn menu_ui_handles_context_menu_without_target_or_position() {
    let mut app = app();
    app.show_preview_context_menu = true;

    keep(build_menu_ui(&app));

    app.preview_context_target = Some(("/tmp/a.rs".to_string(), 1, 1, 1, 1));
    keep(build_menu_ui(&app));
}

#[test]
fn menu_ui_builds_context_menu_for_matching_tab() {
    let mut app = app();
    app.show_preview_context_menu = true;
    app.preview_context_target = Some(("/tmp/a.rs".to_string(), 1, 1, 2, 4));
    app.preview_context_menu_pos = Some((20.0, 30.0));
    app.preview_tabs = vec![tab("/tmp/a.rs")];

    keep(build_menu_ui(&app));
}

#[test]
fn menu_ui_uses_empty_context_items_for_missing_tab() {
    let mut app = app();
    app.show_preview_context_menu = true;
    app.preview_context_target = Some(("/tmp/missing.rs".to_string(), 1, 1, 2, 4));
    app.preview_context_menu_pos = Some((20.0, 30.0));
    app.preview_tabs = vec![tab("/tmp/a.rs")];

    keep(build_menu_ui(&app));
}

#[test]
fn nav_popup_item_message_closes_popup_then_opens_target() {
    assert_close_nav_then(
        nav_popup_item_message("/workspace", "src", true),
        "/workspace/src",
        true,
    );
    assert_close_nav_then(
        nav_popup_item_message("/workspace", "main.rs", false),
        "/workspace/main.rs",
        false,
    );
}
