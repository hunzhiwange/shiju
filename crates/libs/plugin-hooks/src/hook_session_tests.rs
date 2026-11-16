use super::validate;
use serde_json::json;

#[test]
fn codex_plugin_compat_t39_rejects_invalid_input() {
    assert!(
        validate(
            &json!({"hook_event_name":"SessionStart","session_id":"","cwd":"/","source":"startup"})
        )
        .is_err()
    );
    assert!(validate(&json!({"hook_event_name":"SessionStart","session_id":"s","cwd":"relative","source":"resume"})).is_err());
}
