#![allow(dead_code)] // Shared with host integration tests through a forward-only source path.
#[derive(Default)]
pub struct Scene {
    pub labels: Vec<(String, iced::Rectangle)>,
    pub preview: Option<iced::Rectangle>,
    expected_preview: &'static str,
}

impl iced::advanced::widget::Operation for Scene {
    fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn iced::advanced::widget::Operation)) {
        children(self);
    }
    fn text(
        &mut self,
        _: Option<&iced::advanced::widget::Id>,
        bounds: iced::Rectangle,
        label: &str,
    ) {
        self.labels.push((label.into(), bounds));
    }
    fn container(&mut self, id: Option<&iced::advanced::widget::Id>, bounds: iced::Rectangle) {
        if id == Some(&iced::advanced::widget::Id::new(self.expected_preview)) {
            self.preview = Some(bounds);
        }
    }
}

impl Scene {
    pub fn bounds(&self, label: &str) -> iced::Rectangle {
        self.labels
            .iter()
            .find(|(value, _)| value == label)
            .unwrap_or_else(|| panic!("missing {label}"))
            .1
    }
}

pub async fn scene<Message: Clone + 'static>(
    element: iced::Element<'_, Message>,
    expected_preview: &'static str,
    width: f32,
    click: Option<&str>,
) -> (Scene, Vec<Message>) {
    use iced::advanced::{clipboard, renderer::Headless};
    use iced::{Event, Font, Size, Theme, mouse};
    use iced_runtime::user_interface::{Cache, UserInterface};
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in desktop_assets::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    let mut renderer =
        iced::Renderer::new(Font::DEFAULT, 14.0.into(), Some("tiny-skia")).await.unwrap();
    let size = Size::new(width, 1600.0);
    let mut ui = UserInterface::build(element, size, Cache::new(), &mut renderer);
    let mut scene = Scene { expected_preview, ..Default::default() };
    ui.operate(&renderer, &mut scene);
    assert!(scene.preview.is_some(), "preview must use selected avatar");
    // Drawing traverses real font shaping, including Chinese text and both themes.
    for theme in [Theme::Light, Theme::Dark] {
        ui.draw(
            &mut renderer,
            &theme,
            &iced::advanced::renderer::Style { text_color: theme.palette().text },
            mouse::Cursor::Unavailable,
        );
        let pixels =
            renderer.screenshot(Size::new(width as u32, 1600), 1.0, theme.palette().background);
        assert!(
            pixels.chunks_exact(4).any(|pixel| pixel != &pixels[..4]),
            "preview must draw more than a uniform background"
        );
        if let Some(dir) = std::env::var_os("SHIJU_MINI_TASK4_SCREENSHOTS") {
            std::fs::create_dir_all(&dir).unwrap();
            image::save_buffer(
                std::path::PathBuf::from(dir).join(format!(
                    "{width}-{}-{theme}.png",
                    match expected_preview {
                        "companion-preview-none" => "None",
                        "companion-preview-robot" => "Robot",
                        "companion-preview-beauty" => "Beauty",
                        "companion-preview-handsome" => "Handsome",
                        _ => panic!("unknown companion preview"),
                    }
                )),
                &pixels,
                width as u32,
                1600,
                image::ColorType::Rgba8,
            )
            .unwrap();
        }
    }
    let mut messages = Vec::new();
    if let Some(label) = click {
        let point = scene.bounds(label).center();
        ui.update(
            &[
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            ],
            mouse::Cursor::Available(point),
            &mut renderer,
            &mut clipboard::Null,
            &mut messages,
        );
    }
    (scene, messages)
}
