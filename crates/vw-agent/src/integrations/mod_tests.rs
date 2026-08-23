// 验证集成注册表的查找、别名与配置状态汇总。
use super::*;
use serde_json::json;

fn config() -> Config {
    Config::default()
}

#[test]
fn category_labels_and_order_are_stable() {
    assert_eq!(
        IntegrationCategory::all(),
        &[
            IntegrationCategory::Chat,
            IntegrationCategory::AiModel,
            IntegrationCategory::Productivity,
            IntegrationCategory::MusicAudio,
            IntegrationCategory::SmartHome,
            IntegrationCategory::ToolsAutomation,
            IntegrationCategory::MediaCreative,
            IntegrationCategory::Social,
            IntegrationCategory::Platform,
        ]
    );
    assert_eq!(IntegrationCategory::Chat.label(), "Chat Providers");
    assert_eq!(IntegrationCategory::AiModel.label(), "AI Models");
    assert_eq!(IntegrationCategory::MusicAudio.label(), "Music & Audio");
}

#[test]
fn status_and_command_enums_serialize_with_stable_variant_names() {
    assert_eq!(serde_json::to_value(IntegrationStatus::Active).unwrap(), json!("Active"));
    assert_eq!(serde_json::to_value(IntegrationStatus::Available).unwrap(), json!("Available"));
    assert_eq!(serde_json::to_value(IntegrationStatus::ComingSoon).unwrap(), json!("ComingSoon"));

    let command = IntegrationCommands::Search { query: "telegram".to_string() };
    let encoded = serde_json::to_value(&command).expect("serialize command");
    let decoded: IntegrationCommands =
        serde_json::from_value(encoded).expect("deserialize command");
    assert_eq!(decoded, command);
}

#[test]
fn status_icons_cover_all_statuses() {
    assert_eq!(status_icon(IntegrationStatus::Active), "✅");
    assert_eq!(status_icon(IntegrationStatus::Available), "⚪");
    assert_eq!(status_icon(IntegrationStatus::ComingSoon), "🔜");
}

#[test]
fn category_filter_accepts_documented_aliases_case_insensitively() {
    let cases = [
        ("CHAT", IntegrationCategory::Chat),
        ("AI", IntegrationCategory::AiModel),
        ("model", IntegrationCategory::AiModel),
        ("ai-models", IntegrationCategory::AiModel),
        ("PRODUCTIVITY", IntegrationCategory::Productivity),
        ("audio", IntegrationCategory::MusicAudio),
        ("smarthome", IntegrationCategory::SmartHome),
        ("home", IntegrationCategory::SmartHome),
        ("automation", IntegrationCategory::ToolsAutomation),
        ("tools-automation", IntegrationCategory::ToolsAutomation),
        ("creative", IntegrationCategory::MediaCreative),
        ("media-creative", IntegrationCategory::MediaCreative),
        ("SOCIAL", IntegrationCategory::Social),
        ("platforms", IntegrationCategory::Platform),
    ];

    for (input, expected) in cases {
        assert_eq!(parse_category_filter(input), Some(expected), "{input}");
    }
    assert_eq!(parse_category_filter(""), None);
}

#[test]
fn status_filter_accepts_documented_aliases_case_insensitively() {
    assert_eq!(parse_status_filter("ACTIVE"), Some(IntegrationStatus::Active));
    assert_eq!(parse_status_filter("available"), Some(IntegrationStatus::Available));
    assert_eq!(parse_status_filter("comingsoon"), Some(IntegrationStatus::ComingSoon));
    assert_eq!(parse_status_filter("soon"), Some(IntegrationStatus::ComingSoon));
    assert_eq!(parse_status_filter(""), None);
}

#[test]
fn list_command_accepts_each_valid_category_and_status_combination() {
    for category in [
        "chat",
        "ai",
        "productivity",
        "music",
        "smart-home",
        "tools",
        "media",
        "social",
        "platform",
    ] {
        handle_command(
            IntegrationCommands::List {
                category: Some(category.to_string()),
                status: Some("available".to_string()),
            },
            &config(),
        )
        .unwrap_or_else(|error| panic!("{category} should list: {error}"));
    }
}

#[test]
fn list_command_accepts_coming_soon_status_without_category() {
    let result = handle_command(
        IntegrationCommands::List { category: None, status: Some("coming-soon".to_string()) },
        &config(),
    );

    assert!(result.is_ok());
}

#[test]
fn handle_command_dispatches_search_list_and_info() {
    let cfg = config();

    assert!(
        handle_command(
            IntegrationCommands::List {
                category: Some("tools".into()),
                status: Some("active".into())
            },
            &cfg,
        )
        .is_ok()
    );
    assert!(handle_command(IntegrationCommands::Search { query: "browser".into() }, &cfg).is_ok());
    assert!(handle_command(IntegrationCommands::Info { name: "browser".into() }, &cfg).is_ok());
}

#[test]
fn show_info_covers_known_setup_hint_branches() {
    let cfg = config();

    for name in [
        "Telegram",
        "Discord",
        "Slack",
        "OpenRouter",
        "Ollama",
        "iMessage",
        "GitHub",
        "Browser",
        "Cron",
        "Webhooks",
        "Nostr",
    ] {
        show_integration_info(&cfg, name).unwrap_or_else(|error| panic!("{name}: {error}"));
    }
}

#[test]
fn search_matches_descriptions_and_is_case_insensitive() {
    let cfg = config();

    assert!(search_integrations(&cfg, "CHROMIUM").is_ok());
    assert!(search_integrations(&cfg, "workspace apps").is_ok());
    assert!(search_integrations(&cfg, "no-such-integration-description").is_ok());
}
