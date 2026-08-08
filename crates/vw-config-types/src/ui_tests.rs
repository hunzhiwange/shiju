use super::{
    AppSystemSettingsConfig, AppUiConfig, DEFAULT_TIMEZONE, GatewayClientServerConfig,
    GatewayClientSystemSettingsConfig, ModelRoute, PreviewAutoSaveMode, QuickMessageConfig,
    QuickMessageGroup, QuickMessageKind, append_missing_builtin_general_quick_messages,
    append_missing_builtin_git_quick_messages, default_timezone, timezone_options,
    upgrade_legacy_quick_message_actions,
};
use std::collections::HashSet;

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
    assert_eq!(settings.gateway_client.port, crate::gateway::DEFAULT_GATEWAY_PORT);
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
    assert!(!settings.dialogue_flow_show_reasoning_summary);
    assert!(!settings.dialogue_flow_expand_shell_tool_section);
    assert!(!settings.dialogue_flow_expand_edit_tool_section);
    assert!(settings.tui_keybindings.is_empty());
    assert_eq!(settings.preview_auto_save, PreviewAutoSaveMode::OnFocusChange);
}

#[test]
fn app_system_settings_timezone_defaults_to_shanghai() {
    let defaults = AppSystemSettingsConfig::default();
    let missing: AppSystemSettingsConfig =
        serde_json::from_value(serde_json::json!({})).expect("missing timezone should use default");

    assert_eq!(DEFAULT_TIMEZONE, "Asia/Shanghai");
    assert_eq!(default_timezone(), DEFAULT_TIMEZONE);
    assert_eq!(defaults.timezone, DEFAULT_TIMEZONE);
    assert_eq!(missing.timezone, DEFAULT_TIMEZONE);
}

#[test]
fn app_system_settings_timezone_accepts_iana_and_rejects_invalid_values() {
    let settings: AppSystemSettingsConfig = serde_json::from_value(serde_json::json!({
        "timezone": "America/New_York"
    }))
    .expect("valid IANA timezone should deserialize");
    let encoded = serde_json::to_value(&settings).expect("timezone should serialize");

    assert_eq!(settings.timezone, "America/New_York");
    assert_eq!(encoded["timezone"], "America/New_York");

    let error = serde_json::from_value::<AppSystemSettingsConfig>(serde_json::json!({
        "timezone": "Mars/Olympus_Mons"
    }))
    .expect_err("invalid IANA timezone should be rejected");
    assert!(error.to_string().contains("invalid IANA timezone"));
}

#[test]
fn timezone_options_are_unique_and_include_default() {
    let options = timezone_options();
    let unique = options.iter().copied().collect::<HashSet<_>>();

    assert!(options.contains(&DEFAULT_TIMEZONE));
    assert_eq!(unique.len(), options.len());
}

#[test]
fn quick_messages_default_missing_and_empty_values_have_distinct_semantics() {
    let expected_texts = [
        "继续推进",
        "执行下一步",
        "摘要并生成下一个任务的提示词",
        "摘要并创建新会话",
        "为剩余任务制定开发计划",
        "不要兼容旧的代码",
        "修复单元测试",
        "给代码增加单元测试",
        "尽你最大的能力突破阻塞",
        "不对啊，完全不对啊，你仔细检查一下",
        "你实现得太简单了，做得细节一点",
        "多次修改都不对，你这一次需要仔细深度思考解决问题",
        "不要修改代码，这次只做分析和计划",
        "从其他语言移植代码，需要尽量一一对齐，不要尝试简化实现",
        "从第一个任务开始，直到所有任务全部完成才结束，不要轻易结束。",
        "这是一个开放性的需求，你可以自由发挥，做得细节一点，希望给我惊喜",
        "请选择合适的技能，给我制定计划，但是不要急着修改代码",
        "先复述你理解的目标、约束和验收标准，确认没有遗漏后再开始",
        "先定位根因和影响范围，不要用临时绕过掩盖底层问题",
        "请沿完整调用链检查数据流、状态流和错误流，再决定修改位置",
        "请主动覆盖边界条件、并发时序、空状态和失败恢复",
        "完成后逐条对照需求做自我验收，有偏差就继续修正",
        "请检查是否破坏现有行为，并补充能防止回归的测试",
        "请优先复用现有架构和组件，避免无必要的新抽象与依赖",
        "请站在最终用户视角打磨交互、文案、反馈和异常体验",
        "对不确定的假设先用代码和测试验证，不要凭记忆直接实现",
        "交付前检查修改范围、格式、静态检查和相关测试，明确报告结果",
        "刷新 Git 变更",
        "全选 Git 变更",
        "反选 Git 变更",
        "暂存全部变更",
        "暂存并提交全部变更",
        "提交已选变更",
    ];

    let defaults = AppSystemSettingsConfig::default();
    assert_eq!(
        defaults.quick_messages.iter().map(|item| item.text.as_str()).collect::<Vec<_>>(),
        expected_texts
    );
    assert!(
        defaults.quick_messages[..27].iter().all(|item| item.group == QuickMessageGroup::General)
    );
    assert!(defaults.quick_messages[27..].iter().all(|item| item.group == QuickMessageGroup::Git));
    assert_eq!(defaults.quick_messages[3].kind, QuickMessageKind::SummarizeAndCreateNewSession);
    assert!(defaults.quick_messages[..2].iter().all(|item| item.kind == QuickMessageKind::Message));
    assert!(
        defaults.quick_messages[2..27]
            .iter()
            .enumerate()
            .all(|(index, item)| index == 1 || item.kind == QuickMessageKind::FillInput)
    );
    assert_eq!(
        defaults.quick_messages[27..].iter().map(|item| item.kind).collect::<Vec<_>>(),
        vec![
            QuickMessageKind::RefreshGitChanges,
            QuickMessageKind::SelectAllGitChanges,
            QuickMessageKind::InvertGitSelection,
            QuickMessageKind::StageAllGitChanges,
            QuickMessageKind::StageAllAndCommitGitChanges,
            QuickMessageKind::CommitSelectedGitChanges,
        ]
    );
    assert_eq!(
        QuickMessageKind::ALL.map(QuickMessageKind::action_badge),
        [
            None,
            Some("发送"),
            Some("新会话"),
            Some("摘要并新建"),
            Some("刷新"),
            Some("全选"),
            Some("反选"),
            Some("暂存"),
            Some("暂存并提交"),
            Some("提交"),
        ]
    );
    assert_eq!(QuickMessageGroup::General.label(), "常用操作");
    assert_eq!(QuickMessageGroup::Git.label(), "Git 操作");
    assert_eq!(QuickMessageKind::FillInput.label(), "填入输入框");

    let missing: AppSystemSettingsConfig = serde_json::from_value(serde_json::json!({})).unwrap();
    assert_eq!(missing.quick_messages, defaults.quick_messages);

    let empty: AppSystemSettingsConfig =
        serde_json::from_value(serde_json::json!({"quick_messages": []})).unwrap();
    assert!(empty.quick_messages.is_empty());

    let custom = QuickMessageConfig {
        text: "创建新会话".to_string(),
        group: QuickMessageGroup::Git,
        kind: QuickMessageKind::CreateNewSession,
    };
    let encoded = serde_json::to_value(&custom).unwrap();
    assert_eq!(encoded["group"], "git");
    assert_eq!(encoded["kind"], "createNewSession");
    assert_eq!(serde_json::from_value::<QuickMessageConfig>(encoded).unwrap(), custom);
    assert_eq!(
        serde_json::to_value(QuickMessageKind::FillInput).unwrap(),
        serde_json::json!("fillInput")
    );
}

#[test]
fn nonempty_legacy_quick_messages_gain_missing_builtin_git_actions() {
    let defaults = AppSystemSettingsConfig::default();
    let mut legacy = defaults.quick_messages[..5].to_vec();
    legacy[2].kind = QuickMessageKind::Message;
    legacy[3].kind = QuickMessageKind::CreateNewSession;
    legacy[4].kind = QuickMessageKind::Message;

    upgrade_legacy_quick_message_actions(&mut legacy);
    append_missing_builtin_general_quick_messages(&mut legacy);
    append_missing_builtin_git_quick_messages(&mut legacy);

    assert_eq!(legacy, defaults.quick_messages);
    append_missing_builtin_general_quick_messages(&mut legacy);
    append_missing_builtin_git_quick_messages(&mut legacy);
    assert_eq!(legacy, defaults.quick_messages);

    let mut empty = Vec::new();
    append_missing_builtin_general_quick_messages(&mut empty);
    append_missing_builtin_git_quick_messages(&mut empty);
    assert!(empty.is_empty());
}

#[test]
fn existing_custom_quick_messages_gain_new_general_builtins_without_duplicates() {
    let mut messages = vec![
        QuickMessageConfig {
            text: "自定义消息".to_string(),
            group: QuickMessageGroup::General,
            kind: QuickMessageKind::Message,
        },
        QuickMessageConfig {
            text: " 修复单元测试 ".to_string(),
            group: QuickMessageGroup::General,
            kind: QuickMessageKind::Message,
        },
        QuickMessageConfig {
            text: "刷新 Git 变更".to_string(),
            group: QuickMessageGroup::Git,
            kind: QuickMessageKind::RefreshGitChanges,
        },
    ];

    upgrade_legacy_quick_message_actions(&mut messages);
    append_missing_builtin_general_quick_messages(&mut messages);

    assert_eq!(messages.iter().filter(|item| item.text.trim() == "修复单元测试").count(), 1);
    assert_eq!(messages[1].kind, QuickMessageKind::FillInput);
    assert!(messages.iter().any(|item| item.text == "不要兼容旧的代码"));
    let first_git = messages
        .iter()
        .position(|item| item.group == QuickMessageGroup::Git)
        .expect("custom Git action should remain");
    assert!(messages[..first_git].iter().all(|item| item.group == QuickMessageGroup::General));
    assert_eq!(messages.len(), 29);
}

#[test]
fn legacy_builtin_kinds_are_upgraded_without_changing_custom_items() {
    let mut messages = vec![
        QuickMessageConfig {
            text: "摘要并创建新会话".to_string(),
            group: QuickMessageGroup::General,
            kind: QuickMessageKind::CreateNewSession,
        },
        QuickMessageConfig {
            text: "直接新建".to_string(),
            group: QuickMessageGroup::General,
            kind: QuickMessageKind::CreateNewSession,
        },
        QuickMessageConfig {
            text: "修复单元测试".to_string(),
            group: QuickMessageGroup::General,
            kind: QuickMessageKind::Message,
        },
        QuickMessageConfig {
            text: "自定义消息".to_string(),
            group: QuickMessageGroup::General,
            kind: QuickMessageKind::Message,
        },
    ];

    upgrade_legacy_quick_message_actions(&mut messages);

    assert_eq!(messages[0].kind, QuickMessageKind::SummarizeAndCreateNewSession);
    assert_eq!(messages[1].kind, QuickMessageKind::CreateNewSession);
    assert_eq!(messages[2].kind, QuickMessageKind::FillInput);
    assert_eq!(messages[3].kind, QuickMessageKind::Message);
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
    assert_eq!(settings.gateway_client.port, crate::gateway::DEFAULT_GATEWAY_PORT);
    assert_eq!(settings.preview_auto_save, PreviewAutoSaveMode::AfterDelay);
    assert!(!settings.dialogue_flow_show_reasoning_summary);
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
    assert_eq!(server.port, crate::gateway::DEFAULT_GATEWAY_PORT);
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
    assert_eq!(settings.port, crate::gateway::DEFAULT_GATEWAY_PORT);
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
