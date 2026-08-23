//! 覆盖 TUI v2 顶层行为。
//! 测试从用户可见流程出发验证模型、输入和渲染协作。

use std::path::PathBuf;

use crossterm::event::{Event as CrosstermEvent, KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;

use super::app::{
    TodoSessionAccessAction, TuiDeltaRedrawSnapshot, compare_shadow_results,
    dequeue_queued_prompt_command, question_overlay_submission_answers,
    runtime_event_fallback_overlay, select_restore_session_id, should_redraw_after_runtime_delta,
    todo_overlay_items_as_shared_todos, todo_session_unavailable_overlay,
};
use super::controller::{
    TuiController, TuiControllerCommand, TuiControllerEvent, TuiOverlayCommand,
};
use super::input::TuiSlashCommandKind;
use super::render::layout::compute_fullscreen_layout;
use super::render::{
    build_modal_host, build_modified_files_host, build_project_context_host, build_prompt_host,
    build_status_footer, build_status_header,
};
use super::runtime::stream_adapter::{UiRuntimeEvent, UiRuntimeTerminalEvent};
use super::state::{
    TuiAction, TuiScrollState, TuiState, reduce_tui_state, select_status_summary,
    select_transcript_message_anchors, select_visible_grouped_transcript_window,
};
use vw_shared::session::ui_types::ChatSessionMeta;

fn seeded_transcript_state(session_id: &str) -> TuiState {
    TuiState::from_chat_session(&vw_shared::session::ui_types::ChatSession {
        id: session_id.to_string(),
        title: format!("{session_id} title"),
        messages: vec![
            vw_shared::session::ui_types::ChatMessage {
                role: vw_shared::session::ui_types::ChatRole::User,
                content: "历史用户 1".to_string(),
                model: None,
                think_timing: Vec::new(),
                processing_timing: None,
            },
            vw_shared::session::ui_types::ChatMessage {
                role: vw_shared::session::ui_types::ChatRole::Assistant,
                content: "历史助手 1".to_string(),
                model: None,
                think_timing: Vec::new(),
                processing_timing: None,
            },
            vw_shared::session::ui_types::ChatMessage {
                role: vw_shared::session::ui_types::ChatRole::User,
                content: "历史用户 2".to_string(),
                model: None,
                think_timing: Vec::new(),
                processing_timing: None,
            },
            vw_shared::session::ui_types::ChatMessage {
                role: vw_shared::session::ui_types::ChatRole::Assistant,
                content: "历史助手 2".to_string(),
                model: None,
                think_timing: Vec::new(),
                processing_timing: None,
            },
        ],
        message_ids: vec![
            Some(format!("{session_id}-u1")),
            Some(format!("{session_id}-a1")),
            Some(format!("{session_id}-u2")),
            Some(format!("{session_id}-a2")),
        ],
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 1,
        updated_ms: 1,
    })
}

fn configure_scrolled_viewport(state: &mut TuiState, top_message: usize, viewport_messages: usize) {
    state.scroll.top_message = top_message;
    state.scroll.viewport_messages = viewport_messages;
    state.scroll.viewport_height = u16::try_from(viewport_messages).unwrap_or(u16::MAX);
    state.scroll.viewport_width = 40;
    state.scroll.overscan = 0;
    state.scroll.follow_tail = false;
    state.scroll.sticky_message = Some(top_message.saturating_sub(1));
    state.scroll.last_seen_message = state.messages.len().checked_sub(1);
    state.refresh_transcript_layout_for_current_width();
}

fn seed_model_catalog(state: &mut TuiState, entries: &[(&str, &str, &str, &str)]) {
    reduce_tui_state(
        state,
        TuiAction::ModelCatalogReplaced(
            entries
                .iter()
                .map(|(provider_id, provider_name, model_id, model_name)| {
                    super::state::TuiModelCatalogEntry {
                        provider_id: (*provider_id).to_string(),
                        provider_name: (*provider_name).to_string(),
                        model_id: (*model_id).to_string(),
                        model_name: (*model_name).to_string(),
                    }
                })
                .collect(),
        ),
    );
}

#[test]
fn fullscreen_layout_slots_stay_inside_standard_and_narrow_areas() {
    for area in [Rect::new(0, 0, 120, 32), Rect::new(0, 0, 42, 10)] {
        let slots = compute_fullscreen_layout(area, true, true, 0);

        assert_eq!(slots.header.height, 0);
        assert!(slots.scrollable.height >= 1);
        assert!(slots.bottom.height >= 1);
        assert!(slots.header.y >= area.y);
        assert!(slots.scrollable.y >= slots.header.y.saturating_add(slots.header.height));
        assert!(slots.bottom.y >= slots.scrollable.y.saturating_add(slots.scrollable.height));
        assert!(
            slots.bottom.y.saturating_add(slots.bottom.height)
                <= area.y.saturating_add(area.height)
        );

        assert!(slots.bottom_float.is_none());

        let modal = slots.modal.expect("modal slot should be available when requested");
        assert!(modal.width <= area.width);
        assert!(modal.height <= area.height);
        assert!(modal.x >= area.x);
        assert!(modal.y >= area.y);

        assert!(slots.project_context.is_none());
        assert!(slots.modified_files.is_none());
    }
}

#[test]
fn fullscreen_layout_prefers_taller_prompt_host_on_compact_terminals() {
    let compact = compute_fullscreen_layout(Rect::new(0, 0, 80, 24), true, false, 0);
    assert_eq!(compact.header.height, 0);
    assert_eq!(compact.bottom.height, 8);
    assert!(compact.bottom_float.is_none());
    assert_eq!(compact.scrollable.height, 16);

    let roomy = compute_fullscreen_layout(Rect::new(0, 0, 80, 40), true, false, 0);
    assert_eq!(roomy.header.height, 0);
    assert_eq!(roomy.bottom.height, 8);
    assert_eq!(roomy.scrollable.height, 32);
    assert!(roomy.bottom_float.is_none());
}

#[test]
fn fullscreen_layout_hides_sidebars_when_height_is_too_short() {
    let compact_wide = compute_fullscreen_layout(Rect::new(0, 0, 120, 24), true, false, 0);
    assert!(compact_wide.project_context.is_none());
    assert!(compact_wide.modified_files.is_none());

    let roomy_wide = compute_fullscreen_layout(Rect::new(0, 0, 120, 32), true, false, 0);
    assert!(roomy_wide.project_context.is_none());
    assert!(roomy_wide.modified_files.is_none());
}

#[test]
fn fullscreen_layout_expands_prompt_host_for_visible_suggestion_panel() {
    let layout = compute_fullscreen_layout(Rect::new(0, 0, 80, 24), true, false, 2);
    assert_eq!(layout.bottom.height, 10);
    assert_eq!(layout.scrollable.height, 14);
}

#[test]
fn controller_returns_submit_command_for_entered_prompt() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();
    let layout = compute_fullscreen_layout(Rect::new(0, 0, 80, 24), true, false, 0);
    let feedback = super::render::TuiRenderFeedback { layout, cursor: None, ..Default::default() };

    controller.sync_layout(&mut state, &feedback);
    reduce_tui_state(&mut state, TuiAction::StatusModelSet(Some("gpt-test".to_string())));
    reduce_tui_state(&mut state, TuiAction::PromptValueSet("hello skeleton".to_string()));

    let command = controller.handle_event(
        &mut state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        ))),
    );

    assert!(state.scroll.viewport_messages > 0);
    assert!(state.scroll.viewport_height > 0);
    assert!(state.scroll.viewport_width > 0);
    assert!(state.messages.is_empty());
    assert_eq!(state.prompt.value, "hello skeleton");
    assert!(state.prompt.last_submission.is_none());

    let TuiControllerCommand::SubmitPrompt(submission) = command else {
        panic!("enter should produce a submit command");
    };
    assert_eq!(submission.text, "hello skeleton");
    assert_eq!(submission.model.as_deref(), Some("gpt-test"));
    assert_eq!(submission.history_len, 0);
}

#[test]
fn controller_toggles_help_modal_and_escapes_in_stages() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();

    let first = controller.handle_event(
        &mut state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
            KeyCode::F(1),
            KeyModifiers::NONE,
        ))),
    );
    assert_eq!(first, TuiControllerCommand::Continue);
    assert!(state.overlays.active().is_some());

    let second = controller.handle_event(
        &mut state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
            KeyCode::Esc,
            KeyModifiers::NONE,
        ))),
    );
    assert_eq!(second, TuiControllerCommand::Continue);
    assert!(state.overlays.active().is_none());

    reduce_tui_state(&mut state, TuiAction::PromptValueSet("draft".to_string()));
    let third = controller.handle_event(
        &mut state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
            KeyCode::Esc,
            KeyModifiers::NONE,
        ))),
    );
    assert_eq!(third, TuiControllerCommand::Continue);
    assert!(state.prompt.value.is_empty());

    let fourth = controller.handle_event(
        &mut state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
            KeyCode::Esc,
            KeyModifiers::NONE,
        ))),
    );
    assert_eq!(fourth, TuiControllerCommand::Quit);
}

#[test]
fn controller_shift_enter_inserts_newline_in_prompt() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();
    reduce_tui_state(&mut state, TuiAction::PromptValueSet("hello".to_string()));

    let command = controller.handle_event(
        &mut state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::SHIFT,
        ))),
    );

    assert_eq!(command, TuiControllerCommand::Continue);
    assert_eq!(state.prompt.value, "hello\n");
    assert_eq!(state.prompt.cursor.char_index, 6);
}

#[test]
fn controller_enter_routes_slash_command_without_submitting() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();
    reduce_tui_state(&mut state, TuiAction::PromptValueSet("/help".to_string()));

    let command = controller.handle_event(
        &mut state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        ))),
    );

    let TuiControllerCommand::ExecuteSlashCommand(invocation) = command else {
        panic!("slash prompt should route to slash command execution");
    };
    assert_eq!(invocation.kind, Some(TuiSlashCommandKind::Help));
    assert!(state.prompt.value.is_empty());
}

#[test]
fn controller_tab_accepts_top_slash_suggestion() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();
    reduce_tui_state(&mut state, TuiAction::PromptValueSet("/cl".to_string()));

    let command = controller.handle_event(
        &mut state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
            KeyCode::Tab,
            KeyModifiers::NONE,
        ))),
    );

    assert_eq!(command, TuiControllerCommand::Continue);
    assert_eq!(state.prompt.value, "/clear");
}

#[test]
fn controller_up_down_switches_visible_model_suggestion_and_enter_accepts_selected_item() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();
    seed_model_catalog(
        &mut state,
        &[("openai", "OpenAI", "gpt-4.1", "GPT-4.1"), ("openai", "OpenAI", "gpt-5.4", "GPT-5.4")],
    );
    reduce_tui_state(&mut state, TuiAction::PromptValueSet("/model gpt".to_string()));

    let select_next = controller.handle_event(
        &mut state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
            KeyCode::Down,
            KeyModifiers::NONE,
        ))),
    );
    assert_eq!(select_next, TuiControllerCommand::Continue);
    assert_eq!(state.prompt.selected_suggestion_index, Some(1));

    let accept_selected = controller.handle_event(
        &mut state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        ))),
    );
    assert_eq!(accept_selected, TuiControllerCommand::Continue);
    assert_eq!(state.prompt.value, "/model openai/gpt-5.4");
    assert_eq!(state.prompt.selected_suggestion_index, None);

    let execute_selected = controller.handle_event(
        &mut state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        ))),
    );
    let TuiControllerCommand::ExecuteSlashCommand(invocation) = execute_selected else {
        panic!("accepted slash suggestion should execute on the next enter");
    };
    assert_eq!(invocation.kind, Some(TuiSlashCommandKind::Model));
    assert_eq!(invocation.argument.as_deref(), Some("openai/gpt-5.4"));
}

#[test]
fn controller_up_down_recall_history_and_restore_draft() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();
    state.prompt.history.push("first");
    state.prompt.history.push("second");
    reduce_tui_state(&mut state, TuiAction::PromptValueSet("draft".to_string()));

    controller.handle_event(
        &mut state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
            KeyCode::Up,
            KeyModifiers::NONE,
        ))),
    );
    assert_eq!(state.prompt.value, "second");

    controller.handle_event(
        &mut state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
            KeyCode::Up,
            KeyModifiers::NONE,
        ))),
    );
    assert_eq!(state.prompt.value, "first");

    controller.handle_event(
        &mut state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
            KeyCode::Down,
            KeyModifiers::NONE,
        ))),
    );
    assert_eq!(state.prompt.value, "second");

    controller.handle_event(
        &mut state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
            KeyCode::Down,
            KeyModifiers::NONE,
        ))),
    );
    assert_eq!(state.prompt.value, "draft");
}

#[test]
fn controller_busy_enter_queues_submit_and_clears_prompt() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();

    reduce_tui_state(
        &mut state,
        TuiAction::PromptSubmissionStarted(
            super::model::PromptSubmission::new("当前流").with_stream_id(90).with_model("gpt-5.4"),
        ),
    );
    reduce_tui_state(&mut state, TuiAction::PromptValueSet("下一个问题".to_string()));

    let command = controller.handle_event(
        &mut state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        ))),
    );

    assert_eq!(command, TuiControllerCommand::Continue);
    assert_eq!(state.prompt.value, "");
    assert_eq!(state.prompt.queued_commands.len(), 1);
    assert_eq!(state.prompt.queued_commands[0].raw, "下一个问题");
    assert_eq!(state.prompt.queued_commands[0].kind, super::model::QueuedPromptCommandKind::Submit);
}

#[test]
fn controller_busy_enter_queues_slash_command() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();

    reduce_tui_state(
        &mut state,
        TuiAction::PromptSubmissionStarted(
            super::model::PromptSubmission::new("当前流").with_stream_id(91).with_model("gpt-5.4"),
        ),
    );
    reduce_tui_state(&mut state, TuiAction::PromptValueSet("/model gpt-5.5".to_string()));

    let command = controller.handle_event(
        &mut state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        ))),
    );

    assert_eq!(command, TuiControllerCommand::Continue);
    assert_eq!(state.prompt.queued_commands.len(), 1);
    assert_eq!(state.prompt.queued_commands[0].raw, "/model gpt-5.5");
    assert_eq!(
        state.prompt.queued_commands[0].kind,
        super::model::QueuedPromptCommandKind::SlashCommand
    );
}

#[test]
fn controller_busy_escape_clears_draft_then_requests_cancel() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();

    reduce_tui_state(
        &mut state,
        TuiAction::PromptSubmissionStarted(
            super::model::PromptSubmission::new("当前流").with_stream_id(92).with_model("gpt-5.4"),
        ),
    );
    reduce_tui_state(&mut state, TuiAction::PromptValueSet("草稿".to_string()));

    let first = controller.handle_event(
        &mut state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
            KeyCode::Esc,
            KeyModifiers::NONE,
        ))),
    );
    assert_eq!(first, TuiControllerCommand::Continue);
    assert!(state.prompt.value.is_empty());

    let second = controller.handle_event(
        &mut state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
            KeyCode::Esc,
            KeyModifiers::NONE,
        ))),
    );
    assert_eq!(second, TuiControllerCommand::CancelActiveSubmission);
}

#[test]
fn queued_command_replay_restores_deferred_draft_after_queue_drains() {
    let mut state = TuiState::default();
    let mut deferred_prompt_draft = None;

    reduce_tui_state(&mut state, TuiAction::StatusModelSet(Some("gpt-5.4".to_string())));
    state.prompt.queue_command(super::model::QueuedPromptCommand {
        raw: "queued follow-up".to_string(),
        kind: super::model::QueuedPromptCommandKind::Submit,
        enqueued_ms: Some(100),
        ..super::model::QueuedPromptCommand::default()
    });
    reduce_tui_state(&mut state, TuiAction::PromptValueSet("draft kept locally".to_string()));

    let command = dequeue_queued_prompt_command(&mut state, &mut deferred_prompt_draft)
        .expect("queued submit should replay");

    let TuiControllerCommand::SubmitPrompt(submission) = command else {
        panic!("queued submit should become a submit command");
    };
    assert_eq!(submission.text, "queued follow-up");
    assert_eq!(state.prompt.value, "");
    assert_eq!(deferred_prompt_draft.as_deref(), Some("draft kept locally"));

    assert!(dequeue_queued_prompt_command(&mut state, &mut deferred_prompt_draft).is_none());
    assert_eq!(state.prompt.value, "draft kept locally");
    assert!(deferred_prompt_draft.is_none());
}

#[test]
fn queued_slash_command_replay_routes_back_through_slash_executor() {
    let mut state = TuiState::default();
    let mut deferred_prompt_draft = None;

    state.prompt.queue_command(super::model::QueuedPromptCommand {
        raw: "/model gpt-5.4".to_string(),
        kind: super::model::QueuedPromptCommandKind::SlashCommand,
        enqueued_ms: Some(101),
        ..super::model::QueuedPromptCommand::default()
    });

    let command = dequeue_queued_prompt_command(&mut state, &mut deferred_prompt_draft)
        .expect("queued slash command should replay");

    let TuiControllerCommand::ExecuteSlashCommand(invocation) = command else {
        panic!("queued slash command should route through slash execution");
    };
    assert_eq!(invocation.kind, Some(TuiSlashCommandKind::Model));
}

#[test]
fn overlay_layer_blocks_prompt_key_bleed() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();
    reduce_tui_state(&mut state, TuiAction::PromptValueSet("draft".to_string()));
    reduce_tui_state(
        &mut state,
        TuiAction::OverlayPushed(super::model::UiOverlay::Error(super::model::UiErrorOverlay {
            title: "overlay".to_string(),
            message: "active".to_string(),
            recoverable: true,
        })),
    );

    let command = controller.handle_event(
        &mut state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
            KeyCode::Char('x'),
            KeyModifiers::NONE,
        ))),
    );

    assert_eq!(command, TuiControllerCommand::Continue);
    assert_eq!(state.prompt.value, "draft");
}

#[test]
fn controller_f2_and_f3_open_task_overlays() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();

    let question_command = controller.handle_event(
        &mut state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
            KeyCode::F(2),
            KeyModifiers::NONE,
        ))),
    );
    assert_eq!(
        question_command,
        TuiControllerCommand::Overlay(TuiOverlayCommand::OpenPendingQuestions)
    );

    let todo_command = controller.handle_event(
        &mut state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
            KeyCode::F(3),
            KeyModifiers::NONE,
        ))),
    );
    assert_eq!(todo_command, TuiControllerCommand::Overlay(TuiOverlayCommand::OpenTodoPanel));

    let task_command = controller.handle_event(
        &mut state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
            KeyCode::F(4),
            KeyModifiers::NONE,
        ))),
    );
    assert_eq!(task_command, TuiControllerCommand::Overlay(TuiOverlayCommand::OpenTaskPanel));
}

#[test]
fn controller_ctrl_f_opens_search_overlay() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();

    let command = controller.handle_event(
        &mut state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
            KeyCode::Char('f'),
            KeyModifiers::CONTROL,
        ))),
    );

    assert_eq!(command, TuiControllerCommand::Overlay(TuiOverlayCommand::OpenSearchOverlay));
}

#[test]
fn controller_search_overlay_updates_query_and_jumps_to_match() {
    let mut controller = TuiController::default();
    let mut state = seeded_transcript_state("session_search_overlay");
    configure_scrolled_viewport(&mut state, 0, 2);
    reduce_tui_state(&mut state, TuiAction::SearchQuerySet("历史助手".to_string()));

    controller.handle_event(
        &mut state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
            KeyCode::Char(' '),
            KeyModifiers::NONE,
        ))),
    );
    controller.handle_event(
        &mut state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
            KeyCode::Backspace,
            KeyModifiers::NONE,
        ))),
    );

    let command = controller.handle_event(
        &mut state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        ))),
    );

    assert_eq!(command, TuiControllerCommand::Continue);
    assert!(state.overlays.active().is_none());
    assert_eq!(state.scroll.top_message, 1);
    assert!(!state.scroll.follow_tail);
}

#[test]
fn controller_search_overlay_end_and_unread_shortcuts_jump_transcript() {
    let mut controller = TuiController::default();
    let mut state = seeded_transcript_state("session_search_shortcuts");
    configure_scrolled_viewport(&mut state, 0, 2);
    state.scroll.last_seen_message = Some(1);
    reduce_tui_state(&mut state, TuiAction::SearchQuerySet("历史".to_string()));

    controller.handle_event(
        &mut state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
            KeyCode::Char('u'),
            KeyModifiers::NONE,
        ))),
    );

    assert!(state.overlays.active().is_none());
    assert_eq!(state.scroll.top_message, 2);

    reduce_tui_state(&mut state, TuiAction::SearchQuerySet("历史".to_string()));
    controller.handle_event(
        &mut state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
            KeyCode::End,
            KeyModifiers::NONE,
        ))),
    );

    assert!(state.overlays.active().is_none());
    assert!(state.scroll.follow_tail);
}

#[test]
fn controller_task_overlay_jumps_to_selected_step() {
    let mut controller = TuiController::default();
    let mut state = seeded_transcript_state("session_task_overlay");
    configure_scrolled_viewport(&mut state, 0, 2);
    reduce_tui_state(
        &mut state,
        TuiAction::MessagePushed(super::model::UiMessage::Step(super::model::UiStep {
            base: super::model::UiMessageBase::new(super::model::UiMessageId::local("step-1")),
            step_index: 1,
            started_ms: 10,
            finished_ms: Some(11),
            usage: super::model::UiTokenUsage::default(),
            finish_reason: Some("stop".to_string()),
            model: Some("gpt-5.4".to_string()),
            state: super::model::UiStepState::Complete,
        })),
    );
    state.overlays.push(super::model::UiOverlay::Task(super::model::UiTaskOverlay {
        session_id: Some("session_task_overlay".to_string()),
        turn_terminal: super::model::UiTurnTerminal::Done {
            finish_reason: Some("stop".to_string()),
        },
        pending_questions: 0,
        todo_count: 0,
        sync_error: None,
        steps: vec![super::model::UiTaskStepItem {
            message_id: super::model::UiMessageId::local("step-1"),
            step_index: 1,
            state: super::model::UiStepState::Complete,
            started_ms: 10,
            finished_ms: Some(11),
            model: Some("gpt-5.4".to_string()),
            finish_reason: Some("stop".to_string()),
            usage: super::model::UiTokenUsage::default(),
        }],
        selected_index: 0,
    }));

    let command = controller.handle_event(
        &mut state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        ))),
    );

    assert_eq!(command, TuiControllerCommand::Continue);
    assert!(state.overlays.active().is_none());
    assert_eq!(state.scroll.top_message, 3);
}

#[test]
fn controller_question_overlay_collects_answers_and_submits() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();
    state.overlays.push(super::model::UiOverlay::Question(super::model::UiQuestionOverlay {
        request_id: "question_1".to_string(),
        session_id: "session_1".to_string(),
        prompts: vec![super::model::UiQuestionPrompt {
            header: "Workspace".to_string(),
            question: "Which workspace should be used?".to_string(),
            options: vec![
                super::model::UiQuestionOption {
                    label: "alpha".to_string(),
                    description: "first".to_string(),
                    preview: None,
                },
                super::model::UiQuestionOption {
                    label: "beta".to_string(),
                    description: "second".to_string(),
                    preview: None,
                },
            ],
            multiple: false,
            allow_custom_input: true,
        }],
        answers: vec![Vec::new()],
        tool: None,
        selected_index: 0,
    }));

    controller.handle_event(
        &mut state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
            KeyCode::Char('2'),
            KeyModifiers::NONE,
        ))),
    );
    controller.handle_event(
        &mut state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
            KeyCode::Char('x'),
            KeyModifiers::NONE,
        ))),
    );

    let command = controller.handle_event(
        &mut state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        ))),
    );

    let TuiControllerCommand::Overlay(TuiOverlayCommand::QuestionSubmitted(overlay)) = command
    else {
        panic!("question overlay should submit through overlay command");
    };
    assert_eq!(
        question_overlay_submission_answers(&overlay),
        vec![vec!["beta".to_string(), "x".to_string()]]
    );
}

#[test]
fn controller_question_overlay_ctrl_r_requests_reject() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();
    state.overlays.push(super::model::UiOverlay::Question(super::model::UiQuestionOverlay {
        request_id: "question_2".to_string(),
        session_id: "session_2".to_string(),
        prompts: vec![super::model::UiQuestionPrompt {
            header: String::new(),
            question: "Continue?".to_string(),
            options: Vec::new(),
            multiple: false,
            allow_custom_input: true,
        }],
        answers: vec![Vec::new()],
        tool: None,
        selected_index: 0,
    }));

    let command = controller.handle_event(
        &mut state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
            KeyCode::Char('r'),
            KeyModifiers::CONTROL,
        ))),
    );

    let TuiControllerCommand::Overlay(TuiOverlayCommand::QuestionRejected(overlay)) = command
    else {
        panic!("ctrl+r should reject the active question overlay");
    };
    assert_eq!(overlay.request_id, "question_2");
}

#[test]
fn modal_host_renders_tool_question_fallback_lines() {
    let mut state = TuiState::default();
    state.overlays.push(super::model::UiOverlay::Question(super::model::UiQuestionOverlay {
        request_id: "question_tool_1".to_string(),
        session_id: "session_tool_1".to_string(),
        prompts: vec![super::model::UiQuestionPrompt {
            header: "Shell Approval".to_string(),
            question: "Allow the pending shell action to continue?".to_string(),
            options: vec![super::model::UiQuestionOption {
                label: "allow".to_string(),
                description: "continue the tool call".to_string(),
                preview: None,
            }],
            multiple: false,
            allow_custom_input: false,
        }],
        answers: vec![Vec::new()],
        tool: Some(super::model::UiQuestionToolContext {
            message_id: "msg-tool-1".to_string(),
            call_id: "call-tool-1".to_string(),
        }),
        selected_index: 0,
    }));

    let modal = build_modal_host(&state).expect("tool question modal should exist");
    assert_eq!(modal.title, "权限请求");
    assert!(modal.body_lines.iter().any(|line| line.contains("等待你的授权")));
    assert!(modal.body_lines.iter().any(|line| line.contains("工具调用: call-tool-1")));
    assert!(modal.body_lines.iter().any(|line| line.contains("来源消息: msg-tool-1")));
}

#[test]
fn controller_todo_overlay_toggles_selected_item_and_requests_save() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();
    state.overlays.push(super::model::UiOverlay::Todo(super::model::UiTodoOverlay {
        session_id: Some("session_todo".to_string()),
        items: vec![super::model::UiTodoItem {
            id: "todo_1".to_string(),
            content: "wire overlay manager".to_string(),
            status: "pending".to_string(),
            priority: "high".to_string(),
        }],
        selected_index: 0,
        dirty: false,
    }));

    controller.handle_event(
        &mut state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
            KeyCode::Char(' '),
            KeyModifiers::NONE,
        ))),
    );

    let command = controller.handle_event(
        &mut state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
            KeyCode::Char('s'),
            KeyModifiers::NONE,
        ))),
    );

    let TuiControllerCommand::Overlay(TuiOverlayCommand::TodoSave(overlay)) = command else {
        panic!("todo overlay should save through overlay command");
    };
    let todos = todo_overlay_items_as_shared_todos(&overlay);
    assert_eq!(todos.len(), 1);
    assert_eq!(todos[0].status, "completed");
}

#[test]
fn controller_sync_layout_records_viewport_height_and_sticky_anchor() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();
    state.messages = vec![
        super::model::UiMessage::System(super::model::UiSystemMessage {
            base: super::model::UiMessageBase::new(super::model::UiMessageId::local("sys-1")),
            text: "one".to_string(),
            level: super::model::UiSystemMessageLevel::Info,
        }),
        super::model::UiMessage::System(super::model::UiSystemMessage {
            base: super::model::UiMessageBase::new(super::model::UiMessageId::local("sys-2")),
            text: "two".to_string(),
            level: super::model::UiSystemMessageLevel::Info,
        }),
        super::model::UiMessage::System(super::model::UiSystemMessage {
            base: super::model::UiMessageBase::new(super::model::UiMessageId::local("sys-3")),
            text: "three".to_string(),
            level: super::model::UiSystemMessageLevel::Info,
        }),
    ];
    state.refresh_transcript_projection();
    state.scroll.top_message = 2;
    state.scroll.follow_tail = false;

    let layout = compute_fullscreen_layout(Rect::new(0, 0, 80, 24), true, false, 0);
    let feedback = super::render::TuiRenderFeedback { layout, cursor: None, ..Default::default() };
    controller.sync_layout(&mut state, &feedback);

    assert_eq!(state.scroll.viewport_height, feedback.layout.scrollable_body.height);
    assert_eq!(
        state.scroll.viewport_messages,
        feedback.layout.scrollable_body.height as usize
    );
    assert_eq!(state.scroll.viewport_width, feedback.layout.scrollable_body.width);
    assert_eq!(state.scroll.sticky_message, Some(1));
}

#[test]
fn select_restore_session_id_prefers_explicit_binding_then_latest_preview() {
    let previews = vec![
        ChatSessionMeta {
            id: "session_old".to_string(),
            title: "older".to_string(),
            updated_ms: 10,
            message_count: 2,
            call_count: 0,
            last_content: Some("old".to_string()),
        },
        ChatSessionMeta {
            id: "session_new".to_string(),
            title: "newer".to_string(),
            updated_ms: 20,
            message_count: 4,
            call_count: 1,
            last_content: Some("new".to_string()),
        },
    ];

    assert_eq!(
        select_restore_session_id(Some("session_explicit"), &previews),
        Some("session_explicit".to_string())
    );
    assert_eq!(select_restore_session_id(None, &previews), Some("session_new".to_string()));
    assert_eq!(select_restore_session_id(None, &[]), None);
}

#[test]
fn renderer_primitives_expose_status_prompt_and_modal_hosts() {
    let mut state = TuiState::default();
    reduce_tui_state(
        &mut state,
        TuiAction::ProjectWorkspaceRootSet(Some(PathBuf::from("/tmp/tui-v2-worktree"))),
    );
    reduce_tui_state(
        &mut state,
        TuiAction::ProjectInfoSet("~/src/tui-v2-worktree:main • VibeWindow 0.1.0".to_string()),
    );
    reduce_tui_state(
        &mut state,
        TuiAction::ProjectGitStatusSet(crate::cli::session::GitWorkspaceStatus::ReadyDirty(vec![
            "Cargo.toml".to_string(),
            "crates/vw-cli/src/cli/tui_v2/render/mod.rs".to_string(),
        ])),
    );
    reduce_tui_state(&mut state, TuiAction::SessionTitleSet("S3-2 Host".to_string()));
    reduce_tui_state(&mut state, TuiAction::SessionScopeSet(Some("workspace".to_string())));
    reduce_tui_state(
        &mut state,
        TuiAction::SessionPathSet(Some(PathBuf::from("/tmp/tui-v2-host"))),
    );
    reduce_tui_state(&mut state, TuiAction::StatusProviderSet(Some("openai".to_string())));
    reduce_tui_state(&mut state, TuiAction::StatusModelSet(Some("gpt-5.4".to_string())));

    state.scroll = TuiScrollState {
        top_message: 2,
        viewport_messages: 6,
        viewport_height: 8,
        viewport_width: 48,
        overscan: 2,
        follow_tail: false,
        sticky_message: Some(1),
        last_seen_message: None,
    };
    state.prompt.queue_command(super::model::QueuedPromptCommand {
        raw: "/status".to_string(),
        kind: super::model::QueuedPromptCommandKind::SlashCommand,
        enqueued_ms: Some(1),
        ..super::model::QueuedPromptCommand::default()
    });
    state.prompt.queue_command(super::model::QueuedPromptCommand {
        raw: "ship it".to_string(),
        kind: super::model::QueuedPromptCommandKind::Submit,
        enqueued_ms: Some(2),
        ..super::model::QueuedPromptCommand::default()
    });
    state.overlays.push(super::model::UiOverlay::Confirm(super::model::UiConfirmOverlay {
        title: "Confirm Send".to_string(),
        body: "queued commands are parked".to_string(),
        confirm_label: "Send".to_string(),
        cancel_label: "Cancel".to_string(),
        destructive: false,
        intent: super::model::UiConfirmIntent::Generic,
    }));
    state.overlays.push(super::model::UiOverlay::Error(super::model::UiErrorOverlay {
        title: "Recoverable Error".to_string(),
        message: "modal stack wired".to_string(),
        recoverable: true,
    }));

    let status = select_status_summary(&state);
    let header = build_status_header(&state, &status, "TUI", "local endpoint", 1);
    let visible_window = select_visible_grouped_transcript_window(&state);
    let footer = build_status_footer(&state, &status, &visible_window);
    let prompt = build_prompt_host(&state);
    let project_context = build_project_context_host(&state, &status);
    let modified_files = build_modified_files_host(&state);
    let modal = build_modal_host(&state).expect("modal host should exist");

    assert_eq!(header.title, "S3-2 Host");
    assert_eq!(header.badge, "TUI");
    assert_eq!(header.scope, "workspace");
    assert_eq!(header.cwd, "/tmp/tui-v2-worktree");
    assert!(footer.pills.iter().any(|pill| pill.label == "终端 待命"));
    assert!(prompt.helper_text.contains("Enter 发送"));
    assert!(prompt.helper_text.contains("Shift+Enter"));
    assert_eq!(prompt.queued_commands.len(), 2);
    assert!(prompt.queued_commands.iter().any(|pill| pill.label.starts_with("命令")));
    assert!(prompt.queued_commands.iter().any(|pill| pill.label.starts_with("发送")));
    assert!(!prompt.queued_commands.iter().any(|pill| pill.label.contains("/status")));
    assert!(!prompt.footer_pills.is_empty());
    assert!(prompt.footer_pills.iter().any(|pill| pill.label.contains("模型 gpt-5.4")));
    assert!(prompt.footer_pills.iter().all(|pill| !pill.label.starts_with("目录 ")));
    assert!(prompt.helper_text.contains("Enter 发送"));
    assert!(project_context.pills.iter().any(|pill| pill.label == "git 脏区 2"));
    assert!(
        project_context
            .body_lines
            .iter()
            .any(|line| line.contains("项目: ~/src/tui-v2-worktree:main"))
    );
    assert!(!project_context.body_lines.iter().any(|line| line.contains("模型来源:")));
    assert!(modified_files.pills.iter().any(|pill| pill.label == "数量 2"));
    assert!(modified_files.body_lines.iter().any(|line| line == "• Cargo.toml"));
    assert_eq!(modal.title, "Recoverable Error");
    assert_eq!(modal.chips.len(), 1);
    assert!(modal.chips.iter().any(|pill| pill.label == "类型 错误"));
}

#[test]
fn prompt_submission_uses_workspace_root_not_session_file_path() {
    let mut state = TuiState::default();
    reduce_tui_state(
        &mut state,
        TuiAction::ProjectWorkspaceRootSet(Some(PathBuf::from("/tmp/worktree-root"))),
    );
    reduce_tui_state(
        &mut state,
        TuiAction::SessionPathSet(Some(PathBuf::from("/tmp/session-ui/session.json"))),
    );
    reduce_tui_state(&mut state, TuiAction::PromptValueSet("hello root".to_string()));

    let submission =
        super::controller::build_prompt_submission(&state, state.prompt.value.as_str())
            .expect("prompt submission should be built from workspace context");

    assert_eq!(submission.root.as_deref(), Some("/tmp/worktree-root"));
}

#[test]
fn status_header_accepts_shadow_badge_label() {
    let state = TuiState::default();
    let status = select_status_summary(&state);

    let header = build_status_header(&state, &status, "TUI shadow", "local endpoint", 0);

    assert_eq!(header.badge, "TUI shadow");
}

#[test]
fn modal_host_renders_search_and_task_overlay_body_lines() {
    let mut state = TuiState::default();
    reduce_tui_state(&mut state, TuiAction::SearchQuerySet("bridge".to_string()));

    let search_modal = build_modal_host(&state).expect("search modal host should exist");
    assert_eq!(search_modal.title, "搜索");
    assert!(search_modal.body_lines.iter().any(|line| line.contains("查询: bridge")));

    reduce_tui_state(&mut state, TuiAction::OverlayPopped);
    state.overlays.push(super::model::UiOverlay::Task(super::model::UiTaskOverlay {
        session_id: Some("session_task_modal".to_string()),
        turn_terminal: super::model::UiTurnTerminal::Streaming,
        pending_questions: 1,
        todo_count: 2,
        sync_error: Some("todo sync failed".to_string()),
        steps: vec![super::model::UiTaskStepItem {
            message_id: super::model::UiMessageId::local("step-modal-1"),
            step_index: 7,
            state: super::model::UiStepState::Running,
            started_ms: 70,
            finished_ms: None,
            model: Some("gpt-5.4".to_string()),
            finish_reason: None,
            usage: super::model::UiTokenUsage {
                input_tokens: 12,
                output_tokens: 34,
                cached_tokens: 5,
                reasoning_tokens: 8,
            },
        }],
        selected_index: 0,
    }));

    let task_modal = build_modal_host(&state).expect("task modal host should exist");
    assert_eq!(task_modal.title, "任务面板");
    assert!(task_modal.body_lines.iter().any(|line| line.contains("待处理问题: 1")));
    assert!(task_modal.body_lines.iter().any(|line| line.contains("步骤 7")));
    assert!(task_modal.body_lines.iter().any(|line| line.contains("当前步骤: 7")));
    assert!(task_modal.body_lines.iter().any(|line| line.contains("结束=进行中")));
    assert!(
        task_modal
            .body_lines
            .iter()
            .any(|line| line.contains("令牌: 输入=12 输出=34 缓存=5 推理=8"))
    );
}

#[test]
fn modal_host_renders_todo_overlay_drill_down() {
    let mut state = TuiState::default();
    state.overlays.push(super::model::UiOverlay::Todo(super::model::UiTodoOverlay {
        session_id: Some("session_todo_modal".to_string()),
        items: vec![
            super::model::UiTodoItem {
                id: "todo-1".to_string(),
                content: "wire overlay manager".to_string(),
                status: "completed".to_string(),
                priority: "high".to_string(),
            },
            super::model::UiTodoItem {
                id: "todo-2".to_string(),
                content: "add detail body\nfor selected todo".to_string(),
                status: "pending".to_string(),
                priority: "medium".to_string(),
            },
        ],
        selected_index: 1,
        dirty: true,
    }));

    let todo_modal = build_modal_host(&state).expect("todo modal host should exist");
    assert_eq!(todo_modal.title, "待办");
    assert!(todo_modal.body_lines.iter().any(|line| line.contains("会话: session_todo_modal")));
    assert!(todo_modal.body_lines.iter().any(|line| line.contains("状态汇总: 待处理=1 已完成=1")));
    assert!(todo_modal.body_lines.iter().any(|line| line.contains("当前待办: 2/2")));
    assert!(todo_modal.body_lines.iter().any(|line| line.contains("ID: todo-2")));
    assert!(todo_modal.body_lines.iter().any(|line| line.contains("优先级: medium")));
    assert!(todo_modal.body_lines.iter().any(|line| line.contains("  add detail body")));
    assert!(todo_modal.body_lines.iter().any(|line| line.contains("  for selected todo")));
}

#[test]
fn modal_host_renders_runtime_fallback_overlay_body_lines() {
    let mut state = TuiState::default();
    let overlay = runtime_event_fallback_overlay(&UiRuntimeEvent::Unknown {
        event_type: Some("chat.other".to_string()),
    })
    .expect("unknown runtime event should surface a fallback overlay");
    state.overlays.push(super::model::UiOverlay::Error(overlay));

    let modal = build_modal_host(&state).expect("runtime fallback modal should exist");
    assert_eq!(modal.title, "运行时事件回退");
    assert!(modal.body_lines.iter().any(|line| line.contains("不支持或无法解码的运行时事件")));
    assert!(modal.body_lines.iter().any(|line| line.contains("事件类型: chat.other")));
}

#[test]
fn runtime_event_fallback_overlay_classifies_terminal_failures() {
    let session_overlay = runtime_event_fallback_overlay(&UiRuntimeEvent::Terminal(
        UiRuntimeTerminalEvent::Error("gateway runtime session id is required".to_string()),
    ))
    .expect("session runtime error should surface an overlay");
    assert_eq!(session_overlay.title, "会话不可用");
    assert!(session_overlay.message.contains("可用的会话绑定"));

    let timeout_overlay = runtime_event_fallback_overlay(&UiRuntimeEvent::Terminal(
        UiRuntimeTerminalEvent::TimedOut {
            message: "deadline exceeded".to_string(),
            usage: None,
            message_id: None,
            parent_message_id: None,
        },
    ))
    .expect("timeout terminal event should surface an overlay");
    assert_eq!(timeout_overlay.title, "输出超时");
    assert!(timeout_overlay.message.contains("deadline exceeded"));

    assert!(
        runtime_event_fallback_overlay(&UiRuntimeEvent::Terminal(UiRuntimeTerminalEvent::Done {
            finish_reason: Some("stop".to_string()),
            usage: None,
            message_id: None,
            parent_message_id: None,
        },))
        .is_none()
    );
}

#[test]
fn runtime_event_fallback_overlay_classifies_permission_failures() {
    let permission_event = runtime_event_fallback_overlay(&UiRuntimeEvent::Unknown {
        event_type: Some("chat.permission_request".to_string()),
    })
    .expect("permission runtime event should surface a dedicated fallback overlay");
    assert_eq!(permission_event.title, "权限事件回退");
    assert!(permission_event.message.contains("请按 F2 打开待处理请求"));

    let permission_error =
        runtime_event_fallback_overlay(&UiRuntimeEvent::Terminal(UiRuntimeTerminalEvent::Error(
            "tool execution requires approval from supervisor".to_string(),
        )))
        .expect("permission terminal error should surface an approval fallback overlay");
    assert_eq!(permission_error.title, "权限请求失败");
    assert!(permission_error.message.contains("请按 F2 打开待处理请求"));
}

#[test]
fn todo_session_unavailable_overlay_points_to_recovery() {
    let overlay = todo_session_unavailable_overlay(TodoSessionAccessAction::Save);
    assert_eq!(overlay.title, "待办保存失败");
    assert!(overlay.message.contains("当前 TUI 宿主还没有绑定活动会话"));
    assert!(overlay.message.contains("请先新建或恢复一个会话"));
}

#[test]
fn status_footer_surfaces_new_message_pill_from_unseen_range() {
    let mut state = TuiState::default();

    reduce_tui_state(
        &mut state,
        TuiAction::PromptSubmissionStarted(
            super::model::PromptSubmission::new("继续收 S4-3")
                .with_stream_id(70)
                .with_session_id("session_unseen")
                .with_model("gpt-5.4"),
        ),
    );
    reduce_tui_state(
        &mut state,
        TuiAction::AssistantDeltaReceived("sticky prompt 已接入".to_string()),
    );

    state.scroll.top_message = 1;
    state.scroll.viewport_messages = 2;
    state.scroll.viewport_height = 2;
    state.scroll.viewport_width = 32;
    state.scroll.overscan = 0;
    state.scroll.follow_tail = false;
    state.scroll.sticky_message = Some(0);
    state.scroll.last_seen_message = Some(0);
    state.refresh_transcript_layout_for_current_width();

    let status = select_status_summary(&state);
    let visible_window = select_visible_grouped_transcript_window(&state);
    let footer = build_status_footer(&state, &status, &visible_window);

    assert_eq!(
        visible_window.sticky_prompt().map(|prompt| prompt.label()),
        Some("prompt m0".to_string())
    );
    assert!(footer.pills.iter().any(|pill| pill.label == "1 条新消息"));
}

#[test]
fn status_footer_surfaces_token_without_duplicate_step_pill() {
    let mut state = TuiState::default();
    reduce_tui_state(&mut state, TuiAction::SessionScopeSet(Some("workspace".to_string())));
    reduce_tui_state(
        &mut state,
        TuiAction::MessagePushed(super::model::UiMessage::Step(super::model::UiStep {
            base: super::model::UiMessageBase::new(super::model::UiMessageId::local(
                "step-footer-1",
            )),
            step_index: 3,
            started_ms: 10,
            finished_ms: Some(18),
            usage: super::model::UiTokenUsage {
                input_tokens: 5,
                output_tokens: 13,
                cached_tokens: 2,
                reasoning_tokens: 3,
            },
            finish_reason: Some("stop".to_string()),
            model: Some("gpt-5.4".to_string()),
            state: super::model::UiStepState::Complete,
        })),
    );

    state.scroll.viewport_messages = 4;
    state.scroll.viewport_height = 4;
    state.scroll.viewport_width = 40;
    state.refresh_transcript_layout_for_current_width();

    let status = select_status_summary(&state);
    let visible_window = select_visible_grouped_transcript_window(&state);
    let footer = build_status_footer(&state, &status, &visible_window);
    let header = build_status_header(&state, &status, "TUI", "local endpoint", 0);

    assert_eq!(header.scope, "workspace");
    assert!(footer.pills.iter().any(|pill| pill.label == "令牌 5/13"));
    assert!(!footer.pills.iter().any(|pill| pill.label.starts_with("步骤 ")));
}

#[test]
fn shadow_compare_ignores_trailing_newlines_but_catches_behavior_drift() {
    let gateway = crate::cli::processor::SessionProcessorComparableResult {
        output: "assistant answer\n".to_string(),
        usage: vw_shared::session::ui_types::TokenUsage {
            input_tokens: 12,
            output_tokens: 34,
            cached_tokens: 0,
            reasoning_tokens: 5,
        },
        step_finishes: 2,
        terminal: crate::cli::processor::SessionProcessorComparableTerminal::Done {
            finish_reason: Some("stop".to_string()),
            message_id: Some("msg_gateway".to_string()),
            parent_message_id: Some("msg_user".to_string()),
        },
    };
    let legacy_match = crate::cli::processor::SessionProcessorComparableResult {
        output: "assistant answer".to_string(),
        usage: gateway.usage.clone(),
        step_finishes: 2,
        terminal: crate::cli::processor::SessionProcessorComparableTerminal::Done {
            finish_reason: None,
            message_id: None,
            parent_message_id: None,
        },
    };

    assert!(compare_shadow_results(&gateway, &legacy_match).is_ok());

    let legacy_drift = crate::cli::processor::SessionProcessorComparableResult {
        output: "different answer".to_string(),
        usage: vw_shared::session::ui_types::TokenUsage {
            input_tokens: 12,
            output_tokens: 30,
            cached_tokens: 0,
            reasoning_tokens: 5,
        },
        step_finishes: 1,
        terminal: crate::cli::processor::SessionProcessorComparableTerminal::Error(
            "legacy failed".to_string(),
        ),
    };

    let diff = compare_shadow_results(&gateway, &legacy_drift)
        .expect_err("shadow compare should surface behavior drift");
    assert!(diff.contains("terminal"));
    assert!(diff.contains("output"));
    assert!(diff.contains("usage"));
    assert!(diff.contains("steps"));
}

#[test]
fn prompt_host_surfaces_slash_suggestions() {
    let mut state = TuiState::default();
    reduce_tui_state(&mut state, TuiAction::StatusModelSet(Some("gpt-5.4".to_string())));
    reduce_tui_state(&mut state, TuiAction::PromptValueSet("/mo".to_string()));

    let prompt = build_prompt_host(&state);

    assert!(prompt.suggestions.iter().any(|pill| pill.label == "/model"));
    assert_eq!(prompt.suggestion_rows.len(), 1);
    assert!(prompt.suggestion_rows[0].selected);
    assert!(prompt.helper_text.contains("Up/Down 切换"));
}

#[test]
fn prompt_host_keeps_suggestion_host_when_slash_query_has_no_match() {
    let mut state = TuiState::default();
    reduce_tui_state(&mut state, TuiAction::StatusModelSet(Some("gpt-5.4".to_string())));
    reduce_tui_state(&mut state, TuiAction::PromptValueSet("/zzzz".to_string()));

    let prompt = build_prompt_host(&state);

    assert!(prompt.suggestions.is_empty());
    assert!(
        prompt.suggestion_detail.as_deref().is_some_and(|detail| detail.contains("未找到匹配命令"))
    );
}

#[test]
fn offscreen_runtime_delta_skips_redraw_when_tail_stays_outside_viewport() {
    let mut state = seeded_transcript_state("session_delta_skip");
    configure_scrolled_viewport(&mut state, 1, 2);

    reduce_tui_state(
        &mut state,
        TuiAction::PromptSubmissionStarted(
            super::model::PromptSubmission::new("继续收 S4-4")
                .with_stream_id(80)
                .with_session_id("session_delta_skip")
                .with_model("gpt-5.4"),
        ),
    );
    reduce_tui_state(&mut state, TuiAction::AssistantDeltaReceived("first delta".to_string()));

    let before = TuiDeltaRedrawSnapshot::capture(&state);
    reduce_tui_state(&mut state, TuiAction::AssistantDeltaReceived(" second delta".to_string()));

    assert!(!should_redraw_after_runtime_delta(&before, &state));
}

#[test]
fn first_offscreen_runtime_delta_still_requires_redraw_for_new_unseen_message() {
    let mut state = seeded_transcript_state("session_delta_first");
    configure_scrolled_viewport(&mut state, 1, 2);

    reduce_tui_state(
        &mut state,
        TuiAction::PromptSubmissionStarted(
            super::model::PromptSubmission::new("继续收 S4-4")
                .with_stream_id(81)
                .with_session_id("session_delta_first")
                .with_model("gpt-5.4"),
        ),
    );

    let before = TuiDeltaRedrawSnapshot::capture(&state);
    reduce_tui_state(&mut state, TuiAction::AssistantDeltaReceived("first delta".to_string()));

    assert!(should_redraw_after_runtime_delta(&before, &state));
}

#[test]
fn visible_tail_runtime_delta_keeps_redraw_enabled() {
    let mut state = seeded_transcript_state("session_delta_visible");
    configure_scrolled_viewport(&mut state, 2, 4);

    reduce_tui_state(
        &mut state,
        TuiAction::PromptSubmissionStarted(
            super::model::PromptSubmission::new("继续收 S4-4")
                .with_stream_id(82)
                .with_session_id("session_delta_visible")
                .with_model("gpt-5.4"),
        ),
    );
    reduce_tui_state(&mut state, TuiAction::AssistantDeltaReceived("first delta".to_string()));
    configure_scrolled_viewport(&mut state, 3, 4);

    let before = TuiDeltaRedrawSnapshot::capture(&state);
    reduce_tui_state(&mut state, TuiAction::AssistantDeltaReceived(" second delta".to_string()));

    assert!(should_redraw_after_runtime_delta(&before, &state));
}

#[test]
fn controller_scrolls_between_grouped_transcript_anchors() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();

    reduce_tui_state(
        &mut state,
        TuiAction::PromptSubmissionStarted(
            super::model::PromptSubmission::new("继续")
                .with_stream_id(50)
                .with_session_id("session_scroll")
                .with_model("gpt-5.4"),
        ),
    );
    reduce_tui_state(&mut state, TuiAction::AssistantDeltaReceived("assistant body".to_string()));
    reduce_tui_state(
        &mut state,
        TuiAction::StepStarted {
            step_index: 1,
            started_ms: 51,
            model: Some("gpt-5.4".to_string()),
        },
    );
    reduce_tui_state(
        &mut state,
        TuiAction::StepFinished {
            step_index: 1,
            finished_ms: 52,
            usage: super::model::UiTokenUsage {
                input_tokens: 1,
                output_tokens: 1,
                cached_tokens: 0,
                reasoning_tokens: 0,
            },
            finish_reason: Some("stop".to_string()),
            model: Some("gpt-5.4".to_string()),
        },
    );
    reduce_tui_state(
        &mut state,
        TuiAction::MessagePushed(super::model::UiMessage::System(super::model::UiSystemMessage {
            base: super::model::UiMessageBase::new(super::model::UiMessageId::local("sys-after")),
            text: "after".to_string(),
            level: super::model::UiSystemMessageLevel::Info,
        })),
    );

    let anchors = select_transcript_message_anchors(&state);
    assert_eq!(anchors, vec![0, 1, 3]);

    state.scroll.top_message = anchors[0];
    state.scroll.follow_tail = false;

    let first = controller.handle_event(
        &mut state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
            KeyCode::Down,
            KeyModifiers::NONE,
        ))),
    );
    assert_eq!(first, TuiControllerCommand::Continue);
    assert_eq!(state.scroll.top_message, anchors[1]);

    let second = controller.handle_event(
        &mut state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
            KeyCode::Down,
            KeyModifiers::NONE,
        ))),
    );
    assert_eq!(second, TuiControllerCommand::Continue);
    assert_eq!(state.scroll.top_message, anchors[2]);
}
