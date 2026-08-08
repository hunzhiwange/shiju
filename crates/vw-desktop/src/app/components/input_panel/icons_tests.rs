use super::icons::{
    acp_agent_icon, auto_icon, default_acp_icon, icon_svg, image_icon, is_dark_mode, max_icon,
    normalize_acp_agent_icon_name, provider_logo_handle, raw_svg_handle, svg_icon,
    themed_svg_handle,
};
use crate::app::Message;
use crate::app::assets::{self, Icon};
use iced::{Element, Length, Theme};

fn assert_size(element: &Element<'_, Message>, width: Length, height: Length) {
    let size = element.as_widget().size();

    assert_eq!(size.width, width);
    assert_eq!(size.height, height);
}

#[test]
fn icon_builders_use_requested_square_size() {
    let icon: Element<'_, Message> = icon_svg(Icon::Gear, 18.0).into();
    let themed: Element<'_, Message> = themed_svg_handle(auto_icon(), 19.0).into();
    let raw: Element<'_, Message> = raw_svg_handle(max_icon(), 20.0).into();
    let svg = svg_icon(Icon::Plus, 21.0);
    let image = image_icon(Icon::Logo, 22.0);

    assert_size(&icon, Length::Fixed(18.0), Length::Fixed(18.0));
    assert_size(&themed, Length::Fixed(19.0), Length::Fixed(19.0));
    assert_size(&raw, Length::Fixed(20.0), Length::Fixed(20.0));
    assert_size(&svg, Length::Fixed(21.0), Length::Fixed(21.0));
    assert_size(&image, Length::Fixed(22.0), Length::Fixed(22.0));
}

#[test]
fn provider_and_builtin_logo_handles_accept_known_and_unknown_ids() {
    let known: Element<'_, Message> =
        themed_svg_handle(provider_logo_handle("openai"), 12.0).into();
    let unknown: Element<'_, Message> =
        themed_svg_handle(provider_logo_handle("unknown-provider"), 13.0).into();

    assert_size(&known, Length::Fixed(12.0), Length::Fixed(12.0));
    assert_size(&unknown, Length::Fixed(13.0), Length::Fixed(13.0));
}

#[test]
fn theme_darkness_uses_palette_luminance_threshold() {
    assert!(is_dark_mode(&Theme::Dark));
    assert!(!is_dark_mode(&Theme::Light));
}

#[test]
fn acp_agent_icon_normalizes_supported_aliases() {
    let cases = [
        ("AgentClientProtocol-Claude", "claude"),
        (" Claude Code ", "claude"),
        ("Auggie CLI", "auggie"),
        ("Codex-CLI", "codex"),
        ("GitHub Copilot", "copilot"),
        ("Factory Droid", "droid"),
        ("Gemini CLI", "gemini"),
        ("Kiro Agent", "kiro"),
        ("KiloCode", "kilocode"),
        ("Kimi Code CLI", "kimi"),
        ("open-code", "opencode"),
        ("OpenClaw", "openclaw"),
        ("pi-acp", "pi"),
        ("Qoder CLI", "qoder"),
        ("Qwen Code", "qwen"),
        ("Trae CLI", "trae"),
        ("unknown", ""),
    ];

    for (input, expected) in cases {
        assert_eq!(normalize_acp_agent_icon_name(input), expected);
    }
}

#[test]
fn acp_agent_icon_renders_all_supported_agents_and_default() {
    for agent in [
        "auggie", "claude", "codex", "copilot", "cursor", "droid", "gemini", "kiro", "kilocode",
        "kimi", "opencode", "openclaw", "pi", "qoder", "qwen", "trae", "unknown",
    ] {
        let element = acp_agent_icon(agent, 16.0);

        assert_size(&element, Length::Fixed(16.0), Length::Fixed(16.0));
    }
}

#[test]
fn default_acp_icon_uses_requested_size() {
    let element = default_acp_icon(24.0);

    assert_size(&element, Length::Fixed(24.0), Length::Fixed(24.0));
}

#[test]
fn default_acp_icon_uses_robot_instead_of_connector() {
    let source = include_str!("icons.rs");
    assert!(
        source.contains("assets::get_icon(Icon::Robot)"),
        "default ACP backend should use robot icon like slash ACP entries"
    );
    assert!(
        !source.contains("assets::get_icon(Icon::GearWideConnected)"),
        "default ACP backend should not reuse expert-hub connector icon"
    );
}

#[test]
fn raw_svg_handle_accepts_asset_handle_without_theme_style() {
    let element: Element<'_, Message> =
        raw_svg_handle(assets::get_icon(Icon::AppCodex), 15.0).into();

    assert_size(&element, Length::Fixed(15.0), Length::Fixed(15.0));
}

#[test]
fn normalize_acp_agent_icon_name_handles_empty_and_whitespace() {
    assert_eq!(normalize_acp_agent_icon_name(""), "");
    assert_eq!(normalize_acp_agent_icon_name("   "), "");
    assert_eq!(normalize_acp_agent_icon_name("\t\n"), "");
}

#[test]
fn normalize_acp_agent_icon_name_handles_all_direct_matches() {
    for &name in &[
        "claude", "auggie", "codex", "copilot", "cursor", "droid", "gemini", "kiro", "kilocode",
        "kimi", "opencode", "openclaw", "pi", "qoder", "qwen", "trae",
    ] {
        assert_eq!(normalize_acp_agent_icon_name(name), name, "direct match failed for {name:?}");
    }
}

#[test]
fn normalize_acp_agent_icon_name_handles_concatenated_aliases() {
    assert_eq!(normalize_acp_agent_icon_name("claudecode"), "claude");
    assert_eq!(normalize_acp_agent_icon_name("githubcopilot"), "copilot");
    assert_eq!(normalize_acp_agent_icon_name("factorydroid"), "droid");
    assert_eq!(normalize_acp_agent_icon_name("kimi-code-cli"), "kimi");
}

#[test]
fn normalize_acp_agent_icon_name_handles_remaining_cli_and_hyphenated_aliases() {
    let cases = [
        ("claude-code", "claude"),
        ("auggie-cli", "auggie"),
        ("codex cli", "codex"),
        ("copilot-cli", "copilot"),
        ("github-copilot", "copilot"),
        ("factory-droid", "droid"),
        ("gemini-cli", "gemini"),
        ("kiro-cli-chat", "kiro"),
        ("kimi-code", "kimi"),
        ("qoder-cli", "qoder"),
        ("qwen-code", "qwen"),
        ("trae-cli", "trae"),
    ];

    for (input, expected) in cases {
        assert_eq!(normalize_acp_agent_icon_name(input), expected, "alias {input:?}");
    }
}

#[test]
fn is_dark_mode_works_across_common_themes() {
    assert!(is_dark_mode(&Theme::Dark));
    assert!(is_dark_mode(&Theme::Dracula));
    assert!(is_dark_mode(&Theme::CatppuccinMocha));
    assert!(is_dark_mode(&Theme::TokyoNight));
    assert!(!is_dark_mode(&Theme::Light));
    assert!(!is_dark_mode(&Theme::SolarizedLight));
    assert!(!is_dark_mode(&Theme::CatppuccinLatte));
}

#[test]
fn default_acp_icon_accepts_zero_size_without_panic() {
    let element = default_acp_icon(0.0);
    assert_size(&element, Length::Fixed(0.0), Length::Fixed(0.0));
}

#[test]
fn auto_and_max_icon_produce_valid_handles_for_themed_widgets() {
    let auto: Element<'_, Message> = themed_svg_handle(auto_icon(), 10.0).into();
    let max: Element<'_, Message> = themed_svg_handle(max_icon(), 10.0).into();
    assert_size(&auto, Length::Fixed(10.0), Length::Fixed(10.0));
    assert_size(&max, Length::Fixed(10.0), Length::Fixed(10.0));
}

#[test]
fn provider_logo_handle_returns_non_empty_handle_for_any_provider_id() {
    let handle = provider_logo_handle("openai");
    let element: Element<'_, Message> = themed_svg_handle(handle, 14.0).into();
    assert_size(&element, Length::Fixed(14.0), Length::Fixed(14.0));
}
