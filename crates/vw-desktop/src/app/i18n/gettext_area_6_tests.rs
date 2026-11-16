use config_ui::DesktopLocale;

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
    let toast = app.toast_state.active_toast.as_ref().unwrap().localized_message.as_ref().unwrap();
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
    app.general_settings.desktop_locale = DesktopLocale::English;
    let entries = crate::app::components::system_settings_search::catalog(&app);
    let global = entries.iter().find(|entry| entry.label == "Global servers").unwrap();
    assert!(global.matches("Global servers"));
    assert!(global.matches("全局服务器"));
    let skills = entries.iter().find(|entry| entry.label == "System configuration").unwrap();
    assert!(skills.matches("系统配置"));
    assert!(skills.matches("directory provider"));
}

#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_6_settings_render_three_locales_in_both_themes() {
    use iced::advanced::renderer::Headless;
    use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
    use iced::{Color, Font, Pixels, Rectangle, Size, Theme};
    let (mut app, _) = crate::app::App::new();
    app.mcp_settings.new_server_expanded = true;
    for locale in [
        DesktopLocale::SimplifiedChinese,
        DesktopLocale::TraditionalChinese,
        DesktopLocale::English,
    ] {
        app.general_settings.desktop_locale = locale;
        for theme in [Theme::Dark, Theme::Light] {
            for region in ["mcp"] {
                let mut element = crate::app::components::system_settings_mcp::view(&app);
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
