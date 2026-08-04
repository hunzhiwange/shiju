use super::super::input::{
    TuiSlashCommandOutcome, execute_slash_command, parse_slash_command, slash_command_help_text,
};
use super::super::model::{
    UiMessage, UiMessageBase, UiMessageId, UiSystemMessage, UiSystemMessageLevel,
};
use super::super::state::TuiState;
use super::inspect::{InspectCommand, execute_inspect_command, export_transcript, inspect_session};
use crate::cli::session::GitWorkspaceStatus;

#[test]
fn tui_inspect_commands_return_real_session_data_and_export() {
    let mut state = TuiState::default();
    state.session.session_id = Some("session-real".to_string());
    state.session.scope = Some("/workspace".to_string());
    state.status.provider_name = Some("openai".to_string());
    state.status.model_name = Some("gpt-5.6".to_string());
    state.project.git_status = GitWorkspaceStatus::ReadyDirty(vec!["src/main.rs".to_string()]);
    state.messages.push(UiMessage::System(UiSystemMessage {
        base: UiMessageBase::new(UiMessageId::local("status")),
        text: "真实 transcript".to_string(),
        level: UiSystemMessageLevel::Info,
    }));

    let status = inspect_session(&state, InspectCommand::Status);
    assert!(status.lines.iter().any(|line| line == "session: session-real"));
    let files = inspect_session(&state, InspectCommand::Files);
    assert_eq!(files.lines, ["src/main.rs"]);
    let cost = inspect_session(&state, InspectCommand::Cost);
    assert!(cost.capability_notice.unwrap().contains("不伪造"));
    let diff = inspect_session(&state, InspectCommand::Diff);
    assert!(diff.capability_notice.unwrap().contains("不提供"));

    let path = std::env::temp_dir().join(format!(
        "vibewindow-inspect-export-{}-{}.txt",
        std::process::id(),
        state.messages.len()
    ));
    if path.exists() {
        std::fs::remove_file(&path).unwrap();
    }
    export_transcript(&state, &path).unwrap();
    let exported = std::fs::read_to_string(&path).unwrap();
    assert_eq!(exported, inspect_session(&state, InspectCommand::Export).lines.join("\n"));
    assert!(exported.contains("SYSTEM 真实 transcript"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
    }
    std::fs::remove_file(path).unwrap();
}

#[test]
fn inspect_commands_dispatch_to_report_overlay_and_export_securely() {
    let mut state = TuiState::default();
    state.session.session_id = Some("inspect-session".to_string());
    state.messages.push(UiMessage::System(UiSystemMessage {
        base: UiMessageBase::new(UiMessageId::local("inspect")),
        text: "private transcript".to_string(),
        level: UiSystemMessageLevel::Info,
    }));

    for command in ["/status", "/context", "/usage", "/cost", "/stats", "/diff", "/files"] {
        let invocation =
            parse_slash_command(&state, command).expect("inspect command should parse");
        let TuiSlashCommandOutcome::Inspect { command: inspect_command, argument } =
            execute_slash_command(&mut state, &invocation)
        else {
            panic!("{command} should dispatch a typed inspect outcome");
        };
        execute_inspect_command(&mut state, inspect_command, argument.as_deref())
            .expect("inspect report should open");
        let Some(super::super::model::UiOverlay::Report(report)) = state.overlays.active() else {
            panic!("{command} should open a report overlay");
        };
        assert!(!report.title.is_empty());
        state.overlays.pop();
    }

    let unique = format!(
        "vibewindow-inspect-command-{}-{}.txt",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    );
    let path = std::env::temp_dir().join(unique);
    let export = format!("/export {}", path.display());
    let invocation = parse_slash_command(&state, export.as_str()).expect("export command");
    let TuiSlashCommandOutcome::Inspect { command, argument } =
        execute_slash_command(&mut state, &invocation)
    else {
        panic!("export should dispatch a typed inspect outcome");
    };
    execute_inspect_command(&mut state, command, argument.as_deref()).expect("secure export");
    assert!(std::fs::read_to_string(&path).unwrap().contains("private transcript"));
    assert!(
        execute_inspect_command(&mut state, InspectCommand::Export, Some(&path.to_string_lossy()))
            .unwrap_err()
            .contains("already exists")
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
    }
    std::fs::remove_file(path).unwrap();

    let help = slash_command_help_text(&state.command_registry, false);
    assert!(help.contains("/status"));
    assert!(help.contains("/usage"));
    assert!(!help.contains("不可用:"));
}
