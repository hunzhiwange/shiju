#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_6_settings_render_three_locales_in_both_themes() {
    use config_ui::DesktopLocale;
    use iced::advanced::renderer::Headless;
    use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
    use iced::{Color, Font, Pixels, Rectangle, Size, Theme};
    let mut state = super::NetworkFixture::default();
    for locale in [
        DesktopLocale::SimplifiedChinese,
        DesktopLocale::TraditionalChinese,
        DesktopLocale::English,
    ] {
        for theme in [Theme::Dark, Theme::Light] {
            for region in ["tunnel", "web-search"] {
                state.desktop_locale = locale;
                state.app_theme = theme.clone();
                let mut element = match region {
                    "web-search" => super::web_search::view(&state),
                    _ => super::tunnel::view(&state),
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
