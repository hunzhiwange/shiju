use super::*;

// 测试从配置创建策略时所有字段正确映射
#[test]
fn from_config_maps_all_fields() {
    let autonomy_config = AutonomyConfig {
        level: AutonomyLevel::Full,
        workspace_only: false,
        allowed_commands: vec!["docker".into()],
        forbidden_paths: vec!["/secret".into()],
        max_actions_per_hour: 100,
        max_cost_per_day_cents: 1000,
        require_approval_for_medium_risk: false,
        block_high_risk_commands: false,
        shell_redirect_policy: ShellRedirectPolicy::Strip,
        shell_env_passthrough: vec!["DATABASE_URL".into()],
        ..AutonomyConfig::default()
    };
    let workspace = PathBuf::from("/tmp/test-workspace");
    let policy = SecurityPolicy::from_config(&autonomy_config, &workspace);

    assert_eq!(policy.autonomy, AutonomyLevel::Full);
    assert!(!policy.workspace_only);
    assert_eq!(policy.allowed_commands, vec!["docker"]);
    assert_eq!(policy.forbidden_paths, vec!["/secret"]);
    assert_eq!(policy.max_actions_per_hour, 100);
    assert_eq!(policy.max_cost_per_day_cents, 1000);
    assert!(!policy.require_approval_for_medium_risk);
    assert!(!policy.block_high_risk_commands);
    assert!(policy.bash_security.enabled);
    assert!(policy.bash_security.policy.enabled);
    assert!(policy.bash_security.validators.enabled);
    assert!(!policy.bash_security.validators.quoted_newline);
    assert_eq!(policy.shell_redirect_policy, ShellRedirectPolicy::Strip);
    assert_eq!(policy.shell_env_passthrough, vec!["DATABASE_URL"]);
    assert_eq!(policy.workspace_dir, PathBuf::from("/tmp/test-workspace"));
}

#[test]
fn from_configs_uses_bash_security_master_switch() {
    let autonomy_config = AutonomyConfig::default();
    let workspace = PathBuf::from("/tmp/test-workspace");
    let mut security_config = vw_config_types::security::SecurityConfig::default();

    let disabled = SecurityPolicy::from_configs(&autonomy_config, &security_config, &workspace);
    assert!(!disabled.bash_security_enabled);
    assert!(
        disabled
            .validate_command_execution("for item in {one,two}; do echo \"$item\"; done", false)
            .is_ok()
    );

    security_config.bash.enabled = true;
    security_config.bash.policy.command_rules = false;
    security_config.bash.validators.jq = false;
    let enabled = SecurityPolicy::from_configs(&autonomy_config, &security_config, &workspace);
    assert!(enabled.bash_security_enabled);
    assert!(!enabled.bash_security.policy.command_rules);
    assert!(!enabled.bash_security.validators.jq);
    assert!(
        enabled
            .validate_command_execution("for item in {one,two}; do echo \"$item\"; done", false)
            .is_err()
    );
}

#[test]
fn from_default_config_maps_redirect_policy_to_allow() {
    let autonomy_config = AutonomyConfig::default();
    let policy = SecurityPolicy::from_config(
        &autonomy_config,
        std::path::Path::new("/tmp/test-workspace"),
    );

    assert_eq!(policy.shell_redirect_policy, ShellRedirectPolicy::Allow);
}

// 测试从配置创建策略时正确规范化允许的根目录
#[test]
fn from_config_normalizes_allowed_roots() {
    let autonomy_config = AutonomyConfig {
        allowed_roots: vec!["~/Desktop".into(), "shared-data".into()],
        ..AutonomyConfig::default()
    };
    let workspace = PathBuf::from("/tmp/test-workspace");
    let policy = SecurityPolicy::from_config(&autonomy_config, &workspace);

    let expected_home_root = if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home).join("Desktop")
    } else {
        PathBuf::from("~/Desktop")
    };

    assert_eq!(policy.allowed_roots[0], expected_home_root);
    assert_eq!(policy.allowed_roots[1], workspace.join("shared-data"));
}

// 测试解析路径违规消息包含允许根目录的指导信息
#[test]
fn resolved_path_violation_message_includes_allowed_roots_guidance() {
    let p = default_policy();
    let msg = p.resolved_path_violation_message(std::path::Path::new("/tmp/outside.txt"));
    assert!(msg.contains("escapes workspace"));
    assert!(msg.contains("allowed_roots"));
}

// 测试默认策略具有合理的默认值
#[test]
fn default_policy_has_sane_values() {
    let p = SecurityPolicy::default();
    assert_eq!(p.autonomy, AutonomyLevel::Supervised);
    assert!(p.workspace_only);
    assert!(!p.allowed_commands.is_empty());
    assert!(!p.forbidden_paths.is_empty());
    assert!(p.max_actions_per_hour > 0);
    assert!(p.max_cost_per_day_cents > 0);
    assert!(p.require_approval_for_medium_risk);
    assert!(p.block_high_risk_commands);
    assert!(p.bash_security.enabled);
    assert!(p.bash_security.policy.enabled);
    assert!(p.bash_security.validators.enabled);
    assert!(!p.bash_security.validators.quoted_newline);
    assert_eq!(p.shell_redirect_policy, ShellRedirectPolicy::Block);
    assert!(p.shell_env_passthrough.is_empty());
}

// 测试从配置创建策略时生成新的计数器
#[test]
fn from_config_creates_fresh_tracker() {
    let autonomy_config = AutonomyConfig {
        level: AutonomyLevel::Full,
        workspace_only: false,
        allowed_commands: vec![],
        forbidden_paths: vec![],
        max_actions_per_hour: 10,
        max_cost_per_day_cents: 100,
        require_approval_for_medium_risk: true,
        block_high_risk_commands: true,
        ..AutonomyConfig::default()
    };
    let workspace = PathBuf::from("/tmp/test");
    let policy = SecurityPolicy::from_config(&autonomy_config, &workspace);
    assert_eq!(policy.tracker.count(), 0);
    assert!(!policy.is_rate_limited());
}
