use super::updated_input;
use serde_json::json;

#[test]
fn codex_plugin_compat_t36_rejects_invalid_input() {
    assert!(updated_input(&json!({"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow","updatedInput":[]}})).is_err());
}

#[test]
fn codex_plugin_compat_t36_preserves_isolation() {
    let output = json!({"hookSpecificOutput":{"hookEventName":"PreToolUse","updatedInput":{"path":"changed"}}});
    assert_eq!(updated_input(&output).unwrap(), None);
    assert_eq!(updated_input(&json!({})).unwrap(), None);
    let mut allowed = output.clone();
    allowed["hookSpecificOutput"]["permissionDecision"] = json!("allow");
    assert_eq!(updated_input(&allowed).unwrap(), Some(json!({"path":"changed"})));
    assert_eq!(output["hookSpecificOutput"]["updatedInput"]["path"], "changed");
}

