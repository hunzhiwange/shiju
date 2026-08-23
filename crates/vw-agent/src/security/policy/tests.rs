//! 验证安全策略的命令、路径、权限、速率和配置优先级。

use super::*;
use crate::tools::shell::permissions::{Permission, PermissionContext, PermissionMode};
use std::path::{Path, PathBuf};

#[test]
fn default_policy_is_supervised_and_workspace_only() {
    let policy = SecurityPolicy::default();
    assert!(matches!(policy.autonomy, AutonomyLevel::Supervised));
    assert!(policy.workspace_only);
    assert!(policy.block_high_risk_commands);
}

fn context(policy: &SecurityPolicy, approved: bool) -> PermissionContext {
    PermissionContext {
        autonomy: policy.autonomy,
        in_sandbox: false,
        mode: PermissionMode::Normal,
        approved,
        workspace_dir: policy.workspace_dir.clone(),
        allowed_roots: policy.allowed_roots.clone(),
    }
}

#[test]
fn command_validation_allows_low_risk_allowlisted_command() {
    let policy = SecurityPolicy {
        workspace_dir: PathBuf::from("/workspace"),
        allowed_commands: vec!["echo".into()],
        ..SecurityPolicy::default()
    };

    assert_eq!(
        policy.validate_command_execution("echo hello", false).unwrap(),
        CommandRiskLevel::Low
    );
}

#[test]
fn command_validation_requires_approval_for_medium_risk_when_supervised() {
    let policy = SecurityPolicy {
        workspace_dir: PathBuf::from("/workspace"),
        allowed_commands: vec!["git".into()],
        ..SecurityPolicy::default()
    };

    let err = policy.validate_command_execution("git commit -m msg", false).unwrap_err();
    assert!(err.contains("explicit approval"));
    assert_eq!(
        policy.validate_command_execution("git commit -m msg", true).unwrap(),
        CommandRiskLevel::Medium
    );
}

#[test]
fn high_risk_command_can_ask_when_not_blocked() {
    let policy = SecurityPolicy {
        block_high_risk_commands: false,
        allowed_commands: vec!["rm".into()],
        ..SecurityPolicy::default()
    };

    let result = policy.check_shell_permission("rm file", &context(&policy, false));
    assert!(matches!(result.permission, Some(Permission::Ask { .. })));
}

#[test]
fn high_risk_command_is_denied_when_blocked() {
    let policy =
        SecurityPolicy { allowed_commands: vec!["rm".into()], ..SecurityPolicy::default() };

    let result = policy.check_shell_permission("rm file", &context(&policy, true));
    assert!(
        matches!(result.permission, Some(Permission::Deny { ref reason }) if reason.contains("high-risk"))
    );
}

#[test]
fn command_allowlist_rejects_shell_expansion_redirects_and_unsafe_args() {
    let policy = SecurityPolicy::default();

    assert!(policy.is_command_allowed("FOO=bar echo ok"));
    assert!(!policy.is_command_allowed("echo $(date)"));
    assert!(!policy.is_command_allowed("echo hi > out.txt"));
    assert!(!policy.is_command_allowed("sleep 1 &"));
    assert!(!policy.is_command_allowed("git -c alias.x=!sh status"));
    assert!(!policy.is_command_allowed("find . -exec rm {} \\;"));
    assert!(!policy.is_command_allowed(""));
}

#[test]
fn unsafe_shell_patterns_relax_legacy_command_checks() {
    let policy = SecurityPolicy {
        allow_unsafe_shell_patterns: true,
        allowed_commands: vec!["git".into(), "echo".into(), "find".into()],
        ..SecurityPolicy::default()
    };

    assert!(policy.is_command_allowed("echo $(date)"));
    assert!(policy.is_command_allowed("git -c alias.x=status status"));
    assert!(policy.is_command_allowed("find . -exec echo {} \\;"));
}

#[test]
fn path_checks_reject_escape_and_allow_configured_roots() {
    let workspace = PathBuf::from("/workspace/project");
    let policy = SecurityPolicy {
        workspace_dir: workspace.clone(),
        allowed_roots: vec![PathBuf::from("/mnt/shared")],
        ..SecurityPolicy::default()
    };

    assert!(policy.is_path_allowed("src/lib.rs"));
    assert!(!policy.is_path_allowed("../outside"));
    assert!(!policy.is_path_allowed("..%2fsecret"));
    assert!(!policy.is_path_allowed("~other/.ssh/config"));
    assert!(policy.is_resolved_path_allowed(Path::new("/workspace/project/src/lib.rs")));
    assert!(policy.is_resolved_path_allowed(Path::new("/mnt/shared/input.txt")));
    assert!(!policy.is_resolved_path_allowed(Path::new("/var/log/system.log")));
    assert!(
        policy
            .resolved_path_violation_message(Path::new("/outside/file"))
            .contains("allowed_roots")
    );
}

#[test]
fn workspace_only_false_allows_non_forbidden_external_paths() {
    let policy = SecurityPolicy {
        workspace_dir: PathBuf::from("/workspace"),
        workspace_only: false,
        ..SecurityPolicy::default()
    };

    assert!(policy.is_resolved_path_allowed(Path::new("/custom/data.txt")));
    assert!(!policy.is_resolved_path_allowed(Path::new("/etc/passwd")));
}

#[test]
fn tool_operation_enforces_read_only_and_rate_limit() {
    let read_only =
        SecurityPolicy { autonomy: AutonomyLevel::ReadOnly, ..SecurityPolicy::default() };
    assert!(read_only.enforce_tool_operation(ToolOperation::Read, "inspect").is_ok());
    assert!(
        read_only
            .enforce_tool_operation(ToolOperation::Act, "write")
            .unwrap_err()
            .contains("read-only")
    );

    let policy = SecurityPolicy { max_actions_per_hour: 1, ..SecurityPolicy::default() };
    assert!(!policy.is_rate_limited());
    assert!(policy.enforce_tool_operation(ToolOperation::Act, "first").is_ok());
    assert!(policy.is_rate_limited());
    assert!(
        policy
            .enforce_tool_operation(ToolOperation::Act, "second")
            .unwrap_err()
            .contains("Rate limit")
    );
}

#[test]
fn shell_redirect_policy_strip_removes_supported_redirects_before_validation() {
    let strip = SecurityPolicy {
        shell_redirect_policy: ShellRedirectPolicy::Strip,
        allowed_commands: vec!["echo".into()],
        ..SecurityPolicy::default()
    };
    let block = SecurityPolicy {
        shell_redirect_policy: ShellRedirectPolicy::Block,
        allowed_commands: vec!["echo".into()],
        ..SecurityPolicy::default()
    };

    assert_eq!(strip.apply_shell_redirect_policy("echo ok >/dev/null 2>&1"), "echo ok");
    assert_eq!(
        block.apply_shell_redirect_policy("echo ok >/dev/null 2>&1"),
        "echo ok >/dev/null 2>&1"
    );
    assert!(strip.validate_command_execution("echo ok >/dev/null 2>&1", false).is_ok());
    assert!(block.validate_command_execution("echo ok >/dev/null 2>&1", false).is_ok());
}

#[test]
fn shell_redirect_policy_allow_preserves_command() {
    let policy = SecurityPolicy {
        shell_redirect_policy: ShellRedirectPolicy::Allow,
        ..SecurityPolicy::default()
    };
    let command = "printf '原样\\n' > output.txt 2>> errors.log";

    assert_eq!(policy.apply_shell_redirect_policy(command), command);
}

#[test]
fn from_config_resolves_relative_allowed_roots_against_workspace() {
    let config = crate::app::agent::config::AutonomyConfig {
        allowed_roots: vec!["shared".into(), "/abs".into()],
        max_actions_per_hour: 3,
        allow_unsafe_shell_patterns: true,
        ..crate::app::agent::config::AutonomyConfig::default()
    };

    let policy = SecurityPolicy::from_config(&config, Path::new("/workspace"));

    assert!(policy.allowed_roots.contains(&PathBuf::from("/workspace/shared")));
    assert!(policy.allowed_roots.contains(&PathBuf::from("/abs")));
    assert_eq!(policy.max_actions_per_hour, 3);
    assert!(policy.allow_unsafe_shell_patterns);
}

#[test]
fn normalize_path_collapses_dot_and_parent_components() {
    assert_eq!(
        normalize_path(Path::new("/workspace/./src/../Cargo.toml")),
        PathBuf::from("/workspace/Cargo.toml")
    );
    assert_eq!(normalize_path(Path::new("src/./lib.rs")), PathBuf::from("src/lib.rs"));
}

#[test]
fn shell_permission_rejects_read_only_writes_and_variable_expansion() {
    let read_only = SecurityPolicy {
        autonomy: AutonomyLevel::ReadOnly,
        allowed_commands: vec!["touch".into()],
        ..SecurityPolicy::default()
    };
    let write = read_only.check_shell_permission("touch file", &context(&read_only, true));
    assert!(matches!(write.permission, Some(Permission::Deny { .. })));

    let policy =
        SecurityPolicy { allowed_commands: vec!["echo".into()], ..SecurityPolicy::default() };
    let expansion = policy.check_shell_permission("echo $HOME", &context(&policy, true));
    assert!(matches!(expansion.permission, Some(Permission::Deny { .. })));
}

#[test]
fn quoted_shell_newline_blocking_is_configurable() {
    let command = "git commit -m \"title\n\nbody\"";
    let relaxed = SecurityPolicy {
        autonomy: AutonomyLevel::Full,
        workspace_only: false,
        allowed_commands: vec!["*".into()],
        ..SecurityPolicy::default()
    };
    let mut strict = relaxed.clone();
    strict.bash_security.validators.quoted_newline = true;

    assert!(relaxed.validate_command_execution(command, true).is_ok());
    assert!(strict.validate_command_execution(command, true).is_err());
}

#[test]
fn policy_checks_follow_master_group_and_item_precedence() {
    let mut policy = SecurityPolicy::default();
    assert!(policy.policy_check_enabled(|config| config.read_only));

    policy.bash_security.policy.read_only = false;
    assert!(!policy.policy_check_enabled(|config| config.read_only));
    assert!(policy.policy_check_enabled(|config| config.command_rules));

    policy.bash_security.policy.enabled = false;
    assert!(!policy.policy_check_enabled(|config| config.command_rules));

    policy.bash_security.policy.enabled = true;
    policy.bash_security_enabled = false;
    assert!(!policy.policy_check_enabled(|config| config.command_rules));
}

#[test]
fn policy_group_can_be_disabled_without_disabling_validators() {
    let mut policy = SecurityPolicy {
        autonomy: AutonomyLevel::Full,
        workspace_only: false,
        allowed_commands: vec!["*".into()],
        ..SecurityPolicy::default()
    };
    policy.bash_security.policy.enabled = false;

    let result = policy.check_shell_permission(
        "for item in {one,two}; do echo \"$item\"; done",
        &context(&policy, true),
    );

    assert!(matches!(result.permission, Some(Permission::Deny { .. })));
}

#[test]
fn validator_group_can_be_disabled_without_disabling_command_rules() {
    let mut policy =
        SecurityPolicy { allowed_commands: vec!["echo".into()], ..SecurityPolicy::default() };
    policy.bash_security.validators.enabled = false;

    let allowed = policy.check_shell_permission("echo safe", &context(&policy, true));
    let denied = policy.check_shell_permission("curl example.com", &context(&policy, true));

    assert!(matches!(allowed.permission, Some(Permission::Allow)));
    assert!(matches!(denied.permission, Some(Permission::Deny { .. })));
}

#[test]
fn policy_items_gate_only_their_assigned_checks() {
    let mut read_only = SecurityPolicy {
        autonomy: AutonomyLevel::ReadOnly,
        allowed_commands: vec!["touch".into()],
        ..SecurityPolicy::default()
    };
    read_only.bash_security.validators.enabled = false;
    read_only.bash_security.policy.command_rules = false;
    read_only.bash_security.policy.risk = false;
    assert!(matches!(
        read_only.check_shell_permission("touch file", &context(&read_only, true)).permission,
        Some(Permission::Deny { .. })
    ));
    read_only.bash_security.policy.read_only = false;
    assert!(matches!(
        read_only.check_shell_permission("touch file", &context(&read_only, true)).permission,
        Some(Permission::Allow)
    ));

    let mut unsafe_patterns =
        SecurityPolicy { allowed_commands: vec!["echo".into()], ..SecurityPolicy::default() };
    unsafe_patterns.bash_security.validators.enabled = false;
    unsafe_patterns.bash_security.policy.command_rules = false;
    assert!(matches!(
        unsafe_patterns
            .check_shell_permission("echo $HOME", &context(&unsafe_patterns, true))
            .permission,
        Some(Permission::Deny { .. })
    ));
    unsafe_patterns.bash_security.policy.unsafe_shell_patterns = false;
    assert!(matches!(
        unsafe_patterns
            .check_shell_permission("echo $HOME", &context(&unsafe_patterns, true))
            .permission,
        Some(Permission::Allow)
    ));

    let mut command_rules = SecurityPolicy::default();
    command_rules.bash_security.validators.enabled = false;
    assert!(matches!(
        command_rules
            .check_shell_permission("curl example.com", &context(&command_rules, true))
            .permission,
        Some(Permission::Deny { .. })
    ));
    command_rules.bash_security.policy.command_rules = false;
    assert!(matches!(
        command_rules
            .check_shell_permission("curl example.com", &context(&command_rules, true))
            .permission,
        Some(Permission::Allow)
    ));

    let mut workspace_paths = SecurityPolicy {
        workspace_dir: PathBuf::from("/workspace"),
        allowed_commands: vec!["cat".into()],
        ..SecurityPolicy::default()
    };
    workspace_paths.bash_security.validators.enabled = false;
    assert!(matches!(
        workspace_paths
            .check_shell_permission("cat /outside/file", &context(&workspace_paths, true))
            .permission,
        Some(Permission::Deny { .. })
    ));
    workspace_paths.bash_security.policy.workspace_paths = false;
    assert!(matches!(
        workspace_paths
            .check_shell_permission("cat /outside/file", &context(&workspace_paths, true))
            .permission,
        Some(Permission::Allow)
    ));

    let mut risk =
        SecurityPolicy { allowed_commands: vec!["rm".into()], ..SecurityPolicy::default() };
    risk.bash_security.validators.enabled = false;
    assert!(matches!(
        risk.check_shell_permission("rm file", &context(&risk, true)).permission,
        Some(Permission::Deny { .. })
    ));
    risk.bash_security.policy.risk = false;
    assert!(matches!(
        risk.check_shell_permission("rm file", &context(&risk, true)).permission,
        Some(Permission::Allow)
    ));
}

#[test]
fn shell_permission_reports_paths_outside_the_workspace() {
    let policy = SecurityPolicy {
        workspace_dir: PathBuf::from("/workspace"),
        allowed_commands: vec!["cat".into()],
        ..SecurityPolicy::default()
    };

    let result = policy.check_shell_permission("cat /outside/file", &context(&policy, true));

    assert!(
        matches!(result.permission, Some(Permission::Deny { ref reason }) if reason.contains("Path blocked"))
    );
}

#[test]
fn path_violation_guidance_changes_when_allowed_roots_exist() {
    let policy = SecurityPolicy {
        allowed_roots: vec![PathBuf::from("/shared")],
        ..SecurityPolicy::default()
    };

    let message = policy.resolved_path_violation_message(Path::new("/outside/file"));

    assert!(message.contains("matching parent directory"));
    assert!(!message.contains("for example"));
}
