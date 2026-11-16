use super::decision;
use serde_json::json;
#[test]
fn codex_plugin_compat_t37_rejects_invalid_input() {
    assert!(decision(&json!({"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"allow","updatedPermissions":[]}}})).is_err());
}
#[test]
fn codex_plugin_compat_t37_preserves_isolation() {
    assert!(decision(&json!({"hookSpecificOutput":{"hookEventName":"PreToolUse","decision":{"behavior":"allow"}}})).is_err());
    assert_eq!(decision(&json!({})).unwrap(), None);
}
