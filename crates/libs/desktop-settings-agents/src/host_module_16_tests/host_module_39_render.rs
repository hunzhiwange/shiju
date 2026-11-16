#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_6_settings_render_three_locales_in_both_themes() {
    use config_ui::DesktopLocale;
    use iced::advanced::renderer::Headless;
    use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
    use iced::{Color, Font, Pixels, Rectangle, Size, Theme};
    let mut state = super::catalog_fixture::CatalogFixture::default();
    state.agents_settings.config_ready = true;
    state.agents_settings.available_tools =
        vec!["file_read".into(), "bash".into(), "web_search".into()];
    for locale in [
        DesktopLocale::SimplifiedChinese,
        DesktopLocale::TraditionalChinese,
        DesktopLocale::English,
    ] {
        for theme in [Theme::Dark, Theme::Light] {
            for region in ["agents", "agent-tools", "agent-skills", "agent-identity"] {
                state.agents_settings.active_tools_list_tab =
                    crate::state::AGENT_TOOLS_BUILTIN_TAB.to_owned();
                state.agents_settings.active_detail_tab = match region {
                    "agent-tools" => "tools",
                    "agent-skills" => "skills",
                    "agent-identity" => "identity",
                    _ => "basic",
                }
                .to_owned();
                let mut input = state.view_input();
                input.catalog.locale = locale;
                input.theme = &theme;
                let mut element = crate::agents_view::agents_view(input);
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
