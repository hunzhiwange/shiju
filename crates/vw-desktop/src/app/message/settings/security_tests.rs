use super::*;
use crate::app::App;
use crate::app::state::{SecurityListKind, SecuritySettingsTab};
use vw_config_types::security::{OtpMethod, SandboxBackend, SecurityConfig};

fn app() -> App {
    App::new().0
}

fn apply(app: &mut App, message: SettingsMessage) {
    drop(update(app, message));
}

fn add_security_list_value(app: &mut App, kind: SecurityListKind, value: &str) {
    apply(app, SettingsMessage::SecurityListDraftChanged(kind, value.to_string()));
    apply(app, SettingsMessage::SecurityListAdd(kind));
}

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
    app.security_settings.save_error = Some("old error".to_string());
    app.security_settings.sandbox_backend_input = "docker".to_string();

    apply(&mut app, SettingsMessage::SecuritySandboxEnabledChanged(" FALSE ".to_string()));
    assert_eq!(app.security_settings.sandbox_enabled_input, "false");
    assert_eq!(app.security_settings.sandbox_backend_input, "none");
    assert!(app.security_settings.save_error.is_none());

    apply(&mut app, SettingsMessage::SecuritySandboxEnabledChanged("true".to_string()));
    apply(&mut app, SettingsMessage::SecuritySandboxBackendChanged("none".to_string()));
    assert_eq!(app.security_settings.sandbox_backend_input, "auto");
}

#[test]
fn resource_messages_clamp_both_bounds_and_toggle_monitoring() {
    let mut app = app();

    apply(&mut app, SettingsMessage::SecurityResourcesMaxMemoryMbChanged(1));
    assert_eq!(app.security_settings.resources_max_memory_mb, 32);
    apply(&mut app, SettingsMessage::SecurityResourcesMaxMemoryMbChanged(u32::MAX));
    assert_eq!(app.security_settings.resources_max_memory_mb, 65_536);

    apply(&mut app, SettingsMessage::SecurityResourcesMaxCpuTimeSecondsChanged(0));
    assert_eq!(app.security_settings.resources_max_cpu_time_seconds, 1);
    apply(&mut app, SettingsMessage::SecurityResourcesMaxCpuTimeSecondsChanged(u64::MAX));
    assert_eq!(app.security_settings.resources_max_cpu_time_seconds, 86_400);

    apply(&mut app, SettingsMessage::SecurityResourcesMaxSubprocessesChanged(0));
    assert_eq!(app.security_settings.resources_max_subprocesses, 1);
    apply(&mut app, SettingsMessage::SecurityResourcesMaxSubprocessesChanged(u32::MAX));
    assert_eq!(app.security_settings.resources_max_subprocesses, 10_000);

    apply(&mut app, SettingsMessage::SecurityResourcesMemoryMonitoringToggled(false));
    assert!(!app.security_settings.resources_memory_monitoring);
}

#[test]
fn bash_security_message_updates_disabled_by_default_switch() {
    let mut app = app();
    assert!(!app.security_settings.bash.enabled);

    apply(&mut app, SettingsMessage::SecurityBashSecurityEnabledToggled(true));

    assert!(app.security_settings.bash.enabled);
}

#[test]
fn bash_nested_messages_only_update_target_fields_and_persistence_preserves_siblings() {
    use crate::app::message::settings::{BashPolicyCheck, BashValidatorCheck};

    let mut app = app();
    let original_command_rules = app.security_settings.bash.policy.command_rules;
    let original_heredoc = app.security_settings.bash.validators.heredoc;

    apply(
        &mut app,
        SettingsMessage::SecurityBashPolicyCheckToggled(BashPolicyCheck::ReadOnly, false),
    );
    apply(
        &mut app,
        SettingsMessage::SecurityBashValidatorCheckToggled(BashValidatorCheck::QuotedNewline, true),
    );

    assert!(!app.security_settings.bash.policy.read_only);
    assert_eq!(app.security_settings.bash.policy.command_rules, original_command_rules);
    assert!(app.security_settings.bash.validators.quoted_newline);
    assert_eq!(app.security_settings.bash.validators.heredoc, original_heredoc);

    let mut config = SecurityConfig::default();
    config.bash.policy.redirects = false;
    apply_security_settings_to_config(&mut config, &app.security_settings);
    assert!(!config.bash.policy.read_only);
    assert!(config.bash.policy.command_rules);
    assert!(config.bash.validators.quoted_newline);
    assert!(config.bash.validators.heredoc);
}

#[test]
fn audit_messages_update_values_and_clamp_size() {
    let mut app = app();

    apply(&mut app, SettingsMessage::SecurityAuditEnabledToggled(true));
    apply(&mut app, SettingsMessage::SecurityAuditLogPathChanged(" audit.log ".to_string()));
    apply(&mut app, SettingsMessage::SecurityAuditMaxSizeMbChanged(0));
    assert_eq!(app.security_settings.audit_max_size_mb, 1);
    apply(&mut app, SettingsMessage::SecurityAuditMaxSizeMbChanged(u32::MAX));
    apply(&mut app, SettingsMessage::SecurityAuditSignEventsToggled(true));

    assert!(app.security_settings.audit_enabled);
    assert_eq!(app.security_settings.audit_log_path, " audit.log ");
    assert_eq!(app.security_settings.audit_max_size_mb, 10_000);
    assert!(app.security_settings.audit_sign_events);
}

#[test]
fn otp_messages_normalize_method_and_clamp_ttls() {
    let mut app = app();

    apply(&mut app, SettingsMessage::SecurityOtpEnabledToggled(true));
    apply(&mut app, SettingsMessage::SecurityOtpMethodChanged(" PAIRING ".to_string()));
    apply(&mut app, SettingsMessage::SecurityOtpTokenTtlSecsChanged(0));
    assert_eq!(app.security_settings.otp_token_ttl_secs, 1);
    apply(&mut app, SettingsMessage::SecurityOtpTokenTtlSecsChanged(u64::MAX));
    apply(&mut app, SettingsMessage::SecurityOtpCacheValidSecsChanged(0));
    assert_eq!(app.security_settings.otp_cache_valid_secs, 1);
    apply(&mut app, SettingsMessage::SecurityOtpCacheValidSecsChanged(u64::MAX));
    assert!(app.security_settings.otp_enabled);
    assert_eq!(app.security_settings.otp_method_input, "pairing");
    assert_eq!(app.security_settings.otp_token_ttl_secs, 600);
    assert_eq!(app.security_settings.otp_cache_valid_secs, 86_400);
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
        app.security_settings.list_mut(kind).clear();

        add_security_list_value(&mut app, kind, value);
        assert_eq!(app.security_settings.list(kind), &[value.trim().to_string()]);
        assert!(app.security_settings.list_editor.draft(kind).is_empty());

        add_security_list_value(&mut app, kind, value);
        assert_eq!(app.security_settings.list(kind).len(), 1);
        assert!(app.security_settings.save_error.is_some());

        apply(&mut app, SettingsMessage::SecurityListRemoveConfirmed(kind, 0));
        assert_eq!(app.security_settings.list(kind).len(), 1);

        apply(&mut app, SettingsMessage::SecurityListRemoveRequested(kind, 0));
        assert!(app.security_settings.list_editor.is_remove_pending(kind, 0));
        assert_eq!(app.security_settings.list(kind).len(), 1);
        apply(&mut app, SettingsMessage::SecurityListRemoveCanceled);
        assert!(!app.security_settings.list_editor.is_remove_pending(kind, 0));

        apply(&mut app, SettingsMessage::SecurityListRemoveRequested(kind, 0));
        apply(&mut app, SettingsMessage::SecurityListRemoveConfirmed(kind, 0));
        assert!(app.security_settings.list(kind).is_empty());
    }
}

#[test]
fn security_tab_switch_does_not_clear_save_error() {
    let mut app = app();
    app.security_settings.save_error = Some("keep".to_string());

    apply(&mut app, SettingsMessage::SecurityTabSelected(SecuritySettingsTab::Syscall));

    assert_eq!(app.security_settings.active_tab, SecuritySettingsTab::Syscall);
    assert_eq!(app.security_settings.save_error.as_deref(), Some("keep"));
}

#[test]
fn estop_messages_update_state_without_normalizing_freeform_path() {
    let mut app = app();

    apply(&mut app, SettingsMessage::SecurityEstopEnabledToggled(true));
    apply(
        &mut app,
        SettingsMessage::SecurityEstopStateFileChanged(" state/estop.json ".to_string()),
    );
    apply(&mut app, SettingsMessage::SecurityEstopRequireOtpToResumeToggled(true));

    assert!(app.security_settings.estop_enabled);
    assert_eq!(app.security_settings.estop_state_file, " state/estop.json ");
    assert!(app.security_settings.estop_require_otp_to_resume);
}

#[test]
fn syscall_anomaly_messages_update_flags_paths_and_clamp_both_bounds() {
    let mut app = app();

    apply(&mut app, SettingsMessage::SecuritySyscallAnomalyEnabledToggled(true));
    apply(&mut app, SettingsMessage::SecuritySyscallAnomalyStrictModeToggled(true));
    apply(&mut app, SettingsMessage::SecuritySyscallAnomalyAlertOnUnknownSyscallToggled(true));

    apply(&mut app, SettingsMessage::SecuritySyscallAnomalyMaxDeniedEventsPerMinuteChanged(0));
    assert_eq!(app.security_settings.syscall_anomaly_max_denied_events_per_minute, 1);
    apply(
        &mut app,
        SettingsMessage::SecuritySyscallAnomalyMaxDeniedEventsPerMinuteChanged(u32::MAX),
    );
    apply(&mut app, SettingsMessage::SecuritySyscallAnomalyMaxTotalEventsPerMinuteChanged(0));
    assert_eq!(app.security_settings.syscall_anomaly_max_total_events_per_minute, 1);
    apply(
        &mut app,
        SettingsMessage::SecuritySyscallAnomalyMaxTotalEventsPerMinuteChanged(u32::MAX),
    );
    apply(&mut app, SettingsMessage::SecuritySyscallAnomalyMaxAlertsPerMinuteChanged(0));
    assert_eq!(app.security_settings.syscall_anomaly_max_alerts_per_minute, 1);
    apply(&mut app, SettingsMessage::SecuritySyscallAnomalyMaxAlertsPerMinuteChanged(u32::MAX));
    apply(&mut app, SettingsMessage::SecuritySyscallAnomalyAlertCooldownSecsChanged(0));
    assert_eq!(app.security_settings.syscall_anomaly_alert_cooldown_secs, 1);
    apply(&mut app, SettingsMessage::SecuritySyscallAnomalyAlertCooldownSecsChanged(u64::MAX));

    apply(
        &mut app,
        SettingsMessage::SecuritySyscallAnomalyLogPathChanged(" syscall.log ".to_string()),
    );
    assert!(app.security_settings.syscall_anomaly_enabled);
    assert!(app.security_settings.syscall_anomaly_strict_mode);
    assert!(app.security_settings.syscall_anomaly_alert_on_unknown_syscall);
    assert_eq!(app.security_settings.syscall_anomaly_max_denied_events_per_minute, 10_000);
    assert_eq!(app.security_settings.syscall_anomaly_max_total_events_per_minute, 100_000);
    assert_eq!(app.security_settings.syscall_anomaly_max_alerts_per_minute, 10_000);
    assert_eq!(app.security_settings.syscall_anomaly_alert_cooldown_secs, 3600);
    assert_eq!(app.security_settings.syscall_anomaly_log_path, " syscall.log ");
}

#[test]
fn security_guard_messages_update_flags_collection_and_clamp_threshold() {
    let mut app = app();

    apply(&mut app, SettingsMessage::SecurityCanaryTokensToggled(true));
    apply(&mut app, SettingsMessage::SecuritySemanticGuardToggled(true));
    apply(
        &mut app,
        SettingsMessage::SecuritySemanticGuardCollectionChanged(" collection ".to_string()),
    );
    apply(&mut app, SettingsMessage::SecuritySemanticGuardThresholdChanged(-1.0));
    assert_eq!(app.security_settings.semantic_guard_threshold, 0.0);
    apply(&mut app, SettingsMessage::SecuritySemanticGuardThresholdChanged(2.0));

    assert!(app.security_settings.canary_tokens);
    assert!(app.security_settings.semantic_guard);
    assert_eq!(app.security_settings.semantic_guard_collection, " collection ");
    assert_eq!(app.security_settings.semantic_guard_threshold, 1.0);
}

#[test]
fn security_save_normalizes_dependent_inputs_and_clears_previous_error() {
    let mut app = app();
    app.security_settings.save_error = Some("old error".to_string());
    app.security_settings.sandbox_enabled_input = " TRUE ".to_string();
    app.security_settings.sandbox_backend_input = "none".to_string();
    app.security_settings.otp_method_input = " CLI-PROMPT ".to_string();

    apply(&mut app, SettingsMessage::SecuritySave);

    assert_eq!(app.security_settings.sandbox_enabled_input, "true");
    assert_eq!(app.security_settings.sandbox_backend_input, "auto");
    assert_eq!(app.security_settings.otp_method_input, "cli-prompt");
    assert!(app.security_settings.save_error.is_none());
}

#[test]
fn apply_security_settings_to_config_maps_defaults_lists_and_clamped_values() {
    let mut app = app();
    app.security_settings.bash.enabled = true;
    app.security_settings.sandbox_enabled_input = "false".to_string();
    app.security_settings.sandbox_backend_input = "docker".to_string();
    app.security_settings.sandbox_firejail_args =
        vec!["--private".to_string(), "--net=none".to_string()];
    app.security_settings.resources_max_memory_mb = 1;
    app.security_settings.resources_max_cpu_time_seconds = 0;
    app.security_settings.resources_max_subprocesses = 0;
    app.security_settings.resources_memory_monitoring = false;
    app.security_settings.audit_enabled = true;
    app.security_settings.audit_log_path = "   ".to_string();
    app.security_settings.audit_max_size_mb = 0;
    app.security_settings.audit_sign_events = true;
    app.security_settings.otp_enabled = true;
    app.security_settings.otp_method_input = "cli-prompt".to_string();
    app.security_settings.otp_token_ttl_secs = 0;
    app.security_settings.otp_cache_valid_secs = u64::MAX;
    app.security_settings.otp_gated_actions =
        vec!["shell".to_string(), "write".to_string(), "read".to_string()];
    app.security_settings.otp_gated_domains =
        vec!["admin.example".to_string(), "ops.example".to_string()];
    app.security_settings.otp_gated_domain_categories =
        vec!["finance".to_string(), "prod".to_string()];
    app.security_settings.estop_enabled = true;
    app.security_settings.estop_state_file = " ".to_string();
    app.security_settings.estop_require_otp_to_resume = true;
    app.security_settings.syscall_anomaly_enabled = true;
    app.security_settings.syscall_anomaly_strict_mode = true;
    app.security_settings.syscall_anomaly_alert_on_unknown_syscall = true;
    app.security_settings.syscall_anomaly_max_denied_events_per_minute = 0;
    app.security_settings.syscall_anomaly_max_total_events_per_minute = u32::MAX;
    app.security_settings.syscall_anomaly_max_alerts_per_minute = 0;
    app.security_settings.syscall_anomaly_alert_cooldown_secs = u64::MAX;
    app.security_settings.syscall_anomaly_log_path = " ".to_string();
    app.security_settings.syscall_anomaly_baseline_syscalls =
        vec!["open".to_string(), "read".to_string(), "write".to_string()];
    app.security_settings.canary_tokens = false;
    app.security_settings.semantic_guard = true;
    app.security_settings.semantic_guard_collection = " ".to_string();
    app.security_settings.semantic_guard_threshold = 2.0;
    let mut config = SecurityConfig::default();

    apply_security_settings_to_config(&mut config, &app.security_settings);

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
    assert_eq!(config.estop.state_file, vw_config_types::paths::estop_state_file_path());
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
    app.security_settings.sandbox_enabled_input = "true".to_string();
    app.security_settings.sandbox_backend_input = "firejail".to_string();
    app.security_settings.audit_log_path = "logs/audit.log".to_string();
    app.security_settings.otp_method_input = "pairing".to_string();
    app.security_settings.estop_state_file = "/tmp/estop.json".to_string();
    app.security_settings.syscall_anomaly_log_path = "logs/syscall.log".to_string();
    app.security_settings.semantic_guard_collection = "custom_guard".to_string();
    app.security_settings.semantic_guard_threshold = 0.42;
    let mut config = SecurityConfig::default();

    apply_security_settings_to_config(&mut config, &app.security_settings);

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
    app.security_settings.save_error = Some("keep me".to_string());

    apply(&mut app, SettingsMessage::SecurityHelpOpen);
    assert!(app.security_settings.show_help_modal);
    assert_eq!(app.security_settings.save_error.as_deref(), Some("keep me"));

    apply(&mut app, SettingsMessage::SecurityHelpClose);
    assert!(!app.security_settings.show_help_modal);
    assert_eq!(app.security_settings.save_error.as_deref(), Some("keep me"));
}

#[test]
fn unrelated_settings_message_leaves_security_state_unchanged() {
    let mut app = app();
    app.security_settings.show_help_modal = true;
    app.security_settings.save_error = Some("unchanged".to_string());

    apply(&mut app, SettingsMessage::ReliabilityHelpClose);

    assert!(app.security_settings.show_help_modal);
    assert_eq!(app.security_settings.save_error.as_deref(), Some("unchanged"));
}

#[test]
fn numeric_security_messages_preserve_values_inside_allowed_ranges() {
    let mut app = app();

    apply(&mut app, SettingsMessage::SecurityResourcesMaxMemoryMbChanged(2_048));
    apply(&mut app, SettingsMessage::SecurityResourcesMaxCpuTimeSecondsChanged(3_600));
    apply(&mut app, SettingsMessage::SecurityResourcesMaxSubprocessesChanged(64));
    apply(&mut app, SettingsMessage::SecurityAuditMaxSizeMbChanged(256));
    apply(&mut app, SettingsMessage::SecurityOtpTokenTtlSecsChanged(120));
    apply(&mut app, SettingsMessage::SecurityOtpCacheValidSecsChanged(7_200));
    apply(&mut app, SettingsMessage::SecuritySyscallAnomalyAlertCooldownSecsChanged(45));
    apply(&mut app, SettingsMessage::SecuritySemanticGuardThresholdChanged(0.65));

    assert_eq!(app.security_settings.resources_max_memory_mb, 2_048);
    assert_eq!(app.security_settings.resources_max_cpu_time_seconds, 3_600);
    assert_eq!(app.security_settings.resources_max_subprocesses, 64);
    assert_eq!(app.security_settings.audit_max_size_mb, 256);
    assert_eq!(app.security_settings.otp_token_ttl_secs, 120);
    assert_eq!(app.security_settings.otp_cache_valid_secs, 7_200);
    assert_eq!(app.security_settings.syscall_anomaly_alert_cooldown_secs, 45);
    assert_eq!(app.security_settings.semantic_guard_threshold, 0.65);
}
