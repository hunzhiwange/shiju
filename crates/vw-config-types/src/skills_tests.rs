#[test]
fn task_631_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("skills_tests.rs"));
}

#[test]
fn skills_config_defaults_to_vibewindow_directory_provider() {
    let config = super::SkillsConfig::default();
    assert_eq!(config.directory_provider, super::SkillsDirectoryProvider::Vibewindow);
    assert!(!config.open_skills_enabled);
    assert_eq!(config.open_skills_dir, None);
    assert_eq!(config.prompt_injection_mode, super::SkillsPromptInjectionMode::Compact);

    let parsed: super::SkillsConfig = serde_json::from_str("{}").unwrap();
    assert_eq!(parsed.directory_provider, super::SkillsDirectoryProvider::Vibewindow);
    assert!(!parsed.open_skills_enabled);
    assert_eq!(parsed.open_skills_dir, None);
    assert_eq!(parsed.prompt_injection_mode, super::SkillsPromptInjectionMode::Compact);
}

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

#[test]
fn skills_config_deserializes_open_skills_and_prompt_mode_overrides() {
    let parsed: super::SkillsConfig = serde_json::from_value(serde_json::json!({
        "directory_provider": "cursor",
        "open_skills_enabled": true,
        "open_skills_dir": "/tmp/skills",
        "prompt_injection_mode": "full"
    }))
    .unwrap();

    assert_eq!(parsed.directory_provider, super::SkillsDirectoryProvider::Cursor);
    assert!(parsed.open_skills_enabled);
    assert_eq!(parsed.open_skills_dir.as_deref(), Some("/tmp/skills"));
    assert_eq!(parsed.prompt_injection_mode, super::SkillsPromptInjectionMode::Full);
}
