use super::{parse, validate};
use serde_json::json;

#[test]
fn codex_plugin_compat_t38_rejects_invalid_input() {
    assert!(validate(&json!({"hookSpecificOutput":{"hookEventName":"PreToolUse","additionalContext":"wrong turn"}})).is_err());
    assert!(
        validate(
            &json!({"hookSpecificOutput":{"hookEventName":"PostToolUse","updatedMCPToolOutput":{}}})
        )
        .is_err()
    );
}
#[test]
fn codex_plugin_compat_t38_accepts_empty_output() {
    assert!(validate(&json!({})).is_ok());
}

#[test]
fn codex_plugin_compat_t38_returns_block_and_stop_feedback() {
    for output in [
        json!({"decision":"block","reason":"blocked"}),
        json!({"continue":false,"reason":"blocked"}),
    ] {
        assert_eq!(
            parse(Some(0), &serde_json::to_vec(&output).unwrap(), b"").unwrap()["hookSpecificOutput"]
                ["additionalContext"],
            "blocked"
        );
    }
    assert!(
        parse(Some(2), b"", b"fixture stderr").unwrap()["hookSpecificOutput"]["additionalContext"]
            .as_str()
            .unwrap()
            .contains("fixture stderr")
    );
}
