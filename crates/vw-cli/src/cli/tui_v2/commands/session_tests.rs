use super::super::input::{TuiSlashCommandOutcome, execute_slash_command, parse_slash_command};
use super::super::model::{
    UiConfirmIntent, UiMessage, UiMessageBase, UiMessageId, UiSystemMessage, UiSystemMessageLevel,
};
use super::super::state::TuiState;
use super::session::{
    SessionCommand, SessionCommandRuntime, SessionReplacement, execute_session_command,
    session_confirmation_overlay,
};

#[derive(Default)]
struct RecordingSessions {
    calls: Vec<SessionCommand>,
    persisted: usize,
    fail: bool,
}

impl SessionCommandRuntime for RecordingSessions {
    fn persist_current(&mut self, _: &TuiState) -> Result<(), String> {
        self.persisted += 1;
        Ok(())
    }

    fn apply(
        &mut self,
        command: &SessionCommand,
        state: &TuiState,
    ) -> Result<SessionReplacement, String> {
        self.calls.push(command.clone());
        if self.fail {
            return Err("gateway 404".to_string());
        }
        Ok(SessionReplacement {
            session_id: "replacement".to_string(),
            title: "Renamed".to_string(),
            messages: vec![UiMessage::System(UiSystemMessage {
                base: UiMessageBase::new(UiMessageId::local("replacement")),
                text: "loaded atomically".to_string(),
                level: UiSystemMessageLevel::Info,
            })],
            scope: state.session.scope.clone(),
        })
    }
}

#[test]
fn tui_session_commands_apply_gateway_results_atomically() {
    let mut state = TuiState::default();
    state.session.session_id = Some("current".to_string());
    state.session.scope = Some("/workspace".to_string());
    state.status.session_title = "Current".to_string();
    state.prompt.set_value("draft survives");
    state.messages.push(UiMessage::System(UiSystemMessage {
        base: UiMessageBase::new(UiMessageId::local("old")),
        text: "old transcript".to_string(),
        level: UiSystemMessageLevel::Info,
    }));

    let before = state.clone();
    let mut runtime = RecordingSessions { fail: true, ..RecordingSessions::default() };
    let reset = SessionCommand::Reset { message_id: "m-1".to_string() };
    let confirmation = UiConfirmIntent::ResetSession { message_id: "m-1".to_string() };
    assert!(
        execute_session_command(&mut state, &reset, Some(&confirmation), &mut runtime).is_err()
    );
    assert_eq!(state, before, "gateway failure cannot clear the active transcript");
    assert_eq!(runtime.persisted, 1);

    runtime.fail = false;
    execute_session_command(&mut state, &reset, Some(&confirmation), &mut runtime).unwrap();
    assert_eq!(state.session.session_id.as_deref(), Some("replacement"));
    assert_eq!(state.status.session_title, "Renamed");
    assert_eq!(state.prompt.value, "draft survives");
    assert_eq!(state.messages.len(), 1);
    assert_eq!(state.session.scope.as_deref(), Some("/workspace"));

    let mut cancelled = state.clone();
    assert!(
        execute_session_command(&mut cancelled, &SessionCommand::Clear, None, &mut runtime,)
            .unwrap_err()
            .contains("confirmation")
    );
    assert_eq!(cancelled, state);
}

#[test]
fn destructive_session_confirmation_carries_typed_intent() {
    let mut state = TuiState::default();
    state.session.session_id = Some("current".to_string());
    let mut runtime = RecordingSessions::default();
    let reset = SessionCommand::Reset { message_id: "message-7".to_string() };
    let overlay = session_confirmation_overlay(&reset).expect("reset requires confirmation");
    assert_eq!(
        overlay.intent,
        UiConfirmIntent::ResetSession { message_id: "message-7".to_string() }
    );
    assert!(overlay.destructive);

    let wrong = UiConfirmIntent::ClearSession;
    let error = execute_session_command(&mut state, &reset, Some(&wrong), &mut runtime)
        .expect_err("a clear confirmation cannot authorize reset");
    assert!(error.contains("matching confirmation"));
    assert!(runtime.calls.is_empty());

    let right = UiConfirmIntent::ResetSession { message_id: "message-7".to_string() };
    execute_session_command(&mut state, &reset, Some(&right), &mut runtime)
        .expect("matching typed confirmation");
    assert_eq!(runtime.calls, vec![reset]);
}

#[test]
fn session_commands_use_registry_and_emit_typed_lifecycle_intents() {
    let mut state = TuiState::default();
    state.messages.push(UiMessage::System(UiSystemMessage {
        base: UiMessageBase::new(UiMessageId::gateway("message-last")),
        text: "persisted".to_string(),
        level: UiSystemMessageLevel::Info,
    }));

    let cases = [
        ("/rename New title", SessionCommand::Rename { title: "New title".to_string() }),
        ("/branch", SessionCommand::Fork { message_id: None }),
        ("/branch message-1", SessionCommand::Fork { message_id: Some("message-1".to_string()) }),
        ("/compact", SessionCommand::Compact { message_id: "message-last".to_string() }),
        (
            "/rewind gateway:message-2",
            SessionCommand::Reset { message_id: "message-2".to_string() },
        ),
        ("/clear", SessionCommand::Clear),
        ("/new", SessionCommand::Clear),
        ("/reset", SessionCommand::Clear),
    ];
    for (raw, expected) in cases {
        let invocation = parse_slash_command(&state, raw).expect("session command");
        assert_eq!(
            execute_slash_command(&mut state, &invocation),
            TuiSlashCommandOutcome::Session(expected),
            "{raw}"
        );
    }

    let continue_alias =
        parse_slash_command(&state, "/continue session-2").expect("continue alias");
    assert_eq!(
        execute_slash_command(&mut state, &continue_alias),
        TuiSlashCommandOutcome::Resume { session_id: Some("session-2".to_string()) }
    );
    let rewind = parse_slash_command(&state, "/undo").expect("rewind selector alias");
    assert_eq!(
        execute_slash_command(&mut state, &rewind),
        TuiSlashCommandOutcome::OpenRewindSelector
    );
}
