use cli_interaction_model::{UiAssistantMessage, UiMessage, UiMessageBase, UiMessageId, UiTurnTerminal};
use cli_interaction_state::TuiState;
use crate::app::semantic_copy::answer_text;
use crate::commands::{CommandRoute, route_slash_submission};

fn state(text: &str, terminal: UiTurnTerminal) -> TuiState {
    let mut state = TuiState::default();
    state.session.session_id = Some("current".into());
    state.messages.push(UiMessage::Assistant(UiAssistantMessage {
        base: UiMessageBase::new(UiMessageId::new("answer")),
        text: text.into(), usage: Default::default(), step_count: 1, terminal, model: None,
    }));
    state
}
fn done(text: &str) -> TuiState {
    state(text, UiTurnTerminal::Done { finish_reason: None })
}
#[test]
fn workflow_req_02_routes_copy_and_code() {
    assert_eq!(route_slash_submission("/copy").unwrap(), Some(CommandRoute::CopyAnswer { code_block: None }));
    assert_eq!(route_slash_submission("/copy code 2").unwrap(), Some(CommandRoute::CopyAnswer { code_block: Some(2) }));
    assert!(route_slash_submission("/copy code 0").is_err());
    assert!(route_slash_submission("/copy code 1 extra").is_err());
}
#[test]
fn workflow_req_02_preserves_raw_markdown_unicode() {
    let raw = "回答🙂\n\n```rust\n    中文();\n```\n";
    assert_eq!(answer_text(&done(raw), None).unwrap(), raw);
    assert_eq!(answer_text(&done(raw), Some(1)).unwrap(), "    中文();\n");
}
#[test]
fn workflow_req_02_selects_tilde_and_indented_blocks() {
    let state = done("~~~text\n你好\n~~~\n\n    second\n        indented\n");
    assert_eq!(answer_text(&state, Some(2)).unwrap(), "second\n    indented\n");
    assert!(answer_text(&state, Some(3)).is_err());
}
#[test]
fn workflow_req_02_refuses_unfinished_or_cancelled_answer() {
    for terminal in [UiTurnTerminal::Streaming, UiTurnTerminal::Pending, UiTurnTerminal::Cancelled { reason: None }] {
        assert!(answer_text(&state("partial", terminal), None).is_err());
    }
}
#[test]
fn workflow_req_02_refuses_empty_or_foreign_session() {
    assert!(answer_text(&TuiState::default(), None).is_err());
    assert!(answer_text(&done("  "), None).is_err());
    let mut state = done("old answer");
    state.session.session_id = Some("other".into());
    if let UiMessage::Assistant(answer) = &mut state.messages[0] {
        answer.base.session_id = Some("current".into());
    }
    assert!(answer_text(&state, None).is_err());
}
