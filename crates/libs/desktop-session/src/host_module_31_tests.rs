use crate::session::{state::State, view_cache::ViewSnapshot};
struct Snapshot;
impl ViewSnapshot for Snapshot {
    fn content_signature(&self) -> u64 {
        0
    }
    fn estimated_bytes(&self) -> usize {
        0
    }
}
#[test]
fn late_preferences_preserve_local_access_and_update_history_in_existing_runtimes() {
    let mut state = State::<(), Snapshot>::new(&serde_json::json!({}), None, vec![]);
    state.input_text = "local draft".into();
    state.session_reasoning_efforts.insert("__empty__".into(), None);
    let runtime = state.session_runtime_states.get_mut("__empty__").unwrap();
    runtime.full_access_overridden = true;
    runtime.full_access_enabled = false;
    state.restore_access_preferences(&serde_json::json!({"last_llm_full_access": true}));
    assert_eq!(state.input_text, "local draft");
    assert!(!state.session_runtime_states["__empty__"].full_access_enabled);
    assert_eq!(state.session_reasoning_efforts.get("__empty__"), Some(&None));
    state.restore_history_preferences(&serde_json::json!({
        "acp_history_strategy": "recent", "acp_history_recent_count": 90, "chat_send_behavior": "guide"
    }));
    assert_eq!(state.acp_recent_count, 20);
    assert_eq!(state.session_runtime_states["__empty__"].acp_recent_count, 20);
    assert_eq!(state.chat_send_behavior, crate::message::chat::request::ChatSendBehavior::Guide);
    state.active_session_id = Some("active".into());
    state.acp_agent = Some("active-agent".into());
    state.apply_project_defaults("draft-model".into(), false, Some("draft-agent".into()));
    assert_eq!(state.acp_agent.as_deref(), Some("active-agent"));
    assert_eq!(state.model, "draft-model");
    assert_eq!(state.input_text, "local draft");
}
