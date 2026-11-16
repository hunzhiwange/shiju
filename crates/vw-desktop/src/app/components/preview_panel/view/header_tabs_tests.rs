// 保留宿主契约：标签条目数量投影、ProjectMessage/PreviewMessage 的路径和指针映射及宿主模块接线；纯构建已迁至 desktop-editor。
//! 验证预览标题标签区域测试模块的装配入口。
//! 该哨兵确保标题标签实现仍被父视图的测试构建包含。

#[test]
fn header_tabs_tests_are_wired() {
    assert!(module_path!().contains("header_tabs_tests"));
}

use super::header_tabs::{
    build_header_tabs, header_tab_item_count, open_file_message, tab_close_message, tab_drag_end_message, tab_drag_start_message, tab_right_click_message, tab_select_message,
};
use crate::app::components::editor::Editor;
use crate::app::message::{PreviewMessage, ProjectMessage};
use crate::app::{App, Message, PreviewTab};
use iced::widget::Id;
use iced::Point;

fn app() -> App {
    App::new().0
}

fn tab(path: &str, dirty: bool) -> PreviewTab {
    PreviewTab {
        path: path.to_string(),
        title: path.rsplit('/').next().unwrap_or(path).to_string(),
        content: "content".to_string(),
        is_dirty: dirty,
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

#[test]
fn header_tabs_keep_open_file_entry_last_for_empty_and_multiple_tabs() {
    let empty_app = app();
    assert_eq!(header_tab_item_count(&empty_app), 1);
    keep(build_header_tabs(&empty_app));

    let mut populated_app = app();
    populated_app.preview_tabs = vec![tab("/tmp/a.rs", false), tab("/tmp/b.ts", false)];
    assert_eq!(header_tab_item_count(&populated_app), 3);
    keep(build_header_tabs(&populated_app));
}

#[test]
fn tab_messages_preserve_paths_and_pointer_position() {
    let Message::Project(ProjectMessage::FileTreeDragStart(path, None)) =
        tab_drag_start_message("/tmp/a.rs")
    else {
        panic!("expected drag start message");
    };
    assert_eq!(path, "/tmp/a.rs");
    assert!(matches!(tab_drag_end_message(), Message::Project(ProjectMessage::FileTreeDragEnd)));

    let Message::Preview(PreviewMessage::Close(path)) = tab_close_message("/tmp/a.rs") else {
        panic!("expected close message");
    };
    assert_eq!(path, "/tmp/a.rs");

    let Message::Preview(PreviewMessage::Select(path)) = tab_select_message("/tmp/b.rs") else {
        panic!("expected select message");
    };
    assert_eq!(path, "/tmp/b.rs");

    let Message::Preview(PreviewMessage::TabRightClicked(path, x, y)) =
        tab_right_click_message("/tmp/c.rs", Point::new(8.0, 13.0))
    else {
        panic!("expected tab right-click message");
    };
    assert_eq!(path, "/tmp/c.rs");
    assert_eq!((x, y), (8.0, 13.0));
}

#[test]
fn open_file_entry_reuses_existing_project_open_file_message() {
    assert!(matches!(open_file_message(), Message::Project(ProjectMessage::OpenFilePressed)));
}
