use super::{acp_description, acp_title, command_line, setup_hint};
use config_aggregate::AcpAgentConfig;

#[test]
fn acp_catalog_search_combines_with_install_status_and_preserves_order() {
    use crate::acp_state::{AcpInstallFilter, AcpSettingsState};
    use config_ui::DesktopLocale::SimplifiedChinese;

    let mut state = AcpSettingsState::default();
    for name in ["employee-acp", "claude", "codex", "custom"] {
        state.catalog.insert(name.into(), AcpAgentConfig::default());
    }
    state.installed.extend([
        (super::EMPLOYEE_ACP_BACKEND.into(), false),
        ("claude".into(), true),
        ("codex".into(), false),
    ]);
    assert_eq!(
        super::filtered_agents(&state, SimplifiedChinese),
        ["employee-acp", "claude", "codex", "custom"]
    );

    state.install_filter = AcpInstallFilter::Installed;
    assert_eq!(super::filtered_agents(&state, SimplifiedChinese), ["employee-acp", "claude"]);
    for query in [" 数字员工 ", " DIGITAL employees ", "EMPLOYEE-ACP"] {
        state.catalog_query = query.into();
        assert_eq!(super::filtered_agents(&state, SimplifiedChinese), ["employee-acp"]);
    }
    state.catalog_query = "CLAUDE CODE".into();
    assert_eq!(super::filtered_agents(&state, SimplifiedChinese), ["claude"]);
    state.install_filter = AcpInstallFilter::NotInstalled;
    assert!(super::filtered_agents(&state, SimplifiedChinese).is_empty());
    state.catalog_query.clear();
    assert_eq!(super::filtered_agents(&state, SimplifiedChinese), ["codex"]);
}

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
fn settings_metadata_covers_every_builtin_acp() {
    for (agent, spec) in acp_agent_registry::built_in_agent_specs() {
        assert_eq!(acp_title(&agent), spec.display_name, "{agent}");
        assert!(!acp_description(&agent).contains("自定义 ACP"), "{agent}");
    }
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
