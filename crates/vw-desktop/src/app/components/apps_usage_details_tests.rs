//! Shared renderer fixture for desktop combination tests.
use iced::{Element, Event, Font, Rectangle, Renderer, Size, advanced::{clipboard, renderer::Headless, widget::operation::Operation}, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
use crate::app::Message;
#[derive(Default)]
struct Texts(Vec<(String, Rectangle)>);

impl Operation for Texts {
    fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) {
        children(self);
    }

    fn text(&mut self, _: Option<&iced::advanced::widget::Id>, bounds: Rectangle, value: &str) {
        self.0.push((value.into(), bounds));
    }
}

pub(super) struct Scene<'a> {
    ui: UserInterface<'a, Message, iced::Theme, Renderer>,
    texts: Texts,
}

impl<'a> Scene<'a> {
    pub(super) fn new(element: Element<'a, Message>, renderer: &mut Renderer) -> Self {
        let mut ui = UserInterface::build(element, Size::new(640.0, 700.0), Cache::new(), renderer);
        let mut texts = Texts::default();
        ui.operate(renderer, &mut texts);
        Self { ui, texts }
    }

    pub(super) fn bounds(&self, label: &str) -> Option<Rectangle> {
        self.texts.0.iter().find(|(value, _)| value == label).map(|(_, bounds)| *bounds)
    }

    pub(super) fn send(
        &mut self,
        renderer: &mut Renderer,
        label: &str,
        events: &[Event],
    ) -> Vec<Message> {
        let point = self.bounds(label).expect(label).center();
        let mut messages = Vec::new();
        self.ui.update(
            events,
            mouse::Cursor::Available(point),
            renderer,
            &mut clipboard::Null,
            &mut messages,
        );
        messages.retain(|message| !matches!(message, Message::None));
        messages
    }
}

pub(super) async fn renderer() -> Renderer {
    static FONTS: std::sync::Once = std::sync::Once::new();
    FONTS.call_once(|| {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    });
    Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
        .await
        .expect("tiny-skia renderer")
}
