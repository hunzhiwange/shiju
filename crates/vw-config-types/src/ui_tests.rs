use super::{
    AppSystemSettingsConfig, AppUiConfig, GatewayClientServerConfig,
    GatewayClientSystemSettingsConfig, ModelRoute, PreviewAutoSaveMode,
};

#[test]
fn preview_auto_save_modes_have_stable_labels_and_display_text() {
    let labels = PreviewAutoSaveMode::ALL.map(PreviewAutoSaveMode::label);

    assert_eq!(PreviewAutoSaveMode::default(), PreviewAutoSaveMode::OnFocusChange);
    assert_eq!(labels, ["关闭", "延迟保存", "编辑器失焦时保存", "窗口失焦时保存"]);
    assert_eq!(PreviewAutoSaveMode::AfterDelay.to_string(), "延迟保存");
}

#[test]
fn app_system_settings_default_matches_desktop_contract() {
    let settings = AppSystemSettingsConfig::default();

    assert_eq!(settings.gateway_client.host, "127.0.0.1");
    assert_eq!(settings.gateway_client.port, 42617);
    assert_eq!(settings.app_theme, "Light");
    assert_eq!(settings.terminal_shell, "zsh");
    assert_eq!(settings.terminal_theme, "system");
    assert_eq!(settings.terminal_font_family, "JetBrains Mono");
    assert_eq!(settings.terminal_font_size, 13.0);
    assert!(settings.editor_follow_system_theme);
    assert_eq!(settings.editor_theme, "Light");
    assert_eq!(settings.editor_font_size, 14.0);
    assert_eq!(settings.editor_line_height, 20.0);
    assert!(settings.editor_auto_line_height);
    assert!(settings.dialogue_flow_show_reasoning_summary);
    assert!(!settings.dialogue_flow_expand_shell_tool_section);
    assert!(!settings.dialogue_flow_expand_edit_tool_section);
    assert_eq!(settings.preview_auto_save, PreviewAutoSaveMode::OnFocusChange);
}

#[test]
fn serde_defaults_fill_nested_ui_config() {
    let config: AppUiConfig = serde_json::from_value(serde_json::json!({})).unwrap();
    let settings: AppSystemSettingsConfig = serde_json::from_value(serde_json::json!({
        "gateway_client": {
            "host": "10.0.0.8",
            "servers": [{
                "id": "",
                "name": "",
                "host": "gw.internal",
                "port": 8080
            }]
        },
        "preview_auto_save": "afterDelay",
        "model_routes": [{
            "pattern": "code/*",
            "provider": "openai",
            "model": "gpt-5.4",
            "priority": 9
        }]
    }))
    .unwrap();

    assert_eq!(config.system_settings, AppSystemSettingsConfig::default());
    assert_eq!(settings.gateway_client.host, "10.0.0.8");
    assert_eq!(settings.gateway_client.port, 42617);
    assert_eq!(settings.preview_auto_save, PreviewAutoSaveMode::AfterDelay);
    assert_eq!(
        settings.model_routes,
        vec![ModelRoute {
            pattern: "code/*".to_string(),
            provider: "openai".to_string(),
            model: "gpt-5.4".to_string(),
            priority: 9,
        }]
    );
}

#[test]
fn normalized_servers_falls_back_repairs_blanks_and_selects_active() {
    let gateway = GatewayClientSystemSettingsConfig {
        host: "gateway.local".to_string(),
        port: 19999,
        platform_id: "10000".to_string(),
        company_id: "100000000".to_string(),
        bearer_token: "token".to_string(),
        username: "user".to_string(),
        password: "pass".to_string(),
        skey: "skey".to_string(),
        active_server_id: String::new(),
        servers: Vec::new(),
    };
    let servers = gateway.normalized_servers();
    assert_eq!(servers[0].id, "local");
    assert_eq!(servers[0].name, "本地网关");
    assert_eq!(servers[0].host, "gateway.local");
    assert_eq!(servers[0].port, 19999);
    assert_eq!(servers[0].bearer_token, "token");

    let gateway = GatewayClientSystemSettingsConfig {
        active_server_id: "remote".to_string(),
        servers: vec![
            GatewayClientServerConfig {
                id: " ".to_string(),
                name: " ".to_string(),
                host: "one".to_string(),
                ..GatewayClientServerConfig::default()
            },
            GatewayClientServerConfig {
                id: "remote".to_string(),
                name: "".to_string(),
                host: "two".to_string(),
                ..GatewayClientServerConfig::default()
            },
        ],
        ..GatewayClientSystemSettingsConfig::default()
    };
    let normalized = gateway.normalized_servers();
    assert_eq!(normalized[0].id, "local");
    assert_eq!(normalized[0].name, "本地网关");
    assert_eq!(normalized[1].name, "网关 2");
    assert_eq!(gateway.active_server().id, "remote");
}

#[test]
fn set_servers_syncs_active_connection_fields_and_falls_back() {
    let mut gateway = GatewayClientSystemSettingsConfig::default();
    let local = GatewayClientServerConfig {
        id: "local".to_string(),
        host: "127.0.0.1".to_string(),
        port: 42617,
        ..GatewayClientServerConfig::default()
    };
    let remote = GatewayClientServerConfig {
        id: "remote".to_string(),
        name: "Remote".to_string(),
        host: "remote.example".to_string(),
        port: 443,
        platform_id: "20000".to_string(),
        company_id: "200000001".to_string(),
        bearer_token: "token".to_string(),
        username: "u".to_string(),
        password: "p".to_string(),
        skey: "s".to_string(),
    };

    gateway.set_servers(vec![local.clone(), remote.clone()], "remote".to_string());

    assert_eq!(gateway.active_server_id, "remote");
    assert_eq!(gateway.host, "remote.example");
    assert_eq!(gateway.port, 443);
    assert_eq!(gateway.platform_id, "20000");
    assert_eq!(gateway.company_id, "200000001");
    assert_eq!(gateway.bearer_token, "token");
    assert_eq!(gateway.username, "u");
    assert_eq!(gateway.password, "p");
    assert_eq!(gateway.skey, "s");
    assert_eq!(gateway.servers, vec![local, remote]);

    gateway.set_servers(Vec::new(), "missing".to_string());
    assert_eq!(gateway.active_server_id, "local");
    assert_eq!(gateway.servers, vec![GatewayClientServerConfig::default()]);
}

#[test]
fn preview_auto_save_mode_descriptions() {
    assert_eq!(PreviewAutoSaveMode::Off.description(), "不自动保存，需手动保存当前文件。");
    assert_eq!(
        PreviewAutoSaveMode::AfterDelay.description(),
        "编辑内容变更后，等待短暂延迟再自动保存。"
    );
    assert_eq!(
        PreviewAutoSaveMode::OnFocusChange.description(),
        "当编辑器失去焦点时，自动保存当前文件。"
    );
    assert_eq!(
        PreviewAutoSaveMode::OnWindowChange.description(),
        "当应用窗口失去焦点时，自动保存当前文件。"
    );
}

#[test]
fn preview_auto_save_mode_default_is_on_focus_change() {
    assert_eq!(PreviewAutoSaveMode::default(), PreviewAutoSaveMode::OnFocusChange);
}

#[test]
fn preview_auto_save_mode_serde_roundtrip() {
    for variant in PreviewAutoSaveMode::ALL {
        let json = serde_json::to_string(&variant).unwrap();
        let deserialized: PreviewAutoSaveMode = serde_json::from_str(&json).unwrap();
        assert_eq!(variant, deserialized);
    }
}

#[test]
fn preview_auto_save_mode_serde_camel_case_names() {
    assert_eq!(serde_json::to_value(PreviewAutoSaveMode::Off).unwrap(), serde_json::json!("off"));
    assert_eq!(
        serde_json::to_value(PreviewAutoSaveMode::AfterDelay).unwrap(),
        serde_json::json!("afterDelay")
    );
    assert_eq!(
        serde_json::to_value(PreviewAutoSaveMode::OnFocusChange).unwrap(),
        serde_json::json!("onFocusChange")
    );
    assert_eq!(
        serde_json::to_value(PreviewAutoSaveMode::OnWindowChange).unwrap(),
        serde_json::json!("onWindowChange")
    );
}

#[test]
fn preview_auto_save_mode_constant_is_all_variants() {
    assert_eq!(
        PreviewAutoSaveMode::ALL,
        [
            PreviewAutoSaveMode::Off,
            PreviewAutoSaveMode::AfterDelay,
            PreviewAutoSaveMode::OnFocusChange,
            PreviewAutoSaveMode::OnWindowChange,
        ]
    );
}

#[test]
fn app_ui_config_default() {
    let config = AppUiConfig::default();
    assert_eq!(config.system_settings, AppSystemSettingsConfig::default());
}

#[test]
fn app_ui_config_deserializes_empty_object() {
    let config: AppUiConfig = serde_json::from_value(serde_json::json!({})).unwrap();
    assert_eq!(config, AppUiConfig::default());
}

#[test]
fn gateway_client_server_config_default() {
    let server = GatewayClientServerConfig::default();
    assert_eq!(server.id, "local");
    assert_eq!(server.name, "本地网关");
    assert_eq!(server.host, "127.0.0.1");
    assert_eq!(server.port, 42617);
    assert_eq!(server.platform_id, "10000");
    assert_eq!(server.company_id, "100000000");
    assert!(server.bearer_token.is_empty());
    assert!(server.username.is_empty());
    assert!(server.password.is_empty());
    assert!(server.skey.is_empty());
}

#[test]
fn gateway_client_system_settings_config_default() {
    let settings = GatewayClientSystemSettingsConfig::default();
    assert_eq!(settings.host, "127.0.0.1");
    assert_eq!(settings.port, 42617);
    assert_eq!(settings.platform_id, "10000");
    assert_eq!(settings.company_id, "100000000");
    assert!(settings.bearer_token.is_empty());
    assert_eq!(settings.active_server_id, "local");
    assert_eq!(settings.servers, vec![GatewayClientServerConfig::default()]);
}

#[test]
fn gateway_client_tenant_fields_default_serialize_and_reject_blanks() {
    let server: GatewayClientServerConfig = serde_json::from_value(serde_json::json!({
        "id": "legacy"
    }))
    .unwrap();
    assert_eq!(server.platform_id, "10000");
    assert_eq!(server.company_id, "100000000");

    let json = serde_json::to_value(&server).unwrap();
    assert_eq!(json["platform_id"], "10000");
    assert_eq!(json["company_id"], "100000000");

    assert!(
        serde_json::from_value::<GatewayClientServerConfig>(serde_json::json!({
            "platform_id": " ",
            "company_id": "100000000"
        }))
        .is_err()
    );
    assert!(
        serde_json::from_value::<GatewayClientSystemSettingsConfig>(serde_json::json!({
            "platform_id": "10000",
            "company_id": "\t"
        }))
        .is_err()
    );
}

#[test]
fn gateway_client_tenant_fields_preserve_explicit_non_empty_values() {
    let server: GatewayClientServerConfig = serde_json::from_value(serde_json::json!({
        "platform_id": " 20000 ",
        "company_id": "200000001"
    }))
    .expect("non-empty tenant ids deserialize");

    assert_eq!(server.platform_id, " 20000 ");
    assert_eq!(server.company_id, "200000001");
}

#[test]
fn model_route_serde_roundtrip() {
    let route = ModelRoute {
        pattern: "test/*".to_string(),
        provider: "anthropic".to_string(),
        model: "claude-sonnet-4-6".to_string(),
        priority: 10,
    };
    let json = serde_json::to_value(&route).unwrap();
    assert_eq!(json["pattern"], "test/*");
    assert_eq!(json["provider"], "anthropic");
    assert_eq!(json["model"], "claude-sonnet-4-6");
    assert_eq!(json["priority"], 10);

    let deserialized: ModelRoute = serde_json::from_value(json).unwrap();
    assert_eq!(route, deserialized);
}

#[test]
fn model_route_default_fields() {
    let route: ModelRoute = serde_json::from_value(serde_json::json!({})).unwrap();
    assert!(route.pattern.is_empty());
    assert!(route.provider.is_empty());
    assert!(route.model.is_empty());
    assert_eq!(route.priority, 0);
}

#[test]
fn active_server_finds_by_id() {
    let gateway = GatewayClientSystemSettingsConfig {
        active_server_id: "remote".to_string(),
        servers: vec![
            GatewayClientServerConfig {
                id: "local".to_string(),
                host: "127.0.0.1".to_string(),
                ..GatewayClientServerConfig::default()
            },
            GatewayClientServerConfig {
                id: "remote".to_string(),
                host: "remote.example".to_string(),
                ..GatewayClientServerConfig::default()
            },
        ],
        ..GatewayClientSystemSettingsConfig::default()
    };
    let active = gateway.active_server();
    assert_eq!(active.id, "remote");
    assert_eq!(active.host, "remote.example");
}

#[test]
fn active_server_falls_back_to_first_when_id_not_found() {
    let gateway = GatewayClientSystemSettingsConfig {
        active_server_id: "nonexistent".to_string(),
        servers: vec![GatewayClientServerConfig {
            id: "default".to_string(),
            host: "10.0.0.1".to_string(),
            ..GatewayClientServerConfig::default()
        }],
        ..GatewayClientSystemSettingsConfig::default()
    };
    let active = gateway.active_server();
    assert_eq!(active.id, "default");
    assert_eq!(active.host, "10.0.0.1");
}

#[test]
fn set_servers_preserves_existing_when_no_match() {
    let mut gateway = GatewayClientSystemSettingsConfig::default();
    let server = GatewayClientServerConfig {
        id: "alpha".to_string(),
        host: "alpha.local".to_string(),
        ..GatewayClientServerConfig::default()
    };
    gateway.set_servers(vec![server.clone()], "beta".to_string());
    assert_eq!(gateway.active_server_id, "alpha");
    assert_eq!(gateway.host, "alpha.local");
    assert_eq!(gateway.servers, vec![server]);
}
