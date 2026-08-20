use super::*;
use crate::app::state::{ExternalOpenApp, RuntimePlatform};
use serde_json::json;

fn test_app() -> App {
    App::new().0
}

#[test]
fn sort_acp_agent_names_keeps_codex_first_and_sorts_remaining_names() {
    let names = vec!["zeta".to_string(), "codex".to_string(), "alpha".to_string()];

    assert_eq!(sort_acp_agent_names(names), ["codex", "alpha", "zeta"]);
}

#[test]
fn parse_external_open_app_accepts_known_stable_ids() {
    assert_eq!(App::parse_external_open_app("finder"), Some(ExternalOpenApp::Finder));
    assert_eq!(App::parse_external_open_app("vscode"), Some(ExternalOpenApp::VSCode));
    assert_eq!(
        App::parse_external_open_app("android-studio"),
        Some(ExternalOpenApp::AndroidStudio)
    );
    assert_eq!(App::parse_external_open_app("powershell"), Some(ExternalOpenApp::PowerShell));
    assert_eq!(App::parse_external_open_app("unknown"), None);
}

#[test]
fn parse_external_open_app_maps_remaining_stable_ids() {
    let cases = [
        ("cursor", ExternalOpenApp::Cursor),
        ("trae", ExternalOpenApp::Trae),
        ("windsurf", ExternalOpenApp::Windsurf),
        ("kiro", ExternalOpenApp::Kiro),
        ("zed", ExternalOpenApp::Zed),
        ("textmate", ExternalOpenApp::TextMate),
        ("antigravity", ExternalOpenApp::Antigravity),
        ("terminal", ExternalOpenApp::Terminal),
        ("iterm2", ExternalOpenApp::ITerm2),
        ("ghostty", ExternalOpenApp::Ghostty),
        ("xcode", ExternalOpenApp::Xcode),
        ("sublime-text", ExternalOpenApp::SublimeText),
    ];

    for (id, expected) in cases {
        assert_eq!(App::parse_external_open_app(id), Some(expected));
    }
}

#[test]
fn external_open_priority_is_platform_specific() {
    assert_eq!(App::external_open_priority(Some(RuntimePlatform::MacOs))[0], ExternalOpenApp::Trae);
    assert!(
        App::external_open_priority(Some(RuntimePlatform::MacOs))
            .contains(&ExternalOpenApp::Terminal)
    );
    assert!(
        App::external_open_priority(Some(RuntimePlatform::Windows))
            .contains(&ExternalOpenApp::PowerShell)
    );
    assert!(
        !App::external_open_priority(Some(RuntimePlatform::Linux))
            .contains(&ExternalOpenApp::PowerShell)
    );
}

#[test]
fn apply_external_apps_state_selects_available_priority_and_keeps_finder_available() {
    let mut app = test_app();
    app.open_external_app = ExternalOpenApp::VSCode;

    app.apply_external_apps_state(
        Some(RuntimePlatform::MacOs),
        vec![
            ("cursor".to_string(), false),
            ("zed".to_string(), true),
            ("unknown".to_string(), true),
        ],
    );

    assert_eq!(app.open_external_platform, Some(RuntimePlatform::MacOs));
    assert_eq!(app.open_external_app, ExternalOpenApp::Zed);
    assert_eq!(app.open_external_exists.get(&ExternalOpenApp::Finder), Some(&true));
    assert_eq!(app.open_external_exists.get(&ExternalOpenApp::Cursor), Some(&false));
}

#[test]
fn apply_external_apps_state_preserves_current_app_when_still_available() {
    let mut app = test_app();
    app.open_external_platform = Some(RuntimePlatform::Linux);
    app.open_external_app = ExternalOpenApp::Cursor;

    app.apply_external_apps_state(None, vec![("cursor".to_string(), true)]);

    assert_eq!(app.open_external_platform, Some(RuntimePlatform::Linux));
    assert_eq!(app.open_external_app, ExternalOpenApp::Cursor);
}

#[test]
fn apply_external_apps_state_falls_back_when_duplicate_marks_current_unavailable() {
    let mut app = test_app();
    app.open_external_app = ExternalOpenApp::Cursor;

    app.apply_external_apps_state(
        Some(RuntimePlatform::Linux),
        vec![
            ("cursor".to_string(), true),
            ("cursor".to_string(), false),
            ("trae".to_string(), false),
        ],
    );

    assert_eq!(app.open_external_app, ExternalOpenApp::Finder);
    assert_eq!(app.open_external_exists.get(&ExternalOpenApp::Cursor), Some(&false));
    assert_eq!(app.open_external_exists.get(&ExternalOpenApp::Finder), Some(&true));
}

#[test]
fn apply_project_chat_preferences_updates_current_and_empty_runtime_state() {
    let mut app = test_app();

    app.apply_project_chat_preferences("gpt-test".to_string(), true, Some("codex".to_string()));

    assert_eq!(app.model, "gpt-test");
    assert!(app.auto_model);
    assert_eq!(app.acp_agent.as_deref(), Some("codex"));

    let runtime = app.session_runtime_states.get("__empty__").expect("empty runtime state");
    assert_eq!(runtime.model, "gpt-test");
    assert!(runtime.auto_model);
    assert_eq!(runtime.acp_agent.as_deref(), Some("codex"));
}

#[test]
fn startup_agent_config_reload_rehydrates_gateway_backed_settings() {
    let mut app = test_app();
    app.agents_settings.project_path = Some("/workspace/project".to_string());

    let mut config = vw_config_types::config::Config::default();
    config.heartbeat.interval_minutes = 91;
    config.gateway.port = 19_321;
    config.memory.backend = "markdown".to_string();
    config.channels_config.cli = true;
    config.proxy.enabled = true;
    config.acp.insert("claude".to_string(), vw_config_types::config::AcpAgentConfig::default());

    let _ = app.update(Message::StartupAgentConfigLoaded(Ok(Box::new(config))));

    assert_eq!(app.heartbeat_settings.interval_minutes, 91);
    assert_eq!(app.gateway_settings.port, 19_321);
    assert_eq!(app.memory_settings.backend, "markdown");
    assert!(app.channels_settings.cli);
    assert!(app.proxy_settings.enabled);
    assert_eq!(app.acp_agents, ["claude"]);
    assert_eq!(app.agents_settings.project_path.as_deref(), Some("/workspace/project"));
}

#[test]
fn bootstrap_app_config_updates_known_fields_and_clamps_file_manager_width() {
    let mut app = test_app();

    let _ = app.update(Message::BootstrapAppConfig(Ok(json!({
        "model": "gpt-next",
        "auto_model": true,
        "acp_agent": "codex",
        "show_settings": true,
        "workspace_mode": "work",
        "show_file_manager": true,
        "file_manager_width": 9999.0,
        "open_external_app": "cursor",
        "file_tree_expanded": ["src", 7, "crates"]
    }))));

    assert_eq!(app.model, "gpt-next");
    assert!(app.auto_model);
    assert_eq!(app.acp_agent.as_deref(), Some("codex"));
    assert!(app.show_settings);
    assert_eq!(app.workspace_mode, crate::app::state::WorkspaceMode::Work);
    assert!(app.show_file_manager);
    assert_eq!(app.file_manager_width, 600.0);
    assert_eq!(app.open_external_app, ExternalOpenApp::Cursor);
    assert_eq!(app.file_tree_expanded, ["src", "crates"]);
    assert!(app.file_tree_expanded_set.contains("src"));
}

#[test]
fn workspace_mode_bootstrap_preserves_current_mode_for_invalid_values() {
    let mut app = test_app();
    app.workspace_mode = crate::app::state::WorkspaceMode::Work;

    let _ = app.update(Message::BootstrapAppConfig(Ok(json!({
        "workspace_mode": "unsupported"
    }))));

    assert_eq!(app.workspace_mode, crate::app::state::WorkspaceMode::Work);
}

#[test]
fn workspace_mode_bootstrap_accepts_work_mode() {
    let mut app = test_app();

    let _ = app.update(Message::BootstrapAppConfig(Ok(json!({
        "workspace_mode": "work"
    }))));

    assert_eq!(app.workspace_mode, crate::app::state::WorkspaceMode::Work);
}

#[test]
fn startup_app_config_loaded_uses_same_bootstrap_path() {
    let mut app = test_app();

    let _ = app.update(Message::StartupAppConfigLoaded(Ok(json!({
        "model": "startup-model",
        "auto_model": true,
        "file_manager_width": 200.0,
        "open_external_app": "zed"
    }))));

    assert_eq!(app.model, "startup-model");
    assert!(app.auto_model);
    assert_eq!(app.file_manager_width, 200.0);
    assert_eq!(app.file_manager_start_width, 200.0);
    assert_eq!(app.open_external_app, ExternalOpenApp::Zed);
}

#[test]
fn bootstrap_app_config_preserves_invalid_values_and_clamps_minimum_width() {
    let mut app = test_app();
    app.model = "existing-model".to_string();
    app.auto_model = true;
    app.acp_agent = Some("existing-agent".to_string());
    app.open_external_app = ExternalOpenApp::VSCode;

    let _ = app.update(Message::BootstrapAppConfig(Ok(json!({
        "model": 7,
        "auto_model": "yes",
        "acp_agent": null,
        "auto_max_mode": true,
        "show_settings": true,
        "show_file_manager": true,
        "file_manager_show_changes": true,
        "show_terminal": true,
        "file_manager_width": -50.0,
        "open_external_app": "unsupported",
        "file_tree_expanded": ["src", false, "src"]
    }))));

    assert_eq!(app.model, "existing-model");
    assert!(app.auto_model);
    assert_eq!(app.acp_agent.as_deref(), Some("existing-agent"));
    assert!(app.auto_max_mode);
    assert!(app.show_settings);
    assert!(app.show_file_manager);
    assert!(app.file_manager_show_changes);
    assert!(app.terminal.is_visible);
    assert_eq!(app.file_manager_width, 180.0);
    assert_eq!(app.file_manager_start_width, 180.0);
    assert_eq!(app.open_external_app, ExternalOpenApp::VSCode);
    assert_eq!(app.file_tree_expanded, ["src", "src"]);
    assert_eq!(app.file_tree_expanded_set.len(), 1);
}

#[test]
fn bootstrap_browser_config_normalizes_invalid_options() {
    let mut app = test_app();
    let mut browser = vw_config_types::tools::BrowserConfig::default();
    browser.enabled = true;
    browser.allowed_domains = vec!["example.com".to_string(), "docs.example.com".to_string()];
    browser.browser_open = "unsupported".to_string();
    browser.backend = "rust-native".to_string();
    browser.session_name = Some("desk".to_string());
    browser.native_chrome_path = Some("/Applications/Chrome".to_string());
    browser.computer_use.window_allowlist = vec!["Vibe".to_string(), "Terminal".to_string()];
    browser.computer_use.max_coordinate_x = Some(1440);
    browser.computer_use.max_coordinate_y = Some(900);

    let _ = app.update(Message::BootstrapBrowserConfig(Ok(browser)));

    assert!(app.browser_settings.enabled);
    assert_eq!(app.browser_settings.allowed_domains_input, "example.com\ndocs.example.com");
    assert_eq!(app.browser_settings.browser_open, "new_tab");
    assert_eq!(app.browser_settings.backend, "native");
    assert_eq!(app.browser_settings.session_name_input, "desk");
    assert_eq!(app.browser_settings.native_chrome_path_input, "/Applications/Chrome");
    assert_eq!(app.browser_settings.computer_use_window_allowlist_input, "Vibe, Terminal");
    assert_eq!(app.browser_settings.computer_use_max_coordinate_x_input, "1440");
    assert_eq!(app.browser_settings.computer_use_max_coordinate_y_input, "900");
}

#[test]
fn bootstrap_browser_config_accepts_supported_case_insensitive_options() {
    let mut app = test_app();
    let mut browser = vw_config_types::tools::BrowserConfig::default();
    browser.browser_open = " NEW_TAB ".to_string();
    browser.backend = " AGENT-BROWSER ".to_string();
    browser.native_headless = false;
    browser.native_webdriver_url = "http://127.0.0.1:9516".to_string();
    browser.computer_use.endpoint = "http://127.0.0.1:9000".to_string();
    browser.computer_use.api_key = Some("placeholder-key".to_string());
    browser.computer_use.timeout_ms = 4_500;
    browser.computer_use.allow_remote_endpoint = true;

    let _ = app.update(Message::BootstrapBrowserConfig(Ok(browser)));

    assert_eq!(app.browser_settings.browser_open, "new_tab");
    assert_eq!(app.browser_settings.backend, "agent_browser");
    assert!(!app.browser_settings.native_headless);
    assert_eq!(app.browser_settings.native_webdriver_url, "http://127.0.0.1:9516");
    assert_eq!(app.browser_settings.computer_use_endpoint, "http://127.0.0.1:9000");
    assert_eq!(app.browser_settings.computer_use_api_key_input, "placeholder-key");
    assert_eq!(app.browser_settings.computer_use_timeout_ms_input, "4500");
    assert!(app.browser_settings.computer_use_allow_remote_endpoint);
}

#[test]
fn bootstrap_browser_config_accepts_remaining_supported_modes() {
    let mut app = test_app();
    let mut browser = vw_config_types::tools::BrowserConfig::default();
    browser.browser_open = " new_window ".to_string();
    browser.backend = " computer-use ".to_string();

    let _ = app.update(Message::BootstrapBrowserConfig(Ok(browser)));

    assert_eq!(app.browser_settings.browser_open, "new_window");
    assert_eq!(app.browser_settings.backend, "computer_use");

    let mut browser = vw_config_types::tools::BrowserConfig::default();
    browser.browser_open = " default ".to_string();
    browser.backend = " auto ".to_string();

    let _ = app.update(Message::StartupBrowserConfigLoaded(Ok(browser)));

    assert_eq!(app.browser_settings.browser_open, "default");
    assert_eq!(app.browser_settings.backend, "auto");
}

#[test]
fn bootstrap_system_settings_updates_runtime_timezone_and_normalizes_active_gateway() {
    let mut app = test_app();
    let stale_health_key = "http://stale.example:99/v1/health".to_string();
    app.gateway_client_settings.health.insert(stale_health_key.clone(), true);

    let mut system = vw_config_types::ui::AppSystemSettingsConfig::default();
    system.terminal_font_family = "Test Mono".to_string();
    system.terminal_font_size = 15.0;
    system.timezone = "America/New_York".to_string();
    system.project_worktree_enabled =
        std::collections::HashMap::from([("/workspace/demo".to_string(), true)]);
    system.dialogue_flow_show_reasoning_summary = false;
    system.dialogue_flow_expand_shell_tool_section = true;
    system.dialogue_flow_expand_edit_tool_section = true;
    system.preview_auto_save = vw_config_types::ui::PreviewAutoSaveMode::OnWindowChange;
    system.editor_follow_system_theme = false;
    system.app_theme = "missing-theme".to_string();
    system.editor_theme = "missing-editor-theme".to_string();
    system.gateway_client.active_server_id = "remote".to_string();
    system.gateway_client.servers = vec![vw_config_types::ui::GatewayClientServerConfig {
        id: "remote".to_string(),
        name: "Remote".to_string(),
        host: "  ".to_string(),
        port: 0,
        platform_id: "platform".to_string(),
        company_id: "company".to_string(),
        bearer_token: String::new(),
        username: String::new(),
        password: String::new(),
        skey: "placeholder-key".to_string(),
    }];
    let original_theme = app.app_theme.clone();

    let _ = app.update(Message::BootstrapSystemSettings(Ok(system)));

    assert_eq!(app.terminal.font_family, "Test Mono");
    assert_eq!(app.terminal.font_size, 15.0);
    assert_eq!(app.chat_timezone, "America/New_York");
    assert_eq!(app.project_worktree_enabled.get("/workspace/demo"), Some(&true));
    assert!(!app.dialogue_flow_show_reasoning_summary);
    assert!(app.dialogue_flow_expand_shell_tool_section);
    assert!(app.dialogue_flow_expand_edit_tool_section);
    assert_eq!(
        app.preview_auto_save_mode,
        vw_config_types::ui::PreviewAutoSaveMode::OnWindowChange
    );
    assert!(!app.editor_follow_system_theme);
    assert_eq!(app.app_theme, original_theme);
    assert_eq!(app.editor_theme, original_theme);
    assert_eq!(app.gateway_client_settings.selected_server_id, "remote");
    assert_eq!(app.gateway_client_settings.host_input, "127.0.0.1");
    assert_eq!(app.gateway_client_settings.port, 1);
    assert_eq!(app.gateway_client_settings.skey_input, "placeholder-key");
    assert!(!app.gateway_client_settings.health.contains_key(&stale_health_key));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn bootstrap_system_settings_applies_lsp_enabled_state() {
    let mut app = test_app();
    app.lsp_disabled = true;
    let mut system = vw_config_types::ui::AppSystemSettingsConfig::default();
    system.lsp_enabled = true;

    app.apply_bootstrap_system_settings(system.clone());

    assert!(!app.lsp_disabled);

    system.lsp_enabled = false;
    app.apply_bootstrap_system_settings(system);

    assert!(app.lsp_disabled);
}

#[test]
fn app_bootstrap_preserves_custom_items_and_adds_missing_builtin_git_actions() {
    let mut app = test_app();
    let mut system = vw_config_types::ui::AppSystemSettingsConfig::default();
    system.quick_messages = vec![vw_config_types::ui::QuickMessageConfig {
        text: "提交当前分支".to_string(),
        group: vw_config_types::ui::QuickMessageGroup::Git,
        kind: vw_config_types::ui::QuickMessageKind::Message,
    }];

    let _ = app.update(Message::BootstrapSystemSettings(Ok(system.clone())));

    assert_eq!(app.quick_messages.first(), system.quick_messages.first());
    assert_eq!(app.quick_messages[1..], vw_config_types::ui::builtin_git_quick_messages());

    system.quick_messages.clear();
    let _ = app.update(Message::BootstrapSystemSettings(Ok(system)));

    assert!(app.quick_messages.is_empty());
}

#[test]
fn failed_bootstrap_results_preserve_existing_state() {
    let mut app = test_app();
    app.model = "existing-model".to_string();
    app.acp_agents = vec!["existing-agent".to_string()];
    app.archived_session_ids = std::collections::HashSet::from(["existing-session".to_string()]);
    app.open_external_app = ExternalOpenApp::Cursor;

    let _ = app.update(Message::BootstrapAppConfig(Err("app config unavailable".to_string())));
    let _ =
        app.update(Message::BootstrapSystemSettings(Err("system config unavailable".to_string())));
    let _ =
        app.update(Message::BootstrapBrowserConfig(Err("browser config unavailable".to_string())));
    let _ = app.update(Message::BootstrapAcpAgentsLoaded(Err("agents unavailable".to_string())));
    let _ = app.update(Message::BootstrapArchivedSessions(Err("archive unavailable".to_string())));
    let _ = app.update(Message::ExternalAppsLoaded(Err("apps unavailable".to_string())));

    assert_eq!(app.model, "existing-model");
    assert_eq!(app.acp_agents, ["existing-agent"]);
    assert_eq!(app.archived_session_ids.len(), 1);
    assert!(app.archived_session_ids.contains("existing-session"));
    assert_eq!(app.open_external_app, ExternalOpenApp::Cursor);
}

#[test]
fn update_loads_acp_agents_archived_sessions_and_preferences_conditionally() {
    let mut app = test_app();
    app.project_path = Some("/workspace/app".to_string());

    let _ = app.update(Message::BootstrapAcpAgentsLoaded(Ok(vec![
        "zeta".to_string(),
        "codex".to_string(),
        "alpha".to_string(),
    ])));
    assert_eq!(app.acp_agents, ["codex", "alpha", "zeta"]);

    let archived = std::collections::HashSet::from(["session-1".to_string()]);
    let _ = app.update(Message::BootstrapArchivedSessions(Ok(archived.clone())));
    assert_eq!(app.archived_session_ids, archived);

    let _ = app.update(Message::ProjectChatPreferencesLoaded(
        "/workspace/other".to_string(),
        Some(("ignored".to_string(), true, None)),
    ));
    assert_ne!(app.model, "ignored");

    let _ = app.update(Message::ProjectChatPreferencesLoaded(
        "/workspace/app".to_string(),
        Some(("matched".to_string(), false, Some("codex".to_string()))),
    ));
    assert_eq!(app.model, "matched");
    assert!(!app.auto_model);
    assert_eq!(app.acp_agent.as_deref(), Some("codex"));
}

#[test]
fn update_handles_external_apps_and_session_preview_errors() {
    let mut app = test_app();
    app.session_previews =
        std::collections::HashMap::from([("session".to_string(), "preview".to_string())]);

    let _ = app.update(Message::ExternalAppsLoaded(Ok(vw_gateway_client::ExternalAppsStateDto {
        platform: Some("windows".to_string()),
        apps: vec![("powershell".to_string(), true)],
    })));
    assert_eq!(app.open_external_platform, Some(RuntimePlatform::Windows));
    assert_eq!(app.open_external_app, ExternalOpenApp::PowerShell);

    let _ = app.update(Message::SessionPreviewsLoaded(Err("offline".to_string())));
    assert!(app.session_previews.is_empty());
    assert!(app.notifications.iter().any(|notification| {
        notification.message.contains("Failed to load session previews: offline")
    }));
}

#[test]
fn session_previews_loaded_builds_only_non_empty_content() {
    let mut app = test_app();
    let previews = vec![
        crate::app::models::ChatSessionMeta {
            id: "kept".to_string(),
            title: "Kept".to_string(),
            updated_ms: 1,
            message_count: 2,
            call_count: 0,
            last_content: Some(" first line\nsecond line ".to_string()),
        },
        crate::app::models::ChatSessionMeta {
            id: "empty".to_string(),
            title: "Empty".to_string(),
            updated_ms: 2,
            message_count: 1,
            call_count: 0,
            last_content: Some("   ".to_string()),
        },
        crate::app::models::ChatSessionMeta {
            id: "missing".to_string(),
            title: "Missing".to_string(),
            updated_ms: 3,
            message_count: 0,
            call_count: 0,
            last_content: None,
        },
    ];

    let _ = app.update(Message::SessionPreviewsLoaded(Ok(previews)));

    assert_eq!(app.session_previews.len(), 1);
    assert_eq!(
        app.session_previews.get("kept").map(String::as_str),
        Some("first line second line")
    );
}

#[test]
fn gateway_health_checked_retains_known_servers_and_ignores_unknown_results() {
    let mut app = test_app();
    let server = crate::app::state::GatewayClientServerDraft {
        id: "known".to_string(),
        name: "Known".to_string(),
        host: "gateway.example".to_string(),
        port: 4242,
        platform_id: "platform".to_string(),
        company_id: "company".to_string(),
        skey: String::new(),
    };
    let known_key = crate::app::message::gateway_health::server_health_key(&server)
        .expect("known server has a health key");
    let stale_key = "http://stale.example:99/v1/health".to_string();
    app.gateway_client_settings.servers = vec![server];
    app.gateway_client_settings.health.insert(known_key.clone(), false);
    app.gateway_client_settings.health.insert(stale_key.clone(), true);

    let _ = app.update(Message::GatewayHealthChecked(vec![
        (known_key.clone(), true),
        (stale_key.clone(), false),
    ]));

    assert_eq!(app.gateway_client_settings.health.len(), 1);
    assert_eq!(app.gateway_client_settings.health.get(&known_key), Some(&true));
    assert!(!app.gateway_client_settings.health.contains_key(&stale_key));
}

#[test]
fn selected_gateway_health_transition_triggers_only_when_becoming_healthy() {
    let mut app = test_app();
    let selected_server = app.gateway_client_settings.servers[0].clone();
    let selected_key = crate::app::message::gateway_health::server_health_key(&selected_server)
        .expect("selected server has a health key");
    let other_key = "http://other.example:42617/v1/health".to_string();

    assert!(selected_gateway_became_healthy(
        &app.gateway_client_settings,
        &[(selected_key.clone(), true)],
    ));
    assert!(!selected_gateway_became_healthy(
        &app.gateway_client_settings,
        &[(other_key, true)],
    ));

    app.gateway_client_settings.health.insert(selected_key.clone(), false);
    assert!(selected_gateway_became_healthy(
        &app.gateway_client_settings,
        &[(selected_key.clone(), true)],
    ));

    app.gateway_client_settings.health.insert(selected_key.clone(), true);
    assert!(!selected_gateway_became_healthy(
        &app.gateway_client_settings,
        &[(selected_key, true)],
    ));
}

#[test]
fn copy_code_records_content_hash_without_executing_clipboard_task() {
    use std::hash::{Hash, Hasher};

    let mut app = test_app();
    let content = "copy me";
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    content.hash(&mut hasher);
    let expected_hash = hasher.finish();

    let _clipboard_task = app.update(Message::CopyCode(content.to_string()));

    assert_eq!(app.last_copied_code_hash, Some(expected_hash));
    assert!(app.last_copy_time.is_some());
}

#[test]
fn copy_file_missing_path_records_empty_content_hash_without_panicking() {
    use std::hash::{Hash, Hasher};

    let mut app = test_app();
    let missing_path =
        std::env::temp_dir().join(format!("vibe-window-missing-copy-file-{}", std::process::id()));
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    String::new().hash(&mut hasher);
    let expected_hash = hasher.finish();

    let _clipboard_task =
        app.update(Message::CopyFile(missing_path.to_string_lossy().into_owned()));

    assert_eq!(app.last_copied_code_hash, Some(expected_hash));
    assert!(app.last_copy_time.is_some());
}

#[test]
fn copy_feedback_expiration_only_clears_matching_hash_and_close_error_clears_message() {
    let mut app = test_app();
    app.last_copied_code_hash = Some(11);
    app.last_copy_time = Some(web_time::SystemTime::now());

    let _ = app.update(Message::CopyFeedbackExpired(12));
    assert_eq!(app.last_copied_code_hash, Some(11));
    assert!(app.last_copy_time.is_some());

    let _ = app.update(Message::CopyFeedbackExpired(11));
    assert_eq!(app.last_copied_code_hash, None);
    assert_eq!(app.last_copy_time, None);

    app.error_message = Some("boom".to_string());
    let _ = app.update(Message::CloseError);
    assert_eq!(app.error_message, None);
}

#[test]
fn batch_message_applies_each_nested_update_in_order() {
    let mut app = test_app();
    app.error_message = Some("boom".to_string());
    app.last_copied_code_hash = Some(99);
    app.last_copy_time = Some(web_time::SystemTime::now());

    let _ = app.update(Message::Batch(vec![
        Message::BootstrapAcpAgentsLoaded(Ok(vec![
            "zeta".to_string(),
            "codex".to_string(),
            "alpha".to_string(),
        ])),
        Message::CopyFeedbackExpired(99),
        Message::CloseError,
        Message::None,
    ]));

    assert_eq!(app.acp_agents, ["codex", "alpha", "zeta"]);
    assert_eq!(app.last_copied_code_hash, None);
    assert_eq!(app.last_copy_time, None);
    assert_eq!(app.error_message, None);
}
