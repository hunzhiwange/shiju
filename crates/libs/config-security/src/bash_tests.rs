//! 验证 Bash 安全检查的兼容默认值与独立开关。

// 引号内换行的阻断是选择启用的兼容策略，避免旧配置升级后突然拒绝命令。
#[test]
fn quoted_shell_newline_blocking_is_opt_in_when_deserializing() {
    let defaulted: super::BashSecurityConfig = serde_json::from_str("{}").unwrap();
    let enabled: super::BashSecurityConfig =
        serde_json::from_str(r#"{"validators":{"quoted_newline":true}}"#).unwrap();

    assert!(!defaulted.validators.quoted_newline);
    assert!(enabled.validators.quoted_newline);
}

#[test]
fn bash_security_checks_can_be_disabled_independently() {
    let bash: super::BashSecurityConfig = serde_json::from_str(
        r#"{
            "enabled": true,
            "policy": {"command_rules": false},
            "validators": {"jq": false}
        }"#,
    )
    .unwrap();

    assert!(bash.enabled);
    assert!(bash.policy.enabled);
    assert!(!bash.policy.command_rules);
    assert!(bash.policy.workspace_paths);
    assert!(bash.validators.enabled);
    assert!(!bash.validators.jq);
    assert!(bash.validators.metacharacters);
}
