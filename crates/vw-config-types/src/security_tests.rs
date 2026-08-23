//! 验证自治级别、安全开关、命令检查及兼容默认值的配置边界。
//! 既锁定当前宽松默认，也确认用户显式收紧的限制不会被归一化覆盖。

use std::str::FromStr;

// 自治级别接受历史拼写，但未知值必须显式失败；身份格式保持兼容默认。
#[test]
fn parses_autonomy_levels_and_identity_defaults() {
    assert_eq!(super::AutonomyLevel::from_str("readonly").unwrap(), super::AutonomyLevel::ReadOnly);
    assert_eq!(
        super::AutonomyLevel::from_str("read_only").unwrap(),
        super::AutonomyLevel::ReadOnly
    );
    assert_eq!(
        super::AutonomyLevel::from_str("supervised").unwrap(),
        super::AutonomyLevel::Supervised
    );
    assert!(super::AutonomyLevel::from_str("invalid").is_err());

    let identity = super::IdentityConfig::default();
    assert_eq!(identity.format, "openclaw");
    assert!(identity.aieos_path.is_none());
}

// 默认预算与自动批准列表属于运行策略契约，反序列化空对象应与 Default 一致。
#[test]
fn autonomy_defaults_allow_full_access_with_expanded_budgets() {
    let defaulted = super::AutonomyConfig::default();
    let deserialized: super::AutonomyConfig = serde_json::from_str("{}").unwrap();

    for autonomy in [&defaulted, &deserialized] {
        assert_eq!(autonomy.level, super::AutonomyLevel::Full);
        assert!(!autonomy.workspace_only);
        assert!(!autonomy.require_approval_for_medium_risk);
        assert!(!autonomy.block_high_risk_commands);
        assert_eq!(autonomy.shell_redirect_policy, super::ShellRedirectPolicy::Allow);
        assert_eq!(autonomy.max_actions_per_hour, 2_000);
        assert_eq!(autonomy.max_cost_per_day_cents, 50_000);
        assert!(autonomy.auto_approve.contains(&"file_read".to_string()));
        assert!(autonomy.non_cli_excluded_tools.contains(&"bash".to_string()));
    }

    let serialized = serde_json::to_value(&defaulted).unwrap();
    assert_eq!(serialized["shell_redirect_policy"], "allow");
    let round_tripped: super::AutonomyConfig = serde_json::from_value(serialized).unwrap();
    assert_eq!(round_tripped.shell_redirect_policy, super::ShellRedirectPolicy::Allow);
}

// 所有限制项均允许用户独立收紧，不能因其它字段缺省而恢复宽松值。
#[test]
fn autonomy_explicit_restrictive_values_remain_supported() {
    let autonomy: super::AutonomyConfig = serde_json::from_str(
        r#"{
            "level": "supervised",
            "workspace_only": true,
            "require_approval_for_medium_risk": true,
            "block_high_risk_commands": true,
            "shell_redirect_policy": "block",
            "max_actions_per_hour": 20,
            "max_cost_per_day_cents": 500
        }"#,
    )
    .unwrap();

    assert_eq!(autonomy.level, super::AutonomyLevel::Supervised);
    assert!(autonomy.workspace_only);
    assert!(autonomy.require_approval_for_medium_risk);
    assert!(autonomy.block_high_risk_commands);
    assert_eq!(autonomy.shell_redirect_policy, super::ShellRedirectPolicy::Block);
    assert_eq!(autonomy.max_actions_per_hour, 20);
    assert_eq!(autonomy.max_cost_per_day_cents, 500);
}

// 引号内换行的阻断是选择启用的兼容策略，避免旧配置升级后突然拒绝命令。
#[test]
fn quoted_shell_newline_blocking_is_opt_in_when_deserializing() {
    let defaulted: super::BashSecurityConfig = serde_json::from_str("{}").unwrap();
    let enabled: super::BashSecurityConfig =
        serde_json::from_str(r#"{"validators":{"quoted_newline":true}}"#).unwrap();

    assert!(!defaulted.validators.quoted_newline);
    assert!(enabled.validators.quoted_newline);
}

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
