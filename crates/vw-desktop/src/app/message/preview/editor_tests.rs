//! 覆盖预览编辑器消息处理的本地编辑器、菜单和定位行为。

use crate::app::components::editor::Editor;
use crate::app::components::editor::{
    APP_CONTEXT_MENU_INSERT_SELECTION, APP_CONTEXT_MENU_INSERT_SELECTION_POSITIONS,
};
use crate::app::message::preview::{PreviewMessage, update};
use crate::app::preview::PreviewTab;
use crate::app::{App, FocusArea, Message, PreviewAutoSaveMode, message};
use iced::widget::Id;

fn test_app() -> App {
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
        editor: Editor::new(content, "rust"),
        scroll_id: Id::unique(),
        #[cfg(not(target_arch = "wasm32"))]
        lsp_server_key: None,
        #[cfg(not(target_arch = "wasm32"))]
        lsp_uri: None,
        #[cfg(not(target_arch = "wasm32"))]
        lsp_language_id: None,
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn collect_task_messages(task: iced::Task<crate::app::Message>) -> Vec<crate::app::Message> {
    use iced_runtime::futures::futures::StreamExt;

    let Some(mut stream) = iced_runtime::task::into_stream(task.collect()) else {
        return Vec::new();
    };
    while let Some(action) = stream.next().await {
        if let iced_runtime::Action::Output(messages) = action {
            return messages;
        }
    }
    Vec::new()
}









#[test]
fn update_context_menu_and_fullscreen_flags_are_local() {
    let mut app = test_app();
    let path = "/tmp/main.rs".to_string();
    app.active_preview_path = Some(path.clone());
    app.preview_tabs = vec![tab(&path, "fn main() {}")];

    let _ = update(&mut app, PreviewMessage::ContextMenuOpenForActiveEditor(10.0, 20.0));
    assert!(app.show_preview_context_menu);
    assert_eq!(app.preview_context_menu_pos, Some((10.0, 20.0)));
    assert_eq!(app.preview_context_target, Some((path.clone(), 1, 1, 1, 1)));

    app.cursor_position = iced::Point::new(30.0, 40.0);
    let _ = update(&mut app, PreviewMessage::ContextMenuOpenForActiveEditor(10.0, 20.0));
    assert_eq!(app.preview_context_menu_pos, Some((30.0, 40.0)));

    let _ = update(&mut app, PreviewMessage::ContextMenuClose);
    assert!(!app.show_preview_context_menu);
    assert_eq!(app.preview_context_menu_pos, None);
    assert_eq!(app.preview_context_target, None);

    let _ = update(&mut app, PreviewMessage::FullscreenOverlayEntered);
    assert!(app.show_preview_fullscreen_overlay);
    let _ = update(&mut app, PreviewMessage::FullscreenOverlayExited);
    assert!(!app.show_preview_fullscreen_overlay);
}

#[test]
fn context_menu_open_preserves_valid_selection_and_requires_active_path() {
    let mut app = test_app();
    app.show_preview_context_menu = false;
    app.preview_context_menu_pos = Some((7.0, 8.0));

    let _ = update(&mut app, PreviewMessage::ContextMenuOpenForActiveEditor(10.0, 20.0));
    assert!(!app.show_preview_context_menu);
    assert_eq!(app.preview_context_menu_pos, Some((7.0, 8.0)));

    let path = "/tmp/main.rs".to_string();
    let selection = (path.clone(), 2, 3, 4, 5);
    app.active_preview_path = Some(path);
    app.preview_context_target = Some(selection.clone());

    let _ = update(&mut app, PreviewMessage::ContextMenuOpenForActiveEditor(10.0, 20.0));
    assert!(app.show_preview_context_menu);
    assert_eq!(app.preview_context_target, Some(selection));
}



#[cfg(not(target_arch = "wasm32"))]
#[test]
fn preview_context_menu_custom_actions_route_before_editor_update() {
    let mut app = test_app();

    let insert_selection = iced::futures::executor::block_on(collect_task_messages(update(
        &mut app,
        PreviewMessage::EditorEvent(iced_code_editor::Message::CustomContextMenuAction(
            APP_CONTEXT_MENU_INSERT_SELECTION.to_string(),
        )),
    )));
    assert!(matches!(
        insert_selection.as_slice(),
        [Message::Chat(message::ChatMessage::InsertSelected)]
    ));

    let insert_positions = iced::futures::executor::block_on(collect_task_messages(update(
        &mut app,
        PreviewMessage::EditorEvent(iced_code_editor::Message::CustomContextMenuAction(
            APP_CONTEXT_MENU_INSERT_SELECTION_POSITIONS.to_string(),
        )),
    )));
    assert!(matches!(
        insert_positions.as_slice(),
        [Message::Chat(message::ChatMessage::InsertSelectionPositions)]
    ));

    let unknown = iced::futures::executor::block_on(collect_task_messages(update(
        &mut app,
        PreviewMessage::EditorEvent(iced_code_editor::Message::CustomContextMenuAction(
            "app.unknown".to_string(),
        )),
    )));
    assert!(unknown.is_empty());
}



#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn preview_context_menu_reveal_routes_gateway_path_without_local_file_check() {
    let mut app = test_app();
    let path = "/gateway-only/project/文件.rs".to_string();
    app.active_preview_path = Some(path.clone());
    let task = update(
        &mut app,
        PreviewMessage::EditorEvent(iced_code_editor::Message::RevealInFileManager),
    );
    let messages = collect_task_messages(task).await;
    assert!(matches!(
        messages.as_slice(),
        [Message::View(message::ViewMessage::OpenPathInFinder(target))] if target == &path
    ));
}

#[test]
fn update_editor_event_sets_focus_dirty_and_autosave_revision() {
    let mut app = test_app();
    let path = "/tmp/main.rs".to_string();
    app.active_preview_path = Some(path.clone());
    app.preview_tabs = vec![tab(&path, "")];
    app.preview_auto_save_mode = PreviewAutoSaveMode::AfterDelay;

    let _ = update(
        &mut app,
        PreviewMessage::EditorEvent(iced_code_editor::Message::CharacterInput('x')),
    );

    assert_eq!(app.focus_area, FocusArea::Preview);
    assert_eq!(app.preview_tabs[0].editor.content(), "");
    assert!(!app.preview_tabs[0].is_dirty);
    assert_eq!(app.preview_tabs[0].auto_save_revision, 0);

    let _ = update(&mut app, PreviewMessage::EditorEvent(iced_code_editor::Message::MouseRelease));
    assert_eq!(app.preview_tabs[0].auto_save_revision, 0);
}



#[test]
fn non_editing_editor_event_preserves_existing_dirty_state() {
    let mut app = test_app();
    let path = "/tmp/main.rs".to_string();
    let mut dirty_tab = tab(&path, "unchanged");
    dirty_tab.is_dirty = true;
    app.active_preview_path = Some(path);
    app.preview_tabs = vec![dirty_tab];

    let _ = update(&mut app, PreviewMessage::EditorEvent(iced_code_editor::Message::MouseRelease));

    assert_eq!(app.focus_area, FocusArea::Preview);
    assert!(app.preview_tabs[0].is_dirty);
    assert_eq!(app.preview_tabs[0].auto_save_revision, 0);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn close_search_clears_visible_completion_before_editor_focus_changes() {
    let mut app = test_app();
    app.focus_area = FocusArea::None;
    app.lsp_overlay.completion_visible = true;
    app.lsp_overlay.completion_items = vec!["Alpha".to_string()];

    let _ = update(&mut app, PreviewMessage::EditorEvent(iced_code_editor::Message::CloseSearch));

    assert_eq!(app.focus_area, FocusArea::None);
    assert!(!app.lsp_overlay.completion_visible);
    assert!(app.lsp_overlay.completion_items.is_empty());
}











#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn editor_event_completion_keyboard_navigation_returns_preview_messages() {
    let mut app = test_app();
    app.lsp_overlay.completion_visible = true;
    app.lsp_overlay.completion_items = vec!["Alpha".to_string(), "Beta".to_string()];

    let up_task = update(
        &mut app,
        PreviewMessage::EditorEvent(iced_code_editor::Message::ArrowKey(
            iced_code_editor::ArrowDirection::Up,
            false,
        )),
    );
    let up_messages = collect_task_messages(up_task).await;
    assert!(matches!(
        up_messages.as_slice(),
        [crate::app::Message::Preview(PreviewMessage::LspCompletionNavigateUp)]
    ));

    let down_task = update(
        &mut app,
        PreviewMessage::EditorEvent(iced_code_editor::Message::ArrowKey(
            iced_code_editor::ArrowDirection::Down,
            false,
        )),
    );
    let down_messages = collect_task_messages(down_task).await;
    assert!(matches!(
        down_messages.as_slice(),
        [crate::app::Message::Preview(PreviewMessage::LspCompletionNavigateDown)]
    ));

    let enter_task =
        update(&mut app, PreviewMessage::EditorEvent(iced_code_editor::Message::Enter));
    let enter_messages = collect_task_messages(enter_task).await;
    assert!(matches!(
        enter_messages.as_slice(),
        [crate::app::Message::Preview(PreviewMessage::LspCompletionConfirm)]
    ));
}







#[cfg(not(target_arch = "wasm32"))]
#[test]
fn completion_confirm_ignores_hidden_overlay_and_lsp_toggle_updates_status() {
    let mut app = test_app();
    app.lsp_overlay.completion_visible = false;
    app.lsp_overlay.completion_items = vec!["Alpha".to_string()];

    let _ = update(&mut app, PreviewMessage::LspCompletionConfirm);
    assert_eq!(app.lsp_overlay.completion_items, vec!["Alpha".to_string()]);

    let _ = update(&mut app, PreviewMessage::LspToggleDisabled(true));
    assert!(app.lsp_disabled);
    assert_eq!(app.lsp_status.as_deref(), Some("LSP 已禁用"));

    app.lsp_manager = None;
    let _ = update(&mut app, PreviewMessage::LspToggleDisabled(false));
    assert!(!app.lsp_disabled);
    assert_eq!(app.lsp_status.as_deref(), Some("LSP 已启用"));
}
