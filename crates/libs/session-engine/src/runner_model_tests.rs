//! 验证请求权限选项及代理配置快照的局部规则。

#[test]
fn full_access_option_enables_request_full_access() {
    let options = serde_json::json!({ "full_access": true });

    assert!(super::request_full_access_enabled(&options));
}

#[test]
fn missing_full_access_option_defaults_to_disabled() {
    let options = serde_json::json!({});

    assert!(!super::request_full_access_enabled(&options));
}

#[test]
fn false_full_access_option_does_not_enable_request_full_access() {
    let options = serde_json::json!({ "full_access": false });

    assert!(!super::request_full_access_enabled(&options));
}

#[test]
fn non_boolean_full_access_option_defaults_to_disabled() {
    let options = serde_json::json!({ "full_access": "true" });

    assert!(!super::request_full_access_enabled(&options));
}

#[test]
fn non_object_full_access_options_default_to_disabled() {
    assert!(!super::request_full_access_enabled(&serde_json::json!(true)));
    assert!(!super::request_full_access_enabled(&serde_json::Value::Null));
}

#[test]
fn enabled_identity_files_follow_selected_agent_config_snapshot() {
    let mut config = agent_config::runtime_schema::Config::default();
    config.agents.insert(
        "main".to_string(),
        agent_config::runtime_schema::DelegateAgentConfig {
            enabled_identity_files: vec!["AGENTS.md".to_string()],
            ..Default::default()
        },
    );
    config.agents.insert(
        "reviewer".to_string(),
        agent_config::runtime_schema::DelegateAgentConfig {
            enabled_identity_files: vec!["SOUL.md".to_string()],
            ..Default::default()
        },
    );

    assert_eq!(super::enabled_identity_files_for_agent(&config, None), vec!["AGENTS.md"]);
    assert_eq!(super::enabled_identity_files_for_agent(&config, Some("reviewer")), vec!["SOUL.md"]);
    assert!(super::enabled_identity_files_for_agent(&config, Some("missing")).is_empty());
}
