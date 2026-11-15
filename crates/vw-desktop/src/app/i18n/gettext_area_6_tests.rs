use super::settings_extensions::{domain_label, operation_error, text};
use vw_config_types::ui::DesktopLocale;

#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_6_dynamic_feedback_preserves_raw_arguments() {
    use crate::app::message::settings::{HooksMessage, SettingsMessage};
    let (mut app, _) = crate::app::App::new();
    let raw = "用户目录/%n/%s/原始错误";
    let _ = crate::app::message::settings::update(
        &mut app,
        SettingsMessage::SkillsLoaded(Err(raw.to_owned())),
    );
    let status = app.skills_settings.status_message_translation.as_ref().unwrap();
    for (locale, prefix) in [
        (DesktopLocale::SimplifiedChinese, "读取技能目录失败: "),
        (DesktopLocale::TraditionalChinese, "讀取技能目錄失敗："),
        (DesktopLocale::English, "Failed to read skill catalog: "),
    ] {
        assert_eq!(status.render(super::I18n::new(locale)).unwrap(), format!("{prefix}{raw}"));
    }
    let _ = crate::app::message::settings::update(
        &mut app,
        SettingsMessage::Hooks(HooksMessage::CatalogLoaded(Err(raw.to_owned()))),
    );
    let toast = app.active_toast.as_ref().unwrap().localized_message.as_ref().unwrap();
    assert_eq!(
        toast.render(super::I18n::new(DesktopLocale::English)).unwrap(),
        format!("Failed to read plugin hook catalog: {raw}")
    );
    assert_eq!(
        toast.render(super::I18n::new(DesktopLocale::TraditionalChinese)).unwrap(),
        format!("讀取外掛 hook 清單失敗：{raw}")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_6_search_preserves_chinese_navigation() {
    let (mut app, _) = crate::app::App::new();
    app.desktop_locale = DesktopLocale::English;
    let entries = crate::app::components::system_settings_search::catalog(&app);
    let global = entries.iter().find(|entry| entry.label == "Global servers").unwrap();
    assert!(global.matches("Global servers"));
    assert!(global.matches("全局服务器"));
    let skills = entries.iter().find(|entry| entry.label == "System configuration").unwrap();
    assert!(skills.matches("系统配置"));
    assert!(skills.matches("directory provider"));
}

#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_6_mcp_validation_keeps_semantic_error() {
    use crate::app::message::settings::{McpMessage, SettingsMessage};
    let (mut app, _) = crate::app::App::new();
    app.mcp_settings.editor = iced::widget::text_editor::Content::with_text("{");
    let _ =
        crate::app::message::settings::update(&mut app, SettingsMessage::Mcp(McpMessage::Format));
    let message = app.mcp_settings.save_error_translation.as_ref().unwrap();
    assert!(
        message
            .render(super::I18n::new(DesktopLocale::English))
            .unwrap()
            .starts_with("JSON parse failed: ")
    );
    assert!(
        message
            .render(super::I18n::new(DesktopLocale::TraditionalChinese))
            .unwrap()
            .starts_with("JSON 解析失敗：")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_6_settings_render_three_locales_in_both_themes() {
    use iced::advanced::renderer::Headless;
    use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
    use iced::{Color, Font, Pixels, Rectangle, Size, Theme};
    let (mut app, _) = crate::app::App::new();
    app.agents_settings.config_ready = true;
    app.agents_settings.available_tools =
        vec!["file_read".into(), "bash".into(), "web_search".into()];
    app.mcp_settings.new_server_expanded = true;
    for locale in [
        DesktopLocale::SimplifiedChinese,
        DesktopLocale::TraditionalChinese,
        DesktopLocale::English,
    ] {
        app.desktop_locale = locale;
        for theme in [Theme::Dark, Theme::Light] {
            for region in [
                "tunnel",
                "web-search",
                "agents",
                "agent-tools",
                "agent-skills",
                "agent-identity",
                "channels",
                "mcp",
                "skills",
                "plugins",
            ] {
                app.agents_settings.active_tools_list_tab =
                    crate::app::state::AGENT_TOOLS_BUILTIN_TAB.to_owned();
                app.agents_settings.active_detail_tab = match region {
                    "agent-tools" => "tools",
                    "agent-skills" => "skills",
                    "agent-identity" => "identity",
                    _ => "basic",
                }
                .to_string();
                let mut element = match region {
                    "web-search" => crate::app::components::system_settings_web_search::view(&app),
                    "agents" | "agent-tools" | "agent-skills" | "agent-identity" => {
                        crate::app::components::system_settings_agents::view(&app)
                    }
                    "channels" => crate::app::components::system_settings_channels::view(&app),
                    "mcp" => crate::app::components::system_settings_mcp::view(&app),
                    "skills" => crate::app::components::system_settings_skills::view(&app),
                    "plugins" => {
                        crate::app::components::system_settings_skills::plugins::view(&app)
                    }
                    _ => crate::app::components::system_settings_tunnel::view(&app),
                };
                let mut renderer = <iced::Renderer as Headless>::new(
                    Font::DEFAULT,
                    Pixels(16.0),
                    Some("tiny-skia"),
                )
                .await
                .expect("headless renderer");
                let bounds = Size::new(960.0, 800.0);
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
                let pixels = renderer.screenshot(Size::new(960, 800), 1.0, Color::TRANSPARENT);
                assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
                let directory = std::env::temp_dir()
                    .join(format!("shiju-gettext-area-6-{}", std::process::id()));
                std::fs::create_dir_all(&directory).unwrap();
                ::image::RgbaImage::from_raw(960, 800, pixels)
                    .unwrap()
                    .save(directory.join(format!("{region}-{locale:?}-{theme:?}.png")))
                    .unwrap();
            }
        }
    }
}
