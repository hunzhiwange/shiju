use super::hook_mcp::expand;
use serde_json::json;

#[test]
fn codex_plugin_compat_t45_expands_input_and_rejects_missing_fields() {
    assert!(expand(&json!({"x":"${absent}"}), &json!({})).is_err());
    assert_eq!(
        expand(&json!({"x":"${nested.count}"}), &json!({"nested":{"count":3}})).unwrap(),
        json!({"x":3})
    );
}
