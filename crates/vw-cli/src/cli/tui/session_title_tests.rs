use std::path::Path;

use super::app::session_title_request;
use super::model::{UiMessage, UiMessageBase, UiMessageId, UiUserMessage};
use super::state::TuiState;

fn state(title: &str, prompts: &[&str]) -> TuiState {
    let mut state = TuiState::default();
    state.session.session_id = Some("session-one".to_string());
    state.session.title = title.to_string();
    state.session_options.model = Some("model-one".to_string());
    state.session_options.acp_backend = Some("backend-one".to_string());
    for (index, prompt) in prompts.iter().enumerate() {
        state.append_message(UiMessage::User(UiUserMessage {
            base: UiMessageBase::new(UiMessageId::local(format!("user-{index}"))),
            text: prompt.to_string(),
        }));
    }
    state
}

#[test]
fn first_message_generates_title_with_current_model_and_backend() {
    let state = state("TUI vibe-window", &["修复终端标题"]);
    let request = session_title_request(&state, Path::new("/workspace/vibe-window"))
        .expect("default TUI title should be generated");
    assert_eq!(request.content, "修复终端标题");
    assert_eq!(request.preferred_model.as_deref(), Some("model-one"));
    assert_eq!(request.acp_agent.as_deref(), Some("backend-one"));
}

#[test]
fn custom_title_and_later_turns_skip_title_generation() {
    let directory = Path::new("/workspace/vibe-window");
    for state in [
        state("手动标题", &["修复标题"]),
        state("TUI vibe-window", &["第一条", "第二条"]),
        state("TUI vibe-window", &[]),
        state("TUI vibe-window", &["  "]),
    ] {
        assert!(session_title_request(&state, directory).is_none());
    }
}

#[test]
fn ordinary_default_title_also_generates_title() {
    let state = state("New session - 2026-10-25T00:00:00.000Z", &["修复标题"]);
    assert!(session_title_request(&state, Path::new("/workspace/vibe-window")).is_some());
}
