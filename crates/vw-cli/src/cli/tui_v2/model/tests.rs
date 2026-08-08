//! 验证 TUI v2 模型状态转换。
//! 测试确保消息、状态和选择项在事件驱动更新中保持一致。

use vw_shared::question::{Info, OptionInfo, Request, ToolMeta};
use vw_shared::todo::Todo;

use super::overlay::{
    OverlayFocus, OverlayState, UiCommandPaletteOverlay, UiOverlay, UiOverlayKind,
    UiQuestionOverlay, UiTodoOverlay,
};
use super::prompt::{
    PromptMode, PromptMotion, PromptState, PromptSubmission, PromptSubmissionStatus,
    QueuedPromptCommand, QueuedPromptCommandKind,
};
use super::session_mode::SessionOptions;
use super::ui_message::{
    UiAssistantMessage, UiMessage, UiMessageBase, UiMessageId, UiMessageKind, UiTokenUsage,
    UiTurnTerminal,
};

#[test]
fn session_options_defaults_goal_mode_enabled_and_cannot_disable() {
    let options = SessionOptions::default();
    assert!(options.goal_mode_enabled());
    assert!(!options.workflow_mode_enabled());

    let mut options = SessionOptions::default();
    options.set_workflow_mode(true);
    assert!(options.workflow_mode_enabled());
    assert!(options.goal_mode_enabled());

    options.set_goal_mode(false);
    assert!(options.goal_mode_enabled());
    assert!(options.workflow_mode_enabled());

    options.set_goal_mode(true);
    assert!(options.goal_mode_enabled());
    assert!(options.workflow_mode_enabled());
}

#[test]
fn prompt_state_start_and_finish_submission_tracks_busy_lifecycle() {
    let mut state = PromptState::new("继续执行 shadow compare");
    let submission = PromptSubmission::new("继续执行 shadow compare")
        .with_stream_id(7)
        .with_session_id("session_alpha")
        .with_model("gpt-5.4")
        .with_history_len(3);

    state.start_submission(submission);

    assert!(state.is_busy());
    assert_eq!(state.mode, PromptMode::Busy);
    assert_eq!(state.value, "");
    assert_eq!(state.cursor.char_index, 0);
    assert_eq!(state.history.entries, vec!["继续执行 shadow compare".to_string()]);
    assert_eq!(state.active_submission.as_ref().and_then(|value| value.stream_id), Some(7));
    assert_eq!(
        state.active_submission.as_ref().map(|value| &value.status),
        Some(&PromptSubmissionStatus::Streaming)
    );

    state.finish_submission(PromptSubmissionStatus::Done {
        finish_reason: Some("stop".to_string()),
    });

    assert!(!state.is_busy());
    assert_eq!(state.mode, PromptMode::Compose);
    assert!(state.active_submission.is_none());
    assert_eq!(
        state.last_submission.as_ref().map(|value| &value.status),
        Some(&PromptSubmissionStatus::Done { finish_reason: Some("stop".to_string()) })
    );
}

#[test]
fn prompt_state_queue_command_preserves_fifo_order() {
    let mut state = PromptState::default();
    state.queue_command(QueuedPromptCommand {
        raw: "/model gpt-5.4".to_string(),
        kind: QueuedPromptCommandKind::SlashCommand,
        enqueued_ms: Some(10),
        ..QueuedPromptCommand::default()
    });
    state.queue_command(QueuedPromptCommand {
        raw: "继续".to_string(),
        kind: QueuedPromptCommandKind::Submit,
        enqueued_ms: Some(12),
        ..QueuedPromptCommand::default()
    });

    assert_eq!(
        state.pop_queued_command().map(|command| command.raw),
        Some("/model gpt-5.4".to_string())
    );
    assert_eq!(state.pop_queued_command().map(|command| command.raw), Some("继续".to_string()));
    assert!(state.pop_queued_command().is_none());
}

#[test]
fn prompt_state_supports_multiline_cursor_editing() {
    let mut state = PromptState::new("alpha\nbeta");

    assert!(state.move_cursor(PromptMotion::Up));
    assert!(state.move_cursor(PromptMotion::Home));
    state.insert_text(">>");
    assert_eq!(state.value, ">>alpha\nbeta");

    assert!(state.move_cursor(PromptMotion::Down));
    assert!(state.move_cursor(PromptMotion::End));
    state.insert_text("!");
    assert_eq!(state.value, ">>alpha\nbeta!");

    state.backspace();
    assert_eq!(state.value, ">>alpha\nbeta");
}

#[test]
fn prompt_state_history_restores_draft_after_navigation() {
    let mut state = PromptState::default();
    state.history.push("first");
    state.history.push("second");
    state.set_value("draft command");

    assert!(state.select_previous_history());
    assert_eq!(state.value, "second");
    assert!(state.select_previous_history());
    assert_eq!(state.value, "first");
    assert!(state.select_next_history());
    assert_eq!(state.value, "second");
    assert!(state.select_next_history());
    assert_eq!(state.value, "draft command");
    assert!(state.history.selected_index.is_none());
}

#[test]
fn overlay_state_tracks_active_overlay_and_focus() {
    let mut state = OverlayState::default();
    state.push(UiOverlay::CommandPalette(UiCommandPaletteOverlay::default()));

    assert_eq!(state.focus, OverlayFocus::Overlay);
    assert_eq!(state.active().map(UiOverlay::kind), Some(UiOverlayKind::CommandPalette));

    let popped = state.pop();
    assert!(matches!(popped, Some(UiOverlay::CommandPalette(_))));
    assert_eq!(state.focus, OverlayFocus::Prompt);
    assert!(state.active().is_none());
}

#[test]
fn question_overlay_from_request_copies_prompt_and_tool_context() {
    let request = Request {
        id: "q-123".to_string(),
        session_id: "session_xyz".to_string(),
        questions: vec![Info {
            question: "Allow the pending shell action to continue?".to_string(),
            header: "Shell Approval".to_string(),
            options: vec![OptionInfo {
                label: "allow once".to_string(),
                description: "Continue the pending tool call".to_string(),
                preview: None,
            }],
            multiple: Some(false),
            custom: Some(false),
        }],
        tool: Some(ToolMeta { message_id: "msg-1".to_string(), call_id: "call-1".to_string() }),
    };

    let overlay = UiQuestionOverlay::from_request(&request);

    assert_eq!(overlay.request_id, "q-123");
    assert_eq!(overlay.session_id, "session_xyz");
    assert_eq!(overlay.prompts.len(), 1);
    assert_eq!(overlay.answers, vec![Vec::<String>::new()]);
    assert_eq!(overlay.prompts[0].header, "Shell Approval");
    assert!(!overlay.prompts[0].allow_custom_input);
    assert!(overlay.is_tool_backed());
    assert!(overlay.is_permission_request());
    assert_eq!(overlay.modal_title(), "权限请求");
    assert_eq!(overlay.tool.as_ref().map(|tool| tool.call_id.as_str()), Some("call-1"));
}

#[test]
fn todo_overlay_from_todos_preserves_items_and_session_context() {
    let todos = vec![Todo {
        id: "todo-1".to_string(),
        content: "把 compare bridge 接到 PromptState".to_string(),
        status: "in_progress".to_string(),
        priority: "high".to_string(),
    }];

    let overlay = UiTodoOverlay::from_todos(Some("session_todo"), &todos);

    assert_eq!(overlay.session_id.as_deref(), Some("session_todo"));
    assert_eq!(overlay.items.len(), 1);
    assert_eq!(overlay.items[0].content, "把 compare bridge 接到 PromptState");
    assert_eq!(overlay.items[0].priority, "high");
    assert!(!overlay.dirty);
}

#[test]
fn ui_message_kind_and_base_accessors_match_variant() {
    let message = UiMessage::Assistant(UiAssistantMessage {
        base: UiMessageBase::new(UiMessageId::gateway("msg-assistant"))
            .with_parent_id(UiMessageId::gateway("msg-user"))
            .with_session_id("session_msg")
            .with_created_ms(42),
        text: "已完成对照桥接".to_string(),
        usage: UiTokenUsage {
            input_tokens: 12,
            output_tokens: 34,
            cached_tokens: 0,
            reasoning_tokens: 5,
        },
        step_count: 2,
        terminal: UiTurnTerminal::Done { finish_reason: Some("stop".to_string()) },
        model: Some("gpt-5.4".to_string()),
    });

    assert_eq!(message.kind(), UiMessageKind::Assistant);
    assert_eq!(message.id().as_str(), "gateway:msg-assistant");
    assert_eq!(message.base().session_id.as_deref(), Some("session_msg"));
}
