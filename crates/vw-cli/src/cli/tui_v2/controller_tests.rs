use std::path::PathBuf;
use std::time::Duration;

use crossterm::event::{
    Event as CrosstermEvent, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers,
    MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use vw_config_types::skills::SkillsDirectoryProvider;

use super::app::{gateway_stream_request_from_state, queued_command_to_controller_command};
use super::commands::registry::{
    CommandAction, CommandAvailability, CommandExecutionPolicy, CommandRegistry, CommandSource,
    CommandSpec,
};
use super::completion::{CompletionCatalogItem, CompletionKind};
use super::controller::{
    TuiController, TuiControllerCommand, TuiControllerEvent, TuiOverlayCommand,
    TuiRenderFeedbackLike, build_prompt_submission,
};
use super::input::{execute_slash_command, parse_slash_command};
use super::model::integration::{
    IntegrationAction, IntegrationItem, IntegrationKind, IntegrationPanel,
};
use super::model::overlay::UiCommandPaletteOverlay;
use super::model::settings_panel::{SettingsPanel, SettingsValues};
use super::model::{
    AcpBackendOption, DelegateAgentOption, McpServerTransport, PromptMode, PromptSubmission,
    UiConfirmIntent, UiConfirmOverlay, UiMcpOverlay, UiMcpServerInfo, UiMemoryEntry,
    UiMemoryOverlay, UiMessage, UiMessageBase, UiMessageId, UiOverlay, UiQuestionOption,
    UiQuestionOverlay, UiQuestionPrompt, UiSearchMatch, UiSearchOverlay, UiSystemMessage,
    UiSystemMessageLevel, UiTaskOverlay, UiTaskStepItem, UiTodoItem, UiTodoOverlay, UiTokenUsage,
    UiTurnTerminal,
};
use super::render::layout::FullscreenLayoutSlots;
use super::render::{TuiRenderer, build_modal_host, build_prompt_host};
use super::state::{TuiAction, TuiState, reduce_tui_state};
use super::terminal_capabilities::TerminalCapabilities;

#[test]
fn command_parity_matrix_matches_registry_and_every_action_has_terminal_outcome() {
    let mut state = TuiState::default();
    let expected_names_or_aliases = [
        "background",
        "bg",
        "bashes",
        "checkup",
        "color",
        "config",
        "fewer-permission-prompts",
        "plugin",
        "routines",
        "cron",
        "allowed-tools",
    ];
    for token in expected_names_or_aliases {
        assert!(
            state.command_registry.resolve(token).is_some(),
            "/{token} must resolve through the final command registry"
        );
    }

    let available = state
        .command_registry
        .commands()
        .iter()
        .filter(|spec| spec.availability.is_available())
        .cloned()
        .collect::<Vec<_>>();
    for spec in available {
        let argument = if spec.takes_argument {
            Some(match spec.name.as_str() {
                "cd" => ".",
                "effort" => "medium",
                "export" => "target/tui-export.md",
                "help" => "all",
                "acp" => "default",
                "model" => "provider/model",
                "rename" => "renamed",
                "schedule" => "",
                "theme" => "dark",
                _ => "value",
            })
        } else {
            None
        };
        let raw = argument
            .filter(|value| !value.is_empty())
            .map_or_else(|| format!("/{}", spec.name), |value| format!("/{} {value}", spec.name));
        let invocation = parse_slash_command(&state, raw.as_str()).expect("slash invocation");
        let message_count = state.messages.len();
        let _outcome = execute_slash_command(&mut state, &invocation);
        let added_text = format!("{:?}", &state.messages[message_count..]);
        assert!(
            !added_text.contains("当前命令处理器尚未接入"),
            "/{} must have a terminal handler",
            spec.name
        );
    }

    for spec in
        state.command_registry.commands().iter().filter(|spec| !spec.availability.is_available())
    {
        assert!(
            spec.availability.unavailable_reason().is_some_and(|reason| !reason.trim().is_empty()),
            "/{} must explain why it is unavailable",
            spec.name
        );
    }
}

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
            scrollable_body: Rect::new(0, 3, 79, 20),
            scrollbar_track: Some(Rect::new(79, 3, 1, 20)),
            ..FullscreenLayoutSlots::default()
        }),
    );
    assert_eq!(state.scroll.viewport_height, 20);
    assert_eq!(state.scroll.viewport_width, 79);

    controller.sync_layout(
        &mut state,
        &FakeFeedback(FullscreenLayoutSlots {
            scrollable: Rect::new(0, 3, 80, 20),
            scrollable_body: Rect::new(0, 3, 79, 20),
            scrollbar_track: Some(Rect::new(79, 3, 1, 20)),
            ..FullscreenLayoutSlots::default()
        }),
    );
    assert_eq!(state.scroll.viewport_height, 20);
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
    assert!(matches!(state.overlays.active(), Some(UiOverlay::CommandPalette(_))));
    assert_eq!(key(&mut controller, &mut state, KeyCode::Enter), TuiControllerCommand::Continue);
    assert_eq!(state.prompt.value, "/help ");

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
fn prompt_shift_enter_inserts_newline_and_moves_cursor_to_next_line() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();

    for ch in "first".chars() {
        assert_eq!(
            key(&mut controller, &mut state, KeyCode::Char(ch)),
            TuiControllerCommand::Continue
        );
    }
    assert_eq!(
        key_mod(&mut controller, &mut state, KeyCode::Enter, KeyModifiers::SHIFT),
        TuiControllerCommand::Continue
    );
    for ch in "second".chars() {
        assert_eq!(
            key(&mut controller, &mut state, KeyCode::Char(ch)),
            TuiControllerCommand::Continue
        );
    }

    assert_eq!(state.prompt.value, "first\nsecond");
    assert_eq!(state.prompt.cursor.char_index, state.prompt.value.chars().count());
    let cursor_line = state
        .prompt
        .value
        .chars()
        .take(state.prompt.cursor.char_index)
        .filter(|ch| *ch == '\n')
        .count();
    assert_eq!(cursor_line, 1);
}

#[test]
fn prompt_selection_is_visible_multiline_and_editable() {
    let original = "中e\u{301}🙂\n下一行";
    let mut controller = TuiController::default();
    let mut state = TuiState::default();
    reduce_tui_state(&mut state, TuiAction::PromptValueSet(original.to_string()));

    assert_eq!(
        key_mod(&mut controller, &mut state, KeyCode::Char('a'), KeyModifiers::CONTROL),
        TuiControllerCommand::Continue
    );
    assert_eq!(state.prompt.editor.selection.expect("Ctrl+A selection").ordered(), (0, 7));
    assert_eq!(key(&mut controller, &mut state, KeyCode::Left), TuiControllerCommand::Continue);
    assert!(state.prompt.editor.selection.is_none());
    assert_eq!(state.prompt.cursor.char_index, 6);

    assert_eq!(
        key_mod(&mut controller, &mut state, KeyCode::Home, KeyModifiers::SHIFT),
        TuiControllerCommand::Continue
    );
    assert_eq!(state.prompt.editor.selection.expect("Shift+Home selection").ordered(), (4, 6));
    assert_eq!(
        key_mod(&mut controller, &mut state, KeyCode::Up, KeyModifiers::SHIFT),
        TuiControllerCommand::Continue
    );
    assert_eq!(state.prompt.editor.selection.expect("Shift+Up selection").ordered(), (0, 6));

    let selected_background = rendered_symbol_background(&state, "中");
    assert_eq!(rendered_symbol_background(&state, "e\u{301}"), selected_background);
    assert_eq!(rendered_symbol_background(&state, "🙂"), selected_background);
    assert_eq!(rendered_symbol_background(&state, "下"), selected_background);
    assert_ne!(
        rendered_symbol_background(&state, "行"),
        selected_background,
        "selected Unicode graphemes must use a visible high-contrast background"
    );

    assert_eq!(
        key_mod(&mut controller, &mut state, KeyCode::End, KeyModifiers::SHIFT),
        TuiControllerCommand::Continue
    );
    assert_eq!(state.prompt.editor.selection.expect("Shift+End selection").ordered(), (3, 6));
    assert_eq!(
        key_mod(&mut controller, &mut state, KeyCode::Down, KeyModifiers::SHIFT),
        TuiControllerCommand::Continue
    );
    assert!(state.prompt.editor.selection.is_none());
    assert_eq!(state.prompt.cursor.char_index, 6);
    assert_eq!(
        key_mod(&mut controller, &mut state, KeyCode::Left, KeyModifiers::SHIFT),
        TuiControllerCommand::Continue
    );
    assert_eq!(state.prompt.editor.selection.expect("Shift+Left selection").ordered(), (5, 6));
    assert_eq!(
        key_mod(&mut controller, &mut state, KeyCode::Right, KeyModifiers::SHIFT),
        TuiControllerCommand::Continue
    );
    assert!(state.prompt.editor.selection.is_none());
    assert_eq!(
        key_mod(&mut controller, &mut state, KeyCode::Right, KeyModifiers::SHIFT),
        TuiControllerCommand::Continue
    );
    assert_eq!(state.prompt.editor.selection.expect("Shift+Right selection").ordered(), (6, 7));
    assert_eq!(key(&mut controller, &mut state, KeyCode::Right), TuiControllerCommand::Continue);
    assert!(state.prompt.editor.selection.is_none());
    assert_eq!(state.prompt.cursor.char_index, 7);

    let mut input_state = TuiState::default();
    reduce_tui_state(&mut input_state, TuiAction::PromptValueSet(original.to_string()));
    key_mod(&mut controller, &mut input_state, KeyCode::Char('a'), KeyModifiers::CONTROL);
    let undo_depth = input_state.prompt.editor.undo_depth();
    key(&mut controller, &mut input_state, KeyCode::Char('替'));
    assert_eq!(input_state.prompt.value, "替");
    assert_eq!(input_state.prompt.cursor.char_index, 1);
    assert_eq!(input_state.prompt.editor.undo_depth(), undo_depth + 1);
    key_mod(&mut controller, &mut input_state, KeyCode::Char('z'), KeyModifiers::CONTROL);
    assert_eq!(input_state.prompt.value, original);

    let mut backspace_state = TuiState::default();
    reduce_tui_state(&mut backspace_state, TuiAction::PromptValueSet(original.to_string()));
    key_mod(&mut controller, &mut backspace_state, KeyCode::Char('a'), KeyModifiers::CONTROL);
    let undo_depth = backspace_state.prompt.editor.undo_depth();
    key(&mut controller, &mut backspace_state, KeyCode::Backspace);
    assert!(backspace_state.prompt.value.is_empty());
    assert_eq!(backspace_state.prompt.cursor.char_index, 0);
    assert_eq!(backspace_state.prompt.editor.undo_depth(), undo_depth + 1);
    key_mod(&mut controller, &mut backspace_state, KeyCode::Char('z'), KeyModifiers::CONTROL);
    assert_eq!(backspace_state.prompt.value, original);

    let mut delete_state = TuiState::default();
    reduce_tui_state(&mut delete_state, TuiAction::PromptValueSet(original.to_string()));
    key_mod(&mut controller, &mut delete_state, KeyCode::Char('a'), KeyModifiers::CONTROL);
    let undo_depth = delete_state.prompt.editor.undo_depth();
    key(&mut controller, &mut delete_state, KeyCode::Delete);
    assert!(delete_state.prompt.value.is_empty());
    assert_eq!(delete_state.prompt.cursor.char_index, 0);
    assert_eq!(delete_state.prompt.editor.undo_depth(), undo_depth + 1);
    key_mod(&mut controller, &mut delete_state, KeyCode::Char('z'), KeyModifiers::CONTROL);
    assert_eq!(delete_state.prompt.value, original);
}

#[test]
fn ctrl_c_copies_prompt_selection_without_exiting() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();
    reduce_tui_state(&mut state, TuiAction::PromptValueSet("中e\u{301}🙂\n下一行".to_string()));
    key_mod(&mut controller, &mut state, KeyCode::Char('a'), KeyModifiers::CONTROL);
    key_mod(&mut controller, &mut state, KeyCode::Left, KeyModifiers::SHIFT);
    let prompt_before = state.prompt.clone();

    let command = key_mod(&mut controller, &mut state, KeyCode::Char('c'), KeyModifiers::CONTROL);

    assert_eq!(
        command,
        TuiControllerCommand::CopyPromptSelection("中e\u{301}🙂\n下一".to_string())
    );
    assert_eq!(state.prompt, prompt_before);

    state.prompt.editor.selection = None;
    assert_eq!(
        key_mod(&mut controller, &mut state, KeyCode::Char('c'), KeyModifiers::CONTROL),
        TuiControllerCommand::Quit
    );
    assert_eq!(
        key_mod(&mut controller, &mut state, KeyCode::Char('d'), KeyModifiers::CONTROL),
        TuiControllerCommand::Quit
    );
}

#[test]
fn slash_palette_filters_scrolls_and_selects_registry_command() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();
    let mut commands = (0..14)
        .map(|index| CommandSpec {
            name: format!("dynamic-{index:02}"),
            aliases: Vec::new(),
            summary: format!("dynamic command {index}"),
            takes_argument: false,
            argument_hint: None,
            source: CommandSource::Workflow,
            availability: CommandAvailability::Available,
            execution_policy: CommandExecutionPolicy::Queued,
            action: CommandAction::Gateway(format!("workflow.dynamic-{index:02}")),
        })
        .collect::<Vec<_>>();
    commands.extend([
        CommandSpec {
            name: "deploy".to_string(),
            aliases: vec!["ship".to_string()],
            summary: "deploy a workflow".to_string(),
            takes_argument: true,
            argument_hint: Some("<environment>".to_string()),
            source: CommandSource::Workflow,
            availability: CommandAvailability::Available,
            execution_policy: CommandExecutionPolicy::Queued,
            action: CommandAction::Gateway("workflow.deploy".to_string()),
        },
        CommandSpec {
            name: "disabled".to_string(),
            aliases: Vec::new(),
            summary: "disabled workflow".to_string(),
            takes_argument: false,
            argument_hint: None,
            source: CommandSource::Workflow,
            availability: CommandAvailability::Unavailable {
                reason: "workflow service is offline".to_string(),
            },
            execution_policy: CommandExecutionPolicy::Immediate,
            action: CommandAction::Gateway("workflow.disabled".to_string()),
        },
    ]);
    state.command_registry = CommandRegistry::merge(vec![
        (CommandSource::Builtin, Ok(CommandRegistry::builtin().commands().to_vec())),
        (CommandSource::Workflow, Ok(commands)),
    ]);

    key(&mut controller, &mut state, KeyCode::Char('/'));
    let Some(UiOverlay::CommandPalette(palette)) = state.overlays.active() else {
        panic!("slash should open the command palette");
    };
    assert!(palette.items.len() > palette.visible_rows);

    key(&mut controller, &mut state, KeyCode::PageDown);
    let Some(UiOverlay::CommandPalette(palette)) = state.overlays.active() else {
        panic!("palette should remain open");
    };
    assert!(palette.first_visible_index > 0);

    for ch in "dep".chars() {
        key(&mut controller, &mut state, KeyCode::Char(ch));
    }
    let Some(UiOverlay::CommandPalette(palette)) = state.overlays.active() else {
        panic!("palette should remain open while filtering");
    };
    assert_eq!(palette.query, "dep");
    assert_eq!(palette.items.len(), 1);
    assert_eq!(palette.items[0].label, "/deploy");

    key(&mut controller, &mut state, KeyCode::Enter);
    assert!(state.overlays.active().is_none());
    assert_eq!(state.prompt.value, "/deploy ");

    let mut disabled_state = TuiState::default();
    disabled_state.command_registry = state.command_registry.clone();
    key(&mut controller, &mut disabled_state, KeyCode::Char('/'));
    for ch in "disabled".chars() {
        key(&mut controller, &mut disabled_state, KeyCode::Char(ch));
    }
    key(&mut controller, &mut disabled_state, KeyCode::Enter);
    let Some(UiOverlay::CommandPalette(palette)) = disabled_state.overlays.active() else {
        panic!("unavailable command must keep the palette open");
    };
    assert_eq!(disabled_state.prompt.value, "/");
    assert_eq!(palette.notice.as_deref(), Some("workflow service is offline"));
}

#[test]
fn busy_prompt_executes_read_only_commands_queues_actions_and_escape_cancels() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();
    state.prompt.start_submission(PromptSubmission::new("running"));
    reduce_tui_state(&mut state, TuiAction::PromptValueSet("/status".to_string()));

    let TuiControllerCommand::ExecuteSlashCommand(invocation) =
        key(&mut controller, &mut state, KeyCode::Enter)
    else {
        panic!("read-only immediate command should execute while busy");
    };
    assert_eq!(invocation.token, "status");
    assert_eq!(state.prompt.value, "");
    assert!(state.prompt.queued_commands.is_empty());

    reduce_tui_state(&mut state, TuiAction::PromptValueSet("/cd .".to_string()));
    assert_eq!(key(&mut controller, &mut state, KeyCode::Enter), TuiControllerCommand::Continue);
    assert_eq!(state.prompt.queued_commands[0].raw, "/cd .");
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
        intent: UiConfirmIntent::Generic,
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
fn mouse_wheel_scrolls_main_and_transcript_views() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();
    push_system_message(&mut state, "one");
    push_system_message(&mut state, "two");
    push_system_message(&mut state, "three");
    push_system_message(&mut state, "four");
    push_system_message(&mut state, "five");
    push_system_message(&mut state, "six");
    state.scroll.sync_viewport(1, 80);

    key(&mut controller, &mut state, KeyCode::End);
    assert_eq!(state.scroll.top_message, 5);
    mouse(&mut controller, &mut state, MouseEventKind::Moved);
    assert_eq!(state.scroll.top_message, 5);
    assert_eq!(
        mouse(&mut controller, &mut state, MouseEventKind::ScrollUp),
        TuiControllerCommand::Continue
    );
    assert!(!state.scroll.follow_tail);
    assert_eq!(state.scroll.top_message, 2);
    mouse(&mut controller, &mut state, MouseEventKind::ScrollDown);
    assert_eq!(state.scroll.top_message, 5);

    key_mod(&mut controller, &mut state, KeyCode::Char('o'), KeyModifiers::CONTROL);
    assert!(state.transcript_mode.active);
    assert_eq!(state.transcript_mode.top_line, 0);
    mouse(&mut controller, &mut state, MouseEventKind::Moved);
    assert_eq!(state.transcript_mode.top_line, 0);
    mouse(&mut controller, &mut state, MouseEventKind::ScrollDown);
    assert_eq!(state.transcript_mode.top_line, 3);
    mouse(&mut controller, &mut state, MouseEventKind::ScrollUp);
    assert_eq!(state.transcript_mode.top_line, 0);
}

#[test]
fn mouse_scrollbar_drag_and_track_click_seek_main_and_transcript_views() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();
    for index in 0..40 {
        push_system_message(&mut state, format!("message {index}").as_str());
    }
    state.scroll.follow_tail = false;
    state.scroll.top_message = 0;

    let (main_track, main_thumb) = render_and_sync_scrollbar(&mut controller, &mut state, 48, 18);
    assert!(main_thumb.len() >= 2, "main view needs an overflowing scrollbar fixture");

    mouse_at(
        &mut controller,
        &mut state,
        MouseEventKind::Down(MouseButton::Left),
        main_track.x.saturating_sub(1),
        main_track.bottom().saturating_sub(1),
    );
    assert_eq!(state.scroll.top_message, 0, "clicks outside the track must be ignored");

    state.overlays.push(UiOverlay::Question(question_overlay()));
    mouse_at(
        &mut controller,
        &mut state,
        MouseEventKind::Down(MouseButton::Left),
        main_track.x,
        main_track.bottom().saturating_sub(1),
    );
    assert_eq!(state.scroll.top_message, 0, "an active overlay must disable scrollbar seeking");
    let _ = state.overlays.stack.pop();

    mouse_at(
        &mut controller,
        &mut state,
        MouseEventKind::Down(MouseButton::Left),
        main_track.x,
        main_track.bottom().saturating_sub(1),
    );
    assert!(state.scroll.follow_tail, "a tail track click must clamp to the main view tail");
    let main_tail = state.scroll.top_message;
    assert!(main_tail > 0);

    let (main_track, _) = render_and_sync_scrollbar(&mut controller, &mut state, 48, 18);
    mouse_at(
        &mut controller,
        &mut state,
        MouseEventKind::Down(MouseButton::Left),
        main_track.x,
        main_track.y.saturating_add(main_track.height / 2),
    );
    assert!(
        state.scroll.top_message > 0 && state.scroll.top_message < main_tail,
        "a middle track click must seek between the main view endpoints"
    );

    let (main_track, _) = render_and_sync_scrollbar(&mut controller, &mut state, 48, 18);
    mouse_at(
        &mut controller,
        &mut state,
        MouseEventKind::Down(MouseButton::Left),
        main_track.x,
        main_track.y,
    );
    assert_eq!(state.scroll.top_message, 0, "a top track click must clamp to the first anchor");

    let (main_track, main_thumb) = render_and_sync_scrollbar(&mut controller, &mut state, 48, 18);
    mouse_at(
        &mut controller,
        &mut state,
        MouseEventKind::Down(MouseButton::Left),
        main_track.x,
        *main_thumb.first().expect("main thumb row"),
    );
    mouse_at(
        &mut controller,
        &mut state,
        MouseEventKind::Drag(MouseButton::Left),
        main_track.x,
        main_track.y.saturating_add(main_track.height / 2),
    );
    assert!(
        state.scroll.top_message > 0 && state.scroll.top_message < main_tail,
        "main dragging must continuously seek through the middle"
    );
    assert!(!state.scroll.follow_tail);
    let (main_track, _) = render_and_sync_scrollbar(&mut controller, &mut state, 48, 18);
    mouse_at(
        &mut controller,
        &mut state,
        MouseEventKind::Drag(MouseButton::Left),
        main_track.x,
        main_track.bottom(),
    );
    assert!(state.scroll.follow_tail, "main drag must clamp at the final anchor");
    mouse_at(
        &mut controller,
        &mut state,
        MouseEventKind::Up(MouseButton::Left),
        main_track.x,
        main_track.bottom().saturating_sub(1),
    );
    mouse_at(
        &mut controller,
        &mut state,
        MouseEventKind::Drag(MouseButton::Left),
        main_track.x,
        main_track.y,
    );
    assert_eq!(state.scroll.top_message, main_tail, "mouse up must end main scrollbar dragging");

    key_mod(&mut controller, &mut state, KeyCode::Char('o'), KeyModifiers::CONTROL);
    assert!(state.transcript_mode.active);
    let (_, transcript_thumb) = render_and_sync_scrollbar(&mut controller, &mut state, 48, 18);
    assert!(transcript_thumb.len() >= 2, "transcript view needs an overflowing scrollbar fixture");

    state.overlays.push(UiOverlay::Question(question_overlay()));
    let (transcript_track, transcript_thumb) =
        render_and_sync_scrollbar(&mut controller, &mut state, 48, 18);
    assert!(
        transcript_thumb.len() >= 2,
        "transcript mode owns the full screen and must keep its scrollbar interactive"
    );
    mouse_at(
        &mut controller,
        &mut state,
        MouseEventKind::Down(MouseButton::Left),
        transcript_track.x,
        transcript_track.bottom().saturating_sub(1),
    );
    assert_eq!(
        state.transcript_mode.top_line,
        state.transcript_mode.max_top_line(),
        "a tail track click must clamp to the final transcript line"
    );
    let _ = state.overlays.stack.pop();
    let transcript_tail = state.transcript_mode.top_line;

    let (transcript_track, _) = render_and_sync_scrollbar(&mut controller, &mut state, 48, 18);
    mouse_at(
        &mut controller,
        &mut state,
        MouseEventKind::Down(MouseButton::Left),
        transcript_track.x,
        transcript_track.y.saturating_add(transcript_track.height / 2),
    );
    assert!(
        state.transcript_mode.top_line > 0 && state.transcript_mode.top_line < transcript_tail,
        "a middle track click must seek between transcript endpoints"
    );

    let (transcript_track, _) = render_and_sync_scrollbar(&mut controller, &mut state, 48, 18);
    mouse_at(
        &mut controller,
        &mut state,
        MouseEventKind::Down(MouseButton::Left),
        transcript_track.x,
        transcript_track.y,
    );
    assert_eq!(
        state.transcript_mode.top_line, 0,
        "a top track click must clamp to the first transcript line"
    );

    let (transcript_track, transcript_thumb) =
        render_and_sync_scrollbar(&mut controller, &mut state, 48, 18);
    mouse_at(
        &mut controller,
        &mut state,
        MouseEventKind::Down(MouseButton::Left),
        transcript_track.x,
        *transcript_thumb.first().expect("transcript thumb row"),
    );
    mouse_at(
        &mut controller,
        &mut state,
        MouseEventKind::Drag(MouseButton::Left),
        transcript_track.x,
        transcript_track.y.saturating_add(transcript_track.height / 2),
    );
    assert!(
        state.transcript_mode.top_line > 0 && state.transcript_mode.top_line < transcript_tail,
        "transcript dragging must continuously seek through the middle"
    );
    let (transcript_track, _) = render_and_sync_scrollbar(&mut controller, &mut state, 48, 18);
    mouse_at(
        &mut controller,
        &mut state,
        MouseEventKind::Drag(MouseButton::Left),
        transcript_track.x,
        transcript_track.bottom(),
    );
    assert_eq!(
        state.transcript_mode.top_line, transcript_tail,
        "transcript drag must clamp at the final line"
    );
    mouse_at(
        &mut controller,
        &mut state,
        MouseEventKind::Up(MouseButton::Left),
        transcript_track.x,
        transcript_track.bottom().saturating_sub(1),
    );
    mouse_at(
        &mut controller,
        &mut state,
        MouseEventKind::Drag(MouseButton::Left),
        transcript_track.x,
        transcript_track.y,
    );
    assert_eq!(
        state.transcript_mode.top_line, transcript_tail,
        "mouse up must end transcript dragging"
    );

    let mut short_state = TuiState::default();
    push_system_message(&mut short_state, "only");
    let (short_track, short_thumb) =
        render_and_sync_scrollbar(&mut controller, &mut short_state, 48, 18);
    assert!(short_thumb.is_empty(), "non-overflowing content must not expose a thumb");
    mouse_at(
        &mut controller,
        &mut short_state,
        MouseEventKind::Down(MouseButton::Left),
        short_track.x,
        short_track.bottom().saturating_sub(1),
    );
    assert_eq!(short_state.scroll.top_message, 0, "a hidden scrollbar must not seek");
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
    let after_down = match state.overlays.active() {
        Some(UiOverlay::CommandPalette(palette)) => palette.selected_index,
        _ => panic!("slash should open palette"),
    };
    key(&mut controller, &mut state, KeyCode::BackTab);
    let after_back_tab = match state.overlays.active() {
        Some(UiOverlay::CommandPalette(palette)) => palette.selected_index,
        _ => panic!("palette should remain open"),
    };
    assert_ne!(after_back_tab, after_down);
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

#[test]
fn at_file_picker_is_compact_scrollable_and_keyboard_selectable() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();
    state.completion_catalog.items = (0..10)
        .map(|index| {
            let stem = if index == 8 { 'Z' } else { 'Q' };
            CompletionCatalogItem {
                replacement: format!("@src/{stem}{index}.rs"),
                label: format!("@src/{stem}{index}.rs"),
                detail: Some("/workspace".to_string()),
                kind: CompletionKind::File,
                score: 100 - index,
            }
        })
        .collect();
    state.prompt.set_value("@");

    key(&mut controller, &mut state, KeyCode::Up);
    assert_eq!(state.prompt.selected_suggestion_index, Some(0));

    key(&mut controller, &mut state, KeyCode::PageDown);
    assert_eq!(state.prompt.selected_suggestion_index, Some(8));
    let host = build_prompt_host(&state);
    assert_eq!(host.suggestion_rows.len(), 8);
    let selected = host
        .suggestion_rows
        .iter()
        .find(|row| row.selected)
        .expect("selected file remains visible");
    assert_eq!(selected.label, "src/Z8.rs");
    assert_eq!(selected.detail, None);
    assert_ne!(rendered_symbol_background(&state, "Z"), rendered_symbol_background(&state, "Q"));

    key(&mut controller, &mut state, KeyCode::Down);
    key(&mut controller, &mut state, KeyCode::Down);
    assert_eq!(state.prompt.selected_suggestion_index, Some(9));
    key(&mut controller, &mut state, KeyCode::PageUp);
    assert_eq!(state.prompt.selected_suggestion_index, Some(1));
    key(&mut controller, &mut state, KeyCode::PageDown);
    assert_eq!(state.prompt.selected_suggestion_index, Some(9));
    key(&mut controller, &mut state, KeyCode::Enter);
    assert_eq!(state.prompt.value, "@src/Q9.rs");

    state.prompt.set_value("@");
    key(&mut controller, &mut state, KeyCode::PageDown);
    key(&mut controller, &mut state, KeyCode::Tab);
    assert_eq!(state.prompt.value, "@src/Z8.rs");

    state.prompt.set_value("@");
    key(&mut controller, &mut state, KeyCode::Esc);
    assert_eq!(state.prompt.value, "@");
    assert!(state.prompt.completion_hidden);
}

#[test]
fn trailing_slash_completion_accepts_without_executing_command() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();
    state.prompt.set_value("请检查 /cl");
    let messages_before = state.messages.len();

    assert_eq!(key(&mut controller, &mut state, KeyCode::Enter), TuiControllerCommand::Continue);
    assert_eq!(state.prompt.value, "请检查 /clear");
    assert_eq!(
        state.messages.len(),
        messages_before,
        "accepting an embedded slash suggestion does not dispatch the command"
    );
}

#[test]
fn skill_help_distinguishes_selection_from_platform_management() {
    let state = TuiState::default();
    let slash_help = super::input::slash_command_help_text(&state.command_registry, false);
    assert!(slash_help.contains("输入 `$` 选择技能"));
    assert!(slash_help.contains("`/skills` 管理平台与启停"));

    let mut controller = TuiController::default();
    let mut state = TuiState::default();
    assert_eq!(key(&mut controller, &mut state, KeyCode::F(1)), TuiControllerCommand::Continue);
    let Some(UiOverlay::Error(help)) = state.overlays.active() else {
        panic!("F1 opens input help");
    };
    assert!(help.message.contains("输入 `$` 选择技能"));
    assert!(help.message.contains("`/skills` 管理平台与启停"));
}

#[test]
fn settings_model_picker_selects_and_saves_catalog_model() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();
    state.session_capabilities.models =
        vec!["openai/gpt-5.6".to_string(), "anthropic/claude-sonnet".to_string()];
    let mut values = SettingsValues::default();
    values.model = Some("openai/gpt-5.6".to_string());
    state.overlays.push(UiOverlay::Settings(SettingsPanel::for_capabilities(
        values,
        &state.session_capabilities,
    )));

    assert_eq!(
        key(&mut controller, &mut state, KeyCode::Enter),
        TuiControllerCommand::Continue,
        "Enter on the model row opens the picker instead of saving"
    );
    let Some(UiOverlay::Settings(panel)) = state.overlays.active() else {
        panic!("settings overlay remains open");
    };
    assert!(panel.model_picker.is_some());
    let model_picker = build_modal_host(&state).expect("model picker render model");
    assert_eq!(model_picker.title, "模型");
    assert!(model_picker.body_lines.iter().any(|line| line.contains("openai/gpt-5.6")));
    assert!(
        model_picker
            .body_lines
            .iter()
            .any(|line| line.contains("Enter 应用") && line.contains("Esc 取消"))
    );

    key(&mut controller, &mut state, KeyCode::Down);
    key(&mut controller, &mut state, KeyCode::Esc);
    let Some(UiOverlay::Settings(panel)) = state.overlays.active() else {
        panic!("Esc closes only the model picker");
    };
    assert!(panel.model_picker.is_none());
    assert_eq!(panel.pending.model.as_deref(), Some("openai/gpt-5.6"));

    key(&mut controller, &mut state, KeyCode::Enter);
    key(&mut controller, &mut state, KeyCode::Down);
    let TuiControllerCommand::Overlay(TuiOverlayCommand::SettingsSave(panel)) =
        key(&mut controller, &mut state, KeyCode::Enter)
    else {
        panic!("confirmed model is submitted through SettingsSave on the same Enter");
    };
    assert_eq!(panel.pending.model.as_deref(), Some("anthropic/claude-sonnet"));
}

#[test]
fn model_and_acp_picker_confirmation_applies_selection_immediately() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();
    state.session_capabilities.models =
        vec!["openai/gpt-5.6".to_string(), "anthropic/claude-sonnet".to_string()];
    state.session_capabilities.acp_backends = vec![
        AcpBackendOption::default_backend(),
        AcpBackendOption { key: Some("codex".to_string()), label: "codex".to_string() },
    ];

    let mut model_values = SettingsValues::default();
    model_values.model = Some("openai/gpt-5.6".to_string());
    state.overlays.push(UiOverlay::Settings(SettingsPanel::for_capabilities(
        model_values,
        &state.session_capabilities,
    )));
    assert_eq!(key(&mut controller, &mut state, KeyCode::Enter), TuiControllerCommand::Continue);
    key(&mut controller, &mut state, KeyCode::Down);
    let TuiControllerCommand::Overlay(TuiOverlayCommand::SettingsSave(model_panel)) =
        key(&mut controller, &mut state, KeyCode::Enter)
    else {
        panic!("model picker confirmation must save on the same Enter");
    };
    assert_eq!(model_panel.pending.model.as_deref(), Some("anthropic/claude-sonnet"));

    state.overlays.pop();
    let mut acp_panel =
        SettingsPanel::for_capabilities(SettingsValues::default(), &state.session_capabilities);
    acp_panel.selected = 2;
    state.overlays.push(UiOverlay::Settings(acp_panel));
    assert_eq!(key(&mut controller, &mut state, KeyCode::Enter), TuiControllerCommand::Continue);
    key(&mut controller, &mut state, KeyCode::Down);
    let TuiControllerCommand::Overlay(TuiOverlayCommand::SettingsSave(acp_panel)) =
        key(&mut controller, &mut state, KeyCode::Enter)
    else {
        panic!("ACP picker confirmation must save on the same Enter");
    };
    assert_eq!(acp_panel.pending.acp_backend.as_deref(), Some("codex"));
}

#[test]
fn skill_panel_enter_and_space_toggle_only_installed_items() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();
    let mut panel = IntegrationPanel::new(IntegrationKind::Skill);
    panel.finish_refresh(Ok(vec![
        IntegrationItem {
            id: "review-code".to_string(),
            title: "Review Code".to_string(),
            detail: "Review changes".to_string(),
            enabled: true,
            actions: vec![IntegrationAction {
                label: "启用/停用".to_string(),
                enabled: true,
                disabled_reason: None,
            }],
        },
        IntegrationItem {
            id: "not-installed".to_string(),
            title: "Not installed".to_string(),
            detail: "Unavailable".to_string(),
            enabled: false,
            actions: vec![IntegrationAction {
                label: "启用/停用".to_string(),
                enabled: false,
                disabled_reason: Some("未安装的 Skill 不能切换".to_string()),
            }],
        },
    ]));
    state.overlays.push(UiOverlay::Integration(panel));

    assert_eq!(key(&mut controller, &mut state, KeyCode::Enter), TuiControllerCommand::Continue);
    let platform_picker = build_modal_host(&state).expect("skill platform picker");
    assert_eq!(platform_picker.title, "技能平台");
    for provider in SkillsDirectoryProvider::ALL {
        assert!(platform_picker.body_lines.iter().any(|line| line.contains(provider.label())));
    }
    key(&mut controller, &mut state, KeyCode::Down);
    let TuiControllerCommand::Overlay(TuiOverlayCommand::IntegrationSkillProviderSelected(
        _,
        provider,
    )) = key(&mut controller, &mut state, KeyCode::Enter)
    else {
        panic!("skill provider selection reaches app");
    };
    assert_eq!(provider, SkillsDirectoryProvider::Codex);
    key(&mut controller, &mut state, KeyCode::Esc);
    let Some(UiOverlay::Integration(panel)) = state.overlays.active() else {
        panic!("Esc cancels only platform picker");
    };
    assert_eq!(panel.skill_provider, SkillsDirectoryProvider::Vibewindow);

    key(&mut controller, &mut state, KeyCode::Down);
    assert!(matches!(
        key(&mut controller, &mut state, KeyCode::Enter),
        TuiControllerCommand::Overlay(TuiOverlayCommand::IntegrationToggle(_))
    ));

    key(&mut controller, &mut state, KeyCode::Down);
    assert_eq!(
        key(&mut controller, &mut state, KeyCode::Char(' ')),
        TuiControllerCommand::Continue
    );
    assert!(matches!(state.overlays.active(), Some(UiOverlay::Integration(_))));

    let mut external_state = TuiState::default();
    let mut external_panel = IntegrationPanel::new(IntegrationKind::Skill);
    external_panel.set_skill_provider(SkillsDirectoryProvider::Codex);
    external_panel.finish_refresh(Ok(vec![IntegrationItem {
        id: "external".to_string(),
        title: "External".to_string(),
        detail: "Installed".to_string(),
        enabled: true,
        actions: Vec::new(),
    }]));
    external_state.overlays.push(UiOverlay::Integration(external_panel));
    key(&mut controller, &mut external_state, KeyCode::Down);
    assert_eq!(
        key(&mut controller, &mut external_state, KeyCode::Enter),
        TuiControllerCommand::Continue
    );
    assert_eq!(
        key(&mut controller, &mut external_state, KeyCode::Char(' ')),
        TuiControllerCommand::Continue
    );
}

#[test]
fn settings_delegate_agent_picker_routes_selected_agent() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();
    state.session_capabilities.delegate_agents = vec![
        DelegateAgentOption::main(),
        DelegateAgentOption { key: Some("reviewer".to_string()), label: "Reviewer".to_string() },
    ];
    state.overlays.push(UiOverlay::Settings(SettingsPanel::for_capabilities(
        SettingsValues::default(),
        &state.session_capabilities,
    )));
    key(&mut controller, &mut state, KeyCode::Down);
    assert_eq!(key(&mut controller, &mut state, KeyCode::Enter), TuiControllerCommand::Continue);
    let delegate_picker = build_modal_host(&state).expect("delegate picker render model");
    assert_eq!(delegate_picker.title, "委托代理");
    assert!(delegate_picker.body_lines.iter().any(|line| line.contains("Reviewer")));
    key(&mut controller, &mut state, KeyCode::Down);
    key(&mut controller, &mut state, KeyCode::Enter);
    let TuiControllerCommand::Overlay(TuiOverlayCommand::SettingsSave(panel)) =
        key(&mut controller, &mut state, KeyCode::Enter)
    else {
        panic!("confirmed delegate agent uses SettingsSave");
    };
    assert_eq!(panel.pending.delegate_agent.as_deref(), Some("reviewer"));
    state.session_options.delegate_agent.clone_from(&panel.pending.delegate_agent);
    state.overlays.pop();

    let submission = build_prompt_submission(&state, "review this").expect("submission");
    let request = gateway_stream_request_from_state(&state, &submission);
    assert_eq!(request.agent.as_deref(), Some("reviewer"));
    assert!(
        build_prompt_host(&state).footer_pills.iter().any(|pill| pill.label == "代理 Reviewer")
    );

    state.prompt.set_value("queued review");
    state.prompt.mode = PromptMode::Busy;
    key(&mut controller, &mut state, KeyCode::Enter);
    let queued = state.prompt.queued_commands.remove(0);
    assert_eq!(queued.execution.delegate_agent.as_deref(), Some("reviewer"));
    state.session_options.delegate_agent = None;
    let Some(TuiControllerCommand::SubmitPrompt(queued_submission)) =
        queued_command_to_controller_command(&state, queued)
    else {
        panic!("queued submit is restored");
    };
    assert_eq!(queued_submission.execution.delegate_agent.as_deref(), Some("reviewer"));
    let queued_request = gateway_stream_request_from_state(&state, &queued_submission);
    assert_eq!(queued_request.agent.as_deref(), Some("reviewer"));

    let main_submission = build_prompt_submission(&state, "main task").expect("main submission");
    assert_eq!(gateway_stream_request_from_state(&state, &main_submission).agent, None);
}

#[test]
fn settings_acp_picker_switches_backend_and_forces_first_session() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();
    state.session_capabilities.acp_backends = vec![
        AcpBackendOption::default_backend(),
        AcpBackendOption { key: Some("codex".to_string()), label: "codex".to_string() },
        AcpBackendOption { key: Some("zeta".to_string()), label: "zeta".to_string() },
    ];
    state.overlays.push(UiOverlay::Settings(SettingsPanel::for_capabilities(
        SettingsValues::default(),
        &state.session_capabilities,
    )));

    key(&mut controller, &mut state, KeyCode::Down);
    key(&mut controller, &mut state, KeyCode::Down);
    assert_eq!(key(&mut controller, &mut state, KeyCode::Enter), TuiControllerCommand::Continue);
    let acp_picker = build_modal_host(&state).expect("ACP picker render model");
    assert_eq!(acp_picker.title, "ACP 后端");
    assert!(acp_picker.body_lines.iter().any(|line| line.contains("默认 ACP")));
    assert!(acp_picker.body_lines.iter().any(|line| line.contains("codex")));
    assert!(acp_picker.body_lines.iter().any(|line| line.contains("Enter 应用")));
    key(&mut controller, &mut state, KeyCode::Down);
    let TuiControllerCommand::Overlay(TuiOverlayCommand::SettingsSave(panel)) =
        key(&mut controller, &mut state, KeyCode::Enter)
    else {
        panic!("confirmed ACP backend uses SettingsSave on the same Enter");
    };
    assert_eq!(panel.pending.acp_backend.as_deref(), Some("codex"));
    state.session_options.select_acp_backend(panel.pending.acp_backend.clone());
    state.overlays.pop();

    let first = build_prompt_submission(&state, "use ACP").expect("first ACP submission");
    let first_request = gateway_stream_request_from_state(&state, &first);
    assert_eq!(first_request.acp_agent.as_deref(), Some("codex"));
    let first_options = first_request.options.as_ref().expect("ACP options");
    assert_eq!(first_options["acp_test"], true);
    assert_eq!(first_options["acp_agent"], "codex");
    assert_eq!(first_options["acp_force_new_session"], true);
    assert_eq!(first_options["acp_permission_mode"], "approve-reads");
    assert!(first_options.get("cwd").is_some());

    state.session_options.finish_acp_session_start(&first.execution, true);
    let next = build_prompt_submission(&state, "reuse ACP").expect("next ACP submission");
    assert_eq!(
        gateway_stream_request_from_state(&state, &next).options.expect("ACP options")["acp_force_new_session"],
        false
    );

    state.session_options.select_acp_backend(None);
    state.session_options.select_acp_backend(Some("codex".to_string()));
    let failed = build_prompt_submission(&state, "failed ACP").expect("failed ACP submission");
    state.session_options.finish_acp_session_start(&failed.execution, false);
    assert!(state.session_options.acp_force_new_session());

    let mut values = SettingsValues::default();
    values.acp_backend = Some("codex".to_string());
    let mut panel = SettingsPanel::for_capabilities(values, &state.session_capabilities);
    assert!(panel.open_acp_backend_picker());
    panel.move_acp_backend_picker_selection(-1);
    assert!(panel.confirm_acp_backend_picker());
    assert_eq!(panel.pending.acp_backend, None);

    state.prompt.set_value("queued ACP");
    state.prompt.mode = PromptMode::Busy;
    key(&mut controller, &mut state, KeyCode::Enter);
    let queued = state.prompt.queued_commands.remove(0);
    assert_eq!(queued.execution.acp_backend.as_deref(), Some("codex"));
    assert!(queued.execution.acp_force_new_session);
    state.session_options.select_acp_backend(None);
    let Some(TuiControllerCommand::SubmitPrompt(queued_submission)) =
        queued_command_to_controller_command(&state, queued)
    else {
        panic!("queued ACP submit is restored");
    };
    let queued_request = gateway_stream_request_from_state(&state, &queued_submission);
    assert_eq!(queued_request.acp_agent.as_deref(), Some("codex"));
    assert_eq!(queued_request.options.expect("queued ACP options")["acp_force_new_session"], true);
    let default_submission =
        build_prompt_submission(&state, "default backend").expect("default submission");
    let default_request = gateway_stream_request_from_state(&state, &default_submission);
    assert_eq!(default_request.acp_agent, None);
    assert_eq!(default_request.options, None);
}

#[test]
fn settings_subagent_mode_toggles_independently() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();
    state.overlays.push(UiOverlay::Settings(SettingsPanel::for_capabilities(
        SettingsValues::default(),
        &state.session_capabilities,
    )));
    for _ in 0..4 {
        key(&mut controller, &mut state, KeyCode::Down);
    }
    key(&mut controller, &mut state, KeyCode::Right);
    let TuiControllerCommand::Overlay(TuiOverlayCommand::SettingsSave(panel)) =
        key(&mut controller, &mut state, KeyCode::Enter)
    else {
        panic!("subagent mode setting saves");
    };
    assert!(panel.pending.subagent_mode_enabled);
    assert!(!panel.pending.full_access_enabled);
    assert_eq!(panel.pending.acp_backend, None);

    state.session_options.set_subagent_mode(true);
    assert!(build_prompt_host(&state).footer_pills.iter().any(|pill| pill.label == "子智能体 开"));
}

#[test]
fn settings_workflow_and_goal_modes_can_both_be_enabled() {
    let mut controller = TuiController::default();
    let mut state = TuiState::default();
    let values = SettingsValues::default();
    // 默认目标开启；仅开工作流应保持目标
    assert_eq!(values.goal_mode, super::model::session_mode::GoalMode::Enabled);
    assert_eq!(values.workflow_mode, super::model::session_mode::WorkflowMode::Disabled);
    state.overlays.push(UiOverlay::Settings(SettingsPanel::for_capabilities(
        values,
        &state.session_capabilities,
    )));
    for _ in 0..5 {
        key(&mut controller, &mut state, KeyCode::Down);
    }
    key(&mut controller, &mut state, KeyCode::Right);
    let TuiControllerCommand::Overlay(TuiOverlayCommand::SettingsSave(panel)) =
        key(&mut controller, &mut state, KeyCode::Enter)
    else {
        panic!("workflow mode setting saves");
    };
    assert_eq!(panel.pending.workflow_mode, super::model::session_mode::WorkflowMode::Enabled);
    assert_eq!(panel.pending.goal_mode, super::model::session_mode::GoalMode::Enabled);
    assert!(!panel.pending.subagent_mode_enabled);

    state.session_options.set_workflow_mode(true);
    assert!(build_prompt_host(&state).footer_pills.iter().any(|pill| pill.label == "工作流 开"));
    assert!(
        build_prompt_host(&state).footer_pills.iter().all(|pill| !pill.label.starts_with("目标 "))
    );

    let mut goal_state = TuiState::default();
    let mut goal_values = SettingsValues::default();
    goal_values.workflow_mode = super::model::session_mode::WorkflowMode::Enabled;
    goal_values.goal_mode = super::model::session_mode::GoalMode::Disabled;
    goal_state.overlays.push(UiOverlay::Settings(SettingsPanel::for_capabilities(
        goal_values,
        &goal_state.session_capabilities,
    )));
    // 目标模式设置项已移除；保存后仍强制开启。
    let TuiControllerCommand::Overlay(TuiOverlayCommand::SettingsSave(goal_panel)) =
        key(&mut controller, &mut goal_state, KeyCode::Enter)
    else {
        panic!("settings save");
    };
    assert_eq!(goal_panel.pending.goal_mode, super::model::session_mode::GoalMode::Enabled);
    assert_eq!(goal_panel.pending.workflow_mode, super::model::session_mode::WorkflowMode::Enabled);

    goal_state.session_options.set_workflow_mode(true);
    goal_state.session_options.set_goal_mode(false);
    assert!(
        build_prompt_host(&goal_state)
            .footer_pills
            .iter()
            .all(|pill| !pill.label.starts_with("目标 "))
    );
    assert!(goal_state.session_options.workflow_mode_enabled());
    assert!(goal_state.session_options.goal_mode_enabled());
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

fn rendered_symbol_background(state: &TuiState, symbol: &str) -> String {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    let renderer = TuiRenderer::for_capabilities(TerminalCapabilities::default(), false);
    terminal
        .draw(|frame| {
            renderer.render_frame(frame, state, "VW", "local", 0);
        })
        .expect("render prompt selection");
    let buffer = terminal.backend().buffer();
    let area = buffer.area;
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let cell = buffer.cell((x, y)).expect("cell inside buffer");
            if cell.symbol() == symbol {
                return format!("{:?}", cell.bg);
            }
        }
    }
    panic!("rendered symbol {symbol:?} not found");
}

fn mouse(
    controller: &mut TuiController,
    state: &mut TuiState,
    kind: MouseEventKind,
) -> TuiControllerCommand {
    mouse_at(controller, state, kind, 1, 1)
}

fn mouse_at(
    controller: &mut TuiController,
    state: &mut TuiState,
    kind: MouseEventKind,
    column: u16,
    row: u16,
) -> TuiControllerCommand {
    controller.handle_event(
        state,
        TuiControllerEvent::Terminal(CrosstermEvent::Mouse(MouseEvent {
            kind,
            column,
            row,
            modifiers: KeyModifiers::NONE,
        })),
    )
}

fn render_and_sync_scrollbar(
    controller: &mut TuiController,
    state: &mut TuiState,
    width: u16,
    height: u16,
) -> (Rect, Vec<u16>) {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    let renderer = TuiRenderer::for_capabilities(TerminalCapabilities::default(), false);
    let mut feedback = None;
    for _ in 0..2 {
        terminal
            .draw(|frame| {
                feedback = Some(renderer.render_frame(frame, state, "VW", "local", 0));
            })
            .expect("render scrollbar frame");
        let current = feedback.as_ref().expect("render feedback");
        state.apply_transcript_measurements(&current.transcript_measurements);
        controller.sync_layout(state, current);
    }
    let feedback = feedback.expect("final render feedback");
    let track = feedback.scrollbar.map_or_else(
        || {
            if state.transcript_mode.active {
                Rect::new(width.saturating_sub(2), 1, 1, height.saturating_sub(2))
            } else {
                feedback.layout.scrollbar_track.expect("main scrollbar track")
            }
        },
        |scrollbar| scrollbar.track,
    );
    let thumb_rows = feedback
        .scrollbar
        .map_or_else(Vec::new, |scrollbar| (scrollbar.thumb.y..scrollbar.thumb.bottom()).collect());
    (track, thumb_rows)
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
