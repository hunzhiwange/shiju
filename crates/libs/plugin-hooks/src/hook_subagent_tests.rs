use super::{parse, validate};
use serde_json::json;

#[test]
fn codex_plugin_compat_t43_rejects_invalid_input() {
    assert!(validate(&json!({})).is_err());
    assert!(validate(&json!({"session_id":"same","agent_id":"same","agent_type":"reviewer","cwd":"/tmp","hook_event_name":"SubagentStart"})).is_err());
}

#[test]
fn codex_plugin_compat_t43_output_controls_follow_frozen_baseline() {
    assert!(parse(Some(0), br#"{"continue":false}"#, b"", false).is_ok());
    assert!(parse(Some(0), br#"{"continue":false}"#, b"", true).is_ok());
    assert_eq!(parse(Some(2), b"", b"continue child", true).unwrap()["reason"], "continue child");
    assert!(parse(Some(1), b"private payload", b"private error", false).unwrap().is_null());
}
