//! 验证技能目录来源、提示注入模式与安全策略的默认值和解析规则。
//! 外部名称、显示标签和安全子开关分别锁定，避免配置迁移时相互耦合。

// 此哨兵保护拆分后的技能配置测试文件仍由父模块编译。
#[test]
fn task_631_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("skills_tests.rs"));
}

// 默认只读取 VibeWindow 技能目录，开放技能和额外注入能力均需显式开启。
#[test]
fn skills_config_defaults_to_vibewindow_directory_provider() {
    let config = super::SkillsConfig::default();
    assert_eq!(config.directory_provider, super::SkillsDirectoryProvider::Vibewindow);
    assert!(config.auto_load_project_skills);
    assert!(!config.open_skills_enabled);
    assert_eq!(config.open_skills_dir, None);
    assert_eq!(config.prompt_injection_mode, super::SkillsPromptInjectionMode::Compact);

    let parsed: super::SkillsConfig = serde_json::from_str("{}").unwrap();
    assert_eq!(parsed.directory_provider, super::SkillsDirectoryProvider::Vibewindow);
    assert!(parsed.auto_load_project_skills);
    assert!(!parsed.open_skills_enabled);
    assert_eq!(parsed.open_skills_dir, None);
    assert_eq!(parsed.prompt_injection_mode, super::SkillsPromptInjectionMode::Compact);
}

// 每项安全检查都可独立配置；空对象必须保持全部关闭的兼容行为。
#[test]
fn skill_security_defaults_off_and_round_trips_independently() {
    fn assert_all_disabled(security: &super::SkillSecurityConfig) {
        assert!(!security.require_manifest);
        assert!(!security.reject_symlinks);
        assert!(!security.reject_script_files);
        assert!(!security.reject_oversized_text_files);
        assert!(!security.detect_high_risk_content);
        assert!(!security.detect_shell_chaining);
        assert!(!security.validate_manifest);
        assert!(!security.validate_markdown_links);
    }

    assert_all_disabled(&super::SkillSecurityConfig::default());

    let legacy: super::SkillsConfig = serde_json::from_str("{}").unwrap();
    assert_all_disabled(&legacy.security);

    let configured: super::SkillsConfig = serde_json::from_value(serde_json::json!({
        "security": {
            "detect_shell_chaining": true
        }
    }))
    .unwrap();
    assert!(!configured.security.require_manifest);
    assert!(!configured.security.reject_symlinks);
    assert!(!configured.security.reject_script_files);
    assert!(!configured.security.reject_oversized_text_files);
    assert!(!configured.security.detect_high_risk_content);
    assert!(configured.security.detect_shell_chaining);
    assert!(!configured.security.validate_manifest);
    assert!(!configured.security.validate_markdown_links);

    let serialized = serde_json::to_value(&configured).unwrap();
    assert_eq!(
        serialized["security"],
        serde_json::json!({
            "require_manifest": false,
            "reject_symlinks": false,
            "reject_script_files": false,
            "reject_oversized_text_files": false,
            "detect_high_risk_content": false,
            "detect_shell_chaining": true,
            "validate_manifest": false,
            "validate_markdown_links": false
        })
    );

    let round_tripped: super::SkillsConfig = serde_json::from_value(serialized).unwrap();
    assert_eq!(round_tripped.security, configured.security);
}

// 目录来源的 serde 名称、用户标签和枚举顺序共同构成设置界面契约。
#[test]
fn skills_directory_provider_round_trips_as_snake_case() {
    let config = super::SkillsConfig {
        directory_provider: super::SkillsDirectoryProvider::Codex,
        ..Default::default()
    };

    let serialized = serde_json::to_value(config).unwrap();
    assert_eq!(serialized["directory_provider"], "codex");

    let parsed: super::SkillsConfig =
        serde_json::from_value(serde_json::json!({ "directory_provider": "claude" })).unwrap();
    assert_eq!(parsed.directory_provider, super::SkillsDirectoryProvider::Claude);
}

#[test]
fn skills_directory_provider_labels_match_display_and_all_order() {
    let providers = super::SkillsDirectoryProvider::ALL;
    assert_eq!(
        providers,
        [
            super::SkillsDirectoryProvider::Vibewindow,
            super::SkillsDirectoryProvider::Codex,
            super::SkillsDirectoryProvider::Claude,
            super::SkillsDirectoryProvider::Cursor,
        ]
    );

    let labels = providers.map(|provider| provider.label());
    assert_eq!(labels, ["VibeWindow", "Codex", "Claude", "Cursor"]);

    let display_labels = providers.map(|provider| provider.to_string());
    assert_eq!(
        display_labels,
        [
            String::from("VibeWindow"),
            String::from("Codex"),
            String::from("Claude"),
            String::from("Cursor"),
        ]
    );
}

// 解析器接受已公布别名和大小写差异，但未知来源不能静默回退。
#[test]
fn parse_skills_directory_provider_accepts_supported_aliases() {
    let cases = [
        (" vibewindow ", super::SkillsDirectoryProvider::Vibewindow),
        ("VIBE_WINDOW", super::SkillsDirectoryProvider::Vibewindow),
        ("vibe-window", super::SkillsDirectoryProvider::Vibewindow),
        ("CoDeX", super::SkillsDirectoryProvider::Codex),
        ("claude", super::SkillsDirectoryProvider::Claude),
        ("CLAUDE_CODE", super::SkillsDirectoryProvider::Claude),
        ("claude-code", super::SkillsDirectoryProvider::Claude),
        (" cursor ", super::SkillsDirectoryProvider::Cursor),
    ];

    for (raw, expected) in cases {
        assert_eq!(super::parse_skills_directory_provider(raw), Some(expected));
    }
}

#[test]
fn parse_skills_directory_provider_rejects_unknown_values() {
    for raw in ["", "vibe window", "claude/code", "codex-cli", "unknown"] {
        assert_eq!(super::parse_skills_directory_provider(raw), None);
    }
}

// 提示注入模式同样先清理输入再匹配，错误值需向调用方返回明确失败。
#[test]
fn parse_skills_prompt_injection_mode_accepts_trimmed_case_insensitive_values() {
    assert_eq!(
        super::parse_skills_prompt_injection_mode(" compact "),
        Some(super::SkillsPromptInjectionMode::Compact)
    );
    assert_eq!(
        super::parse_skills_prompt_injection_mode("FULL"),
        Some(super::SkillsPromptInjectionMode::Full)
    );
}

#[test]
fn parse_skills_prompt_injection_mode_rejects_unknown_values() {
    for raw in ["", "metadata_only", "full compact", "verbose"] {
        assert_eq!(super::parse_skills_prompt_injection_mode(raw), None);
    }
}

#[test]
fn skills_prompt_injection_mode_round_trips_as_snake_case() {
    let serialized = serde_json::to_value(super::SkillsPromptInjectionMode::Full).unwrap();
    assert_eq!(serialized, serde_json::json!("full"));

    let parsed: super::SkillsPromptInjectionMode = serde_json::from_str("\"compact\"").unwrap();
    assert_eq!(parsed, super::SkillsPromptInjectionMode::Compact);
}

// 完整配置读取确认开放技能目录与注入模式覆盖项不会被默认值吞掉。
#[test]
fn skills_config_deserializes_open_skills_and_prompt_mode_overrides() {
    let parsed: super::SkillsConfig = serde_json::from_value(serde_json::json!({
        "directory_provider": "cursor",
        "auto_load_project_skills": false,
        "open_skills_enabled": true,
        "open_skills_dir": "/tmp/skills",
        "prompt_injection_mode": "full"
    }))
    .unwrap();

    assert_eq!(parsed.directory_provider, super::SkillsDirectoryProvider::Cursor);
    assert!(!parsed.auto_load_project_skills);
    assert!(parsed.open_skills_enabled);
    assert_eq!(parsed.open_skills_dir.as_deref(), Some("/tmp/skills"));
    assert_eq!(parsed.prompt_injection_mode, super::SkillsPromptInjectionMode::Full);
}
