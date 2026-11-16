//! Exit protocol parsing follows the implementation without a host runtime.
#[test]
fn codex_plugin_compat_t34_rejects_invalid_input() {
    use crate::hook_command::interpret;
    assert!(interpret(None, b"").is_err());
    assert!(
        interpret(Some(2), b"sensitive output").unwrap_err().to_string().contains("exit status 2")
    );
    assert!(interpret(Some(0), br#"{"continue":false}"#).is_err());
    assert!(
        interpret(Some(0), br#"{"hookSpecificOutput":{"permissionDecision":"deny"}}"#).is_err()
    );
    assert!(interpret(Some(0), b"plain output").is_ok());
}

#[test]
fn codex_plugin_compat_t34_preserves_isolation() {
    use crate::hook_command::interpret;
    let secret = br#"{"continue":false,"stopReason":"private fixture"}"#;
    let error = interpret(Some(0), secret).unwrap_err().to_string();
    assert!(!error.contains("private fixture"));
    assert!(interpret(Some(0), b"{}").is_ok());
    // A nonblocking failure must not apply control effects from its stdout.
    assert!(interpret(Some(7), secret).is_ok());
    assert_eq!(secret, br#"{"continue":false,"stopReason":"private fixture"}"#);
}
