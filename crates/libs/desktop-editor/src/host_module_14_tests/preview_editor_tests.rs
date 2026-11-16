use super::fixture::Fixture;
use crate::components::editor::Editor;
use crate::preview::{PreviewTab, message::PreviewMessage};
use config_ui::PreviewAutoSaveMode;
use iced::widget::Id;
fn test_app() -> Fixture {
    Fixture::default()
}
fn update(
    app: &mut Fixture,
    message: PreviewMessage,
) -> iced::Task<crate::preview::editor::Effect> {
    app.dispatch(message)
}
fn update_context_target_from_editor_event(app: &mut Fixture, event: &iced_code_editor::Message) {
    crate::preview::selection::update_context_target_from_editor_event(
        &app.active_preview_path,
        &app.preview_tabs,
        &mut app.preview_context_target,
        event,
    );
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
fn preview_context_menu_request_preserves_selection_and_clears_lsp_overlay() {
    let mut app = test_app();
    let path = "/tmp/main.rs".to_string();
    app.active_preview_path = Some(path.clone());
    app.preview_tabs = vec![tab(&path, "first\nsecond")];
    app.preview_context_target = Some((path.clone(), 1, 1, 2, 3));
    app.lsp_overlay_path = Some(path.clone());
    app.lsp_overlay.show_hover("hover".to_string());
    app.lsp_overlay.set_completions(vec!["completion".to_string()], iced::Point::new(55.0, 1.0));

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
    let Some((_, start_line, start_col, end_line, end_col)) = app.preview_context_target.as_ref()
    else {
        panic!("right-click outside the selection should move the context target");
    };
    assert_eq!((*start_line, *start_col), (*end_line, *end_col));
}

#[test]
fn enter_uses_editor_modified_state_for_dirty_and_autosave() {
    let mut app = test_app();
    let path = "/tmp/main.rs".to_string();
    app.active_preview_path = Some(path.clone());
    app.preview_tabs = vec![tab(&path, "first")];
    app.preview_auto_save_mode = PreviewAutoSaveMode::AfterDelay;

    let _ = update(&mut app, PreviewMessage::EditorEvent(iced_code_editor::Message::Enter));

    assert!(app.preview_tabs[0].editor.inner.is_modified());
    assert!(app.preview_tabs[0].is_dirty);
    assert_eq!(app.preview_tabs[0].auto_save_revision, 1);
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
