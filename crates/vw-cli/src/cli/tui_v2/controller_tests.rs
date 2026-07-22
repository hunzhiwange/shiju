use std::path::PathBuf;
use std::time::Duration;

use crossterm::event::{
    Event as CrosstermEvent, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers,
    MouseEvent, MouseEventKind,
};
use ratatui::layout::Rect;

use super::controller::{
    TuiController, TuiControllerCommand, TuiControllerEvent, TuiOverlayCommand,
    TuiRenderFeedbackLike, build_prompt_submission,
};
use super::model::overlay::UiCommandPaletteOverlay;
use super::model::{
    McpServerTransport, PromptSubmission, UiConfirmOverlay, UiMcpOverlay, UiMcpServerInfo,
    UiMemoryEntry, UiMemoryOverlay, UiMessage, UiMessageBase, UiMessageId, UiOverlay,
    UiQuestionOption, UiQuestionOverlay, UiQuestionPrompt, UiSearchMatch, UiSearchOverlay,
    UiSystemMessage, UiSystemMessageLevel, UiTaskOverlay, UiTaskStepItem, UiTodoItem,
    UiTodoOverlay, UiTokenUsage, UiTurnTerminal,
};
use super::render::layout::FullscreenLayoutSlots;
use super::state::{TuiAction, TuiState, reduce_tui_state};

#[test]
fn controller_tick_and_sync_layout_are_stable() {
    let mut controller = TuiController::new(Duration::from_millis(10));
    let mut state = TuiState::default();
    assert_eq!(
        controller.handle_event(&mut state, TuiControllerEvent::Tick),
        TuiControllerCommand::Continue
    );

    controller.sync_layout(
        &mut state,
        &FakeFeedback(FullscreenLayoutSlots {
            scrollable: Rect::new(0, 3, 80, 20),
            ..FullscreenLayoutSlots::default()
        }),
    );
    assert_eq!(state.scroll.viewport_height, 18);
    assert_eq!(state.scroll.viewport_width, 78);

    controller.sync_layout(
        &mut state,
        &FakeFeedback(FullscreenLayoutSlots {
            scrollable: Rect::new(0, 3, 80, 20),
            ..FullscreenLayoutSlots::default()
        }),
    );
    assert_eq!(state.scroll.viewport_height, 18);
}

#[test]
fn controller_poll_tick_advances_spinner_without_terminal_input() {
    let mut controller = TuiController::new(Duration::ZERO);

    match controller.next_event() {
        Ok(event) => {
            assert!(matches!(event, TuiControllerEvent::Tick));
            assert_eq!(controller.spinner_frame(), 1);
        }
        Err(error) => {
            assert!(format!("{error:#}").contains("Failed to initialize input reader"));
        }
    }
}

#[test]
fn build_prompt_submission_trims_text_and_attaches_context() {
    let mut state = TuiState::default();
    state.session.session_id = Some("session-1".to_string());
    state.project.workspace_root = Some(PathBuf::from("/tmp/work"));
    state.status.model_name = Some("gpt-5.4".to_string());
    state.prompt.history.entries = vec!["old".to_string()];

    assert!(build_prompt_submission(&state, "   ").is_none());
    let submission = build_prompt_submission(&state, "  hello  ").expect("non-empty submit");
    assert_eq!(submission.text, "hello");
    assert_eq!(submission.session_id.as_deref(), Some("session-1"));
    assert_eq!(submission.root.as_deref(), Some("/tmp/work"));
    assert_eq!(submission.model.as_deref(), Some("gpt-5.4"));
    assert_eq!(submission.history_len, 1);
    assert!(submission.stream_id.is_some());
}

#[test]
fn prompt_key_events_edit_accept_suggestions_execute_submit_and_escape() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();

    assert_eq!(
        key(&mut controller, &mut state, KeyCode::Char('/')),
        TuiControllerCommand::Continue
    );
    assert_eq!(key(&mut controller, &mut state, KeyCode::Tab), TuiControllerCommand::Continue);
    assert_eq!(state.prompt.value, "/help");

    let command = key(&mut controller, &mut state, KeyCode::Enter);
    let TuiControllerCommand::ExecuteSlashCommand(invocation) = command else {
        panic!("slash command should execute");
    };
    assert_eq!(invocation.raw, "/help");

    assert_eq!(
        key(&mut controller, &mut state, KeyCode::Char('h')),
        TuiControllerCommand::Continue
    );
    assert_eq!(
        key(&mut controller, &mut state, KeyCode::Char('i')),
        TuiControllerCommand::Continue
    );
    let command = key(&mut controller, &mut state, KeyCode::Enter);
    let TuiControllerCommand::SubmitPrompt(submission) = command else {
        panic!("plain text should submit");
    };
    assert_eq!(submission.text, "hi");

    reduce_tui_state(&mut state, TuiAction::PromptValueSet("draft".to_string()));
    assert_eq!(key(&mut controller, &mut state, KeyCode::Esc), TuiControllerCommand::Continue);
    assert_eq!(state.prompt.value, "");
    assert_eq!(key(&mut controller, &mut state, KeyCode::Esc), TuiControllerCommand::Quit);
}

#[test]
fn busy_prompt_enter_queues_and_escape_cancels() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();
    state.prompt.start_submission(PromptSubmission::new("running"));
    reduce_tui_state(&mut state, TuiAction::PromptValueSet("/help".to_string()));

    assert_eq!(key(&mut controller, &mut state, KeyCode::Enter), TuiControllerCommand::Continue);
    assert_eq!(state.prompt.value, "");
    assert_eq!(state.prompt.queued_commands[0].raw, "/help");
    assert_eq!(
        key(&mut controller, &mut state, KeyCode::Esc),
        TuiControllerCommand::CancelActiveSubmission
    );
}

#[test]
fn prompt_shortcuts_open_expected_overlay_commands_and_help_overlay() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();

    assert_eq!(
        key_mod(&mut controller, &mut state, KeyCode::Char('f'), KeyModifiers::CONTROL),
        TuiControllerCommand::Overlay(TuiOverlayCommand::OpenSearchOverlay)
    );
    assert_eq!(
        key(&mut controller, &mut state, KeyCode::F(2)),
        TuiControllerCommand::Overlay(TuiOverlayCommand::OpenPendingQuestions)
    );
    assert_eq!(
        key(&mut controller, &mut state, KeyCode::F(3)),
        TuiControllerCommand::Overlay(TuiOverlayCommand::OpenTodoPanel)
    );
    assert_eq!(
        key(&mut controller, &mut state, KeyCode::F(4)),
        TuiControllerCommand::Overlay(TuiOverlayCommand::OpenTaskPanel)
    );
    assert_eq!(key(&mut controller, &mut state, KeyCode::F(1)), TuiControllerCommand::Continue);
    assert!(matches!(state.overlays.active(), Some(UiOverlay::Error(_))));
}

#[test]
fn confirm_error_search_question_todo_and_task_overlay_keys_are_handled() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();
    let confirm = UiConfirmOverlay {
        title: "Exit".to_string(),
        body: "Leave?".to_string(),
        confirm_label: "Exit".to_string(),
        cancel_label: "Stay".to_string(),
        destructive: false,
    };
    state.overlays.push(UiOverlay::Confirm(confirm.clone()));
    assert_eq!(
        key(&mut controller, &mut state, KeyCode::Enter),
        TuiControllerCommand::Overlay(TuiOverlayCommand::ConfirmAccepted(confirm))
    );
    assert_eq!(key(&mut controller, &mut state, KeyCode::Esc), TuiControllerCommand::Continue);

    state.overlays.push(UiOverlay::Error(super::model::UiErrorOverlay {
        title: "Recoverable".to_string(),
        message: "try again".to_string(),
        recoverable: true,
    }));
    assert_eq!(key(&mut controller, &mut state, KeyCode::Enter), TuiControllerCommand::Continue);

    push_system_message(&mut state, "Alpha beta");
    let message_id = state.messages[0].id().clone();
    state.overlays.push(UiOverlay::Search(UiSearchOverlay {
        query: String::new(),
        matches: vec![UiSearchMatch {
            message_id: Some(message_id.clone()),
            start: 0,
            end: 5,
            preview: "Alpha".to_string(),
        }],
        selected_index: Some(0),
        case_sensitive: false,
    }));
    assert_eq!(
        key(&mut controller, &mut state, KeyCode::Char('a')),
        TuiControllerCommand::Continue
    );
    assert_eq!(
        key_mod(&mut controller, &mut state, KeyCode::Char('s'), KeyModifiers::CONTROL),
        TuiControllerCommand::Continue
    );
    assert_eq!(key(&mut controller, &mut state, KeyCode::Enter), TuiControllerCommand::Continue);

    state.overlays.push(UiOverlay::Question(question_overlay()));
    assert_eq!(
        key(&mut controller, &mut state, KeyCode::Char('1')),
        TuiControllerCommand::Continue
    );
    assert_eq!(key(&mut controller, &mut state, KeyCode::Tab), TuiControllerCommand::Continue);
    assert!(matches!(
        key(&mut controller, &mut state, KeyCode::Enter),
        TuiControllerCommand::Overlay(TuiOverlayCommand::QuestionSubmitted(_))
    ));
    assert!(matches!(
        key_mod(&mut controller, &mut state, KeyCode::Char('r'), KeyModifiers::CONTROL),
        TuiControllerCommand::Overlay(TuiOverlayCommand::QuestionRejected(_))
    ));

    state.overlays.push(UiOverlay::Todo(UiTodoOverlay {
        session_id: Some("session".to_string()),
        items: vec![UiTodoItem {
            id: "1".to_string(),
            content: "one".to_string(),
            status: "pending".to_string(),
            priority: "medium".to_string(),
        }],
        selected_index: 0,
        dirty: false,
    }));
    assert_eq!(key(&mut controller, &mut state, KeyCode::Enter), TuiControllerCommand::Continue);
    assert!(matches!(
        key(&mut controller, &mut state, KeyCode::Char('s')),
        TuiControllerCommand::Overlay(TuiOverlayCommand::TodoSave(_))
    ));

    state.overlays.push(UiOverlay::Task(UiTaskOverlay {
        session_id: None,
        turn_terminal: UiTurnTerminal::Pending,
        pending_questions: 0,
        todo_count: 0,
        sync_error: None,
        steps: vec![UiTaskStepItem {
            message_id,
            step_index: 1,
            state: super::model::UiStepState::Complete,
            started_ms: 1,
            finished_ms: Some(2),
            model: None,
            finish_reason: None,
            usage: UiTokenUsage::default(),
        }],
        selected_index: 0,
    }));
    assert_eq!(key(&mut controller, &mut state, KeyCode::Enter), TuiControllerCommand::Continue);
}

#[test]
fn terminal_non_press_events_are_ignored() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();

    assert_eq!(
        controller.handle_event(
            &mut state,
            TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent {
                code: KeyCode::Char('x'),
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Release,
                state: KeyEventState::NONE,
            })),
        ),
        TuiControllerCommand::Continue
    );
    assert_eq!(
        controller.handle_event(
            &mut state,
            TuiControllerEvent::Terminal(CrosstermEvent::Resize(80, 24)),
        ),
        TuiControllerCommand::Continue
    );
    assert_eq!(
        controller.handle_event(
            &mut state,
            TuiControllerEvent::Terminal(CrosstermEvent::Mouse(MouseEvent {
                kind: MouseEventKind::Moved,
                column: 1,
                row: 1,
                modifiers: KeyModifiers::NONE,
            })),
        ),
        TuiControllerCommand::Continue
    );
    assert!(state.prompt.value.is_empty());
}

#[test]
fn controller_rejects_modified_prompt_text_and_invalid_question_choices() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();

    assert_eq!(
        key_mod(&mut controller, &mut state, KeyCode::Char('x'), KeyModifiers::ALT),
        TuiControllerCommand::Continue
    );
    assert!(state.prompt.value.is_empty());

    state.overlays.push(UiOverlay::Question(UiQuestionOverlay {
        request_id: "empty".to_string(),
        session_id: "session".to_string(),
        prompts: Vec::new(),
        answers: Vec::new(),
        tool: None,
        selected_index: 0,
    }));
    assert_eq!(
        key(&mut controller, &mut state, KeyCode::Char('9')),
        TuiControllerCommand::Continue
    );
    assert!(active_question(&state).answers.is_empty());
}

#[test]
fn ctrl_exit_confirms_when_session_has_context() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();

    assert_eq!(
        key_mod(&mut controller, &mut state, KeyCode::Char('c'), KeyModifiers::CONTROL),
        TuiControllerCommand::Quit
    );

    state.session.session_id = Some("session".to_string());
    assert_eq!(
        key_mod(&mut controller, &mut state, KeyCode::Char('c'), KeyModifiers::CONTROL),
        TuiControllerCommand::Overlay(TuiOverlayCommand::ConfirmExit)
    );

    state.session.session_id = None;
    push_system_message(&mut state, "existing");
    assert_eq!(
        key_mod(&mut controller, &mut state, KeyCode::Char('d'), KeyModifiers::CONTROL),
        TuiControllerCommand::Overlay(TuiOverlayCommand::ConfirmExit)
    );
}

#[test]
fn prompt_editing_covers_newline_delete_cursor_and_shortcuts() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();

    key(&mut controller, &mut state, KeyCode::Char('a'));
    key(&mut controller, &mut state, KeyCode::Char('b'));
    key(&mut controller, &mut state, KeyCode::Backspace);
    assert_eq!(state.prompt.value, "a");
    key(&mut controller, &mut state, KeyCode::Char('b'));
    key(&mut controller, &mut state, KeyCode::Left);
    key(&mut controller, &mut state, KeyCode::Right);
    key(&mut controller, &mut state, KeyCode::Left);
    key(&mut controller, &mut state, KeyCode::Delete);
    assert_eq!(state.prompt.value, "a");

    key_mod(&mut controller, &mut state, KeyCode::Enter, KeyModifiers::SHIFT);
    key(&mut controller, &mut state, KeyCode::Char('z'));
    assert_eq!(state.prompt.value, "a\nz");

    key(&mut controller, &mut state, KeyCode::Up);
    key(&mut controller, &mut state, KeyCode::Down);

    key(&mut controller, &mut state, KeyCode::Home);
    assert_eq!(state.prompt.cursor.char_index, 2);
    key(&mut controller, &mut state, KeyCode::End);
    assert_eq!(state.prompt.cursor.char_index, 3);

    assert_eq!(
        key(&mut controller, &mut state, KeyCode::F(5)),
        TuiControllerCommand::Overlay(TuiOverlayCommand::OpenMcpPanel)
    );
    assert_eq!(
        key(&mut controller, &mut state, KeyCode::F(6)),
        TuiControllerCommand::Overlay(TuiOverlayCommand::OpenMemoryPanel)
    );
}

#[test]
fn prompt_boundaries_cover_empty_enter_busy_blank_and_suggestion_previous() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();

    assert_eq!(key(&mut controller, &mut state, KeyCode::Enter), TuiControllerCommand::Continue);

    state.prompt.start_submission(PromptSubmission::new("running"));
    assert_eq!(key(&mut controller, &mut state, KeyCode::Enter), TuiControllerCommand::Continue);
    assert!(state.prompt.queued_commands.is_empty());
    state
        .prompt
        .finish_submission(super::model::PromptSubmissionStatus::Done { finish_reason: None });

    key(&mut controller, &mut state, KeyCode::Char('/'));
    key(&mut controller, &mut state, KeyCode::Down);
    let after_down = state.prompt.selected_suggestion_index;
    key(&mut controller, &mut state, KeyCode::BackTab);
    assert_ne!(state.prompt.selected_suggestion_index, after_down);
}

#[test]
fn search_overlay_keys_cover_empty_wrap_clear_unread_and_tail_paths() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();

    state.overlays.push(UiOverlay::Search(UiSearchOverlay::default()));
    assert_eq!(key(&mut controller, &mut state, KeyCode::Esc), TuiControllerCommand::Continue);
    assert!(state.overlays.active().is_none());

    state.overlays.push(UiOverlay::Search(UiSearchOverlay::default()));
    assert_eq!(key(&mut controller, &mut state, KeyCode::Up), TuiControllerCommand::Continue);
    assert_eq!(active_search(&state).selected_index, None);
    assert_eq!(key(&mut controller, &mut state, KeyCode::Down), TuiControllerCommand::Continue);
    assert_eq!(key(&mut controller, &mut state, KeyCode::Delete), TuiControllerCommand::Continue);
    assert!(active_search(&state).query.is_empty());
    assert_eq!(key(&mut controller, &mut state, KeyCode::Enter), TuiControllerCommand::Continue);
    assert!(state.overlays.active().is_some());

    push_system_message(&mut state, "one");
    push_system_message(&mut state, "two");
    let first_id = state.messages[0].id().clone();
    let second_id = state.messages[1].id().clone();
    state.scroll.last_seen_message = Some(0);
    let _ = state.overlays.stack.pop();
    state.overlays.push(UiOverlay::Search(UiSearchOverlay {
        query: "tw".to_string(),
        matches: vec![
            UiSearchMatch {
                message_id: Some(first_id),
                start: 0,
                end: 3,
                preview: "one".to_string(),
            },
            UiSearchMatch {
                message_id: Some(second_id),
                start: 0,
                end: 3,
                preview: "two".to_string(),
            },
        ],
        selected_index: Some(0),
        case_sensitive: false,
    }));

    key(&mut controller, &mut state, KeyCode::Up);
    assert_eq!(active_search(&state).selected_index, Some(1));
    key(&mut controller, &mut state, KeyCode::Backspace);
    assert_eq!(active_search(&state).query, "t");
    key(&mut controller, &mut state, KeyCode::End);
    assert!(state.overlays.active().is_none());
    assert!(state.scroll.follow_tail);

    state.overlays.push(UiOverlay::Search(UiSearchOverlay::default()));
    key(&mut controller, &mut state, KeyCode::Char('u'));
    assert!(state.overlays.active().is_none());
    assert_eq!(state.scroll.top_message, 0);

    state.overlays.push(UiOverlay::Search(UiSearchOverlay {
        matches: vec![UiSearchMatch {
            message_id: Some(UiMessageId::new("missing")),
            start: 0,
            end: 1,
            preview: "missing".to_string(),
        }],
        selected_index: Some(0),
        ..UiSearchOverlay::default()
    }));
    assert_eq!(key(&mut controller, &mut state, KeyCode::Enter), TuiControllerCommand::Continue);
    assert!(matches!(state.overlays.active(), Some(UiOverlay::Search(_))));
}

#[test]
fn question_overlay_keys_cover_custom_answers_selection_bounds_and_multi_toggle() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();

    state.overlays.push(UiOverlay::Question(question_overlay()));
    assert_eq!(
        key_mod(&mut controller, &mut state, KeyCode::Char('p'), KeyModifiers::CONTROL),
        TuiControllerCommand::Continue
    );
    key(&mut controller, &mut state, KeyCode::Tab);
    key(&mut controller, &mut state, KeyCode::Char('x'));
    key(&mut controller, &mut state, KeyCode::Char('y'));
    assert_eq!(active_question(&state).answers[1], vec!["__custom__:xy".to_string()]);
    key(&mut controller, &mut state, KeyCode::Backspace);
    assert_eq!(active_question(&state).answers[1], vec!["__custom__:x".to_string()]);
    key(&mut controller, &mut state, KeyCode::Delete);
    assert!(active_question(&state).answers[1].is_empty());
    key(&mut controller, &mut state, KeyCode::Down);
    assert_eq!(active_question(&state).selected_index, 1);
    key(&mut controller, &mut state, KeyCode::Up);
    assert_eq!(active_question(&state).selected_index, 0);

    let _ = state.overlays.stack.pop();
    state.overlays.push(UiOverlay::Question(multiple_question_overlay()));
    key(&mut controller, &mut state, KeyCode::Char('1'));
    assert_eq!(active_question(&state).answers[0], vec!["Allow".to_string()]);
    key(&mut controller, &mut state, KeyCode::Char('2'));
    assert_eq!(active_question(&state).answers[0], vec!["Allow".to_string(), "Deny".to_string()]);
    key(&mut controller, &mut state, KeyCode::Char('1'));
    assert_eq!(active_question(&state).answers[0], vec!["Deny".to_string()]);

    let _ = state.overlays.stack.pop();
    state.overlays.push(UiOverlay::Question(UiQuestionOverlay {
        request_id: "empty".to_string(),
        session_id: "session".to_string(),
        prompts: Vec::new(),
        answers: Vec::new(),
        tool: None,
        selected_index: 9,
    }));
    key(&mut controller, &mut state, KeyCode::Tab);
    assert_eq!(active_question(&state).selected_index, 0);
    assert_eq!(key(&mut controller, &mut state, KeyCode::Esc), TuiControllerCommand::Continue);
    assert!(state.overlays.active().is_none());
}

#[test]
fn todo_task_mcp_memory_and_generic_overlays_cover_navigation_boundaries() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();

    state.overlays.push(UiOverlay::Todo(UiTodoOverlay::default()));
    key(&mut controller, &mut state, KeyCode::Down);
    key(&mut controller, &mut state, KeyCode::Enter);
    assert_eq!(active_todo(&state).selected_index, 0);
    assert!(matches!(
        key(&mut controller, &mut state, KeyCode::Char('r')),
        TuiControllerCommand::Overlay(TuiOverlayCommand::TodoRefresh(_))
    ));
    assert_eq!(key(&mut controller, &mut state, KeyCode::Esc), TuiControllerCommand::Continue);
    assert!(state.overlays.active().is_none());

    state.overlays.push(UiOverlay::Todo(UiTodoOverlay {
        items: vec![
            UiTodoItem {
                id: "1".to_string(),
                content: "one".to_string(),
                status: "completed".to_string(),
                priority: "medium".to_string(),
            },
            UiTodoItem {
                id: "2".to_string(),
                content: "two".to_string(),
                status: "pending".to_string(),
                priority: "low".to_string(),
            },
        ],
        ..UiTodoOverlay::default()
    }));
    key(&mut controller, &mut state, KeyCode::Down);
    assert_eq!(active_todo(&state).selected_index, 1);
    key(&mut controller, &mut state, KeyCode::Up);
    assert_eq!(active_todo(&state).selected_index, 0);
    key(&mut controller, &mut state, KeyCode::Char(' '));
    assert_eq!(active_todo(&state).items[0].status, "pending");

    state.overlays.push(UiOverlay::Task(UiTaskOverlay {
        steps: vec![task_step("missing-1", 1), task_step("missing-2", 2)],
        ..UiTaskOverlay::default()
    }));
    assert_eq!(key(&mut controller, &mut state, KeyCode::Esc), TuiControllerCommand::Continue);
    assert!(!matches!(state.overlays.active(), Some(UiOverlay::Task(_))));

    state.overlays.push(UiOverlay::Task(UiTaskOverlay {
        steps: vec![task_step("missing-1", 1), task_step("missing-2", 2)],
        ..UiTaskOverlay::default()
    }));
    key(&mut controller, &mut state, KeyCode::Up);
    assert_eq!(active_task(&state).selected_index, 1);
    key(&mut controller, &mut state, KeyCode::Down);
    assert_eq!(active_task(&state).selected_index, 0);
    assert_eq!(key(&mut controller, &mut state, KeyCode::Enter), TuiControllerCommand::Continue);
    assert!(matches!(state.overlays.active(), Some(UiOverlay::Task(_))));

    state.overlays.push(UiOverlay::Mcp(UiMcpOverlay {
        servers: vec![
            UiMcpServerInfo {
                name: "one".to_string(),
                transport: McpServerTransport::Stdio,
                address: "cmd".to_string(),
            },
            UiMcpServerInfo {
                name: "two".to_string(),
                transport: McpServerTransport::Http,
                address: "http://localhost".to_string(),
            },
        ],
        config_source: "project".to_string(),
        selected_index: 0,
    }));
    assert_eq!(key(&mut controller, &mut state, KeyCode::Esc), TuiControllerCommand::Continue);
    assert!(!matches!(state.overlays.active(), Some(UiOverlay::Mcp(_))));

    state.overlays.push(UiOverlay::Mcp(UiMcpOverlay {
        servers: vec![
            UiMcpServerInfo {
                name: "one".to_string(),
                transport: McpServerTransport::Stdio,
                address: "cmd".to_string(),
            },
            UiMcpServerInfo {
                name: "two".to_string(),
                transport: McpServerTransport::Http,
                address: "http://localhost".to_string(),
            },
        ],
        config_source: "project".to_string(),
        selected_index: 0,
    }));
    key(&mut controller, &mut state, KeyCode::Up);
    assert_eq!(active_mcp(&state).selected_index, 1);
    key(&mut controller, &mut state, KeyCode::Down);
    assert_eq!(active_mcp(&state).selected_index, 0);

    state.overlays.push(UiOverlay::Memory(UiMemoryOverlay {
        entries: vec![
            UiMemoryEntry {
                scope: "project".to_string(),
                filename: "A.md".to_string(),
                path: "/tmp/A.md".to_string(),
                preview_lines: Vec::new(),
                total_lines: 0,
            },
            UiMemoryEntry {
                scope: "global".to_string(),
                filename: "B.md".to_string(),
                path: "/tmp/B.md".to_string(),
                preview_lines: vec!["line".to_string()],
                total_lines: 1,
            },
        ],
        selected_index: 0,
    }));
    assert_eq!(key(&mut controller, &mut state, KeyCode::Esc), TuiControllerCommand::Continue);
    assert!(!matches!(state.overlays.active(), Some(UiOverlay::Memory(_))));

    state.overlays.push(UiOverlay::Memory(UiMemoryOverlay {
        entries: vec![
            UiMemoryEntry {
                scope: "project".to_string(),
                filename: "A.md".to_string(),
                path: "/tmp/A.md".to_string(),
                preview_lines: Vec::new(),
                total_lines: 0,
            },
            UiMemoryEntry {
                scope: "global".to_string(),
                filename: "B.md".to_string(),
                path: "/tmp/B.md".to_string(),
                preview_lines: vec!["line".to_string()],
                total_lines: 1,
            },
        ],
        selected_index: 0,
    }));
    key(&mut controller, &mut state, KeyCode::Up);
    assert_eq!(active_memory(&state).selected_index, 1);
    key(&mut controller, &mut state, KeyCode::Tab);
    assert_eq!(active_memory(&state).selected_index, 0);

    state.overlays.push(UiOverlay::CommandPalette(UiCommandPaletteOverlay::default()));
    assert_eq!(
        key(&mut controller, &mut state, KeyCode::Char('x')),
        TuiControllerCommand::Continue
    );
    assert!(matches!(state.overlays.active(), Some(UiOverlay::CommandPalette(_))));
    key(&mut controller, &mut state, KeyCode::Esc);
    assert!(!matches!(state.overlays.active(), Some(UiOverlay::CommandPalette(_))));
}

#[test]
fn scroll_keys_cover_empty_and_anchor_navigation() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();

    assert_eq!(key(&mut controller, &mut state, KeyCode::PageDown), TuiControllerCommand::Continue);
    assert_eq!(state.scroll.top_message, 0);

    push_system_message(&mut state, "one");
    push_system_message(&mut state, "two");
    push_system_message(&mut state, "three");
    state.scroll.sync_viewport(1, 80);

    key(&mut controller, &mut state, KeyCode::End);
    assert_eq!(state.scroll.top_message, 2);
    assert!(state.scroll.follow_tail);
    key(&mut controller, &mut state, KeyCode::PageUp);
    assert_eq!(state.scroll.top_message, 1);
    assert!(!state.scroll.follow_tail);
    key(&mut controller, &mut state, KeyCode::Up);
    assert_eq!(state.scroll.top_message, 0);
    key(&mut controller, &mut state, KeyCode::Home);
    assert_eq!(state.scroll.top_message, 0);
}

fn key(
    controller: &mut TuiController,
    state: &mut TuiState,
    code: KeyCode,
) -> TuiControllerCommand {
    key_mod(controller, state, code, KeyModifiers::NONE)
}

fn key_mod(
    controller: &mut TuiController,
    state: &mut TuiState,
    code: KeyCode,
    modifiers: KeyModifiers,
) -> TuiControllerCommand {
    controller.handle_event(
        state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(code, modifiers))),
    )
}

fn push_system_message(state: &mut TuiState, text: &str) {
    reduce_tui_state(
        state,
        TuiAction::MessagePushed(UiMessage::System(UiSystemMessage {
            base: UiMessageBase::new(UiMessageId::new(format!("message-{}", state.messages.len()))),
            text: text.to_string(),
            level: UiSystemMessageLevel::Info,
        })),
    );
}

fn question_overlay() -> UiQuestionOverlay {
    UiQuestionOverlay {
        request_id: "req".to_string(),
        session_id: "session".to_string(),
        prompts: vec![
            UiQuestionPrompt {
                header: "Approval".to_string(),
                question: "Allow?".to_string(),
                options: vec![UiQuestionOption {
                    label: "Allow".to_string(),
                    description: "continue".to_string(),
                    preview: None,
                }],
                multiple: false,
                allow_custom_input: false,
            },
            UiQuestionPrompt {
                header: "Details".to_string(),
                question: "Why?".to_string(),
                options: Vec::new(),
                multiple: false,
                allow_custom_input: true,
            },
        ],
        answers: vec![Vec::new(), Vec::new()],
        tool: None,
        selected_index: 0,
    }
}

fn multiple_question_overlay() -> UiQuestionOverlay {
    UiQuestionOverlay {
        request_id: "multi".to_string(),
        session_id: "session".to_string(),
        prompts: vec![UiQuestionPrompt {
            header: "Approval".to_string(),
            question: "Allow?".to_string(),
            options: vec![
                UiQuestionOption {
                    label: "Allow".to_string(),
                    description: "continue".to_string(),
                    preview: None,
                },
                UiQuestionOption {
                    label: "Deny".to_string(),
                    description: "stop".to_string(),
                    preview: None,
                },
            ],
            multiple: true,
            allow_custom_input: false,
        }],
        answers: vec![Vec::new()],
        tool: None,
        selected_index: 0,
    }
}

fn task_step(message_id: &str, step_index: u32) -> UiTaskStepItem {
    UiTaskStepItem {
        message_id: UiMessageId::new(message_id),
        step_index,
        state: super::model::UiStepState::Complete,
        started_ms: 1,
        finished_ms: Some(2),
        model: None,
        finish_reason: None,
        usage: UiTokenUsage::default(),
    }
}

fn active_search(state: &TuiState) -> &UiSearchOverlay {
    let Some(UiOverlay::Search(overlay)) = state.overlays.active() else {
        panic!("search overlay should be active");
    };
    overlay
}

fn active_question(state: &TuiState) -> &UiQuestionOverlay {
    let Some(UiOverlay::Question(overlay)) = state.overlays.active() else {
        panic!("question overlay should be active");
    };
    overlay
}

fn active_todo(state: &TuiState) -> &UiTodoOverlay {
    let Some(UiOverlay::Todo(overlay)) = state.overlays.active() else {
        panic!("todo overlay should be active");
    };
    overlay
}

fn active_task(state: &TuiState) -> &UiTaskOverlay {
    let Some(UiOverlay::Task(overlay)) = state.overlays.active() else {
        panic!("task overlay should be active");
    };
    overlay
}

fn active_mcp(state: &TuiState) -> &UiMcpOverlay {
    let Some(UiOverlay::Mcp(overlay)) = state.overlays.active() else {
        panic!("mcp overlay should be active");
    };
    overlay
}

fn active_memory(state: &TuiState) -> &UiMemoryOverlay {
    let Some(UiOverlay::Memory(overlay)) = state.overlays.active() else {
        panic!("memory overlay should be active");
    };
    overlay
}

struct FakeFeedback(FullscreenLayoutSlots);

impl TuiRenderFeedbackLike for FakeFeedback {
    fn layout(&self) -> &FullscreenLayoutSlots {
        &self.0
    }
}
