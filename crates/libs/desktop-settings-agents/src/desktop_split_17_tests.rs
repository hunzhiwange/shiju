//! Catalog and view construction coverage independent of desktop startup.
use crate::acp::{acp_title, ordered_agents};
use crate::acp_view::{Input, view};
use config_aggregate::AcpAgentConfig;

// These construction tests do not inspect help or icon rendering. Those callbacks
// remain covered by the host's real rendering and message mapping tests.
fn input<'a>(state: &'a crate::acp_state::AcpSettingsState, theme: &'a iced::Theme) -> Input<'a> {
    Input {
        state,
        theme,
        locale: config_ui::DesktopLocale::SimplifiedChinese,
        frame: 0,
        help: |_, _, _, label, _| iced::widget::row![label],
        icon: |_, size| iced::widget::Space::new().width(size).height(size).into(),
    }
}

#[test]
fn ordered_agents_uses_known_rank_then_alphabetical_custom_names() {
    let mut state = crate::acp_state::AcpSettingsState::default();
    state.catalog.clear();
    for name in ["zeta", "codex", "claude", "alpha", "qwen", "gemini"] {
        state.catalog.insert(name.to_string(), AcpAgentConfig::default());
    }

    assert_eq!(
        ordered_agents(&state.catalog),
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
    let mut state = crate::acp_state::AcpSettingsState::default();
    let theme = iced::Theme::Light;
    let _: iced::Element<'_, crate::message::Event> = view(input(&state, &theme));

    state.catalog.insert(
        "codex".to_string(),
        AcpAgentConfig {
            command: "npx".to_string(),
            args: vec!["@openai/codex".to_string(), "--acp".to_string()],
            ..AcpAgentConfig::default()
        },
    );
    state.enabled.insert("codex".to_string());
    state.copied_agent = Some("codex".to_string());
    state.status_message = Some("saved".to_string());
    state.save_error = Some("failed".to_string());

    let _: iced::Element<'_, crate::message::Event> = view(input(&state, &theme));
}

#[test]
fn acp_view_builds_saving_agent_state() {
    let mut state = crate::acp_state::AcpSettingsState::default();
    let theme = iced::Theme::Light;
    state.catalog.insert(
        "codex".to_string(),
        AcpAgentConfig {
            command: "npx".to_string(),
            args: vec!["@openai/codex".to_string(), "--acp".to_string()],
            ..AcpAgentConfig::default()
        },
    );
    state.saving_agent = Some("codex".to_string());

    let _: iced::Element<'_, crate::message::Event> = view(input(&state, &theme));
}

#[test]
fn acp_catalog_displays_registered_agents() {
    for (key, title) in [
        ("auggie", "Auggie CLI"),
        ("claude", "Claude Code"),
        ("codex", "Codex CLI"),
        ("copilot", "GitHub Copilot"),
        ("droid", "Factory droid"),
        ("gemini", "Gemini CLI"),
        ("iflow", "iFlow"),
        ("kilocode", "Kilocode"),
        ("opencode", "OpenCode"),
        ("openclaw", "OpenClaw"),
        ("qwen", "Qwen Code"),
        ("kimi", "Kimi Code CLI"),
        ("kiro", "Kiro"),
        ("qoder", "Qoder CLI"),
        ("trae", "TRAE CLI"),
        ("cursor", "Cursor"),
        ("pi", "PI"),
        ("codebuddy", "CodeBuddy"),
        ("goose", "Goose"),
        ("vibe", "Mistral Vibe"),
        ("hermes", "Hermes Agent"),
        ("snow", "Snow CLI"),
    ] {
        assert_eq!(acp_title(key), title, "{key}");
    }
    assert_eq!(acp_title("my-custom-agent"), "自定义 ACP");
}
