//! 本组真实 UI 用例共享的事件、布局与截图驱动。
use iced::advanced::widget::{Id, Operation, operation};
use iced::advanced::{clipboard, renderer::Headless};
use iced::keyboard::{self, Key};
use iced::{Element, Event, Font, Point, Rectangle, Renderer, Size, Theme, Vector, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
pub type Ui<'a, Message> = UserInterface<'a, Message, Theme, Renderer>;
pub async fn renderer() -> Renderer {
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

pub fn scene<'a, Message: Clone + std::fmt::Debug + 'a>(
    view: Element<'a, Message>,
    renderer: &mut Renderer,
    size: Size<u32>,
) -> Ui<'a, Message> {
    UserInterface::build(view, viewport(size).size(), Cache::new(), renderer)
}

pub fn viewport(size: Size<u32>) -> Rectangle {
    Rectangle::with_size(Size::new(size.width as f32, size.height as f32))
}

pub fn send<Message: Clone + std::fmt::Debug>(
    ui: &mut Ui<'_, Message>,
    renderer: &mut Renderer,
    point: Point,
    events: &[Event],
) -> Vec<Message> {
    let mut messages = Vec::new();
    ui.update(
        events,
        mouse::Cursor::Available(point),
        renderer,
        &mut clipboard::Null,
        &mut messages,
    );
    messages
}

pub fn click<Message: Clone + std::fmt::Debug>(
    ui: &mut Ui<'_, Message>,
    renderer: &mut Renderer,
    point: Point,
) -> Vec<Message> {
    send(
        ui,
        renderer,
        point,
        &[
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        ],
    )
}

pub fn key<Message: Clone + std::fmt::Debug>(
    ui: &mut Ui<'_, Message>,
    renderer: &mut Renderer,
    point: Point,
    key: Key,
    text: Option<&str>,
) -> Vec<Message> {
    send(
        ui,
        renderer,
        point,
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

pub fn hover<Message: Clone + std::fmt::Debug>(
    ui: &mut Ui<'_, Message>,
    renderer: &mut Renderer,
    point: Point,
) {
    let messages =
        send(ui, renderer, point, &[Event::Mouse(mouse::Event::CursorMoved { position: point })]);
    assert!(messages.is_empty(), "{messages:?}");
}

pub fn scroll<Message: Clone + std::fmt::Debug>(
    ui: &mut Ui<'_, Message>,
    renderer: &mut Renderer,
    point: Point,
    y: f32,
) {
    let messages = send(
        ui,
        renderer,
        point,
        &[Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels { x: 0.0, y },
        })],
    );
    assert!(messages.is_empty(), "{messages:?}");
}

pub fn capture<Message: Clone + std::fmt::Debug>(
    ui: &mut Ui<'_, Message>,
    renderer: &mut Renderer,
    theme: &Theme,
    size: Size<u32>,
    name: &str,
    point: Point,
) -> Vec<u8> {
    let messages = send(
        ui,
        renderer,
        point,
        &[Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now()))],
    );
    assert!(messages.is_empty(), "{messages:?}");
    // Iced rebuilds the overlay between update and draw, as it does in the application.
    ui.draw(
        renderer,
        theme,
        &iced::advanced::renderer::Style { text_color: theme.palette().text },
        mouse::Cursor::Available(point),
    );
    let pixels = renderer.screenshot(size, 1.0, theme.palette().background);
    if let Some(directory) = std::env::var_os("SHIJU_UI_REFRESH_42_SCREENSHOTS") {
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
pub struct Bounds {
    pub labels: Vec<(String, Rectangle)>,
    pub inputs: Vec<(String, Rectangle)>,
    pub scrolls: Vec<(Option<Id>, Rectangle, Vector)>,
    pub focusable: Vec<Rectangle>,
}

impl Operation for Bounds {
    fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) {
        children(self);
    }

    fn text(&mut self, _: Option<&Id>, bounds: Rectangle, value: &str) {
        self.labels.push((value.into(), bounds));
    }

    fn text_input(
        &mut self,
        _: Option<&Id>,
        bounds: Rectangle,
        input: &mut dyn operation::TextInput,
    ) {
        self.inputs.push((input.text().into(), bounds));
    }

    fn scrollable(
        &mut self,
        id: Option<&Id>,
        bounds: Rectangle,
        _: Rectangle,
        offset: Vector,
        _: &mut dyn operation::Scrollable,
    ) {
        self.scrolls.push((id.cloned(), bounds, offset));
    }

    fn focusable(&mut self, _: Option<&Id>, bounds: Rectangle, _: &mut dyn operation::Focusable) {
        self.focusable.push(bounds);
    }
}

pub fn bounds<Message: Clone + std::fmt::Debug>(
    ui: &mut Ui<'_, Message>,
    renderer: &Renderer,
) -> Bounds {
    let mut result = Bounds::default();
    ui.operate(renderer, &mut result);
    result
}

pub fn label(layout: &Bounds, name: &str) -> Rectangle {
    layout
        .labels
        .iter()
        .find(|(text, _)| text == name)
        .unwrap_or_else(|| panic!("missing {name:?} in {:?}", layout.labels))
        .1
}

pub fn inside(inner: Rectangle, outer: Rectangle) {
    assert!(
        inner.width > 0.0
            && inner.height > 0.0
            && inner.x >= outer.x
            && inner.y >= outer.y
            && inner.x + inner.width <= outer.x + outer.width
            && inner.y + inner.height <= outer.y + outer.height,
        "{inner:?} outside {outer:?}",
    );
}

/// 为字体与图形全局资源保留原用例的独立进程边界。
pub fn isolated(test: &str) -> bool {
    const CHILD: &str = "SHIJU_CHAT_INPUT_UI_CHILD";
    if std::env::var_os(CHILD).is_some() {
        return true;
    }
    let (_, test) = test.split_once("::").expect("qualified test name");
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", test])
        .env(CHILD, "1")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("1 passed; 0 failed"),
        "{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    false
}
