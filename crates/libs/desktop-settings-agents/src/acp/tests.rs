use super::{acp_title, acp_description, setup_hint, command_line};
use config_aggregate::AcpAgentConfig;
#[test]
fn acp_title_description_and_setup_hint_cover_known_and_custom_agents() {
    assert_eq!(acp_title("codex"), "Codex CLI");
    assert_eq!(acp_title("claude"), "Claude Code");
    assert_eq!(acp_title("gemini"), "Gemini CLI");
    assert_eq!(acp_title("copilot"), "GitHub Copilot");
    assert_eq!(acp_title("openclaw"), "OpenClaw");
    assert_eq!(acp_title("pi"), "PI");
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
        ("cursor", "Cursor", "Cursor"),
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
