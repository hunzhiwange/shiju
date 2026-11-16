use super::validate;
use serde_json::json;

#[test]
fn codex_plugin_compat_t41_rejects_invalid_metadata() {
    assert!(validate(&json!({"hook_event_name":"PreModelRequest"})).is_err());
}
