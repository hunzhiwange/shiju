//! 从已加载配置构造本领域设置；不执行网络或宿主操作。

pub fn build_security_settings(
    full_agent_cfg: &config_aggregate::Config,
) -> crate::security_state::SecuritySettingsState {
    let security_cfg = &full_agent_cfg.security;

    crate::security_state::SecuritySettingsState {
        active_tab: crate::security_state::SecuritySettingsTab::Bash,
        bash: security_cfg.bash.clone(),
        sandbox_enabled_input: match security_cfg.sandbox.enabled {
            None => "auto".to_string(),
            Some(true) => "true".to_string(),
            Some(false) => "false".to_string(),
        },
        sandbox_backend_input: match security_cfg.sandbox.backend {
            config_security::SandboxBackend::Auto => "auto".to_string(),
            config_security::SandboxBackend::Landlock => "landlock".to_string(),
            config_security::SandboxBackend::Firejail => "firejail".to_string(),
            config_security::SandboxBackend::Bubblewrap => "bubblewrap".to_string(),
            config_security::SandboxBackend::Docker => "docker".to_string(),
            config_security::SandboxBackend::None => "none".to_string(),
        },
        sandbox_firejail_args: security_cfg.sandbox.firejail_args.clone(),
        resources_max_memory_mb: security_cfg.resources.max_memory_mb.clamp(32, 65_536),
        resources_max_cpu_time_seconds: security_cfg
            .resources
            .max_cpu_time_seconds
            .clamp(1, 86_400),
        resources_max_subprocesses: security_cfg.resources.max_subprocesses.clamp(1, 10_000),
        resources_memory_monitoring: security_cfg.resources.memory_monitoring,
        audit_enabled: security_cfg.audit.enabled,
        audit_log_path: {
            let value = security_cfg.audit.log_path.trim().to_string();
            if value.is_empty() { "audit.log".to_string() } else { value }
        },
        audit_max_size_mb: security_cfg.audit.max_size_mb.clamp(1, 10_000),
        audit_sign_events: security_cfg.audit.sign_events,
        otp_enabled: security_cfg.otp.enabled,
        otp_method_input: match security_cfg.otp.method {
            config_security::OtpMethod::Totp => "totp".to_string(),
            config_security::OtpMethod::Pairing => "pairing".to_string(),
            config_security::OtpMethod::CliPrompt => "cli-prompt".to_string(),
        },
        otp_token_ttl_secs: security_cfg.otp.token_ttl_secs.clamp(1, 600),
        otp_cache_valid_secs: security_cfg.otp.cache_valid_secs.clamp(1, 86_400),
        otp_gated_actions: security_cfg.otp.gated_actions.clone(),
        otp_gated_domains: security_cfg.otp.gated_domains.clone(),
        otp_gated_domain_categories: security_cfg.otp.gated_domain_categories.clone(),
        estop_enabled: security_cfg.estop.enabled,
        estop_state_file: {
            let value = security_cfg.estop.state_file.trim().to_string();
            if value.is_empty() { config_runtime_paths::estop_state_file_path() } else { value }
        },
        estop_require_otp_to_resume: security_cfg.estop.require_otp_to_resume,
        syscall_anomaly_enabled: security_cfg.syscall_anomaly.enabled,
        syscall_anomaly_strict_mode: security_cfg.syscall_anomaly.strict_mode,
        syscall_anomaly_alert_on_unknown_syscall: security_cfg
            .syscall_anomaly
            .alert_on_unknown_syscall,
        syscall_anomaly_max_denied_events_per_minute: security_cfg
            .syscall_anomaly
            .max_denied_events_per_minute
            .clamp(1, 10_000),
        syscall_anomaly_max_total_events_per_minute: security_cfg
            .syscall_anomaly
            .max_total_events_per_minute
            .clamp(1, 100_000),
        syscall_anomaly_max_alerts_per_minute: security_cfg
            .syscall_anomaly
            .max_alerts_per_minute
            .clamp(1, 10_000),
        syscall_anomaly_alert_cooldown_secs: security_cfg
            .syscall_anomaly
            .alert_cooldown_secs
            .clamp(1, 3600),
        syscall_anomaly_log_path: {
            let value = security_cfg.syscall_anomaly.log_path.trim().to_string();
            if value.is_empty() { "syscall-anomalies.log".to_string() } else { value }
        },
        syscall_anomaly_baseline_syscalls: security_cfg.syscall_anomaly.baseline_syscalls.clone(),
        list_editor: crate::security_state::SecurityListEditorState::default(),
        canary_tokens: security_cfg.canary_tokens.clone(),
        semantic_guard: security_cfg.semantic_guard,
        semantic_guard_collection: {
            let value = security_cfg.semantic_guard_collection.trim().to_string();
            if value.is_empty() { "semantic_guard".to_string() } else { value }
        },
        semantic_guard_threshold: security_cfg.semantic_guard_threshold.clamp(0.0, 1.0),
        show_help_modal: false,
        save_error: None,
    }
}
