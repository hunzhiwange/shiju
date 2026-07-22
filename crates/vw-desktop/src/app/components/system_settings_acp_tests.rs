const SOURCE: &str = include_str!("system_settings_acp.rs");

use super::{acp_description, acp_title, command_line, ordered_agents, setup_hint, view};
use vw_config_types::config::AcpAgentConfig;

#[test]
fn system_settings_acp_tests_are_wired() {
    assert!(module_path!().contains("system_settings_acp_tests"));
}

#[test]
fn system_settings_acp_page_keeps_core_actions() {
    for needle in [
        "AcpMessage::Refresh",
        "AcpMessage::SetEnabled",
        "AcpMessage::CopyCommand",
        "\"✓\"",
        "settings_success_banner",
        "\"openclaw\"",
        "GitHub Copilot",
    ] {
        assert!(SOURCE.contains(needle), "missing ACP page needle: {needle}");
    }
}

#[test]
fn acp_title_description_and_setup_hint_cover_known_and_custom_agents() {
    assert_eq!(acp_title("codex"), "Codex CLI");
    assert_eq!(acp_title("claude"), "Claude Code");
    assert_eq!(acp_title("gemini"), "Gemini CLI");
    assert_eq!(acp_title("copilot"), "GitHub Copilot");
    assert_eq!(acp_title("openclaw"), "OpenClaw");
    assert_eq!(acp_title("pi"), "Pi ACP");
    assert_eq!(acp_title("custom"), "自定义 ACP");

    assert!(acp_description("codex").contains("Codex"));
    assert!(acp_description("opencode").contains("OpenCode"));
    assert!(acp_description("qwen").contains("Qwen"));
    assert!(acp_description("custom").contains("自定义 ACP 后端"));

    assert!(setup_hint("codex").contains("Node.js"));
    assert!(setup_hint("claude").contains("Claude Code"));
    assert!(setup_hint("gemini").contains("Gemini CLI"));
    assert!(setup_hint("copilot").contains("GitHub Copilot"));
    assert!(setup_hint("custom").contains("PATH"));
}

#[test]
fn acp_metadata_covers_remaining_catalog_agents() {
    let cases = [
        ("auggie", "Auggie CLI", "Auggie"),
        ("cursor", "Cursor Agent", "Cursor"),
        ("droid", "Factory droid", "droid"),
        ("iflow", "iFlow", "iFlow"),
        ("kilocode", "Kilocode", "Kilocode"),
        ("opencode", "OpenCode", "OpenCode"),
        ("qwen", "Qwen Code", "Qwen"),
    ];

    for (agent, title, description_needle) in cases {
        assert_eq!(acp_title(agent), title);
        assert!(acp_description(agent).contains(description_needle));
    }
    assert!(setup_hint("openclaw").contains("openclaw"));
    assert!(setup_hint("pi").contains("npx"));
}

#[test]
fn command_line_filters_empty_parts_and_preserves_arguments() {
    let config = AcpAgentConfig {
        command: "npx".to_string(),
        args: vec![
            "".to_string(),
            "  ".to_string(),
            "@openai/codex".to_string(),
            "--acp".to_string(),
        ],
        ..AcpAgentConfig::default()
    };

    assert_eq!(command_line(&config), "npx @openai/codex --acp");
}

#[test]
fn ordered_agents_uses_known_rank_then_alphabetical_custom_names() {
    let (mut app, _) = crate::app::App::new();
    app.acp_settings.catalog.clear();
    for name in ["zeta", "codex", "claude", "alpha", "qwen", "gemini"] {
        app.acp_settings.catalog.insert(name.to_string(), AcpAgentConfig::default());
    }

    assert_eq!(
        ordered_agents(&app),
        vec![
            "claude".to_string(),
            "gemini".to_string(),
            "codex".to_string(),
            "qwen".to_string(),
            "alpha".to_string(),
            "zeta".to_string(),
        ]
    );
}

#[test]
fn acp_view_builds_empty_and_populated_catalog_states() {
    let (mut app, _) = crate::app::App::new();
    let _: iced::Element<'_, crate::app::Message> = view(&app);

    app.acp_settings.catalog.insert(
        "codex".to_string(),
        AcpAgentConfig {
            command: "npx".to_string(),
            args: vec!["@openai/codex".to_string(), "--acp".to_string()],
            ..AcpAgentConfig::default()
        },
    );
    app.acp_settings.enabled.insert("codex".to_string());
    app.acp_settings.copied_agent = Some("codex".to_string());
    app.acp_settings.status_message = Some("saved".to_string());
    app.acp_settings.save_error = Some("failed".to_string());

    let _: iced::Element<'_, crate::app::Message> = view(&app);
}

#[test]
fn acp_view_builds_saving_agent_state() {
    let (mut app, _) = crate::app::App::new();
    app.acp_settings.catalog.insert(
        "codex".to_string(),
        AcpAgentConfig {
            command: "npx".to_string(),
            args: vec!["@openai/codex".to_string(), "--acp".to_string()],
            ..AcpAgentConfig::default()
        },
    );
    app.acp_settings.saving_agent = Some("codex".to_string());

    let _: iced::Element<'_, crate::app::Message> = view(&app);
}
