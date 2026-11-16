// Tests migrated from desktop host state; production types retain their original owners.
mod behavior_0 {
use crate::app_init::state::*;
#[test]
fn autonomy_settings_defaults_keep_agent_tool_available() {
    let autonomy = AutonomySettingsState::default();

    assert!(!autonomy.non_cli_excluded_tools.iter().any(|tool| tool == "AgentTool"));
}
}

mod behavior_1 {
use crate::app_init::state::*;
#[test]
fn autonomy_settings_from_config_preserves_unknown_saved_tools() {
    let mut config = config_security::AutonomyConfig::default();
    config.auto_approve = vec!["known_tool".to_string(), "retired_tool".to_string()];
    config.always_ask = vec!["missing_tool".to_string()];
    config.non_cli_excluded_tools = vec!["legacy_tool".to_string()];

    let autonomy = AutonomySettingsState::from_config(&config);

    assert_eq!(autonomy.auto_approve, config.auto_approve);
    assert_eq!(autonomy.always_ask, config.always_ask);
    assert_eq!(autonomy.non_cli_excluded_tools, config.non_cli_excluded_tools);
}
}

mod behavior_2 {
use crate::security_state::*;
#[test]
fn acp_security_and_autonomy_defaults_are_populated_1() {
    let security = SecuritySettingsState::default();
    assert!(!security.bash.enabled);
    assert!(security.bash.policy.enabled);
    assert!(security.bash.validators.enabled);
    assert_eq!(security.sandbox_enabled_input, "false");
    assert_eq!(security.sandbox_backend_input, "none");
    assert!(security.sandbox_firejail_args.is_empty());
    assert_eq!(security.resources_max_memory_mb, 512);
    assert_eq!(security.resources_max_cpu_time_seconds, 60);
    assert_eq!(security.resources_max_subprocesses, 10);
    assert!(security.resources_memory_monitoring);
    assert!(security.audit_enabled);
    assert_eq!(security.audit_log_path, "audit.log");
    assert_eq!(security.audit_max_size_mb, 100);
    assert!(!security.audit_sign_events);
    assert!(!security.otp_enabled);
    assert_eq!(security.otp_method_input, "totp");
    assert_eq!(security.otp_token_ttl_secs, 30);
    assert_eq!(security.otp_cache_valid_secs, 300);
    assert!(security.otp_gated_actions.iter().any(|action| action == "shell"));
    assert!(security.otp_gated_domains.is_empty());
    assert!(security.otp_gated_domain_categories.is_empty());
    assert!(!security.estop_enabled);
    assert_eq!(security.estop_state_file, config_runtime_paths::ESTOP_STATE_FILE_PATH);
    assert!(security.estop_require_otp_to_resume);
    assert!(!security.syscall_anomaly_enabled);
    assert!(!security.syscall_anomaly_strict_mode);
    assert!(!security.syscall_anomaly_alert_on_unknown_syscall);
    assert_eq!(security.syscall_anomaly_max_denied_events_per_minute, 5);
    assert_eq!(security.syscall_anomaly_max_total_events_per_minute, 120);
    assert_eq!(security.syscall_anomaly_max_alerts_per_minute, 30);
    assert_eq!(security.syscall_anomaly_alert_cooldown_secs, 20);
    assert_eq!(security.syscall_anomaly_log_path, "syscall-anomalies.log");
    assert!(security.syscall_anomaly_baseline_syscalls.is_empty());
    assert!(security.canary_tokens);
    assert!(!security.semantic_guard);
    assert_eq!(security.semantic_guard_collection, "semantic_guard");
    assert_eq!(security.semantic_guard_threshold, 0.82);
    assert!(!security.show_help_modal);
    assert!(security.save_error.is_none());


}
}

mod behavior_3 {
use crate::app_init::state::*;
#[test]
fn acp_security_and_autonomy_defaults_are_populated_2() {
    let autonomy = AutonomySettingsState::default();
    assert_eq!(autonomy.active_tab, AutonomySettingsTab::Permissions);
    assert_eq!(autonomy.level, config_security::AutonomyLevel::Full);
    assert!(!autonomy.workspace_only);
    assert!(autonomy.allowed_commands.contains(&"cargo".to_string()));
    assert!(autonomy.forbidden_paths.contains(&"/etc".to_string()));
    assert_eq!(autonomy.max_actions_per_hour, 2_000);
    assert_eq!(autonomy.max_cost_per_day_cents, 50_000);
    assert!(!autonomy.require_approval_for_medium_risk);
    assert!(!autonomy.block_high_risk_commands);
    assert_eq!(
        autonomy.shell_redirect_policy,
        config_security::ShellRedirectPolicy::Allow
    );
    assert!(autonomy.shell_env_passthrough.is_empty());
    assert!(autonomy.auto_approve.contains(&"file_read".to_string()));
    assert!(autonomy.always_ask.is_empty());
    assert!(autonomy.allowed_roots.is_empty());
    assert!(autonomy.non_cli_excluded_tools.contains(&"shell".to_string()));
    for kind in [
        AutonomyListKind::AllowedCommands,
        AutonomyListKind::ForbiddenPaths,
        AutonomyListKind::ShellEnvPassthrough,
        AutonomyListKind::AutoApprove,
        AutonomyListKind::AlwaysAsk,
        AutonomyListKind::AllowedRoots,
        AutonomyListKind::NonCliExcludedTools,
    ] {
        assert!(autonomy.list_editor.draft(kind).is_empty());
    }
    assert!(autonomy.non_cli_approval_approvers_input.is_empty());
    assert_eq!(
        autonomy.non_cli_natural_language_approval_mode,
        config_security::NonCliNaturalLanguageApprovalMode::Direct
    );
    assert!(autonomy.non_cli_natural_language_approval_mode_by_channel_input.is_empty());
    assert!(!autonomy.show_help_modal);
    assert!(autonomy.save_error.is_none());
}
}
