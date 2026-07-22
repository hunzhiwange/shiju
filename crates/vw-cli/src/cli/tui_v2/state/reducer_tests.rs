use vw_shared::session::ui_types::{ChatMessage, ChatRole, ChatSession};

use super::{
    TuiAction, TuiTerminalUpdate, TuiToolCallUpdate, TuiToolResultUpdate, reduce_tui_state,
};
use crate::cli::tui_v2::model::{
    PromptMode, PromptMotion, PromptSubmission, PromptSubmissionStatus, QueuedPromptCommand,
    QueuedPromptCommandKind, UiMessage, UiOverlay, UiSearchOverlay, UiTodoOverlay, UiTokenUsage,
    UiToolCallState, UiTurnTerminal,
};
use crate::cli::tui_v2::state::{TuiScrollState, TuiState};

fn snapshot(id: &str) -> ChatSession {
    ChatSession {
        id: id.to_string(),
        title: "Snapshot".to_string(),
        messages: vec![ChatMessage {
            role: ChatRole::Assistant,
            content: "hello".to_string(),
            think_timing: Vec::new(),
        }],
        message_ids: vec![Some("assistant-1".to_string())],
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 1,
        updated_ms: 2,
    }
}

#[test]
fn prompt_actions_cover_editing_history_mode_selection_and_queue() {
    let mut state = TuiState::default();

    reduce_tui_state(&mut state, TuiAction::PromptValueSet("abc".to_string()));
    reduce_tui_state(&mut state, TuiAction::PromptCursorMove(PromptMotion::Left));
    reduce_tui_state(&mut state, TuiAction::PromptInsert("X".to_string()));
    reduce_tui_state(&mut state, TuiAction::PromptBackspace);
    reduce_tui_state(&mut state, TuiAction::PromptDelete);
    reduce_tui_state(&mut state, TuiAction::PromptCursorMove(PromptMotion::Home));
    reduce_tui_state(&mut state, TuiAction::PromptCursorMove(PromptMotion::End));
    reduce_tui_state(&mut state, TuiAction::PromptModeSet(PromptMode::Search));
    reduce_tui_state(&mut state, TuiAction::PromptSuggestionSelectionSet(Some(4)));
    reduce_tui_state(
        &mut state,
        TuiAction::PromptCommandQueued(QueuedPromptCommand {
            raw: "/model".to_string(),
            kind: QueuedPromptCommandKind::SlashCommand,
            enqueued_ms: Some(5),
        }),
    );
    state.prompt.history.push("older");
    state.prompt.history.push("newer");
    reduce_tui_state(&mut state, TuiAction::PromptHistoryPrevious);
    reduce_tui_state(&mut state, TuiAction::PromptHistoryNext);

    assert_eq!(state.prompt.mode, PromptMode::Search);
    assert_eq!(state.prompt.selected_suggestion_index, None);
    assert_eq!(state.prompt.queued_commands.len(), 1);
    assert_eq!(state.prompt.value, "ab");
}

#[test]
fn submission_tool_terminal_overlay_and_scroll_actions_are_reduced() {
    let mut state = TuiState::from_chat_session(&snapshot("s1"));

    reduce_tui_state(
        &mut state,
        TuiAction::PromptSubmissionStarted(
            PromptSubmission::new("run it").with_stream_id(99).with_model("model-a"),
        ),
    );
    reduce_tui_state(&mut state, TuiAction::AssistantDeltaReceived("answer".to_string()));
    reduce_tui_state(
        &mut state,
        TuiAction::ToolCallUpdated(TuiToolCallUpdate {
            tool_name: "grep".to_string(),
            summary: Some("Search".to_string()),
            arguments: Some("needle".to_string()),
            state: UiToolCallState::Running,
            result: None,
        }),
    );
    reduce_tui_state(
        &mut state,
        TuiAction::ToolCallUpdated(TuiToolCallUpdate {
            tool_name: "grep".to_string(),
            summary: Some("Search done".to_string()),
            arguments: None,
            state: UiToolCallState::Complete,
            result: Some(TuiToolResultUpdate { content: "match".to_string(), is_error: false }),
        }),
    );
    reduce_tui_state(
        &mut state,
        TuiAction::AssistantTerminalUpdated(TuiTerminalUpdate {
            terminal: UiTurnTerminal::Cancelled { reason: Some("stop".to_string()) },
            usage: Some(UiTokenUsage {
                input_tokens: 1,
                output_tokens: 2,
                cached_tokens: 0,
                reasoning_tokens: 0,
            }),
            message_id: Some("assistant-raw".to_string()),
            parent_message_id: Some("user-raw".to_string()),
        }),
    );
    reduce_tui_state(&mut state, TuiAction::SearchQuerySet("answer".to_string()));
    reduce_tui_state(
        &mut state,
        TuiAction::OverlayPushed(UiOverlay::Search(UiSearchOverlay {
            query: "missing".to_string(),
            ..UiSearchOverlay::default()
        })),
    );
    reduce_tui_state(
        &mut state,
        TuiAction::TodoOverlayReplaced(Some(UiTodoOverlay {
            session_id: Some("s1".to_string()),
            items: Vec::new(),
            selected_index: 0,
            dirty: false,
        })),
    );
    reduce_tui_state(&mut state, TuiAction::TaskSyncErrorSet(Some("sync failed".to_string())));
    reduce_tui_state(
        &mut state,
        TuiAction::ScrollSet(TuiScrollState {
            top_message: 99,
            viewport_messages: 3,
            viewport_height: 3,
            viewport_width: 80,
            overscan: 1,
            follow_tail: false,
            sticky_message: None,
            last_seen_message: None,
        }),
    );

    assert_eq!(state.session.updated_ms, 99);
    assert_eq!(
        state.status.turn_terminal,
        UiTurnTerminal::Cancelled { reason: Some("stop".to_string()) }
    );
    assert_eq!(
        state.prompt.last_submission.as_ref().map(|submission| &submission.status),
        Some(&PromptSubmissionStatus::Cancelled { reason: Some("stop".to_string()) })
    );
    assert!(state.messages.iter().any(|message| matches!(message, UiMessage::ToolCall(call) if call.summary.as_deref() == Some("Search done") && call.state == UiToolCallState::Complete)));
    assert!(state.messages.iter().any(
        |message| matches!(message, UiMessage::ToolResult(result) if result.content == "match")
    ));
    assert_eq!(state.status.last_error.as_deref(), Some("sync failed"));
    assert!(state.tasks.todo_overlay.is_some());
}

#[test]
fn session_preview_actions_clear_and_refresh_derived_metadata() {
    let mut state = TuiState::from_chat_session(&snapshot("s1"));
    assert!(state.session.preview.is_some());

    reduce_tui_state(&mut state, TuiAction::SessionPreviewSet(None));
    assert!(state.session.preview.is_none());

    reduce_tui_state(&mut state, TuiAction::SessionUpdatedMsSet(77));

    assert_eq!(state.session.updated_ms, 77);
    assert_eq!(state.session.preview.as_ref().map(|preview| preview.updated_ms), Some(77));
}
