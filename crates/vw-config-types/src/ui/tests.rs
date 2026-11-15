use super::{
    AppSystemSettingsConfig, AppUiConfig, DEFAULT_TIMEZONE, ModelRoute, PreviewAutoSaveMode,
    QuickMessageConfig, QuickMessageGroup, QuickMessageKind, default_timezone,
};

// 系统设置默认值必须与桌面端实际启动配置一致，而非依赖组件库默认。
#[test]
fn app_system_settings_default_matches_desktop_contract() {
    let settings = AppSystemSettingsConfig::default();

    assert_eq!(settings.gateway_client.host, "127.0.0.1");
    assert_eq!(settings.gateway_client.port, config_gateway::DEFAULT_GATEWAY_PORT);
    assert_eq!(settings.app_theme, "Light");
    assert_eq!(settings.terminal_shell, "zsh");
    assert_eq!(settings.terminal_theme, "system");
    assert_eq!(settings.terminal_font_family, "JetBrains Mono");
    assert_eq!(settings.terminal_font_size, 15.0);
    assert!(settings.editor_follow_system_theme);
    assert_eq!(settings.editor_theme, "Light");
    assert_eq!(settings.editor_font_size, 16.0);
    assert_eq!(settings.editor_line_height, 19.6);
    assert!(!settings.editor_auto_line_height);
    assert!(!settings.lsp_enabled);
    assert!(!settings.dialogue_flow_show_reasoning_summary);
    assert!(!settings.dialogue_flow_expand_shell_tool_section);
    assert!(!settings.dialogue_flow_expand_edit_tool_section);
    assert!(settings.tui_keybindings.is_empty());
    assert_eq!(settings.preview_auto_save, PreviewAutoSaveMode::OnFocusChange);
}

// 时区既要提供稳定默认，也只接受有效 IANA 标识，选项列表不得重复。
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

// 缺失快捷消息触发内置默认，显式空列表则表示用户主动清空，两者不可合并。
#[test]
fn quick_messages_default_missing_and_empty_values_have_distinct_semantics() {
    let expected_texts = [
        "继续推进",
        "执行下一步",
        "摘要并生成下一个任务的提示词",
        "压缩当前会话",
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
            Some("压缩"),
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

// 嵌套 UI 配置从空对象读取时，应与各子配置的 serde 默认保持一致。
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
    assert_eq!(settings.editor_font_size, 16.0);
    assert_eq!(settings.editor_line_height, 19.6);
    assert!(!settings.editor_auto_line_height);
    assert_eq!(settings.gateway_client.host, "10.0.0.8");
    assert_eq!(settings.gateway_client.port, config_gateway::DEFAULT_GATEWAY_PORT);
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
fn desktop_locale_round_trips_three_values() {
    use super::DesktopLocale;
    for (locale, value) in [
        (DesktopLocale::SimplifiedChinese, "zh-CN"),
        (DesktopLocale::TraditionalChinese, "zh-TW"),
        (DesktopLocale::English, "en"),
    ] {
        let settings = AppSystemSettingsConfig { desktop_locale: locale, ..Default::default() };
        let json = serde_json::to_value(&settings).unwrap();
        assert_eq!(json["desktop_locale"], value);
        assert_eq!(
            serde_json::from_value::<AppSystemSettingsConfig>(json).unwrap().desktop_locale,
            locale
        );
    }
}

#[test]
fn desktop_locale_defaults_and_rejects_invalid_values() {
    use super::DesktopLocale;
    assert_eq!(AppSystemSettingsConfig::default().desktop_locale, DesktopLocale::SimplifiedChinese);
    assert_eq!(
        serde_json::from_value::<AppSystemSettingsConfig>(serde_json::json!({}))
            .unwrap()
            .desktop_locale,
        DesktopLocale::SimplifiedChinese
    );
    for value in [
        serde_json::json!("fr"),
        serde_json::json!("zh"),
        serde_json::json!(""),
        serde_json::Value::Null,
        serde_json::json!(1),
    ] {
        assert!(
            serde_json::from_value::<AppSystemSettingsConfig>(
                serde_json::json!({"desktop_locale": value})
            )
            .is_err()
        );
    }
}

#[test]
fn task_completion_notification_defaults_off_and_roundtrips() {
    let legacy: AppSystemSettingsConfig = serde_json::from_value(serde_json::json!({})).unwrap();
    assert!(!legacy.task_completion_notification);
    assert!(!AppSystemSettingsConfig::default().task_completion_notification);
    let enabled =
        AppSystemSettingsConfig { task_completion_notification: true, ..Default::default() };
    let restored: AppSystemSettingsConfig =
        serde_json::from_value(serde_json::to_value(&enabled).unwrap()).unwrap();
    assert!(restored.task_completion_notification);
}

#[test]
fn terminal_typography_missing_font_size_uses_terminal_default() {
    let settings: AppSystemSettingsConfig = serde_json::from_str("{}").unwrap();
    assert_eq!(settings.terminal_font_size, 15.0);
}

#[test]
fn terminal_typography_preserves_explicit_font_sizes() {
    for size in [12.0, 13.0, 15.0, 20.0] {
        let settings: AppSystemSettingsConfig =
            serde_json::from_value(serde_json::json!({"terminal_font_size": size})).unwrap();
        assert_eq!(settings.terminal_font_size, size);
        let restored: AppSystemSettingsConfig =
            serde_json::from_value(serde_json::to_value(&settings).unwrap()).unwrap();
        assert_eq!(restored.terminal_font_size, size);
    }
}

#[test]
fn voice_task_02_ui_defaults_preserve_voice_defaults() {
    let config: config_voice::VoiceConfig = serde_json::from_str("{}").unwrap();
    let legacy: AppUiConfig = serde_json::from_str("{}").unwrap();
    assert_eq!(legacy.voice, config);
}

#[test]
fn voice_task_02_config_roundtrip_preserves_gateway_reference() {
    use config_voice::{RealtimeModel, TranscriptionModel};
    use serde_json::json;

    let value = json!({"voice":{"dictation_enabled":true,"realtime_enabled":true,
        "provider_id":"voice_primary","language":"zh","dictionary":["ShiJu"],"voice":"cedar"}});
    let config: AppUiConfig = serde_json::from_value(value).unwrap();
    let roundtrip: AppUiConfig =
        serde_json::from_str(&serde_json::to_string(&config).unwrap()).unwrap();
    assert_eq!(roundtrip, config);
    assert_eq!(config.voice.provider_id.as_deref(), Some("voice_primary"));
    assert_eq!(config.voice.transcription_model, TranscriptionModel::GptTranscribe);
    assert_eq!(config.voice.realtime_model, RealtimeModel::GptRealtime21);
}
