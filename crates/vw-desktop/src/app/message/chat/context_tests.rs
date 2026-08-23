//! 验证聊天上下文路径显示、文本追加和预览编辑状态同步。
//!
//! 预览夹具区分项目内外路径和活动会话，文本追加不得覆盖旧编辑器的独立状态。

#![allow(unused_must_use)]

use super::ChatMessage;
use crate::app::PreviewTab;
use crate::app::components::editor::Editor;
use iced::widget::{Id, text_editor};

fn has_scheduled_action(task: iced::Task<crate::app::Message>) -> bool {
    task.units() > 0
}

fn preview_tab(path: &str, content: &str) -> PreviewTab {
    PreviewTab {
        path: path.to_string(),
        title: path.rsplit('/').next().unwrap_or(path).to_string(),
        content: content.to_string(),
        is_dirty: false,
        truncated: false,
        auto_save_revision: 0,
        editor: Editor::new(content, "text"),
        scroll_id: Id::unique(),
        #[cfg(not(target_arch = "wasm32"))]
        lsp_server_key: None,
        #[cfg(not(target_arch = "wasm32"))]
        lsp_uri: None,
        #[cfg(not(target_arch = "wasm32"))]
        lsp_language_id: None,
    }
}

#[test]
fn context_tests_module_is_wired() {
    assert!(module_path!().ends_with("context_tests"));
}

#[test]
fn mention_display_path_makes_project_paths_relative() {
    let (mut app, _) = crate::app::App::new();
    app.project_path = Some("/workspace".to_string());

    assert_eq!(super::mention_display_path(&app, "/workspace/src/main.rs"), "src/main.rs");
    assert_eq!(super::mention_display_path(&app, "/workspace"), "");
}

#[test]
fn mention_display_path_preserves_outside_path_and_normalizes_separators() {
    let (mut app, _) = crate::app::App::new();
    app.project_path = Some("/workspace".to_string());

    assert_eq!(super::mention_display_path(&app, r"C:\repo\src\main.rs"), "C:/repo/src/main.rs");
}

#[test]
fn mention_display_path_without_project_normalizes_separators() {
    let (app, _) = crate::app::App::new();

    assert_eq!(super::mention_display_path(&app, r"src\nested\lib.rs"), "src/nested/lib.rs");
}

#[test]
fn append_text_adds_text_to_current_runtime_editor() {
    let (mut app, _) = crate::app::App::new();

    super::update(&mut app, ChatMessage::AppendText("first".to_string()));
    super::update(&mut app, ChatMessage::AppendText("second".to_string()));

    assert_eq!(app.current_session_runtime().input_editor.text(), "first\nsecond");
    assert_eq!(app.input_editor.text(), "first\nsecond");
}

#[test]
fn append_text_ignores_empty_text() {
    let (mut app, _) = crate::app::App::new();
    app.input_editor = text_editor::Content::with_text("keep");
    app.current_session_runtime_mut().input_editor = text_editor::Content::with_text("keep");

    let task = super::update(&mut app, ChatMessage::AppendText(String::new()));

    assert_eq!(app.current_session_runtime().input_editor.text(), "keep");
    assert_eq!(app.input_editor.text(), "keep");
    assert!(!has_scheduled_action(task));
}

#[test]
fn append_text_updates_active_session_without_overwriting_legacy_editor() {
    let (mut app, _) = crate::app::App::new();
    app.input_editor = text_editor::Content::with_text("legacy draft");
    app.active_session_id = Some("session-a".to_string());
    app.current_session_runtime_mut().input_editor =
        text_editor::Content::with_text("session draft");

    super::update(&mut app, ChatMessage::AppendText("addition".to_string()));

    assert_eq!(app.current_session_runtime().input_editor.text(), "session draft\naddition");
    assert_eq!(app.input_editor.text(), "legacy draft");
}

#[test]
fn append_text_to_empty_active_session_avoids_leading_newline_and_preserves_legacy_editor() {
    let (mut app, _) = crate::app::App::new();
    app.input_editor = text_editor::Content::with_text("legacy draft");
    app.active_session_id = Some("session-a".to_string());

    let task = super::update(&mut app, ChatMessage::AppendText("first".to_string()));

    assert_eq!(app.current_session_runtime().input_editor.text(), "first");
    assert_eq!(app.input_editor.text(), "legacy draft");
    assert!(!has_scheduled_action(task));
}

#[test]
fn insert_position_uses_project_relative_path_and_closes_context_menu() {
    let (mut app, _) = crate::app::App::new();
    app.project_path = Some("/workspace".to_string());
    app.current_session_runtime_mut().input_editor = text_editor::Content::with_text("prompt");
    app.show_preview_context_menu = true;

    let task = super::update(
        &mut app,
        ChatMessage::InsertPosition("/workspace/src/main.rs".to_string(), 12, 4),
    );

    assert_eq!(
        app.current_session_runtime().input_editor.text(),
        "prompt\n文件:src/main.rs 行:12 列:4"
    );
    assert_eq!(app.input_editor.text(), "prompt\n文件:src/main.rs 行:12 列:4");
    assert!(!app.show_preview_context_menu);
    assert!(!has_scheduled_action(task));
}

#[test]
fn insert_position_normalizes_path_and_isolates_active_session() {
    let (mut app, _) = crate::app::App::new();
    app.input_editor = text_editor::Content::with_text("legacy draft");
    app.active_session_id = Some("session-a".to_string());
    app.show_preview_context_menu = true;

    super::update(&mut app, ChatMessage::InsertPosition(r"src\main.rs".to_string(), 0, usize::MAX));

    assert_eq!(
        app.current_session_runtime().input_editor.text(),
        format!("文件:src/main.rs 行:0 列:{}", usize::MAX)
    );
    assert_eq!(app.input_editor.text(), "legacy draft");
    assert!(!app.show_preview_context_menu);
}

#[test]
fn insert_selection_positions_uses_context_target_when_present() {
    let (mut app, _) = crate::app::App::new();
    app.project_path = Some("/workspace".to_string());
    app.current_session_runtime_mut().input_editor = text_editor::Content::with_text("prompt");
    app.preview_context_target = Some(("/workspace/src/lib.rs".to_string(), 1, 2, 3, 4));
    app.show_preview_context_menu = true;

    let task = super::update(&mut app, ChatMessage::InsertSelectionPositions);

    assert_eq!(app.current_session_runtime().input_editor.text(), "prompt\n@src/lib.rs:1:2-3:4");
    assert_eq!(app.input_editor.text(), "prompt\n@src/lib.rs:1:2-3:4");
    assert!(!app.show_preview_context_menu);
    assert!(!has_scheduled_action(task));
}

#[test]
fn insert_selection_positions_updates_active_session_only() {
    let (mut app, _) = crate::app::App::new();
    app.input_editor = text_editor::Content::with_text("legacy draft");
    app.active_session_id = Some("session-a".to_string());
    app.preview_context_target = Some((r"src\lib.rs".to_string(), 0, 0, usize::MAX, usize::MAX));

    super::update(&mut app, ChatMessage::InsertSelectionPositions);

    assert_eq!(
        app.current_session_runtime().input_editor.text(),
        format!("@src/lib.rs:0:0-{}:{}", usize::MAX, usize::MAX)
    );
    assert_eq!(app.input_editor.text(), "legacy draft");
}

#[test]
fn insert_selection_positions_without_target_only_closes_menu() {
    let (mut app, _) = crate::app::App::new();
    app.current_session_runtime_mut().input_editor = text_editor::Content::with_text("keep");
    app.show_preview_context_menu = true;

    let task = super::update(&mut app, ChatMessage::InsertSelectionPositions);

    assert_eq!(app.current_session_runtime().input_editor.text(), "keep");
    assert!(!app.show_preview_context_menu);
    assert!(!has_scheduled_action(task));
}

#[test]
fn insert_selected_without_active_preview_only_closes_menu() {
    let (mut app, _) = crate::app::App::new();
    app.show_preview_context_menu = true;

    let task = super::update(&mut app, ChatMessage::InsertSelected);

    assert!(!app.show_preview_context_menu);
    assert!(!has_scheduled_action(task));
}

#[test]
fn insert_selected_without_matching_tab_only_closes_menu() {
    let (mut app, _) = crate::app::App::new();
    app.active_preview_path = Some("/workspace/missing.rs".to_string());
    app.preview_tabs = vec![preview_tab("/workspace/other.rs", "other")];
    app.show_preview_context_menu = true;

    let task = super::update(&mut app, ChatMessage::InsertSelected);

    assert!(!app.show_preview_context_menu);
    assert!(!has_scheduled_action(task));
}

#[test]
fn insert_selected_with_matching_tab_closes_menu_and_schedules_copy_chain() {
    let (mut app, _) = crate::app::App::new();
    let path = "/workspace/main.rs";
    app.active_preview_path = Some(path.to_string());
    app.preview_tabs = vec![preview_tab(path, "fn main() {}")];
    app.show_preview_context_menu = true;

    let task = super::update(&mut app, ChatMessage::InsertSelected);

    assert!(!app.show_preview_context_menu);
    assert!(has_scheduled_action(task));
}

#[test]
fn explicit_noop_context_messages_preserve_state() {
    let (mut app, _) = crate::app::App::new();
    app.current_session_runtime_mut().input_editor = text_editor::Content::with_text("keep");
    app.show_preview_context_menu = true;

    let active_match_task = super::update(&mut app, ChatMessage::InsertActiveMatch);
    let selection_range_task = super::update(&mut app, ChatMessage::InsertSelectionRange);

    assert_eq!(app.current_session_runtime().input_editor.text(), "keep");
    assert!(app.show_preview_context_menu);
    assert!(!has_scheduled_action(active_match_task));
    assert!(!has_scheduled_action(selection_range_task));
}

#[test]
fn unrelated_message_hits_fallback_without_changing_context_state() {
    let (mut app, _) = crate::app::App::new();
    app.current_session_runtime_mut().input_editor = text_editor::Content::with_text("keep");
    app.show_preview_context_menu = true;

    let task = super::update(&mut app, ChatMessage::InputChanged("ignored".to_string()));

    assert_eq!(app.current_session_runtime().input_editor.text(), "keep");
    assert!(app.show_preview_context_menu);
    assert!(!has_scheduled_action(task));
}
