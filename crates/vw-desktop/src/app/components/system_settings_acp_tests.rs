//! 验证 ACP 设置源码保持所需控件、消息连线与安全字段约束。
//! 源码级断言用于守住静态配置契约，不替代实际视图的编译与渲染测试。

const SOURCE: &str = concat!(include_str!("system_settings_acp.rs"), include_str!("../../../../libs/desktop-settings-agents/src/acp.rs"), include_str!("../../../../libs/desktop-settings-agents/src/acp_view.rs"));

use crate::app::state::AcpDiagnosticStatus;
use config_aggregate::AcpAgentConfig;

#[tokio::test(flavor = "multi_thread")]
async fn acp_status_cards_and_editor_render_all_results_and_animate_in_both_themes() {
    use iced::advanced::Renderer as _;
    use iced::advanced::renderer::Headless;
    use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
    use iced::{Color, Font, Pixels, Rectangle, Size, Theme};

    let (mut app, _) = crate::app::App::new();
    let config = AcpAgentConfig { command: "中文路径/agent".into(), ..Default::default() };
    app.acp_settings.editing_agent = Some("codex".into());
    app.acp_settings.defaults.insert("codex".into(), AcpAgentConfig::default());
    app.acp_settings.new_agent_name_input = "中文代理".into();
    app.acp_settings.new_agent_command_input = config.command.clone();
    for theme in [Theme::Dark, Theme::Light] {
        let mut renderer =
            <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
                .await
                .expect("headless renderer");
        for status in [
            AcpDiagnosticStatus::Running,
            AcpDiagnosticStatus::Connected,
            AcpDiagnosticStatus::AuthenticationRequired,
            AcpDiagnosticStatus::StartupFailed,
            AcpDiagnosticStatus::HandshakeFailed,
            AcpDiagnosticStatus::TimedOut,
            AcpDiagnosticStatus::Cancelled,
            AcpDiagnosticStatus::Unavailable,
        ] {
            app.acp_settings.diagnostic_status = Some(status);
            app.acp_settings.diagnostic_results.insert("中文代理".into(), status);
            let running = status == AcpDiagnosticStatus::Running;
            app.acp_settings.diagnostic_agent = running.then(|| "中文代理".into());
            app.acp_settings.diagnostic_id = running.then(|| "active-test".into());
            for editor in [false, true] {
                let mut first_frame = None;
                for frame in 0..if running { 2 } else { 1 } {
                    app.status_animation_frame = frame;
                    let mut element = if editor {
                        super::new_agent_form(&app)
                    } else {
                        super::acp_card(&app, "中文代理".into(), &config)
                    };
                    let bounds = Size::new(960.0, 640.0);
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
                        &renderer::Style { text_color: theme.palette().text },
                        Layout::new(&node),
                        mouse::Cursor::Unavailable,
                        &Rectangle::with_size(bounds),
                    );
                    let pixels = renderer.screenshot(Size::new(960, 640), 1.0, Color::TRANSPARENT);
                    assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
                    if let Some(previous) = first_frame {
                        assert_ne!(pixels, previous, "running indicator must visibly animate");
                    }
                    if !editor
                        && frame == 0
                        && let Some(directory) = std::env::var_os("ACP_STATUS_SCREENSHOTS")
                    {
                        let directory = std::path::PathBuf::from(directory);
                        std::fs::create_dir_all(&directory).unwrap();
                        image::save_buffer(
                            directory.join(format!("{theme:?}-{status:?}.png")),
                            &pixels,
                            960,
                            640,
                            image::ColorType::Rgba8,
                        )
                        .unwrap();
                    }
                    first_frame = Some(pixels);
                    renderer.reset(Rectangle::with_size(bounds));
                }
            }
        }
    }
}

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
            app.acp_settings.diagnostic_status = Some(AcpDiagnosticStatus::AuthenticationRequired);
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
    app.acp_settings.diagnostic_status = Some(AcpDiagnosticStatus::AuthenticationRequired);
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
            assert_eq!(rows.len(), 3, "title, scrolling fields and diagnostics, actions");
            let header = rows[0].bounds();
            let viewport = rows[1].bounds();
            let footer = rows[2].bounds();
            assert!(header.y >= 0.0);
            assert!(viewport.y >= header.y + header.height);
            assert!(viewport.y + viewport.height <= footer.y);
            assert!(footer.y + footer.height <= card.bounds().height);
            let fields_container = rows[1].children().next().expect("scroll content");
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
