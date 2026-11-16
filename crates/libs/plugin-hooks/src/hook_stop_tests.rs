use super::{Outcome, outcome, parse, validate};
use serde_json::{Value, json};

#[test]
fn codex_plugin_compat_t44_rejects_invalid_input() {
    assert!(validate(&json!({})).is_err());
    let mut input = json!({"session_id":"s","turn_id":"t","model":"m","permission_mode":"default","cwd":"relative","hook_event_name":"Stop","stop_hook_active":false});
    assert!(validate(&input).is_err());
    input["cwd"] = json!("/tmp");
    input["stop_hook_active"] = Value::Null;
    assert!(validate(&input).is_err());
    let invalid = parse(Some(0), br#"{"decision":"block"}"#, b"", true).unwrap();
    assert!(outcome(&invalid, true).contexts.is_empty());
}

#[test]
fn codex_plugin_compat_t44_stop_precedes_block_and_exit_two_continues() {
    let block = parse(Some(2), b"ignored", b" finish checks ", true).unwrap();
    assert_eq!(outcome(&block, true).contexts, ["finish checks"]);
    let stop =
        parse(Some(0), br#"{"continue":false,"decision":"block","reason":"ignored"}"#, b"", true)
            .unwrap();
    let mut combined = Outcome::default();
    combined.merge(outcome(&block, true));
    combined.merge(outcome(&stop, true));
    combined.merge(outcome(&block, true));
    assert!(combined.stop);
    assert!(combined.contexts.is_empty());
}

#[test]
fn codex_plugin_compat_t44_stop_without_block_reason_still_honors_continue_false() {
    let value =
        parse(Some(0), br#"{"continue":false,"decision":"block"}"#, b"", true)
            .unwrap();
    assert!(outcome(&value, true).stop);
}
