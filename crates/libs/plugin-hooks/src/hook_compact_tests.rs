use serde_json::json;

#[test]
fn codex_plugin_compat_t42_rejects_invalid_input() {
    use super::{parse, validate};
    assert!(validate(&json!({})).is_err());
    assert!(parse(Some(0), br#"{"continue":false,"stopReason":"fixture"}"#).is_err());
    assert!(parse(Some(2), b"private diagnostic").is_ok());
    assert!(parse(Some(0), br#"{"decision":"block"}"#).is_ok());
    assert!(parse(Some(0), br#"{"continue":false,"stopReason":42}"#).is_ok());
    assert!(parse(Some(0), br#"{"continue":false,"decision":"block"}"#).is_ok());
}
