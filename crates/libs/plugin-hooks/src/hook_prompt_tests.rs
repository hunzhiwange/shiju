use super::{parse, validate};
use serde_json::json;

#[test]
fn codex_plugin_compat_t40_rejects_invalid_input() {
    assert!(validate(&json!({"hook_event_name":"UserPromptSubmit","prompt":42})).is_err());
}

#[test]
fn codex_plugin_compat_t40_validates_control_and_plain_context() {
    assert!(
        parse(Some(2), b"", b"policy feedback")
            .unwrap_err()
            .to_string()
            .contains("policy feedback")
    );
    assert!(
        parse(Some(0), br#"{"continue":false,"stopReason":"stop here"}"#, b"")
            .unwrap_err()
            .to_string()
            .contains("stop here")
    );
    assert!(parse(Some(0), br#"{"decision":"block"}"#, b"").is_err());
    assert!(
        parse(Some(0), br#"{"hookSpecificOutput":{"hookEventName":"PostToolUse"}}"#, b"").is_err()
    );
    assert!(parse(Some(0), b"{invalid", b"").is_err());
    assert_eq!(
        parse(Some(0), b"plain context", b"").unwrap()["hookSpecificOutput"]["additionalContext"],
        "plain context"
    );
    assert!(parse(Some(1), b"ignored", b"").unwrap().is_null());
}
