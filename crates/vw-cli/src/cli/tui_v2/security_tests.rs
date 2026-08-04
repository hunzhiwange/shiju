use std::path::Path;

use crossterm::event::{
    Event as CrosstermEvent, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent,
    MouseEventKind,
};
use ratatui::Terminal;
use ratatui::backend::TestBackend;

use super::commands::registry::{
    CommandAction, CommandAvailability, CommandExecutionPolicy, CommandRegistry, CommandSource,
    CommandSpec,
};
use super::app::{full_access_confirmation, gateway_stream_request_from_state};
use super::controller::{
    TuiController, TuiControllerCommand, TuiControllerEvent, build_prompt_submission,
};
use super::model::integration::IntegrationKind;
use super::model::settings_panel::{SettingsPanel, SettingsValues};
use super::model::PermissionMode;
use super::model::{
    UiConfirmIntent, UiConfirmOverlay, UiMessage, UiMessageBase, UiMessageId, UiOverlay,
    UiSystemMessage, UiSystemMessageLevel, UiThinkingBlock, UiThinkingPhase,
};
use super::render::{TuiRenderer, build_modal_host, build_prompt_host};
use super::runtime::search::validated_open_effect;
use super::state::TuiState;
use super::transcript_controller::transcript_plain_text_lines;

#[test]
fn tui_security_control_sequences_effects_and_recovery_are_contained() {
    let injected = "\u{1b}]0;owned\u{7}正文\u{9b}31m";
    let mut state = TuiState::default();
    state.append_message(UiMessage::System(UiSystemMessage {
        base: UiMessageBase::new(UiMessageId::local("security-injection")),
        text: injected.to_string(),
        level: UiSystemMessageLevel::Warning,
    }));
    state.scroll.sync_viewport(8, 80);
    state.refresh_transcript_layout_for_current_width();

    let backend = TestBackend::new(80, 12);
    let mut terminal = Terminal::new(backend).expect("security terminal");
    terminal
        .draw(|frame| {
            TuiRenderer::default().render_frame(frame, &state, "ready", "local", 0);
        })
        .expect("malicious payload must not crash rendering");
    let rendered =
        terminal.backend().buffer().content().iter().map(|cell| cell.symbol()).collect::<String>();
    assert!(!rendered.chars().any(is_terminal_control));
    assert_eq!(
        rendered.matches('�').count(),
        3,
        "controls must be rendered as visible replacement characters: {rendered:?}"
    );

    let workspace = tempfile::tempdir().expect("workspace");
    std::fs::create_dir(workspace.path().join("src")).expect("src");
    std::fs::write(workspace.path().join("src/lib.rs"), "pub fn safe() {}\n").expect("file");
    assert!(validated_open_effect(workspace.path(), Path::new("src/lib.rs"), Some(1)).is_ok());
    assert!(validated_open_effect(workspace.path(), Path::new("../escape"), None).is_err());
    assert!(validated_open_effect(workspace.path(), Path::new(".env"), None).is_err());
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink("/tmp", workspace.path().join("outside"))
            .expect("outside symlink");
        assert!(validated_open_effect(workspace.path(), Path::new("outside/file"), None).is_err());
    }

    let hidden_reasoning = "private chain-of-thought";
    state.messages.push(UiMessage::Thinking(UiThinkingBlock {
        base: UiMessageBase::new(UiMessageId::local("hidden-reasoning")),
        summary: None,
        content: hidden_reasoning.to_string(),
        timing: Vec::new(),
        collapsed: true,
        phase: UiThinkingPhase::HiddenHistory,
    }));
    let copied_transcript = transcript_plain_text_lines(&state.messages).join("\n");
    assert!(!copied_transcript.contains(hidden_reasoning));
    assert!(copied_transcript.contains("[hidden reasoning]"));

    state.prompt.set_value("未提交草稿");
    state.overlays.push(UiOverlay::Confirm(UiConfirmOverlay {
        title: "确认".to_string(),
        body: "保持 modal 隔离".to_string(),
        confirm_label: "确认".to_string(),
        cancel_label: "取消".to_string(),
        destructive: false,
        intent: UiConfirmIntent::Generic,
    }));
    let active_overlay = state.overlays.active().cloned();
    let command = TuiController::default().handle_event(
        &mut state,
        TuiControllerEvent::Terminal(CrosstermEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 1,
            row: 1,
            modifiers: KeyModifiers::NONE,
        })),
    );
    assert_eq!(command, TuiControllerCommand::Continue);
    assert_eq!(state.overlays.active(), active_overlay.as_ref());
    assert_eq!(state.prompt.value, "未提交草稿");

    state.status.last_error = Some("gateway disconnected".to_string());
    assert_eq!(state.prompt.value, "未提交草稿");
}

#[test]
fn tui_security_dynamic_commands_cannot_escalate_and_source_errors_are_redacted() {
    let privileged = CommandSpec {
        name: "unsafe".to_string(),
        aliases: Vec::new(),
        summary: "must be rejected".to_string(),
        takes_argument: false,
        argument_hint: None,
        source: CommandSource::Skill,
        availability: CommandAvailability::Available,
        execution_policy: CommandExecutionPolicy::Queued,
        action: CommandAction::LocalEffect("open-shell".to_string()),
    };
    let registry = CommandRegistry::merge(vec![
        (CommandSource::Builtin, Ok(CommandRegistry::builtin().commands().to_vec())),
        (CommandSource::Skill, Ok(vec![privileged])),
        (
            CommandSource::Mcp,
            Err("request failed token=top-secret secret=also-secret authorization=Bearer-secret"
                .to_string()),
        ),
    ]);

    assert!(registry.resolve("unsafe").is_none());
    let errors = registry.errors().join("\n");
    assert!(!errors.contains("top-secret"));
    assert!(!errors.contains("also-secret"));
    assert!(!errors.contains("Bearer-secret"));
    assert!(errors.matches("[redacted]").count() >= 3);
    assert!(registry.resolve("status").is_some(), "dynamic source failure must preserve builtins");

    let destructive_intents = [
        UiConfirmIntent::IntegrationDelete {
            kind: IntegrationKind::Plugin,
            id: "plugin-id".to_string(),
        },
        UiConfirmIntent::ScheduleDelete { id: "cron-id".to_string() },
    ];
    assert!(matches!(
        &destructive_intents[0],
        UiConfirmIntent::IntegrationDelete { kind: IntegrationKind::Plugin, id }
            if id == "plugin-id"
    ));
    assert!(matches!(
        &destructive_intents[1],
        UiConfirmIntent::ScheduleDelete { id } if id == "cron-id"
    ));
}

#[test]
fn tui_defaults_to_full_access_and_snapshots_it_into_requests() {
    let mut state = TuiState::default();
    assert!(state.session_options.full_access_enabled());

    let submission = build_prompt_submission(&state, "default full access").expect("submission");
    assert!(submission.execution.full_access_enabled);
    let request = gateway_stream_request_from_state(&state, &submission);
    assert_eq!(request.options.as_ref().unwrap()["full_access"], true);

    state.session_options.select_acp_backend(Some("codex".to_string()));
    let acp_submission =
        build_prompt_submission(&state, "default ACP full access").expect("ACP submission");
    assert!(acp_submission.execution.full_access_enabled);
    let acp_request = gateway_stream_request_from_state(&state, &acp_submission);
    let acp_options = acp_request.options.as_ref().expect("ACP options");
    assert_eq!(acp_options["full_access"], true);
    assert_eq!(acp_options["acp_permission_mode"], "approve-all");
}

#[test]
fn full_access_overrides_plan_allowed_tool_filter() {
    let mut state = TuiState::default();
    state.session_options.permission_mode = PermissionMode::Plan;

    state.session_options.set_full_access(true);
    let full_access_submission =
        build_prompt_submission(&state, "full access plan").expect("full access submission");
    let full_access_request =
        gateway_stream_request_from_state(&state, &full_access_submission);
    assert_eq!(full_access_request.allowed_tools, None);

    state.session_options.set_full_access(false);
    let standard_access_submission =
        build_prompt_submission(&state, "standard access plan").expect("standard submission");
    let standard_access_request =
        gateway_stream_request_from_state(&state, &standard_access_submission);
    assert_eq!(
        standard_access_request.allowed_tools,
        Some(vec!["todowrite".to_string()])
    );
}

#[test]
fn full_access_requires_confirmation_and_scopes_auto_approval() {
    let mut state = TuiState::default();
    state.session_options.set_full_access(false);
    state.overlays.push(UiOverlay::Settings(SettingsPanel::for_capabilities(
        SettingsValues::default(),
        &state.session_capabilities,
    )));

    let settings = build_modal_host(&state).expect("settings render model");
    assert!(
        settings.body_lines.iter().any(|line| line.contains("完全访问")),
        "settings must expose an explicit full-access switch"
    );
    let full_access_enabled = state.session_options.full_access_enabled();
    let Some(UiOverlay::Settings(panel)) = state.overlays.stack.last_mut() else {
        panic!("settings remains active");
    };
    panel.pending.full_access_enabled = true;
    let confirmation = full_access_confirmation(panel, full_access_enabled)
        .expect("enabling full access requires confirmation");
    assert!(confirmation.destructive);
    assert!(matches!(
        confirmation.intent,
        UiConfirmIntent::EnableFullAccess { ref values } if values.full_access_enabled
    ));

    state.overlays.pop();
    state.overlays.push(UiOverlay::Confirm(confirmation));
    assert_eq!(
        TuiController::default().handle_event(
            &mut state,
            TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
                KeyCode::Esc,
                KeyModifiers::NONE,
            ))),
        ),
        TuiControllerCommand::Continue
    );
    assert!(!state.session_options.full_access_enabled(), "cancel keeps full access disabled");

    state.session_options.set_full_access(true);
    let captured = build_prompt_submission(&state, "captured full access").expect("submission");
    state.session_options.set_full_access(false);
    let captured_request = gateway_stream_request_from_state(&state, &captured);
    assert_eq!(captured_request.options.as_ref().unwrap()["full_access"], true);

    state.session_options.select_acp_backend(Some("codex".to_string()));
    state.session_options.set_full_access(true);
    assert!(state.session_options.acp_force_new_session());
    let acp_submission = build_prompt_submission(&state, "ACP full access").expect("ACP submit");
    let acp_request = gateway_stream_request_from_state(&state, &acp_submission);
    let acp_options = acp_request.options.as_ref().expect("ACP options");
    assert_eq!(acp_options["full_access"], true);
    assert_eq!(acp_options["acp_permission_mode"], "approve-all");

    state
        .session_options
        .finish_acp_session_start(&acp_submission.execution, true);
    state.session_options.set_full_access(false);
    assert!(state.session_options.acp_force_new_session());
    let safe_submission = build_prompt_submission(&state, "ACP safe").expect("ACP safe submit");
    let safe_options =
        gateway_stream_request_from_state(&state, &safe_submission).options.expect("ACP options");
    assert!(safe_options.get("full_access").is_none());
    assert_eq!(safe_options["acp_permission_mode"], "approve-reads");
    assert!(
        build_prompt_host(&state)
            .footer_pills
            .iter()
            .any(|pill| pill.label == "完全访问 关闭")
    );
}

fn is_terminal_control(character: char) -> bool {
    (character.is_control() && !matches!(character, '\n' | '\t'))
        || ('\u{80}'..='\u{9f}').contains(&character)
}
