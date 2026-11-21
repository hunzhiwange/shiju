//! 验证技能目录来源、提示注入模式与安全策略的默认值和解析规则。
//! 外部名称、显示标签和安全子开关分别锁定，避免配置迁移时相互耦合。

// 此哨兵保护拆分后的技能配置测试文件仍由父模块编译。
#[test]
fn task_631_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("skills_tests.rs"));
}

// 项目技能默认加载，提示使用紧凑模式。
#[test]
fn skills_config_defaults_to_project_auto_load_and_compact_mode() {
    let config = super::SkillsConfig::default();
    assert!(config.auto_load_project_skills);
    assert_eq!(config.prompt_injection_mode, super::SkillsPromptInjectionMode::Compact);

    let serialized = serde_json::to_value(&config).unwrap();
    let mut keys = serialized.as_object().unwrap().keys().map(String::as_str).collect::<Vec<_>>();
    // serde_json 的 preserve_order 特性可由工作区其他依赖启用，键集合不依赖迭代顺序。
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["auto_load_project_skills", "prompt_injection_mode", "security"]
    );

    let parsed: super::SkillsConfig = serde_json::from_str("{}").unwrap();
    assert!(parsed.auto_load_project_skills);
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

// 旧目录提供方字段可随其他技能配置一起读取。
#[test]
fn skills_config_ignores_legacy_directory_provider() {
    let parsed: super::SkillsConfig = serde_json::from_value(serde_json::json!({
        "directory_provider": "cursor",
        "auto_load_project_skills": false,
        "prompt_injection_mode": "full"
    }))
    .unwrap();

    assert!(serde_json::to_value(&parsed).unwrap().get("directory_provider").is_none());
    assert!(!parsed.auto_load_project_skills);
    assert_eq!(parsed.prompt_injection_mode, super::SkillsPromptInjectionMode::Full);
}
