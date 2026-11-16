use super::*;
use cli_interaction_model::{UiMessage, UiMessageBase, UiMessageId, UiToolResult, UiUserMessage};
use cli_interaction_state::TuiPersistedMessage;

fn state() -> TuiState {
    let mut state = TuiState::default();
    state.session.session_id = Some("current".to_string());
    state
}

fn add_user(state: &mut TuiState, id: &str, text: &str) {
    state.session.persisted_messages.push(TuiPersistedMessage {
        raw_message_id: Some(id.to_string()),
        ..Default::default()
    });
    state.messages.push(UiMessage::User(UiUserMessage {
        base: UiMessageBase::new(UiMessageId::gateway(id)).with_session_id("current"),
        text: text.to_string(),
    }));
}

fn app(state: TuiState) -> V3GatewayApp {
    use cli_gateway_transport::gateway::{GatewaySessionSeed, GatewayUiRuntime};
    use vw_gateway_client::{GatewayClient, GatewayEndpoint};
    V3GatewayApp::new(
        GatewayUiRuntime::new(
            GatewayClient::new(GatewayEndpoint::new("127.0.0.1", 1)).unwrap(),
            GatewaySessionSeed::new(std::path::PathBuf::from("/workspace")),
        ),
        state,
    )
}

#[test]
fn workflow_req_04_identical_content_retains_distinct_ids() {
    let mut state = state();
    add_user(&mut state, "one", "相同正文");
    add_user(&mut state, "two", "相同正文");
    let items = session_message_items(&state);
    assert_eq!(items[0].id, "one");
    assert_eq!(items[1].id, "two");
    assert!(items[1].label.starts_with("#2 · 用户"));
}

#[test]
fn workflow_req_04_unicode_summary_is_bounded_and_full_body_searchable() {
    let mut state = state();
    add_user(&mut state, "long", &format!("{}尾部关键词", "中文🙂\n".repeat(200)));
    let items = session_message_items(&state);
    assert!(items[0].label.ends_with('…'));
    assert!(items[0].label.chars().count() < 120);
    assert!(!items[0].label.contains('\n'));
    assert!(items[0].search_value.as_ref().unwrap().contains("尾部关键词"));
}

#[test]
fn workflow_req_04_missing_body_never_uses_row_position() {
    let mut state = state();
    add_user(&mut state, "missing", "不应该显示");
    state.messages.clear();
    state.messages.push(UiMessage::User(UiUserMessage {
        base: UiMessageBase::new(UiMessageId::gateway("other")).with_session_id("current"),
        text: "另一条消息".into(),
    }));
    let items = session_message_items(&state);
    assert!(items[0].label.contains("正文缺失"));
    assert!(!items[0].search_value.as_ref().unwrap().contains("另一条消息"));
}

#[test]
fn workflow_req_04_tool_result_uses_canonical_identity() {
    let mut state = state();
    add_user(&mut state, "tool", "placeholder");
    state.messages[0] = UiMessage::ToolResult(UiToolResult {
        base: UiMessageBase::new(UiMessageId::gateway("tool")).with_session_id("current"),
        call_id: None,
        tool_name: "shell".into(),
        command: None,
        content: "工具输出关键词".into(),
        is_error: false,
    });
    let items = session_message_items(&state);
    assert!(items[0].label.contains("工具结果"));
    assert!(items[0].search_value.as_ref().unwrap().contains("工具输出关键词"));
}

#[test]
fn workflow_req_04_old_selection_rejected_after_switch() {
    let mut state = state();
    add_user(&mut state, "old", "旧正文");
    let old_id = session_message_items(&state)[0].id.clone();
    let mut app = app(state);
    app.commit_initial_session_state(TuiState::default());
    assert!(app.fork_session(Some(&old_id)).is_err());
    assert!(app.open_reset_confirmation(&old_id).is_err());
    assert!(app.open_compact_confirmation(&old_id).is_err());
}

#[test]
fn workflow_req_04_picker_leaves_canonical_state_unchanged() {
    let mut state = state();
    add_user(&mut state, "message", "正文");
    let mut app = app(state);
    let before = app.state.clone();
    app.open_session_message_picker("session_reset", "选择回退消息").unwrap();
    assert_eq!(app.state, before);
    // Closing the widget's selection layer does not dispatch a session mutation.
}

#[test]
fn workflow_req_04_foreign_session_body_is_missing() {
    let mut state = state();
    add_user(&mut state, "same-id", "其它会话正文");
    state.session.session_id = Some("different".into());
    assert!(session_message_items(&state)[0].label.contains("正文缺失"));
}
