use super::*;

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("runtime_tests"));
}

#[test]
fn hooks_and_runtime_defaults_match_secure_native_runtime() {
    let hooks = HooksSettingsState::default();
    assert!(hooks.enabled);
    assert!(!hooks.command_logger);
    assert!(hooks.save_error.is_none());

    let runtime = RuntimeSettingsState::default();
    assert_eq!(runtime.kind, "native");
    assert_eq!(runtime.docker_image, "alpine:3.20");
    assert_eq!(runtime.docker_network, "none");
    assert_eq!(runtime.docker_memory_limit_mb_input, "512");
    assert_eq!(runtime.docker_cpu_limit_input, "1");
    assert!(runtime.docker_read_only_rootfs);
    assert!(runtime.docker_mount_workspace);
    assert!(runtime.docker_allowed_workspace_roots_input.is_empty());
    assert_eq!(runtime.wasm_tools_dir, "tools/wasm");
    assert_eq!(runtime.wasm_fuel_limit_input, "1000000");
    assert_eq!(runtime.wasm_memory_limit_mb_input, "64");
    assert_eq!(runtime.wasm_max_module_size_mb_input, "50");
    assert!(!runtime.wasm_allow_workspace_read);
    assert!(!runtime.wasm_allow_workspace_write);
    assert!(runtime.wasm_allowed_hosts_input.is_empty());
    assert!(runtime.wasm_require_workspace_relative_tools_dir);
    assert!(runtime.wasm_reject_symlink_modules);
    assert!(runtime.wasm_reject_symlink_tools_dir);
    assert!(runtime.wasm_strict_host_validation);
    assert_eq!(runtime.wasm_capability_escalation_mode, "deny");
    assert_eq!(runtime.wasm_module_hash_policy, "warn");
    assert!(runtime.wasm_module_sha256_input.is_empty());
    assert_eq!(runtime.reasoning_enabled_input, "auto");
    assert!(runtime.reasoning_level_input.is_empty());
    assert!(runtime.save_error.is_none());
}

#[test]
fn skills_enums_and_default_state_are_stable() {
    assert_eq!(SkillsSettingsState::default().active_tab, SkillsSettingsTab::Skills);
    assert_ne!(SkillsSettingsTab::Skills, SkillsSettingsTab::SystemConfig);
    assert_ne!(SkillsDirectoryScope::Project, SkillsDirectoryScope::Global);

    let item = SkillsCatalogItem {
        id: "skill".to_string(),
        title: "Skill".to_string(),
        description: "Description".to_string(),
        kind: SkillsCatalogKind::Recommended,
        resource_count: 2,
        installed: true,
        enabled: true,
        source: "system".to_string(),
        source_path: Some("/tmp/skill".to_string()),
    };
    assert_eq!(item.id, "skill");
    assert_eq!(item.kind, SkillsCatalogKind::Recommended);
    assert_eq!(item.resource_count, 2);
    assert!(item.installed);
    assert!(item.enabled);

    let detail = SkillsSelectedDetail {
        id: "skill".to_string(),
        title: "Skill".to_string(),
        description: "Description".to_string(),
        kind: SkillsCatalogKind::System,
        installed: false,
        enabled: false,
        source: "bundle".to_string(),
        source_path: None,
        document_name: "SKILL.md".to_string(),
        document_content: "body".to_string(),
        can_install: true,
        can_toggle: false,
        can_delete: false,
    };
    assert_eq!(detail.document_name, "SKILL.md");
    assert!(detail.can_install);
    assert!(!detail.can_toggle);
    assert!(!detail.can_delete);

    let state = SkillsSettingsState::default();
    assert!(!state.open_skills_enabled);
    assert!(state.auto_load_project_skills);
    assert_eq!(
        state.directory_provider,
        vw_config_types::skills::SkillsDirectoryProvider::Vibewindow
    );
    assert_eq!(
        state.prompt_injection_mode,
        vw_config_types::skills::SkillsPromptInjectionMode::Compact
    );
    assert_eq!(state.active_tab, SkillsSettingsTab::Skills);
    assert_eq!(state.directory_scope, SkillsDirectoryScope::Project);
    assert!(!state.loading);
    assert!(state.catalog.is_empty());
    assert!(state.selected_skill_id.is_none());
    assert!(state.selected_skill_detail.is_none());
    assert!(!state.detail_loading);
    assert!(state.detail_error.is_none());
    assert!(state.status_message.is_none());
    assert!(!state.status_is_error);
    assert!(!state.show_help_modal);
    assert!(state.save_error.is_none());
}

#[test]
fn skills_catalog_models_preserve_metadata_and_support_every_kind() {
    let tabs = [
        SkillsSettingsTab::Skills,
        SkillsSettingsTab::DiscoveryOrder,
        SkillsSettingsTab::SystemConfig,
    ];
    assert_eq!(tabs.len(), 3);

    let scopes = [
        SkillsDirectoryScope::Project,
        SkillsDirectoryScope::Ancestor,
        SkillsDirectoryScope::Global,
        SkillsDirectoryScope::Bundled,
        SkillsDirectoryScope::All,
    ];
    assert_eq!(scopes.len(), 5);

    let kinds =
        [SkillsCatalogKind::Recommended, SkillsCatalogKind::System, SkillsCatalogKind::Personal];
    assert_eq!(kinds.len(), 3);

    let plugin = PluginCatalogItem {
        id: "plugin-id".to_string(),
        display_name: "Plugin".to_string(),
        version: "1.2.3".to_string(),
        description: "Plugin description".to_string(),
        enabled: true,
        component_count: 4,
        source_path: Some("plugins/plugin-id".to_string()),
    };
    assert_eq!(plugin.id, "plugin-id");
    assert_eq!(plugin.display_name, "Plugin");
    assert_eq!(plugin.version, "1.2.3");
    assert_eq!(plugin.description, "Plugin description");
    assert!(plugin.enabled);
    assert_eq!(plugin.component_count, 4);
    assert_eq!(plugin.source_path.as_deref(), Some("plugins/plugin-id"));

    let state = SkillsSettingsState::default();
    assert!(state.open_skills_dir_input.is_empty());
    assert!(state.query.is_empty());
    assert!(state.plugin_catalog.is_empty());
    assert!(!state.plugin_loading);
    assert!(state.selected_plugin_id.is_none());
    assert!(state.plugin_status_message.is_none());
    assert!(!state.plugin_status_is_error);
}

#[test]
fn web_search_browser_and_gateway_defaults_are_ready_for_empty_config() {
    let web = WebSearchSettingsState::default();
    assert!(!web.enabled);
    assert_eq!(web.provider, "duckduckgo");
    assert!(web.api_key_input.is_empty());
    assert!(web.api_url_input.is_empty());
    assert!(web.brave_api_key_input.is_empty());
    assert_eq!(web.max_results_input, "5");
    assert_eq!(web.timeout_secs_input, "15");
    assert_eq!(web.user_agent, vw_config_types::tools::DEFAULT_USER_AGENT);
    assert!(!web.show_help_modal);
    assert!(web.save_error.is_none());

    let browser = BrowserSettingsState::default();
    assert!(!browser.enabled);
    assert!(browser.allowed_domains_input.is_empty());
    assert!(browser.allowed_domains_editor.text().is_empty());
    assert_eq!(browser.browser_open, "new_tab");
    assert!(browser.session_name_input.is_empty());
    assert_eq!(browser.backend, "agent_browser");
    assert!(browser.native_headless);
    assert_eq!(browser.native_webdriver_url, "http://127.0.0.1:9515");
    assert!(browser.native_chrome_path_input.is_empty());
    assert_eq!(browser.computer_use_endpoint, "http://127.0.0.1:8787/v1/actions");
    assert!(browser.computer_use_api_key_input.is_empty());
    assert_eq!(browser.computer_use_timeout_ms_input, "15000");
    assert!(!browser.computer_use_allow_remote_endpoint);
    assert!(browser.computer_use_window_allowlist_input.is_empty());
    assert!(browser.computer_use_max_coordinate_x_input.is_empty());
    assert!(browser.computer_use_max_coordinate_y_input.is_empty());
    assert!(browser.save_error.is_none());

    let gateway = GatewaySettingsState::default();
    assert_eq!(gateway.active_tab, GatewaySettingsTab::Config);
    assert_eq!(gateway.port, vw_config_types::gateway::DEFAULT_GATEWAY_PORT);
    assert_eq!(gateway.host_input, "127.0.0.1");
    assert!(!gateway.auth_enabled);
    assert!(!gateway.allow_public_bind);
    assert!(gateway.skeys.is_empty());
    assert!(gateway.new_skey_name_input.is_empty());
    assert!(gateway.new_skey_expires_at_input.is_empty());
    assert!(gateway.new_skey_calendar_month.is_empty());
    assert!(!gateway.new_skey_calendar_open);
    assert!(gateway.last_created_skey.is_none());
    assert!(!gateway.last_created_skey_copied);
    assert_eq!(gateway.webhook_rate_limit_per_minute, 60);
    assert!(!gateway.trust_forwarded_headers);
    assert_eq!(gateway.rate_limit_max_keys, 10_000);
    assert_eq!(gateway.idempotency_ttl_secs, 300);
    assert_eq!(gateway.idempotency_max_keys, 10_000);
    assert!(!gateway.node_control_enabled);
    assert!(gateway.node_control_auth_token_input.is_empty());
    assert!(gateway.node_control_allowed_node_ids_input.is_empty());
    assert!(gateway.service_action_running.is_none());
    assert!(gateway.service_action_output.is_none());
    assert!(gateway.copied_service_command.is_none());
    assert!(gateway.pending_service_command_confirmation.is_none());
    assert!(!gateway.show_help_modal);
    assert!(gateway.save_error.is_none());
}

#[test]
fn gateway_client_server_draft_round_trips_config_and_clamps_port() {
    let config = vw_config_types::ui::GatewayClientServerConfig {
        id: "remote".to_string(),
        name: "Remote".to_string(),
        host: "gw.example".to_string(),
        port: 0,
        platform_id: "20000".to_string(),
        company_id: "200000001".to_string(),
        bearer_token: "bearer".to_string(),
        username: "user".to_string(),
        password: "pass".to_string(),
        skey: "skey".to_string(),
    };

    let draft = GatewayClientServerDraft::from_config(&config);
    assert_eq!(draft.id, "remote");
    assert_eq!(draft.name, "Remote");
    assert_eq!(draft.host, "gw.example");
    assert_eq!(draft.port, 1);
    assert_eq!(draft.platform_id, "20000");
    assert_eq!(draft.company_id, "200000001");
    assert_eq!(draft.skey, "skey");

    let round_trip = draft.to_config();
    assert_eq!(round_trip.id, "remote");
    assert_eq!(round_trip.name, "Remote");
    assert_eq!(round_trip.host, "gw.example");
    assert_eq!(round_trip.port, 1);
    assert_eq!(round_trip.platform_id, "20000");
    assert_eq!(round_trip.company_id, "200000001");
    assert!(round_trip.bearer_token.is_empty());
    assert!(round_trip.username.is_empty());
    assert!(round_trip.password.is_empty());
    assert_eq!(round_trip.skey, "skey");
}

#[test]
fn gateway_client_server_draft_uses_bearer_token_when_skey_is_blank() {
    let config = vw_config_types::ui::GatewayClientServerConfig {
        id: "legacy".to_string(),
        name: "Legacy".to_string(),
        host: "legacy.example".to_string(),
        port: u16::MAX,
        platform_id: "platform".to_string(),
        company_id: "company".to_string(),
        bearer_token: "legacy-token".to_string(),
        username: String::new(),
        password: String::new(),
        skey: "  \n".to_string(),
    };

    let draft = GatewayClientServerDraft::from_config(&config);

    assert_eq!(draft.port, u16::MAX);
    assert_eq!(draft.skey, "legacy-token");
}

#[test]
fn gateway_client_server_draft_to_config_clamps_zero_and_clears_legacy_credentials() {
    let draft = GatewayClientServerDraft {
        id: "manual".to_string(),
        name: "Manual".to_string(),
        host: "127.0.0.1".to_string(),
        port: 0,
        platform_id: "10000".to_string(),
        company_id: "100000000".to_string(),
        skey: "current-skey".to_string(),
    };

    let config = draft.to_config();

    assert_eq!(config.port, 1);
    assert!(config.bearer_token.is_empty());
    assert!(config.username.is_empty());
    assert!(config.password.is_empty());
    assert_eq!(config.skey, "current-skey");
}

#[test]
fn gateway_client_agents_coordination_and_cost_defaults_are_stable() {
    let client = GatewayClientSettingsState::default();
    assert_eq!(client.servers.len(), 1);
    assert_eq!(client.selected_server_id, client.servers[0].id);
    assert_eq!(client.name_input, client.servers[0].name);
    assert!(client.health.is_empty());
    assert_eq!(client.host_input, "127.0.0.1");
    assert_eq!(client.port, vw_config_types::gateway::DEFAULT_GATEWAY_PORT);
    assert_eq!(client.platform_id_input, "10000");
    assert_eq!(client.company_id_input, "100000000");
    assert!(client.skey_input.is_empty());
    assert!(client.pending_remove_server_id.is_none());
    assert!(!client.show_help_modal);
    assert!(client.save_error.is_none());

    let price = CostPriceInput::default();
    assert!(price.model.is_empty());
    assert!(price.input_price.is_empty());
    assert!(price.output_price.is_empty());

    let cost = CostSettingsState::default();
    assert!(!cost.enabled);
    assert_eq!(cost.daily_limit_usd_input, "10");
    assert_eq!(cost.monthly_limit_usd_input, "100");
    assert_eq!(cost.warn_at_percent_input, "80");
    assert!(!cost.allow_override);
    assert!(cost.prices.is_empty());
    assert!(!cost.show_help_modal);
    assert!(cost.save_error.is_none());
}

#[test]
fn gateway_tab_variants_and_runtime_state_clones_are_independent() {
    assert_ne!(GatewaySettingsTab::Config, GatewaySettingsTab::Skeys);

    let mut original = RuntimeSettingsState {
        kind: "docker".to_string(),
        docker_image: "custom/image:latest".to_string(),
        save_error: Some("old error".to_string()),
        ..RuntimeSettingsState::default()
    };
    let cloned = original.clone();

    original.kind = "wasm".to_string();
    original.docker_image = "changed".to_string();
    original.save_error = None;

    assert_eq!(cloned.kind, "docker");
    assert_eq!(cloned.docker_image, "custom/image:latest");
    assert_eq!(cloned.save_error.as_deref(), Some("old error"));
}

#[test]
fn memory_and_reliability_defaults_match_runtime_config() {
    let memory = MemorySettingsState::default();
    assert_eq!(memory.backend, "sqlite");
    assert!(memory.auto_save);
    assert!(memory.hygiene_enabled);
    assert_eq!(memory.archive_after_days, 7);
    assert_eq!(memory.purge_after_days, 30);
    assert_eq!(memory.conversation_retention_days, 30);
    assert_eq!(memory.embedding_provider, "none");
    assert_eq!(memory.embedding_model, "text-embedding-3-small");
    assert_eq!(memory.embedding_dimensions, 1536);
    assert_eq!(memory.vector_weight, 0.7);
    assert_eq!(memory.keyword_weight, 0.3);
    assert_eq!(memory.min_relevance_score, 0.4);
    assert_eq!(memory.embedding_cache_size, 10_000);
    assert_eq!(memory.chunk_max_tokens, 512);
    assert!(!memory.response_cache_enabled);
    assert_eq!(memory.response_cache_ttl_minutes, 60);
    assert_eq!(memory.response_cache_max_entries, 5_000);
    assert!(!memory.snapshot_enabled);
    assert!(!memory.snapshot_on_hygiene);
    assert!(memory.auto_hydrate);
    assert_eq!(memory.sqlite_open_timeout_secs, 0);
    assert!(memory.qdrant_url_input.is_empty());
    assert_eq!(memory.qdrant_collection, "vibewindow_memories");
    assert!(memory.qdrant_api_key_input.is_empty());
    assert!(memory.save_error.is_none());

    let reliability = ReliabilitySettingsState::default();
    assert_eq!(reliability.provider_retries, 2);
    assert_eq!(reliability.provider_backoff_ms, 500);
    assert_eq!(reliability.channel_initial_backoff_secs, 2);
    assert_eq!(reliability.channel_max_backoff_secs, 60);
    assert_eq!(reliability.scheduler_poll_secs, 15);
    assert_eq!(reliability.scheduler_retries, 2);
    assert!(!reliability.show_help_modal);
    assert!(reliability.save_error.is_none());
}
