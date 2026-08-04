use super::*;

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("infrastructure_tests"));
}

#[test]
fn multimodal_and_http_request_defaults_match_config_ui() {
    let multimodal = MultimodalSettingsState::default();
    assert_eq!(multimodal.max_images, 10);
    assert_eq!(multimodal.max_image_size_mb, 5);
    assert!(multimodal.allow_remote_fetch);
    assert!(!multimodal.ocr_enabled);
    assert!(multimodal.ocr_model.is_empty());
    assert!(multimodal.save_error.is_none());

    let http = HttpRequestSettingsState::default();
    assert!(!http.enabled);
    assert!(http.allowed_domains.is_empty());
    assert!(http.new_allowed_domain_input.is_empty());
    assert_eq!(http.max_response_size, 1_000_000);
    assert_eq!(http.timeout_secs, 30);
    assert_eq!(http.user_agent, HttpRequestConfig::default().user_agent);
    assert!(http.save_error.is_none());
}

#[test]
fn acp_security_and_autonomy_defaults_are_populated() {
    let acp = AcpSettingsState::default();
    assert!(acp.catalog.is_empty());
    assert!(acp.enabled.is_empty());
    assert!(!acp.loading);
    assert!(acp.saving_agent.is_none());
    assert!(acp.copied_agent.is_none());
    assert!(acp.save_error.is_none());
    assert!(acp.status_message.is_none());

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
    assert_eq!(security.estop_state_file, vw_config_types::paths::ESTOP_STATE_FILE_PATH);
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

    let autonomy = AutonomySettingsState::default();
    assert_eq!(autonomy.active_tab, AutonomySettingsTab::Permissions);
    assert_eq!(autonomy.level, vw_config_types::security::AutonomyLevel::Full);
    assert!(!autonomy.workspace_only);
    assert!(autonomy.allowed_commands.contains(&"cargo".to_string()));
    assert!(autonomy.forbidden_paths.contains(&"/etc".to_string()));
    assert_eq!(autonomy.max_actions_per_hour, 2_000);
    assert_eq!(autonomy.max_cost_per_day_cents, 50_000);
    assert!(!autonomy.require_approval_for_medium_risk);
    assert!(!autonomy.block_high_risk_commands);
    assert_eq!(
        autonomy.shell_redirect_policy,
        vw_config_types::security::ShellRedirectPolicy::Allow
    );
    assert!(autonomy.shell_env_passthrough.is_empty());
    assert!(autonomy.auto_approve.contains(&"file_read".to_string()));
    assert!(autonomy.always_ask.is_empty());
    assert!(autonomy.allowed_roots.is_empty());
    assert!(autonomy.non_cli_excluded_tools.contains(&"shell".to_string()));
    for kind in AutonomyListKind::ALL {
        assert!(autonomy.list_editor.draft(kind).is_empty());
    }
    assert!(autonomy.non_cli_approval_approvers_input.is_empty());
    assert_eq!(
        autonomy.non_cli_natural_language_approval_mode,
        vw_config_types::security::NonCliNaturalLanguageApprovalMode::Direct
    );
    assert!(autonomy.non_cli_natural_language_approval_mode_by_channel_input.is_empty());
    assert!(!autonomy.show_help_modal);
    assert!(autonomy.save_error.is_none());
}

#[test]
fn autonomy_settings_defaults_keep_agent_tool_available() {
    let autonomy = AutonomySettingsState::default();

    assert!(!autonomy.non_cli_excluded_tools.iter().any(|tool| tool == "AgentTool"));
}

#[test]
fn autonomy_settings_from_config_preserves_unknown_saved_tools() {
    let mut config = vw_config_types::security::AutonomyConfig::default();
    config.auto_approve = vec!["known_tool".to_string(), "retired_tool".to_string()];
    config.always_ask = vec!["missing_tool".to_string()];
    config.non_cli_excluded_tools = vec!["legacy_tool".to_string()];

    let autonomy = AutonomySettingsState::from_config(&config);

    assert_eq!(autonomy.auto_approve, config.auto_approve);
    assert_eq!(autonomy.always_ask, config.always_ask);
    assert_eq!(autonomy.non_cli_excluded_tools, config.non_cli_excluded_tools);
}

#[test]
fn mcp_new_server_default_is_enabled_stdio_with_empty_inputs() {
    let server = McpNewServerState::default();

    assert!(server.name_input.is_empty());
    assert_eq!(server.transport, McpNewServerTransport::Stdio);
    assert!(server.command_input.is_empty());
    assert!(server.args_input.is_empty());
    assert!(server.url_input.is_empty());
    assert_eq!(server.timeout_input, "30");
    assert!(server.enabled);
    assert!(!server.required);
}

#[test]
fn mcp_transports_expose_stable_labels_and_display_values() {
    let expected = ["stdio", "http", "sse"];

    for (transport, label) in McpNewServerTransport::ALL.into_iter().zip(expected) {
        assert_eq!(transport.label(), label);
        assert_eq!(transport.to_string(), label);
    }
}

#[test]
fn mcp_settings_default_opens_servers_with_empty_runtime_state() {
    let state = McpSettingsState::default();

    assert_eq!(state.active_tab, McpSettingsTab::Servers);
    assert_eq!(state.editor.text(), "[]");
    assert!(state.runtime_overrides.is_empty());
    assert!(!state.new_server_expanded);
    assert!(state.expanded_previews.is_empty());
    assert_eq!(state.new_server.transport, McpNewServerTransport::Stdio);
    assert!(!state.loading);
    assert!(!state.inspect_loading);
    assert!(!state.saving);
    assert_eq!(state.server_count, 0);
    assert!(state.inspect.is_none());
    assert!(state.inspect_error.is_none());
    assert!(state.save_error.is_none());
    assert!(state.status_message.is_none());
}

#[test]
fn observability_storage_proxy_and_tunnel_defaults_are_stable() {
    let observability = ObservabilitySettingsState::default();
    assert_eq!(observability.backend, "none");
    assert!(observability.otel_endpoint_input.is_empty());
    assert!(observability.otel_service_name_input.is_empty());
    assert_eq!(observability.runtime_trace_mode, "none");
    assert_eq!(observability.runtime_trace_path_input, "state/runtime-trace.jsonl");
    assert_eq!(observability.runtime_trace_max_entries, 200);
    assert!(!observability.show_help_modal);
    assert!(observability.save_error.is_none());

    let storage = StorageSettingsState::default();
    assert!(storage.provider.is_empty());
    assert!(storage.db_url_input.is_empty());
    assert_eq!(storage.schema, "public");
    assert_eq!(storage.table, "memories");
    assert!(storage.connect_timeout_secs_input.is_empty());
    assert!(!storage.tls);
    assert!(storage.save_error.is_none());

    let proxy = ProxySettingsState::default();
    assert!(!proxy.enabled);
    assert!(proxy.http_proxy.is_empty());
    assert!(proxy.https_proxy.is_empty());
    assert!(proxy.all_proxy.is_empty());
    assert!(proxy.no_proxy_input.is_empty());
    assert_eq!(proxy.scope, vw_config_types::proxy::ProxyScope::Vibewindow);
    assert!(proxy.services_input.is_empty());
    assert!(!proxy.show_help_modal);
    assert!(proxy.save_error.is_none());

    let tunnel = TunnelSettingsState::default();
    assert_eq!(tunnel.provider, "none");
    assert!(!tunnel.tailscale_funnel);
    assert!(tunnel.cloudflare_token.is_empty());
    assert!(tunnel.tailscale_hostname.is_empty());
    assert!(tunnel.ngrok_auth_token.is_empty());
    assert!(tunnel.ngrok_domain.is_empty());
    assert!(tunnel.custom_start_command.is_empty());
    assert!(tunnel.custom_health_url.is_empty());
    assert!(tunnel.custom_url_pattern.is_empty());
    assert!(tunnel.save_error.is_none());
}

#[test]
fn composio_and_transcription_defaults_are_ready_for_empty_config() {
    let composio = ComposioSettingsState::default();
    assert!(!composio.enabled);
    assert!(composio.api_key_input.is_empty());
    assert_eq!(composio.entity_id_input, "default");
    assert!(composio.save_error.is_none());

    let transcription = TranscriptionSettingsState::default();
    assert!(!transcription.enabled);
    assert_eq!(transcription.api_url, "https://api.groq.com/openai/v1/audio/transcriptions");
    assert_eq!(transcription.model, "whisper-large-v3-turbo");
    assert!(transcription.language.is_empty());
    assert_eq!(transcription.max_duration_secs, 120);
    assert!(!transcription.show_help_modal);
    assert!(transcription.save_error.is_none());
}

#[test]
fn themes_settings_default_opens_list_with_basics_editor_tab() {
    let state = ThemesSettingsState::default();
    assert_eq!(state.mode, ThemesSettingsMode::List);
    assert_eq!(state.editor_tab, ThemeEditorTab::Basics);
    assert!(state.entries.is_empty());
    assert_eq!(state.page_index, 0);
    assert!(state.draft.is_none());
    assert!(state.error.is_none());
    assert!(state.color_picker_target.is_none());
}
