//! 覆盖预览编辑器消息处理的本地编辑器、菜单和定位行为。

use super::{
    cursor_from_point, editor_message_may_change_content, normalize_pos,
    update_context_target_from_editor_event,
};
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
fn content_change_classifier_matches_editing_events() {
    assert!(editor_message_may_change_content(&iced_code_editor::Message::CharacterInput('x')));
    assert!(editor_message_may_change_content(&iced_code_editor::Message::Backspace));
    assert!(editor_message_may_change_content(&iced_code_editor::Message::Paste("x".to_string())));
    assert!(editor_message_may_change_content(&iced_code_editor::Message::Undo));
    assert!(!editor_message_may_change_content(&iced_code_editor::Message::MouseRelease));
    assert!(!editor_message_may_change_content(&iced_code_editor::Message::MouseHover(
        iced::Point::new(1.0, 1.0)
    )));
}

#[test]
fn content_change_classifier_covers_all_remaining_editing_events() {
    let editing_events = [
        iced_code_editor::Message::Delete,
        iced_code_editor::Message::Enter,
        iced_code_editor::Message::Tab,
        iced_code_editor::Message::Cut,
        iced_code_editor::Message::DeleteSelection,
        iced_code_editor::Message::Redo,
        iced_code_editor::Message::ReplaceNext,
        iced_code_editor::Message::ReplaceAll,
        iced_code_editor::Message::ImeCommit("输入".to_string()),
    ];

    for event in editing_events {
        assert!(editor_message_may_change_content(&event));
    }
}

#[test]
fn normalize_pos_orders_line_and_column_pairs() {
    assert_eq!(normalize_pos((1, 1), (2, 1)), ((1, 1), (2, 1)));
    assert_eq!(normalize_pos((2, 1), (1, 9)), ((1, 9), (2, 1)));
    assert_eq!(normalize_pos((3, 8), (3, 2)), ((3, 2), (3, 8)));
    assert_eq!(normalize_pos((4, 5), (4, 5)), ((4, 5), (4, 5)));
}

#[test]
fn cursor_from_point_respects_gutter_lines_and_wide_chars() {
    let editor = Editor::new("abc\n中文", "text");

    assert_eq!(cursor_from_point(&editor.inner, iced::Point::new(1.0, 1.0)), None);

    let first = cursor_from_point(&editor.inner, iced::Point::new(55.0, 1.0)).expect("first line");
    assert_eq!(first.0, 1);
    assert!(first.1 >= 1);

    let second =
        cursor_from_point(&editor.inner, iced::Point::new(55.0, editor.inner.line_height() + 1.0))
            .expect("second line");
    assert_eq!(second.0, 2);
    assert!(second.1 >= 1);
}

#[test]
fn cursor_from_point_clamps_beyond_content_and_uses_character_midpoints() {
    let editor = Editor::new("ab\n中", "text");
    let text_start = 50.0;

    assert_eq!(
        cursor_from_point(
            &editor.inner,
            iced::Point::new(text_start + editor.inner.char_width() * 0.4, 1.0),
        ),
        Some((1, 1))
    );
    assert_eq!(
        cursor_from_point(
            &editor.inner,
            iced::Point::new(text_start + editor.inner.char_width() * 0.6, 1.0),
        ),
        Some((1, 2))
    );
    assert_eq!(
        cursor_from_point(&editor.inner, iced::Point::new(10_000.0, 10_000.0)),
        Some((2, 2))
    );
}

#[test]
fn cursor_from_point_accepts_origin_when_line_numbers_are_disabled() {
    let mut editor = Editor::new("abc", "text");
    editor.set_line_numbers_enabled(false);

    assert_eq!(cursor_from_point(&editor.inner, iced::Point::new(0.0, 0.0)), Some((1, 1)));
}

#[test]
fn editor_events_update_context_target_for_click_and_drag() {
    let mut app = test_app();
    let path = "/tmp/main.rs".to_string();
    app.active_preview_path = Some(path.clone());
    app.preview_tabs = vec![tab(&path, "abc\n中文")];

    update_context_target_from_editor_event(
        &mut app,
        &iced_code_editor::Message::MouseClick(iced::Point::new(55.0, 1.0)),
    );
    let Some((target_path, start_line, start_col, end_line, end_col)) =
        app.preview_context_target.clone()
    else {
        panic!("expected context target");
    };
    assert_eq!(target_path, path);
    assert_eq!((start_line, start_col), (end_line, end_col));

    let second_line_y = app.preview_tabs[0].editor.inner.line_height() + 1.0;
    update_context_target_from_editor_event(
        &mut app,
        &iced_code_editor::Message::MouseDrag(iced::Point::new(85.0, second_line_y)),
    );
    let Some((_, drag_start_line, _, drag_end_line, _)) = app.preview_context_target.clone() else {
        panic!("expected drag target");
    };
    assert!(drag_start_line <= drag_end_line);

    app.active_preview_path = Some("/tmp/other.rs".to_string());
    update_context_target_from_editor_event(
        &mut app,
        &iced_code_editor::Message::MouseClick(iced::Point::new(55.0, 1.0)),
    );
    assert_eq!(
        app.preview_context_target.as_ref().map(|item| item.0.as_str()),
        Some(path.as_str())
    );
}

#[test]
fn context_target_ignores_missing_active_tab_and_invalid_gutter_drag() {
    let mut app = test_app();
    let original = ("/tmp/original.rs".to_string(), 2, 3, 4, 5);
    app.preview_context_target = Some(original.clone());

    update_context_target_from_editor_event(
        &mut app,
        &iced_code_editor::Message::MouseClick(iced::Point::new(55.0, 1.0)),
    );
    assert_eq!(app.preview_context_target, Some(original.clone()));

    let path = "/tmp/main.rs".to_string();
    app.active_preview_path = Some(path.clone());
    app.preview_tabs = vec![tab(&path, "abc")];
    update_context_target_from_editor_event(
        &mut app,
        &iced_code_editor::Message::MouseDrag(iced::Point::new(1.0, 1.0)),
    );
    assert_eq!(app.preview_context_target, Some(original));
}

#[test]
fn context_target_drag_with_mismatched_path_starts_new_selection() {
    let mut app = test_app();
    let path = "/tmp/main.rs".to_string();
    app.active_preview_path = Some(path.clone());
    app.preview_tabs = vec![tab(&path, "abc")];
    app.preview_context_target = Some(("/tmp/other.rs".to_string(), 9, 9, 9, 9));

    update_context_target_from_editor_event(
        &mut app,
        &iced_code_editor::Message::MouseDrag(iced::Point::new(55.0, 1.0)),
    );

    let Some((target_path, start_line, start_col, end_line, end_col)) = app.preview_context_target
    else {
        panic!("expected new drag target");
    };
    assert_eq!(target_path, path);
    assert_eq!((start_line, start_col), (end_line, end_col));
}

#[test]
fn context_target_ignores_non_pointer_editor_events() {
    let mut app = test_app();
    let path = "/tmp/main.rs".to_string();
    let original = (path.clone(), 2, 3, 4, 5);
    app.active_preview_path = Some(path.clone());
    app.preview_tabs = vec![tab(&path, "abc")];
    app.preview_context_target = Some(original.clone());

    update_context_target_from_editor_event(
        &mut app,
        &iced_code_editor::Message::CharacterInput('x'),
    );

    assert_eq!(app.preview_context_target, Some(original));
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

#[test]
fn context_menu_edit_commands_close_menu_and_clear_transient_completion() {
    let messages = [
        PreviewMessage::ContextMenuCopy,
        PreviewMessage::ContextMenuCut,
        PreviewMessage::ContextMenuPaste,
        PreviewMessage::ContextMenuDelete,
    ];

    for message in messages {
        let mut app = test_app();
        app.show_preview_context_menu = true;
        #[cfg(not(target_arch = "wasm32"))]
        {
            app.lsp_overlay.completion_visible = true;
            app.lsp_overlay.completion_items = vec!["Alpha".to_string()];
        }

        let _ = update(&mut app, message);

        assert!(!app.show_preview_context_menu);
        #[cfg(not(target_arch = "wasm32"))]
        {
            assert!(!app.lsp_overlay.completion_visible);
            assert!(app.lsp_overlay.completion_items.is_empty());
        }
    }
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
#[test]
fn preview_context_menu_request_preserves_selection_and_clears_lsp_overlay() {
    let mut app = test_app();
    let path = "/tmp/main.rs".to_string();
    app.active_preview_path = Some(path.clone());
    app.preview_tabs = vec![tab(&path, "first\nsecond")];
    app.preview_context_target = Some((path.clone(), 1, 1, 2, 3));
    app.lsp_overlay_path = Some(path.clone());
    app.lsp_overlay.show_hover("hover".to_string());
    app.lsp_overlay.set_completions(
        vec!["completion".to_string()],
        iced::Point::new(55.0, 1.0),
    );

    let _ = update(
        &mut app,
        PreviewMessage::EditorEvent(iced_code_editor::Message::ContextMenuRequested(
            iced::Point::new(55.0, 1.0),
        )),
    );

    assert_eq!(app.preview_context_target, Some((path, 1, 1, 2, 3)));
    assert!(!app.lsp_overlay.hover_visible);
    assert!(!app.lsp_overlay.completion_visible);

    let second_line_y = app.preview_tabs[0].editor.inner.line_height() + 1.0;
    let _ = update(
        &mut app,
        PreviewMessage::EditorEvent(iced_code_editor::Message::ContextMenuRequested(
            iced::Point::new(10_000.0, second_line_y),
        )),
    );
    let Some((_, start_line, start_col, end_line, end_col)) =
        app.preview_context_target.as_ref()
    else {
        panic!("right-click outside the selection should move the context target");
    };
    assert_eq!((*start_line, *start_col), (*end_line, *end_col));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn preview_context_menu_reveal_rejects_missing_file_without_spawning() {
    let missing = std::env::temp_dir().join(format!(
        "vibe-window-missing-reveal-{}",
        std::process::id()
    ));

    let error = super::reveal_in_file_manager(&missing.to_string_lossy())
        .expect_err("missing paths must not be sent to the file manager");

    assert!(error.contains("文件不存在"));
    assert!(error.contains(&missing.to_string_lossy().to_string()));
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
fn enter_uses_editor_modified_state_for_dirty_and_autosave() {
    let mut app = test_app();
    let path = "/tmp/main.rs".to_string();
    app.active_preview_path = Some(path.clone());
    app.preview_tabs = vec![tab(&path, "first")];
    app.preview_auto_save_mode = PreviewAutoSaveMode::AfterDelay;

    let _ = update(
        &mut app,
        PreviewMessage::EditorEvent(iced_code_editor::Message::Enter),
    );

    assert!(app.preview_tabs[0].editor.inner.is_modified());
    assert!(app.preview_tabs[0].is_dirty);
    assert_eq!(app.preview_tabs[0].auto_save_revision, 1);
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
#[test]
fn editor_mouse_hover_deadlines_follow_overlay_path_and_interactivity() {
    let mut app = test_app();
    let path = "/tmp/main.rs".to_string();
    app.active_preview_path = Some(path.clone());
    app.lsp_overlay_path = Some(path);
    app.lsp_overlay.hover_visible = true;
    app.lsp_overlay.hover_interactive = false;

    let _ = update(&mut app, PreviewMessage::EditorMouseExited);
    assert!(app.lsp_hover_hide_deadline.is_some());

    let _ = update(&mut app, PreviewMessage::EditorMouseEntered);
    assert!(app.lsp_hover_hide_deadline.is_none());

    let _ = update(&mut app, PreviewMessage::LspHoverExited);
    assert!(!app.lsp_overlay.hover_interactive);
    assert!(app.lsp_hover_hide_deadline.is_some());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn lsp_hover_enter_enables_interaction_and_cancels_hide_deadline() {
    let mut app = test_app();
    app.lsp_overlay.hover_interactive = false;
    app.lsp_hover_hide_deadline = Some(std::time::Instant::now());

    let _ = update(&mut app, PreviewMessage::LspHoverEntered);

    assert!(app.lsp_overlay.hover_interactive);
    assert!(app.lsp_hover_hide_deadline.is_none());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn completion_navigation_wraps_in_both_directions() {
    let mut app = test_app();
    app.lsp_overlay.completion_visible = true;
    app.lsp_overlay.completion_items = vec!["Alpha".to_string(), "Beta".to_string()];

    let _ = update(&mut app, PreviewMessage::LspCompletionNavigateUp);
    assert_eq!(app.lsp_overlay.completion_selected, 1);

    let _ = update(&mut app, PreviewMessage::LspCompletionNavigateDown);
    assert_eq!(app.lsp_overlay.completion_selected, 0);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn completion_navigation_ignores_hidden_or_empty_lists() {
    let mut app = test_app();
    app.lsp_overlay.completion_selected = 3;

    let _ = update(&mut app, PreviewMessage::LspCompletionNavigateUp);
    assert_eq!(app.lsp_overlay.completion_selected, 3);

    app.lsp_overlay.completion_visible = true;
    let _ = update(&mut app, PreviewMessage::LspCompletionNavigateDown);
    assert_eq!(app.lsp_overlay.completion_selected, 3);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn closing_completion_clears_only_local_overlay_state() {
    let mut app = test_app();
    app.lsp_overlay.all_completions = vec!["Alpha".to_string()];
    app.lsp_overlay.completion_items = vec!["Alpha".to_string()];
    app.lsp_overlay.completion_filter = "Al".to_string();
    app.lsp_overlay.completion_visible = true;
    app.lsp_overlay.completion_suppressed = true;
    app.lsp_overlay.completion_selected = 1;

    let _ = update(&mut app, PreviewMessage::LspCompletionClosed);

    assert!(app.lsp_overlay.all_completions.is_empty());
    assert!(app.lsp_overlay.completion_items.is_empty());
    assert!(app.lsp_overlay.completion_filter.is_empty());
    assert!(!app.lsp_overlay.completion_visible);
    assert!(!app.lsp_overlay.completion_suppressed);
    assert_eq!(app.lsp_overlay.completion_selected, 0);
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
fn editor_event_completion_left_right_and_punctuation_clear_completion() {
    let mut app = test_app();
    let path = "/tmp/main.rs".to_string();
    app.active_preview_path = Some(path.clone());
    app.preview_tabs = vec![tab(&path, "alpha")];
    app.lsp_overlay.completion_visible = true;
    app.lsp_overlay.completion_items = vec!["Alpha".to_string()];

    let _ = update(
        &mut app,
        PreviewMessage::EditorEvent(iced_code_editor::Message::ArrowKey(
            iced_code_editor::ArrowDirection::Left,
            false,
        )),
    );
    assert!(!app.lsp_overlay.completion_visible);

    app.lsp_overlay.completion_visible = true;
    app.lsp_overlay.completion_items = vec!["Alpha".to_string()];
    app.lsp_overlay.all_completions = vec!["Alpha".to_string()];

    let _ = update(
        &mut app,
        PreviewMessage::EditorEvent(iced_code_editor::Message::CharacterInput('.')),
    );

    assert!(!app.lsp_overlay.completion_visible);
    assert!(app.lsp_overlay.completion_items.is_empty());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn editor_event_mouse_release_schedules_hover_hide_when_hover_is_visible() {
    let mut app = test_app();
    let path = "/tmp/main.rs".to_string();
    app.active_preview_path = Some(path.clone());
    app.preview_tabs = vec![tab(&path, "alpha")];
    app.lsp_overlay.show_hover("docs".to_string());
    app.lsp_overlay.hover_interactive = true;

    let _ = update(&mut app, PreviewMessage::EditorEvent(iced_code_editor::Message::MouseRelease));

    assert!(!app.lsp_overlay.hover_interactive);
    assert!(app.lsp_hover_hide_deadline.is_some());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn completion_select_and_confirm_clear_overlay_even_when_index_is_missing() {
    let mut app = test_app();
    app.lsp_overlay.completion_visible = true;
    app.lsp_overlay.completion_items = vec!["Alpha".to_string()];

    let _ = update(&mut app, PreviewMessage::LspCompletionSelected(4));

    assert!(!app.lsp_applying_completion);
    assert!(!app.lsp_overlay.completion_visible);
    assert!(app.lsp_overlay.completion_suppressed);

    app.lsp_overlay.completion_visible = true;
    app.lsp_overlay.completion_items = vec!["Alpha".to_string()];
    app.lsp_overlay.completion_selected = 9;

    let _ = update(&mut app, PreviewMessage::LspCompletionConfirm);

    assert!(!app.lsp_applying_completion);
    assert!(!app.lsp_overlay.completion_visible);
    assert!(app.lsp_overlay.completion_suppressed);
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
