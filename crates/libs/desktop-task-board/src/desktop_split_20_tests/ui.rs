use iced::advanced::{
    clipboard,
    renderer::{Headless, Style},
    widget::{Id, Operation, operation},
};
use iced::keyboard::{self, Key};
use iced::{Element, Event, Font, Point, Rectangle, Renderer, Size, Theme, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};

pub(super) type Ui<'a, M> = UserInterface<'a, M, Theme, Renderer>;

pub(super) async fn renderer() -> Renderer {
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in desktop_assets::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
        .await
        .unwrap()
}

pub(super) fn scene<'a, M: Clone + std::fmt::Debug>(
    view: Element<'a, M>,
    renderer: &mut Renderer,
    size: Size<u32>,
) -> Ui<'a, M> {
    UserInterface::build(
        view,
        Size::new(size.width as f32, size.height as f32),
        Cache::new(),
        renderer,
    )
}

pub(super) fn send<M: Clone + std::fmt::Debug>(
    ui: &mut Ui<'_, M>,
    renderer: &mut Renderer,
    cursor: mouse::Cursor,
    events: &[Event],
) -> Vec<M> {
    let mut messages = Vec::new();
    ui.update(events, cursor, renderer, &mut clipboard::Null, &mut messages);
    messages
}

pub(super) fn click<M: Clone + std::fmt::Debug>(
    ui: &mut Ui<'_, M>,
    renderer: &mut Renderer,
    point: Point,
) -> Vec<M> {
    send(
        ui,
        renderer,
        mouse::Cursor::Available(point),
        &[
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        ],
    )
}

pub(super) fn capture<M: Clone + std::fmt::Debug>(
    ui: &mut Ui<'_, M>,
    renderer: &mut Renderer,
    theme: &Theme,
    size: Size<u32>,
    name: &str,
    cursor: mouse::Cursor,
) -> Vec<u8> {
    assert!(
        send(
            ui,
            renderer,
            cursor,
            &[Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now()))],
        )
        .is_empty()
    );
    ui.draw(renderer, theme, &Style { text_color: theme.palette().text }, cursor);
    let pixels = renderer.screenshot(size, 1.0, theme.palette().background);
    if let Some(directory) = std::env::var_os("SHIJU_UI_REFRESH_35_SCREENSHOTS") {
        ::image::save_buffer(
            std::path::PathBuf::from(directory).join(format!("{name}.png")),
            &pixels,
            size.width,
            size.height,
            ::image::ColorType::Rgba8,
        )
        .unwrap();
    }
    pixels
}

#[derive(Default)]
pub(super) struct Labels {
    pub(super) text: Vec<(String, Rectangle)>,
    pub(super) inputs: Vec<(String, Rectangle)>,
    pub(super) scrolls: Vec<(Option<Id>, Rectangle, iced::Vector)>,
}

impl Operation for Labels {
    fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) {
        children(self);
    }

    fn text(&mut self, _: Option<&Id>, bounds: Rectangle, value: &str) {
        self.text.push((value.into(), bounds));
    }
    fn text_input(
        &mut self,
        _: Option<&Id>,
        bounds: Rectangle,
        state: &mut dyn operation::TextInput,
    ) {
        self.inputs.push((state.text().into(), bounds));
    }
    fn scrollable(
        &mut self,
        id: Option<&Id>,
        bounds: Rectangle,
        _: Rectangle,
        translation: iced::Vector,
        _: &mut dyn operation::Scrollable,
    ) {
        self.scrolls.push((id.cloned(), bounds, translation));
    }
}

pub(super) fn labels<M>(ui: &mut Ui<'_, M>, renderer: &Renderer) -> Labels {
    let mut labels = Labels::default();
    ui.operate(renderer, &mut labels);
    labels
}

impl Labels {
    pub(super) fn bounds(&self, label: &str) -> Rectangle {
        self.text
            .iter()
            .find(|(value, _)| value == label)
            .unwrap_or_else(|| panic!("missing {label}: {:?}", self.text))
            .1
    }
    pub(super) fn input(&self, value: &str) -> Rectangle {
        self.inputs.iter().find(|(text, _)| text == value).expect("fixture input").1
    }

    pub(super) fn scroll(&self, id: &str) -> (Rectangle, iced::Vector) {
        let id = Id::from(id.to_owned());
        self.scrolls
            .iter()
            .find(|(actual, _, _)| actual.as_ref() == Some(&id))
            .map(|(_, bounds, offset)| (*bounds, *offset))
            .expect("fixture scroll")
    }
}

pub(super) fn key<M: Clone + std::fmt::Debug>(
    ui: &mut Ui<'_, M>,
    renderer: &mut Renderer,
    key: Key,
    text: Option<&str>,
) -> Vec<M> {
    send(
        ui,
        renderer,
        mouse::Cursor::Unavailable,
        &[Event::Keyboard(keyboard::Event::KeyPressed {
            modified_key: key.clone(),
            key,
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::empty(),
            text: text.map(Into::into),
            repeat: false,
        })],
    )
}

pub(super) fn wheel<M: Clone + std::fmt::Debug>(
    ui: &mut Ui<'_, M>,
    renderer: &mut Renderer,
    point: Point,
    y: f32,
) -> Vec<M> {
    send(
        ui,
        renderer,
        mouse::Cursor::Available(point),
        &[Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels { x: 0.0, y },
        })],
    )
}

pub(super) fn hover<M: Clone + std::fmt::Debug>(
    ui: &mut Ui<'_, M>,
    renderer: &mut Renderer,
    point: Point,
) -> mouse::Cursor {
    let cursor = mouse::Cursor::Available(point);
    assert!(
        send(ui, renderer, cursor, &[Event::Mouse(mouse::Event::CursorMoved { position: point })])
            .is_empty()
    );
    cursor
}
