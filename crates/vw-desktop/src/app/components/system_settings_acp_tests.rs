//! 验证 ACP 设置源码保持所需控件、消息连线与安全字段约束。
//! 源码级断言用于守住静态配置契约，不替代实际视图的编译与渲染测试。

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
        "AcpMessage::NewAgentPanelToggled",
        "AcpMessage::AddAgent",
        "NewAgentNameChanged",
        "NewAgentCommandChanged",
        "ArgumentChanged",
        "AcpMessage::TestAgent",
        "AcpMessage::InstallAgent",
        "\"✓\"",
        "settings_error_banner",
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

#[tokio::test(flavor = "multi_thread")]
async fn acp_catalog_displays_registered_agents_render_both_themes() {
    use iced::advanced::renderer::Headless;
    use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
    use iced::{Color, Font, Pixels, Rectangle, Size, Theme};
    let (mut app, _) = crate::app::App::new();
    for agent in super::ACP_ORDER.iter().copied().chain(["中文自定义代理"]) {
        for theme in [Theme::Dark, Theme::Light] {
            let config = AcpAgentConfig { command: "示例命令".into(), ..Default::default() };
            app.acp_settings.enabled.insert(agent.into());
            let mut element = super::acp_card(&app, agent.into(), &config);
            let mut renderer =
                <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
                    .await
                    .expect("headless renderer");
            let bounds = Size::new(960.0, 500.0);
            let mut tree = Tree::new(element.as_widget());
            let node = element.as_widget_mut().layout(
                &mut tree,
                &renderer,
                &layout::Limits::new(Size::ZERO, bounds),
            );
            element.as_widget().draw(
                &tree,
                &mut renderer,
                &theme,
                &renderer::Style::default(),
                Layout::new(&node),
                mouse::Cursor::Unavailable,
                &Rectangle::with_size(bounds),
            );
            let pixels = renderer.screenshot(Size::new(960, 500), 1.0, Color::TRANSPARENT);
            assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0), "{agent}");
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn acp_editor_validates_and_saves_fields_render_both_themes() {
    use iced::advanced::renderer::Headless;
    use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
    use iced::{Color, Font, Pixels, Rectangle, Size, Theme};
    let (mut app, _) = crate::app::App::new();
    app.agents_settings.active_config_tab = crate::app::state::AgentConfigTab::Acp;
    for name in super::ACP_ORDER {
        app.acp_settings.catalog.insert((*name).into(), AcpAgentConfig::default());
    }
    for agent in super::ACP_ORDER.iter().copied().chain(["中文自定义代理"]) {
        for theme in [Theme::Dark, Theme::Light] {
            let config = AcpAgentConfig { command: "示例命令".into(), ..Default::default() };
            app.acp_settings.enabled.insert(agent.into());
            app.acp_settings.new_agent_expanded = true;
            app.acp_settings.new_agent_name_input = agent.into();
            app.acp_settings.new_agent_command_input = config.command.clone();
            app.acp_settings.editor_args = vec!["中文参数,空格 ".into()];
            app.acp_settings.editor_env = vec![("TOKEN".into(), "中文值".into())];
            let mut element = crate::app::components::system_settings_agents::view(&app);
            let mut renderer =
                <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
                    .await
                    .expect("headless renderer");
            let bounds = Size::new(960.0, 500.0);
            let mut tree = Tree::new(element.as_widget());
            let node = element.as_widget_mut().layout(
                &mut tree,
                &renderer,
                &layout::Limits::new(Size::ZERO, bounds),
            );
            assert_eq!(node.size(), bounds, "the long catalog must stay inside the viewport");
            element.as_widget().draw(
                &tree,
                &mut renderer,
                &theme,
                &renderer::Style::default(),
                Layout::new(&node),
                mouse::Cursor::Unavailable,
                &Rectangle::with_size(bounds),
            );
            let pixels = renderer.screenshot(Size::new(960, 500), 1.0, Color::TRANSPARENT);
            assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0), "{agent}");
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn acp_diagnostic_ignores_stale_results_render_both_themes() {
    use iced::advanced::renderer::Headless;
    use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
    use iced::{Color, Font, Pixels, Rectangle, Size, Theme};
    let (mut app, _) = crate::app::App::new();
    for agent in super::ACP_ORDER.iter().copied().chain(["中文自定义代理"]) {
        for theme in [Theme::Dark, Theme::Light] {
            let config = AcpAgentConfig { command: "示例命令".into(), ..Default::default() };
            app.acp_settings.enabled.insert(agent.into());
            app.acp_settings.new_agent_expanded = true;
            app.acp_settings.new_agent_name_input = agent.into();
            app.acp_settings.new_agent_command_input = config.command.clone();
            app.acp_settings.diagnostic_status = Some("启动成功 · 握手成功 · 需要认证".into());
            app.acp_settings.editor_args = vec!["中文参数,空格 ".into()];
            app.acp_settings.editor_env = vec![("TOKEN".into(), "中文值".into())];
            let mut element = super::new_agent_form(&app);
            let mut renderer =
                <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
                    .await
                    .expect("headless renderer");
            let bounds = Size::new(960.0, 500.0);
            let mut tree = Tree::new(element.as_widget());
            let node = element.as_widget_mut().layout(
                &mut tree,
                &renderer,
                &layout::Limits::new(Size::ZERO, bounds),
            );
            element.as_widget().draw(
                &tree,
                &mut renderer,
                &theme,
                &renderer::Style::default(),
                Layout::new(&node),
                mouse::Cursor::Unavailable,
                &Rectangle::with_size(bounds),
            );
            let pixels = renderer.screenshot(Size::new(960, 500), 1.0, Color::TRANSPARENT);
            assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0), "{agent}");
        }
    }
}

/// Exercise shaping and layout with overflowing fields, rather than only building Element.
#[tokio::test(flavor = "multi_thread")]
async fn acp_configuration_flow_regression_long_editor_keeps_actions_visible() {
    use iced::advanced::renderer::Headless;
    use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
    use iced::{Color, Font, Pixels, Rectangle, Size, Theme};

    let (mut app, _) = crate::app::App::new();
    app.acp_settings.new_agent_expanded = true;
    app.acp_settings.new_agent_name_input = "中文自定义代理".into();
    app.acp_settings.new_agent_command_input = "/usr/bin/python3".into();
    app.acp_settings.diagnostic_status =
        Some("启动成功 · 握手成功 · 需要认证（未自动认证）".into());
    for theme in [Theme::Dark, Theme::Light] {
        let mut fixed_bounds = None;
        for count in [1, 40] {
            let mut renderer =
                <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
                    .await
                    .expect("headless renderer");
            app.acp_settings.editor_args =
                (0..count).map(|i| format!("参数 {i}, 保留空格 ")).collect();
            app.acp_settings.editor_env =
                (0..count).map(|i| (format!("KEY_{i}"), "中文值".into())).collect();
            let mut element = super::new_agent_form(&app);
            let bounds = Size::new(960.0, 640.0);
            let mut tree = Tree::new(element.as_widget());
            let node = element.as_widget_mut().layout(
                &mut tree,
                &renderer,
                &layout::Limits::new(Size::ZERO, bounds),
            );
            let card = Layout::new(&node);
            let column = card.children().next().expect("modal content");
            let rows: Vec<_> = column.children().collect();
            assert_eq!(rows.len(), 4, "title, diagnostic, scrolling fields, actions");
            let header = rows[0].bounds();
            let viewport = rows[2].bounds();
            let footer = rows[3].bounds();
            assert!(header.y >= 0.0);
            assert!(viewport.y >= header.y + header.height);
            assert!(viewport.y + viewport.height <= footer.y);
            assert!(footer.y + footer.height <= card.bounds().height);
            let fields_container = rows[2].children().next().expect("scroll content");
            let fields = fields_container.children().next().expect("padded fields").bounds();
            assert!((fields.x - fields_container.bounds().x - 7.0).abs() < 0.1);
            assert!((fields_container.bounds().width - fields.width - 14.0).abs() < 0.1);
            if count == 40 {
                assert!(fields.height > viewport.height, "long form must actually overflow");
            }
            if let Some(expected) = fixed_bounds {
                assert_eq!((header, footer), expected, "field count must not move modal actions");
            }
            fixed_bounds = Some((header, footer));
            element.as_widget().draw(
                &tree,
                &mut renderer,
                &theme,
                &renderer::Style { text_color: theme.palette().text },
                card,
                mouse::Cursor::Unavailable,
                &Rectangle::with_size(bounds),
            );
            let pixels = renderer.screenshot(Size::new(960, 640), 1.0, Color::TRANSPARENT);
            assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
            if count == 40
                && let Some(directory) = std::env::var_os("ACP_REGRESSION_SCREENSHOTS")
            {
                let directory = std::path::PathBuf::from(directory);
                std::fs::create_dir_all(&directory).unwrap();
                image::save_buffer(
                    directory.join(if theme == Theme::Dark { "dark.png" } else { "light.png" }),
                    &pixels,
                    960,
                    640,
                    image::ColorType::Rgba8,
                )
                .unwrap();
            }
        }
    }
    assert!(SOURCE.contains("Scrollbar::new().width(4).scroller_width(4)"));
}
