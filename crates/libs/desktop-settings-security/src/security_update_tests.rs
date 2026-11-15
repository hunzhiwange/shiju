// 验证沙箱与一次性密码安全选项的规范化约束。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;
use crate::security_state::{SecurityListKind, SecuritySettingsTab};
use config_security::{OtpMethod, SandboxBackend, SecurityConfig};

fn app() -> SecuritySettingsState {
    SecuritySettingsState::default()
}

fn apply(app: &mut SecuritySettingsState, message: Message) {
    let _ = update(app, message);
}

fn add_security_list_value(app: &mut SecuritySettingsState, kind: SecurityListKind, value: &str) {
    apply(app, Message::SecurityListDraftChanged(kind, value.to_string()));
    apply(app, Message::SecurityListAdd(kind));
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
#[test]
fn sandbox_enabled_normalization_accepts_known_values_and_defaults_unknown_values() {
    for (input, expected) in [
        (" AUTO ", "auto"),
        ("True", "true"),
        (" FALSE ", "false"),
        ("enabled", "false"),
        ("", "false"),
    ] {
        assert_eq!(normalize_security_sandbox_enabled(input), expected);
    }
}

#[test]
fn sandbox_backend_normalization_accepts_known_values_and_defaults_unknown_values() {
    for (input, expected) in [
        (" AUTO ", "auto"),
        ("LANDLOCK", "landlock"),
        ("Firejail", "firejail"),
        (" BUBBLEWRAP ", "bubblewrap"),
        ("Docker", "docker"),
        (" NONE ", "none"),
        ("podman", "none"),
    ] {
        assert_eq!(normalize_security_sandbox_backend(input), expected);
    }
}

#[test]
fn sandbox_backend_normalization_enforces_enabled_state() {
    assert_eq!(normalize_security_sandbox_backend_for_enabled("docker", "false"), "none");
    assert_eq!(normalize_security_sandbox_backend_for_enabled("none", "true"), "auto");
    assert_eq!(normalize_security_sandbox_backend_for_enabled("docker", "true"), "docker");
    assert_eq!(normalize_security_sandbox_backend_for_enabled("none", "auto"), "none");
    assert_eq!(normalize_security_sandbox_backend_for_enabled("unknown", "auto"), "none");
}

#[test]
fn otp_method_normalization_accepts_known_values_and_defaults_unknown_values() {
    for (input, expected) in [
        (" TOTP ", "totp"),
        ("Pairing", "pairing"),
        (" CLI-PROMPT ", "cli-prompt"),
        ("sms", "totp"),
    ] {
        assert_eq!(normalize_security_otp_method(input), expected);
    }
}

#[test]
fn sandbox_messages_normalize_state() {
    let mut app = app();
    app.save_error = Some("old error".to_string());
    app.sandbox_backend_input = "docker".to_string();

    apply(&mut app, Message::SecuritySandboxEnabledChanged(" FALSE ".to_string()));
    assert_eq!(app.sandbox_enabled_input, "false");
    assert_eq!(app.sandbox_backend_input, "none");
    assert!(app.save_error.is_none());

    apply(&mut app, Message::SecuritySandboxEnabledChanged("true".to_string()));
    apply(&mut app, Message::SecuritySandboxBackendChanged("none".to_string()));
    assert_eq!(app.sandbox_backend_input, "auto");
}

#[test]
fn resource_messages_clamp_both_bounds_and_toggle_monitoring() {
    let mut app = app();

    apply(&mut app, Message::SecurityResourcesMaxMemoryMbChanged(1));
    assert_eq!(app.resources_max_memory_mb, 32);
    apply(&mut app, Message::SecurityResourcesMaxMemoryMbChanged(u32::MAX));
    assert_eq!(app.resources_max_memory_mb, 65_536);

    apply(&mut app, Message::SecurityResourcesMaxCpuTimeSecondsChanged(0));
    assert_eq!(app.resources_max_cpu_time_seconds, 1);
    apply(&mut app, Message::SecurityResourcesMaxCpuTimeSecondsChanged(u64::MAX));
    assert_eq!(app.resources_max_cpu_time_seconds, 86_400);

    apply(&mut app, Message::SecurityResourcesMaxSubprocessesChanged(0));
    assert_eq!(app.resources_max_subprocesses, 1);
    apply(&mut app, Message::SecurityResourcesMaxSubprocessesChanged(u32::MAX));
    assert_eq!(app.resources_max_subprocesses, 10_000);

    apply(&mut app, Message::SecurityResourcesMemoryMonitoringToggled(false));
    assert!(!app.resources_memory_monitoring);
}

#[test]
fn bash_security_message_updates_disabled_by_default_switch() {
    let mut app = app();
    assert!(!app.bash.enabled);

    apply(&mut app, Message::SecurityBashSecurityEnabledToggled(true));

    assert!(app.bash.enabled);
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn bash_nested_messages_only_update_target_fields_and_persistence_preserves_siblings() {
    use crate::security_message::{BashPolicyCheck, BashValidatorCheck};

    let mut app = app();
    let original_command_rules = app.bash.policy.command_rules;
    let original_heredoc = app.bash.validators.heredoc;

    apply(
        &mut app,
        Message::SecurityBashPolicyCheckToggled(BashPolicyCheck::ReadOnly, false),
    );
    apply(
        &mut app,
        Message::SecurityBashValidatorCheckToggled(BashValidatorCheck::QuotedNewline, true),
    );

    assert!(!app.bash.policy.read_only);
    assert_eq!(app.bash.policy.command_rules, original_command_rules);
    assert!(app.bash.validators.quoted_newline);
    assert_eq!(app.bash.validators.heredoc, original_heredoc);

    let mut config = SecurityConfig::default();
    config.bash.policy.redirects = false;
    apply_security_settings_to_config(&mut config, &app);
    assert!(!config.bash.policy.read_only);
    assert!(config.bash.policy.command_rules);
    assert!(config.bash.validators.quoted_newline);
    assert!(config.bash.validators.heredoc);
}

#[test]
fn audit_messages_update_values_and_clamp_size() {
    let mut app = app();

    apply(&mut app, Message::SecurityAuditEnabledToggled(true));
    apply(&mut app, Message::SecurityAuditLogPathChanged(" audit.log ".to_string()));
    apply(&mut app, Message::SecurityAuditMaxSizeMbChanged(0));
    assert_eq!(app.audit_max_size_mb, 1);
    apply(&mut app, Message::SecurityAuditMaxSizeMbChanged(u32::MAX));
    apply(&mut app, Message::SecurityAuditSignEventsToggled(true));

    assert!(app.audit_enabled);
    assert_eq!(app.audit_log_path, " audit.log ");
    assert_eq!(app.audit_max_size_mb, 10_000);
    assert!(app.audit_sign_events);
}

#[test]
fn otp_messages_normalize_method_and_clamp_ttls() {
    let mut app = app();

    apply(&mut app, Message::SecurityOtpEnabledToggled(true));
    apply(&mut app, Message::SecurityOtpMethodChanged(" PAIRING ".to_string()));
    apply(&mut app, Message::SecurityOtpTokenTtlSecsChanged(0));
    assert_eq!(app.otp_token_ttl_secs, 1);
    apply(&mut app, Message::SecurityOtpTokenTtlSecsChanged(u64::MAX));
    apply(&mut app, Message::SecurityOtpCacheValidSecsChanged(0));
    assert_eq!(app.otp_cache_valid_secs, 1);
    apply(&mut app, Message::SecurityOtpCacheValidSecsChanged(u64::MAX));
    assert!(app.otp_enabled);
    assert_eq!(app.otp_method_input, "pairing");
    assert_eq!(app.otp_token_ttl_secs, 600);
    assert_eq!(app.otp_cache_valid_secs, 86_400);
}

#[test]
fn security_lists_add_trimmed_values_remove_items_and_reject_duplicates() {
    let cases = [
        (SecurityListKind::SandboxFirejailArgs, "  --private  "),
        (SecurityListKind::OtpGatedActions, "  deploy  "),
        (SecurityListKind::OtpGatedDomains, "  admin.example  "),
        (SecurityListKind::OtpGatedDomainCategories, "  production  "),
        (SecurityListKind::SyscallAnomalyBaselineSyscalls, "  openat  "),
    ];
    assert_eq!(cases.map(|(kind, _)| kind), SecurityListKind::ALL);

    for (kind, value) in cases {
        let mut app = app();
        app.list_mut(kind).clear();

        add_security_list_value(&mut app, kind, value);
        assert_eq!(app.list(kind), &[value.trim().to_string()]);
        assert!(app.list_editor.draft(kind).is_empty());

        add_security_list_value(&mut app, kind, value);
        assert_eq!(app.list(kind).len(), 1);
        assert!(app.save_error.is_some());

        apply(&mut app, Message::SecurityListRemoveConfirmed(kind, 0));
        assert_eq!(app.list(kind).len(), 1);

        apply(&mut app, Message::SecurityListRemoveRequested(kind, 0));
        assert!(app.list_editor.is_remove_pending(kind, 0));
        assert_eq!(app.list(kind).len(), 1);
        apply(&mut app, Message::SecurityListRemoveCanceled);
        assert!(!app.list_editor.is_remove_pending(kind, 0));

        apply(&mut app, Message::SecurityListRemoveRequested(kind, 0));
        apply(&mut app, Message::SecurityListRemoveConfirmed(kind, 0));
        assert!(app.list(kind).is_empty());
    }
}

#[test]
fn security_tab_switch_does_not_clear_save_error() {
    let mut app = app();
    app.save_error = Some("keep".to_string());

    apply(&mut app, Message::SecurityTabSelected(SecuritySettingsTab::Syscall));

    assert_eq!(app.active_tab, SecuritySettingsTab::Syscall);
    assert_eq!(app.save_error.as_deref(), Some("keep"));
}

#[test]
fn estop_messages_update_state_without_normalizing_freeform_path() {
    let mut app = app();

    apply(&mut app, Message::SecurityEstopEnabledToggled(true));
    apply(
        &mut app,
        Message::SecurityEstopStateFileChanged(" state/estop.json ".to_string()),
    );
    apply(&mut app, Message::SecurityEstopRequireOtpToResumeToggled(true));

    assert!(app.estop_enabled);
    assert_eq!(app.estop_state_file, " state/estop.json ");
    assert!(app.estop_require_otp_to_resume);
}

#[test]
fn syscall_anomaly_messages_update_flags_paths_and_clamp_both_bounds() {
    let mut app = app();

    apply(&mut app, Message::SecuritySyscallAnomalyEnabledToggled(true));
    apply(&mut app, Message::SecuritySyscallAnomalyStrictModeToggled(true));
    apply(&mut app, Message::SecuritySyscallAnomalyAlertOnUnknownSyscallToggled(true));

    apply(&mut app, Message::SecuritySyscallAnomalyMaxDeniedEventsPerMinuteChanged(0));
    assert_eq!(app.syscall_anomaly_max_denied_events_per_minute, 1);
    apply(
        &mut app,
        Message::SecuritySyscallAnomalyMaxDeniedEventsPerMinuteChanged(u32::MAX),
    );
    apply(&mut app, Message::SecuritySyscallAnomalyMaxTotalEventsPerMinuteChanged(0));
    assert_eq!(app.syscall_anomaly_max_total_events_per_minute, 1);
    apply(
        &mut app,
        Message::SecuritySyscallAnomalyMaxTotalEventsPerMinuteChanged(u32::MAX),
    );
    apply(&mut app, Message::SecuritySyscallAnomalyMaxAlertsPerMinuteChanged(0));
    assert_eq!(app.syscall_anomaly_max_alerts_per_minute, 1);
    apply(&mut app, Message::SecuritySyscallAnomalyMaxAlertsPerMinuteChanged(u32::MAX));
    apply(&mut app, Message::SecuritySyscallAnomalyAlertCooldownSecsChanged(0));
    assert_eq!(app.syscall_anomaly_alert_cooldown_secs, 1);
    apply(&mut app, Message::SecuritySyscallAnomalyAlertCooldownSecsChanged(u64::MAX));

    apply(
        &mut app,
        Message::SecuritySyscallAnomalyLogPathChanged(" syscall.log ".to_string()),
    );
    assert!(app.syscall_anomaly_enabled);
    assert!(app.syscall_anomaly_strict_mode);
    assert!(app.syscall_anomaly_alert_on_unknown_syscall);
    assert_eq!(app.syscall_anomaly_max_denied_events_per_minute, 10_000);
    assert_eq!(app.syscall_anomaly_max_total_events_per_minute, 100_000);
    assert_eq!(app.syscall_anomaly_max_alerts_per_minute, 10_000);
    assert_eq!(app.syscall_anomaly_alert_cooldown_secs, 3600);
    assert_eq!(app.syscall_anomaly_log_path, " syscall.log ");
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
#[test]
fn security_guard_messages_update_flags_collection_and_clamp_threshold() {
    let mut app = app();

    apply(&mut app, Message::SecurityCanaryTokensToggled(true));
    apply(&mut app, Message::SecuritySemanticGuardToggled(true));
    apply(
        &mut app,
        Message::SecuritySemanticGuardCollectionChanged(" collection ".to_string()),
    );
    apply(&mut app, Message::SecuritySemanticGuardThresholdChanged(-1.0));
    assert_eq!(app.semantic_guard_threshold, 0.0);
    apply(&mut app, Message::SecuritySemanticGuardThresholdChanged(2.0));

    assert!(app.canary_tokens);
    assert!(app.semantic_guard);
    assert_eq!(app.semantic_guard_collection, " collection ");
    assert_eq!(app.semantic_guard_threshold, 1.0);
}

#[test]
fn security_save_normalizes_dependent_inputs_and_clears_previous_error() {
    let mut app = app();
    app.save_error = Some("old error".to_string());
    app.sandbox_enabled_input = " TRUE ".to_string();
    app.sandbox_backend_input = "none".to_string();
    app.otp_method_input = " CLI-PROMPT ".to_string();

    apply(&mut app, Message::SecuritySave);

    assert_eq!(app.sandbox_enabled_input, "true");
    assert_eq!(app.sandbox_backend_input, "auto");
    assert_eq!(app.otp_method_input, "cli-prompt");
    assert!(app.save_error.is_none());
}

#[test]
fn apply_security_settings_to_config_maps_defaults_lists_and_clamped_values() {
    let mut app = app();
    app.bash.enabled = true;
    app.sandbox_enabled_input = "false".to_string();
    app.sandbox_backend_input = "docker".to_string();
    app.sandbox_firejail_args =
        vec!["--private".to_string(), "--net=none".to_string()];
    app.resources_max_memory_mb = 1;
    app.resources_max_cpu_time_seconds = 0;
    app.resources_max_subprocesses = 0;
    app.resources_memory_monitoring = false;
    app.audit_enabled = true;
    app.audit_log_path = "   ".to_string();
    app.audit_max_size_mb = 0;
    app.audit_sign_events = true;
    app.otp_enabled = true;
    app.otp_method_input = "cli-prompt".to_string();
    app.otp_token_ttl_secs = 0;
    app.otp_cache_valid_secs = u64::MAX;
    app.otp_gated_actions =
        vec!["shell".to_string(), "write".to_string(), "read".to_string()];
    app.otp_gated_domains =
        vec!["admin.example".to_string(), "ops.example".to_string()];
    app.otp_gated_domain_categories =
        vec!["finance".to_string(), "prod".to_string()];
    app.estop_enabled = true;
    app.estop_state_file = " ".to_string();
    app.estop_require_otp_to_resume = true;
    app.syscall_anomaly_enabled = true;
    app.syscall_anomaly_strict_mode = true;
    app.syscall_anomaly_alert_on_unknown_syscall = true;
    app.syscall_anomaly_max_denied_events_per_minute = 0;
    app.syscall_anomaly_max_total_events_per_minute = u32::MAX;
    app.syscall_anomaly_max_alerts_per_minute = 0;
    app.syscall_anomaly_alert_cooldown_secs = u64::MAX;
    app.syscall_anomaly_log_path = " ".to_string();
    app.syscall_anomaly_baseline_syscalls =
        vec!["open".to_string(), "read".to_string(), "write".to_string()];
    app.canary_tokens = false;
    app.semantic_guard = true;
    app.semantic_guard_collection = " ".to_string();
    app.semantic_guard_threshold = 2.0;
    let mut config = SecurityConfig::default();

    apply_security_settings_to_config(&mut config, &app);

    assert!(config.bash.enabled);
    assert_eq!(config.sandbox.enabled, Some(false));
    assert_eq!(config.sandbox.backend, SandboxBackend::None);
    assert_eq!(config.sandbox.firejail_args, vec!["--private", "--net=none"]);
    assert_eq!(config.resources.max_memory_mb, 32);
    assert_eq!(config.resources.max_cpu_time_seconds, 1);
    assert_eq!(config.resources.max_subprocesses, 1);
    assert!(!config.resources.memory_monitoring);
    assert!(config.audit.enabled);
    assert_eq!(config.audit.log_path, "audit.log");
    assert_eq!(config.audit.max_size_mb, 1);
    assert!(config.audit.sign_events);
    assert!(config.otp.enabled);
    assert_eq!(config.otp.method, OtpMethod::CliPrompt);
    assert_eq!(config.otp.token_ttl_secs, 1);
    assert_eq!(config.otp.cache_valid_secs, 86_400);
    assert_eq!(config.otp.gated_actions, vec!["shell", "write", "read"]);
    assert_eq!(config.otp.gated_domains, vec!["admin.example", "ops.example"]);
    assert_eq!(config.otp.gated_domain_categories, vec!["finance", "prod"]);
    assert!(config.estop.enabled);
    assert_eq!(config.estop.state_file, config_runtime_paths::estop_state_file_path());
    assert!(config.estop.require_otp_to_resume);
    assert!(config.syscall_anomaly.enabled);
    assert!(config.syscall_anomaly.strict_mode);
    assert!(config.syscall_anomaly.alert_on_unknown_syscall);
    assert_eq!(config.syscall_anomaly.max_denied_events_per_minute, 1);
    assert_eq!(config.syscall_anomaly.max_total_events_per_minute, 100_000);
    assert_eq!(config.syscall_anomaly.max_alerts_per_minute, 1);
    assert_eq!(config.syscall_anomaly.alert_cooldown_secs, 3600);
    assert_eq!(config.syscall_anomaly.log_path, "syscall-anomalies.log");
    assert_eq!(config.syscall_anomaly.baseline_syscalls, vec!["open", "read", "write"]);
    assert!(!config.canary_tokens);
    assert!(config.semantic_guard);
    assert_eq!(config.semantic_guard_collection, "semantic_guard");
    assert_eq!(config.semantic_guard_threshold, 1.0);
}

#[test]
fn apply_security_settings_to_config_maps_explicit_backend_paths_and_pairing_method() {
    let mut app = app();
    app.sandbox_enabled_input = "true".to_string();
    app.sandbox_backend_input = "firejail".to_string();
    app.audit_log_path = "logs/audit.log".to_string();
    app.otp_method_input = "pairing".to_string();
    app.estop_state_file = "/tmp/estop.json".to_string();
    app.syscall_anomaly_log_path = "logs/syscall.log".to_string();
    app.semantic_guard_collection = "custom_guard".to_string();
    app.semantic_guard_threshold = 0.42;
    let mut config = SecurityConfig::default();

    apply_security_settings_to_config(&mut config, &app);

    assert_eq!(config.sandbox.enabled, Some(true));
    assert_eq!(config.sandbox.backend, SandboxBackend::Firejail);
    assert_eq!(config.audit.log_path, "logs/audit.log");
    assert_eq!(config.otp.method, OtpMethod::Pairing);
    assert_eq!(config.estop.state_file, "/tmp/estop.json");
    assert_eq!(config.syscall_anomaly.log_path, "logs/syscall.log");
    assert_eq!(config.semantic_guard_collection, "custom_guard");
    assert_eq!(config.semantic_guard_threshold, 0.42);
}

#[test]
fn help_messages_toggle_modal_without_clearing_save_error() {
    let mut app = app();
    app.save_error = Some("keep me".to_string());

    apply(&mut app, Message::SecurityHelpOpen);
    assert!(app.show_help_modal);
    assert_eq!(app.save_error.as_deref(), Some("keep me"));

    apply(&mut app, Message::SecurityHelpClose);
    assert!(!app.show_help_modal);
    assert_eq!(app.save_error.as_deref(), Some("keep me"));
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn numeric_security_messages_preserve_values_inside_allowed_ranges() {
    let mut app = app();

    apply(&mut app, Message::SecurityResourcesMaxMemoryMbChanged(2_048));
    apply(&mut app, Message::SecurityResourcesMaxCpuTimeSecondsChanged(3_600));
    apply(&mut app, Message::SecurityResourcesMaxSubprocessesChanged(64));
    apply(&mut app, Message::SecurityAuditMaxSizeMbChanged(256));
    apply(&mut app, Message::SecurityOtpTokenTtlSecsChanged(120));
    apply(&mut app, Message::SecurityOtpCacheValidSecsChanged(7_200));
    apply(&mut app, Message::SecuritySyscallAnomalyAlertCooldownSecsChanged(45));
    apply(&mut app, Message::SecuritySemanticGuardThresholdChanged(0.65));

    assert_eq!(app.resources_max_memory_mb, 2_048);
    assert_eq!(app.resources_max_cpu_time_seconds, 3_600);
    assert_eq!(app.resources_max_subprocesses, 64);
    assert_eq!(app.audit_max_size_mb, 256);
    assert_eq!(app.otp_token_ttl_secs, 120);
    assert_eq!(app.otp_cache_valid_secs, 7_200);
    assert_eq!(app.syscall_anomaly_alert_cooldown_secs, 45);
    assert_eq!(app.semantic_guard_threshold, 0.65);
}
