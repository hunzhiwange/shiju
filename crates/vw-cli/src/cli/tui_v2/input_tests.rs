//! 验证 TUI v2 输入模型的编辑行为。
//! 用例覆盖光标移动、文本修改和提交边界，保证终端输入可预测。

use super::commands::registry::{
    CommandAction, CommandAvailability, CommandExecutionPolicy, CommandRegistry, CommandSource,
    CommandSpec,
};
use super::input::{
    TuiPromptSuggestionMotion, TuiSlashCommandInvocation, TuiSlashCommandKind,
    TuiSlashCommandOutcome, apply_first_suggestion, execute_slash_command,
    move_prompt_suggestion_selection, parse_slash_command as parse_slash_command_with_state,
    prompt_suggestions, selected_prompt_suggestion, selected_suggestion_index,
};
use super::completion::active_completion_token;
use super::state::{
    TuiAction, TuiModelCatalogEntry, TuiSessionPreview, TuiState, reduce_tui_state,
};

fn parse_slash_command(input: &str) -> Option<TuiSlashCommandInvocation> {
    parse_slash_command_with_state(&TuiState::default(), input)
}

#[test]
fn parse_slash_command_maps_aliases_and_arguments() {
    let invocation = parse_slash_command("/quit").expect("slash command should parse");
    assert_eq!(invocation.kind, Some(TuiSlashCommandKind::Exit));
    assert_eq!(invocation.token, "quit");

    let model = parse_slash_command("/model gpt-5.4").expect("model slash command should parse");
    assert_eq!(model.kind, Some(TuiSlashCommandKind::Model));
    assert_eq!(model.argument.as_deref(), Some("gpt-5.4"));
}

#[test]
fn parse_slash_command_ignores_non_slash_and_normalizes_empty_arguments() {
    assert!(parse_slash_command("hello").is_none());

    let blank = parse_slash_command("  /   ").expect("bare slash should parse");
    assert_eq!(blank.raw, "/");
    assert_eq!(blank.token, "");
    assert_eq!(blank.argument, None);
    assert_eq!(blank.kind, None);

    let upper = parse_slash_command("/MODEL   gpt-5.4   ").expect("case-insensitive command");
    assert_eq!(upper.kind, Some(TuiSlashCommandKind::Model));
    assert_eq!(upper.argument.as_deref(), Some("gpt-5.4"));
}

#[test]
fn prompt_suggestions_cover_command_prefix_and_model_argument() {
    let mut state = TuiState::default();
    reduce_tui_state(&mut state, TuiAction::StatusModelSet(Some("gpt-5.4".to_string())));
    reduce_tui_state(&mut state, TuiAction::PromptValueSet("/cl".to_string()));

    let command_suggestions = prompt_suggestions(&state);
    assert!(command_suggestions.iter().any(|item| item.label == "/clear"));

    reduce_tui_state(
        &mut state,
        TuiAction::ModelCatalogReplaced(vec![TuiModelCatalogEntry {
            provider_id: "openai".to_string(),
            provider_name: "OpenAI".to_string(),
            model_id: "gpt-5.4".to_string(),
            model_name: "GPT-5.4".to_string(),
        }]),
    );
    reduce_tui_state(&mut state, TuiAction::PromptValueSet("/model ".to_string()));
    let model_suggestions = prompt_suggestions(&state);
    assert_eq!(
        model_suggestions.first().map(|item| item.replacement.as_str()),
        Some("/model openai/gpt-5.4")
    );

    reduce_tui_state(&mut state, TuiAction::PromptValueSet("/model open".to_string()));
    let filtered_suggestions = prompt_suggestions(&state);
    assert_eq!(
        filtered_suggestions.first().map(|item| item.replacement.as_str()),
        Some("/model openai/gpt-5.4")
    );
}

#[test]
fn trailing_slash_token_suggestions_keep_message_prefix_and_close_on_space() {
    let mut state = TuiState::default();
    let value = "请检查 /cl";
    let (start, token) = active_completion_token(value).expect("active trailing token");
    assert_eq!(token, "/cl");
    assert_eq!(&value[..start], "请检查 ");

    reduce_tui_state(&mut state, TuiAction::PromptValueSet(value.to_string()));
    assert!(
        prompt_suggestions(&state)
            .iter()
            .any(|item| item.replacement == "请检查 /clear")
    );
    assert_eq!(apply_first_suggestion(&state).as_deref(), Some("请检查 /clear"));

    reduce_tui_state(&mut state, TuiAction::PromptValueSet("请检查 /cl ".to_string()));
    assert!(prompt_suggestions(&state).is_empty());
}

#[test]
fn prompt_suggestion_selection_clamps_wraps_and_applies_replacement() {
    let mut state = TuiState::default();
    reduce_tui_state(&mut state, TuiAction::PromptValueSet("/".to_string()));
    state.prompt.set_selected_suggestion_index(Some(99));

    let suggestions = prompt_suggestions(&state);
    let selected = selected_prompt_suggestion(&state).expect("last command should be selected");
    assert_eq!(Some(&selected), suggestions.last());
    assert_eq!(move_prompt_suggestion_selection(&state, TuiPromptSuggestionMotion::Next), Some(0));
    assert_eq!(
        move_prompt_suggestion_selection(&state, TuiPromptSuggestionMotion::Previous),
        Some(suggestions.len().saturating_sub(2))
    );
    assert_eq!(
        apply_first_suggestion(&state).as_deref(),
        suggestions.last().map(|item| item.replacement.as_str())
    );

    reduce_tui_state(&mut state, TuiAction::PromptValueSet(selected.replacement));
    state.prompt.set_selected_suggestion_index(Some(0));
    assert_eq!(apply_first_suggestion(&state), None);
}

#[test]
fn prompt_suggestions_cover_resume_preview_without_model_free_text() {
    let mut state = TuiState::default();
    state.session.preview = Some(TuiSessionPreview {
        id: "session-1".to_string(),
        title: "Previous work".to_string(),
        updated_ms: 42,
        message_count: 3,
        call_count: 1,
        last_content: Some("done".to_string()),
    });
    reduce_tui_state(&mut state, TuiAction::PromptValueSet("/resume ".to_string()));
    let resume = prompt_suggestions(&state);
    assert_eq!(resume.len(), 1);
    assert_eq!(resume[0].replacement, "/resume session-1");

    reduce_tui_state(&mut state, TuiAction::PromptValueSet("/resume session-1".to_string()));
    assert!(prompt_suggestions(&state).is_empty());

    reduce_tui_state(&mut state, TuiAction::StatusModelSet(Some("custom-fast".to_string())));
    reduce_tui_state(&mut state, TuiAction::PromptValueSet("/model fast".to_string()));
    assert!(prompt_suggestions(&state).is_empty());
}

#[test]
fn execute_slash_command_clear_emits_typed_session_intent_and_preserves_context() {
    let mut state = TuiState::default();
    state.session.session_id = Some("session_clear".to_string());
    reduce_tui_state(&mut state, TuiAction::StatusModelSet(Some("gpt-5.4".to_string())));
    reduce_tui_state(
        &mut state,
        TuiAction::MessagePushed(super::model::UiMessage::System(super::model::UiSystemMessage {
            base: super::model::UiMessageBase::new(super::model::UiMessageId::local("sys-1")),
            text: "stale".to_string(),
            level: super::model::UiSystemMessageLevel::Info,
        })),
    );

    let outcome = execute_slash_command(
        &mut state,
        &parse_slash_command("/clear").expect("clear command should parse"),
    );

    assert_eq!(
        outcome,
        TuiSlashCommandOutcome::Session(super::commands::session::SessionCommand::Clear)
    );
    assert_eq!(state.session.session_id.as_deref(), Some("session_clear"));
    assert_eq!(state.status.model_name.as_deref(), Some("gpt-5.4"));
    assert_eq!(state.messages.len(), 1);
    assert!(state.overlays.active().is_none());
    assert!(state.session.persisted_messages.is_empty());
}

#[test]
fn execute_slash_command_exit_opens_confirm_overlay() {
    let mut state = TuiState::default();

    let outcome = execute_slash_command(
        &mut state,
        &parse_slash_command("/exit").expect("exit command should parse"),
    );

    assert_eq!(outcome, TuiSlashCommandOutcome::Continue);
    let Some(super::model::UiOverlay::Confirm(overlay)) = state.overlays.active() else {
        panic!("exit should open a confirm overlay");
    };
    assert_eq!(overlay.confirm_label, "退出");
}

#[test]
fn execute_slash_command_model_and_unknown_emit_system_feedback() {
    let mut state = TuiState::default();
    reduce_tui_state(
        &mut state,
        TuiAction::ModelCatalogReplaced(vec![TuiModelCatalogEntry {
            provider_id: "openai".to_string(),
            provider_name: "OpenAI".to_string(),
            model_id: "gpt-5.4".to_string(),
            model_name: "GPT-5.4".to_string(),
        }]),
    );

    let model_outcome = execute_slash_command(
        &mut state,
        &parse_slash_command("/model openai/gpt-5.4").expect("model command should parse"),
    );
    assert_eq!(model_outcome, TuiSlashCommandOutcome::Continue);
    assert_eq!(state.status.model_name, None, "runtime commit owns model mutation");
    let super::model::UiMessage::System(model_message) =
        state.messages.last().expect("model argument should emit warning")
    else {
        panic!("model argument should emit system feedback");
    };
    assert!(model_message.text.contains("does not accept arguments"));

    let unknown_outcome = execute_slash_command(
        &mut state,
        &parse_slash_command("/does-not-exist").expect("unknown slash command should still parse"),
    );
    assert_eq!(unknown_outcome, TuiSlashCommandOutcome::Continue);
    let super::model::UiMessage::System(message) =
        state.messages.last().expect("unknown command should emit warning")
    else {
        panic!("unknown slash command should emit system feedback");
    };
    assert!(message.text.contains("未知的斜杠命令"));
}

#[test]
fn execute_slash_command_help_and_model_hint_emit_info_messages() {
    let mut state = TuiState::default();
    reduce_tui_state(&mut state, TuiAction::StatusModelSet(Some("gpt-5.4".to_string())));

    let help = execute_slash_command(
        &mut state,
        &parse_slash_command("/help").expect("help command should parse"),
    );
    assert_eq!(help, TuiSlashCommandOutcome::Continue);
    let super::model::UiMessage::System(help_message) =
        state.messages.last().expect("help should append a system message")
    else {
        panic!("help should emit system feedback");
    };
    assert!(help_message.text.contains("/resume"));
    assert_eq!(help_message.level, super::model::UiSystemMessageLevel::Info);

    let model_hint = execute_slash_command(
        &mut state,
        &parse_slash_command("/model").expect("model command should parse"),
    );
    assert_eq!(
        model_hint,
        TuiSlashCommandOutcome::Settings(
            super::commands::settings::SettingsCommand::OpenModelPicker
        )
    );
}

#[test]
fn execute_slash_command_acp_opens_picker_and_rejects_free_text() {
    let mut state = TuiState::default();

    assert_eq!(
        execute_slash_command(
            &mut state,
            &parse_slash_command("/acp").expect("ACP command should parse")
        ),
        TuiSlashCommandOutcome::Settings(
            super::commands::settings::SettingsCommand::OpenAcpBackendPicker
        )
    );
    assert_eq!(
        execute_slash_command(
            &mut state,
            &parse_slash_command("/acp codex").expect("ACP command should parse")
        ),
        TuiSlashCommandOutcome::Continue
    );
    assert_eq!(
        execute_slash_command(
            &mut state,
            &parse_slash_command("/acp default").expect("ACP command should parse")
        ),
        TuiSlashCommandOutcome::Continue
    );
    assert_eq!(state.messages.len(), 2);
    assert!(state.messages.iter().all(|message| {
        matches!(message, super::model::UiMessage::System(message) if message.text.contains("does not accept arguments"))
    }));
}

#[test]
fn execute_slash_command_resume_returns_restore_intent() {
    let mut state = TuiState::default();

    let latest = execute_slash_command(
        &mut state,
        &parse_slash_command("/resume").expect("resume command should parse"),
    );
    assert_eq!(latest, TuiSlashCommandOutcome::Resume { session_id: None });

    let explicit = execute_slash_command(
        &mut state,
        &parse_slash_command("/resume session_123").expect("resume command should parse"),
    );
    assert_eq!(
        explicit,
        TuiSlashCommandOutcome::Resume { session_id: Some("session_123".to_string()) }
    );
}

#[test]
fn parse_slash_command_parses_all_command_kinds() {
    let help = parse_slash_command("/help").expect("help");
    assert_eq!(help.kind, Some(TuiSlashCommandKind::Help));
    assert_eq!(help.raw, "/help");
    assert_eq!(help.argument, None);

    let clear = parse_slash_command("/clear").expect("clear");
    assert_eq!(clear.kind, Some(TuiSlashCommandKind::Clear));

    let new_alias = parse_slash_command("/new").expect("new alias");
    assert_eq!(new_alias.kind, Some(TuiSlashCommandKind::Clear));

    let exit = parse_slash_command("/exit").expect("exit");
    assert_eq!(exit.kind, Some(TuiSlashCommandKind::Exit));

    let resume_with_arg = parse_slash_command("/resume s1").expect("resume with arg");
    assert_eq!(resume_with_arg.kind, Some(TuiSlashCommandKind::Resume));
    assert_eq!(resume_with_arg.argument.as_deref(), Some("s1"));

    let acp = parse_slash_command("/acp codex").expect("ACP with backend");
    assert_eq!(acp.kind, Some(TuiSlashCommandKind::Acp));
    assert_eq!(acp.argument.as_deref(), Some("codex"));
}

#[test]
fn parse_slash_command_treats_whitespace_only_argument_as_none() {
    let cmd = parse_slash_command("/resume   ").expect("resume with trailing whitespace");
    assert_eq!(cmd.argument, None, "whitespace-only argument should normalize to None");

    let cmd = parse_slash_command("/model   gpt-5.4   ").expect("model with extra spacing");
    assert_eq!(cmd.argument.as_deref(), Some("gpt-5.4"), "argument should be trimmed");
}

#[test]
fn selected_suggestion_index_returns_none_for_empty_suggestions() {
    let state = TuiState::default();
    assert!(selected_suggestion_index(&state, &[]).is_none());
}

#[test]
fn selection_functions_gracefully_handle_non_slash_input() {
    let state = TuiState::default();

    assert!(selected_prompt_suggestion(&state).is_none(), "no suggestions when prompt is empty");
    assert!(
        apply_first_suggestion(&state).is_none(),
        "no suggestion to apply when prompt is empty"
    );
    assert!(
        move_prompt_suggestion_selection(&state, TuiPromptSuggestionMotion::Next).is_none(),
        "no movement when no suggestions"
    );
    assert!(
        move_prompt_suggestion_selection(&state, TuiPromptSuggestionMotion::Previous).is_none(),
        "no movement when no suggestions"
    );
}

#[test]
fn merged_registry_drives_parse_completion_and_dispatch_with_availability() {
    let mut state = TuiState::default();
    state.command_registry = CommandRegistry::merge(vec![
        (CommandSource::Builtin, Ok(CommandRegistry::builtin().commands().to_vec())),
        (
            CommandSource::Workflow,
            Ok(vec![
                CommandSpec {
                    name: "deploy".to_string(),
                    aliases: vec!["ship".to_string()],
                    summary: "deploy the selected workflow".to_string(),
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
                    summary: "disabled command".to_string(),
                    takes_argument: false,
                    argument_hint: None,
                    source: CommandSource::Workflow,
                    availability: CommandAvailability::Unavailable {
                        reason: "workflow service is offline".to_string(),
                    },
                    execution_policy: CommandExecutionPolicy::Immediate,
                    action: CommandAction::Gateway("workflow.disabled".to_string()),
                },
            ]),
        ),
    ]);

    let parsed =
        parse_slash_command_with_state(&state, "/ship production").expect("dynamic command");
    assert_eq!(parsed.action, Some(CommandAction::Gateway("workflow.deploy".to_string())));
    assert_eq!(parsed.execution_policy, Some(CommandExecutionPolicy::Queued));

    reduce_tui_state(&mut state, TuiAction::PromptValueSet("/d".to_string()));
    let suggestions = prompt_suggestions(&state);
    assert!(suggestions.iter().any(|item| {
        item.label == "/deploy"
            && item.detail.as_deref().is_some_and(|detail| {
                detail.contains("<environment>") && detail.contains("Workflow")
            })
    }));
    assert!(suggestions.iter().any(|item| {
        item.label == "/disabled"
            && item
                .detail
                .as_deref()
                .is_some_and(|detail| detail.contains("workflow service is offline"))
    }));

    let unavailable = state.command_registry.dispatch("disabled", None).unwrap_err();
    assert!(unavailable.contains("workflow service is offline"));
}

#[test]
fn execute_slash_command_model_hint_differs_by_catalog_state() {
    let mut empty_state = TuiState::default();
    assert_eq!(
        execute_slash_command(&mut empty_state, &parse_slash_command("/model").expect("model")),
        TuiSlashCommandOutcome::Settings(
            super::commands::settings::SettingsCommand::OpenModelPicker
        )
    );

    let mut catalog_state = TuiState::default();
    reduce_tui_state(
        &mut catalog_state,
        TuiAction::ModelCatalogReplaced(vec![TuiModelCatalogEntry {
            provider_id: "openai".to_string(),
            provider_name: "OpenAI".to_string(),
            model_id: "gpt-5.4".to_string(),
            model_name: "GPT-5.4".to_string(),
        }]),
    );
    assert_eq!(
        execute_slash_command(&mut catalog_state, &parse_slash_command("/model").expect("model")),
        TuiSlashCommandOutcome::Settings(
            super::commands::settings::SettingsCommand::OpenModelPicker
        )
    );
}

#[test]
fn prompt_suggestions_returns_empty_for_non_slash_input() {
    let state = TuiState::default();
    assert!(prompt_suggestions(&state).is_empty(), "empty prompt has no suggestions");

    let mut state = TuiState::default();
    reduce_tui_state(&mut state, TuiAction::PromptValueSet("plain text".to_string()));
    assert!(prompt_suggestions(&state).is_empty(), "plain text has no slash suggestions");
}

#[test]
fn model_suggestions_omit_an_exact_qualified_catalog_match() {
    let mut state = TuiState::default();
    reduce_tui_state(
        &mut state,
        TuiAction::ModelCatalogReplaced(vec![TuiModelCatalogEntry {
            provider_id: "openai".to_string(),
            provider_name: "OpenAI".to_string(),
            model_id: "gpt-5.4".to_string(),
            model_name: "GPT-5.4".to_string(),
        }]),
    );
    reduce_tui_state(&mut state, TuiAction::PromptValueSet("/model openai/gpt-5.4".to_string()));

    assert!(prompt_suggestions(&state).is_empty());
}
