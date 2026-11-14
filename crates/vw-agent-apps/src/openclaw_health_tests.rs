use super::openclaw_health::*;
use serde_json::json;

#[test]
fn openclaw_health_reports_without_mutation_four_warnings() {
    let source = json!({"tools":{"profile":"private-token"}, "agents":{"defaults":{"timeout":null}},
        "env":{"vars":"private-env", "shellEnv":["private-token"]}});
    let before = source.clone();
    let warnings = scan_openclaw_health_from_value(&source);
    assert_eq!(source, before);
    assert_eq!(warnings.iter().map(|w| w.code.as_str()).collect::<Vec<_>>(),
        ["invalid_tools_profile", "legacy_agents_timeout", "stringified_env_vars", "stringified_env_shell_env"]);
    assert_eq!(warnings.iter().map(|w| w.path.as_deref()).collect::<Vec<_>>(),
        [Some("tools.profile"), Some("agents.defaults.timeout"), Some("env.vars"), Some("env.shellEnv")]);
    let output = serde_json::to_string(&warnings).unwrap();
    assert!(!output.contains("private-token"));
    assert!(!output.contains("private-env"));
}

#[test]
fn openclaw_health_reports_without_mutation_missing_valid_and_malformed() {
    assert!(scan_openclaw_health_from_bytes(None).is_empty());
    for profile in ["minimal", "coding", "messaging", "full"] {
        assert!(scan_openclaw_health_from_value(&json!({"tools":{"profile":profile},
            "agents":{"defaults":{"timeoutSeconds":30}}, "env":{"vars":{}, "shellEnv":{}}})).is_empty());
    }
    // The source diagnoses unsupported strings only, not absent/non-string profiles.
    assert!(scan_openclaw_health_from_value(&json!({"tools":{"profile":42}})).is_empty());
    assert!(scan_openclaw_health_from_bytes(Some(b"{ // comment\n \"env\": {}, }" )).is_empty());
    for source in [b"".as_slice(), b"{\"token\":\"private-token\",", b"[]"] {
        let warnings = scan_openclaw_health_from_bytes(Some(source));
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].code, "config_parse_failed");
        assert!(!warnings[0].message.contains("private-token"));
    }
    for value in [json!(null), json!(false), json!(42), json!([]), json!("{}")] {
        assert_eq!(scan_openclaw_health_from_value(&json!({"env":{"vars":value,"shellEnv":value}})).len(), 2);
    }
}

#[test]
fn openclaw_health_reports_without_mutation_file_bytes() {
    let temp = tempfile::tempdir().unwrap();
    assert!(scan_openclaw_config_health(temp.path()).unwrap().is_empty());
    assert!(!temp.path().join(".openclaw").exists());
    std::fs::create_dir(temp.path().join(".openclaw")).unwrap();
    let path = temp.path().join(".openclaw/openclaw.json");
    let source = b"{\"env\":{\"vars\":\"secret\"}}";
    std::fs::write(&path, source).unwrap();
    assert_eq!(scan_openclaw_config_health(temp.path()).unwrap()[0].code, "stringified_env_vars");
    assert_eq!(std::fs::read(&path).unwrap(), source);
    std::fs::remove_file(&path).unwrap();
    std::os::unix::fs::symlink(temp.path().join("outside"), &path).unwrap();
    assert!(scan_openclaw_config_health(temp.path()).is_err());
}
