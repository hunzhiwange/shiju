use crate::{openclaw_env::*, providers::live::{patch_json::object_value, write_guard::WriteError}};
use serde_json::json;

#[test]
fn openclaw_env_preserves_nested_values() {
    let env = OpenClawEnvConfig::parse(r#"{"vars":{"TOKEN":"fixture"},"shellEnv":{"enabled":true,"timeoutMs":1200},"extra":[null,1,{"nested":false}]}"#).unwrap();
    let source = b"{\n// untouched\n\"tools\": {\"profile\":\"minimal\"},\n\"env\": {\"OLD\":\"old\"}\n}";
    let next = prepare_env_config(Some(source), &env).unwrap();
    assert!(String::from_utf8_lossy(&next).contains("// untouched\n\"tools\": {\"profile\":\"minimal\"}"));
    let read = get_env_config(Some(&next), || Ok(())).unwrap().unwrap();
    assert!(read == env);
    assert_eq!(read.as_object()["shellEnv"]["enabled"], json!(true));
    assert!(!read.as_object().contains_key("OLD"));
    assert_eq!(prepare_env_config(Some(&next), &env).unwrap(), next);
}

#[test]
fn openclaw_env_preserves_nested_values_clear_and_missing() {
    assert!(get_env_config(None, || Ok(())).unwrap().is_none());
    let empty = OpenClawEnvConfig::parse("{}").unwrap();
    let next = prepare_env_config(Some(br#"{"env":{"vars":{"OLD":"fixture"}},"other":42}"#), &empty).unwrap();
    assert_eq!(object_value(&next).unwrap(), json!({"env":{},"other":42}));
    assert!(get_env_config(Some(&next), || Ok(())).unwrap().unwrap().as_object().is_empty());
}

#[test]
fn openclaw_env_preserves_nested_values_rejects_invalid_input() {
    for raw in ["", "  ", "broken", "[]", "null", "true", "1", r#"{"secret":"fixture",}"#] {
        assert!(OpenClawEnvConfig::parse(raw).is_err());
    }
    let env = OpenClawEnvConfig::parse("{}").unwrap();
    for source in ["broken", "[]", "null", r#"{"env":null}"#, r#"{"env":[]}"#] {
        assert!(get_env_config(Some(source.as_bytes()), || Ok(())).is_err());
        assert!(prepare_env_config(Some(source.as_bytes()), &env).is_err());
    }
}

#[test]
fn openclaw_env_preserves_nested_values_requires_sensitive_authorization() {
    for pre in [None, Some(b"broken".as_slice()), Some(br#"{"env":{"TOKEN":"fixture"}}"#.as_slice())] {
        assert!(matches!(get_env_config(pre, || Err(WriteError::Conflict)), Err(WriteError::Conflict)));
    }
}
