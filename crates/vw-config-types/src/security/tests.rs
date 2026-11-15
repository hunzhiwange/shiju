//! 验证顶层安全配置与各安全子系统的组合默认值。

// 沙箱和系统调用告警默认关闭；显式开启 Bash 安全后各子检查仍可单独控制。
#[test]
fn security_defaults_disable_sandbox_and_syscall_alerting() {
    let security = super::SecurityConfig::default();

    assert!(!security.bash.enabled);
    assert_eq!(security.sandbox.enabled, Some(false));
    assert_eq!(security.sandbox.backend, super::SandboxBackend::None);
    assert!(!security.syscall_anomaly.enabled);
    assert!(!security.syscall_anomaly.alert_on_unknown_syscall);
}

#[test]
fn missing_security_fields_use_disabled_defaults() {
    let security: super::SecurityConfig = serde_json::from_str("{}").unwrap();
    let sandbox: super::SandboxConfig = serde_json::from_str("{}").unwrap();
    let syscall_anomaly: super::SyscallAnomalyConfig = serde_json::from_str("{}").unwrap();

    assert!(!security.bash.enabled);
    assert_eq!(sandbox.enabled, Some(false));
    assert_eq!(sandbox.backend, super::SandboxBackend::None);
    assert!(!syscall_anomaly.enabled);
    assert!(!syscall_anomaly.alert_on_unknown_syscall);
}

#[test]
fn bash_security_can_be_enabled_explicitly() {
    let security: super::SecurityConfig =
        serde_json::from_str(r#"{"bash":{"enabled":true}}"#).unwrap();

    assert!(security.bash.enabled);
    assert!(security.bash.policy.enabled);
    assert!(security.bash.policy.read_only);
    assert!(security.bash.policy.unsafe_shell_patterns);
    assert!(security.bash.policy.command_rules);
    assert!(security.bash.policy.workspace_paths);
    assert!(security.bash.policy.forbidden_paths);
    assert!(security.bash.policy.risk);
    assert!(security.bash.policy.redirects);
    assert!(security.bash.validators.enabled);
    assert!(security.bash.validators.empty_command);
    assert!(security.bash.validators.heredoc);
    assert!(security.bash.validators.git_commit);
    assert!(security.bash.validators.jq);
    assert!(security.bash.validators.metacharacters);
    assert!(security.bash.validators.dangerous_vars);
    assert!(security.bash.validators.substitution);
    assert!(security.bash.validators.redirection);
    assert!(security.bash.validators.newline_injection);
    assert!(security.bash.validators.ifs_injection);
    assert!(security.bash.validators.proc_environ);
    assert!(security.bash.validators.malformed);
    assert!(security.bash.validators.obfuscated_flags);
    assert!(security.bash.validators.backslash);
    assert!(security.bash.validators.brace_expansion);
    assert!(security.bash.validators.unicode_whitespace);
    assert!(security.bash.validators.hash_comment);
    assert!(security.bash.validators.comment_quote_desync);
    assert!(!security.bash.validators.quoted_newline);
    assert!(security.bash.validators.zsh_dangerous);
    assert!(security.bash.validators.control_chars);
}
