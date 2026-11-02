use super::settings_extensions::{domain_label, operation_error, text};
use vw_config_types::ui::DesktopLocale;

#[test]
fn desktop_gettext_area_6_uses_chinese_keys() {
    let raw = "用户/%n/原始错误";
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "钩子配置"),
        (DesktopLocale::TraditionalChinese, "鉤子設定"),
        (DesktopLocale::English, "Hook configuration"),
    ] {
        assert_eq!(text(locale, "钩子配置"), expected);
        assert!(operation_error(locale, raw).ends_with(raw));
        assert!(domain_label(locale, 42).ends_with("42"));
    }
}

#[test]
fn desktop_gettext_area_6_catalog_covers_hooks_and_http() {
    let en = super::catalog::parse_catalog(include_str!("locales/en.json")).unwrap();
    let tw = super::catalog::parse_catalog(include_str!("locales/zh-TW.json")).unwrap();
    for source in [
        "钩子配置",
        "网络请求配置",
        "操作失败：%s",
        "域名 %u",
        "例如：example.com 或 *.example.org 或 *",
    ] {
        assert!(en.contains_key(source), "{source}");
        assert!(tw.contains_key(source), "{source}");
    }
    let raw = "原始 %s %n";
    let chinese = operation_error(DesktopLocale::SimplifiedChinese, raw);
    let english = operation_error(DesktopLocale::English, raw);
    assert_eq!(chinese, format!("操作失败：{raw}"));
    assert_eq!(english, format!("Operation failed: {raw}"));
}

#[test]
fn desktop_gettext_area_6_network_and_agent_basic_cover_visible_sources() {
    for source in ["隧道配置", "网页搜索配置", "子智能体委托", "max_results 必须是整数"]
    {
        let en = super::I18n::new(DesktopLocale::English).__(source);
        let tw = super::I18n::new(DesktopLocale::TraditionalChinese).__(source);
        assert_ne!(en, source);
        assert_ne!(tw, source);
        assert_eq!(super::I18n::new(DesktopLocale::SimplifiedChinese).__(source), source);
    }
}

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

#[test]
fn desktop_gettext_area_6_remaining_sections_have_catalog_entries() {
    let en = super::catalog::parse_catalog(include_str!("locales/en.json")).unwrap();
    let tw = super::catalog::parse_catalog(include_str!("locales/zh-TW.json")).unwrap();
    for source in [
        "智能体配置",
        "技能访问策略",
        "只保留只读分析能力。",
        "群聊回复模式",
        "全部消息",
        "飞书实例 %s",
        "工作项目",
        "MCP 配置",
        "%u 个服务器",
        "另有 %u 个工具未展开",
        "技能配置",
        "发现顺序",
        "安装插件",
        "原声明：技能目录 %u · MCP 声明组 %u · 钩子声明组 %u · 界面 %s",
        "读取技能目录失败: %s",
        "已删除插件: %s",
        "隧道配置",
        "网页搜索配置",
    ] {
        assert!(en.contains_key(source), "missing English: {source}");
        assert!(tw.contains_key(source), "missing Traditional Chinese: {source}");
    }
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
