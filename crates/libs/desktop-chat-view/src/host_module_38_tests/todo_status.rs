#![cfg(not(target_arch = "wasm32"))]
#![allow(dead_code, unused_imports)]
use crate::host_module_5_tests::fixture::{Message, RenderFixture};
use desktop_i18n::I18n;
use session_ui_types::ChatRole;
fn fixture() -> RenderFixture {
    RenderFixture::new().0
}
use config_ui::DesktopLocale;
use iced::advanced::{
    Layout, clipboard, layout,
    renderer::{Headless, Style},
    widget::{Id, Operation, Tree, operation},
};
use iced::keyboard::{self, Key, key::Named};
use iced::widget::{button, container};
use iced::{Element, Event, Font, Point, Rectangle, Renderer, Size, Theme, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};

type Ui<'a> = UserInterface<'a, Message, Theme, Renderer>;

async fn renderer() -> Renderer {
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

fn root() -> Element<'static, Message> {
    container(button("underlay").on_press(Message::None))
        .width(iced::Fill)
        .height(iced::Fill)
        .into()
}

fn scene<'a>(view: Element<'a, Message>, renderer: &mut Renderer, size: Size<u32>) -> Ui<'a> {
    UserInterface::build(
        view,
        Size::new(size.width as f32, size.height as f32),
        Cache::new(),
        renderer,
    )
}

fn send(
    ui: &mut Ui<'_>,
    renderer: &mut Renderer,
    cursor: mouse::Cursor,
    events: &[Event],
) -> Vec<Message> {
    let mut messages = Vec::new();
    ui.update(events, cursor, renderer, &mut clipboard::Null, &mut messages);
    messages
}

fn click(ui: &mut Ui<'_>, renderer: &mut Renderer, point: Point) -> Vec<Message> {
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

fn key(ui: &mut Ui<'_>, renderer: &mut Renderer, key: Key, text: Option<&str>) -> Vec<Message> {
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

fn capture(
    ui: &mut Ui<'_>,
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
    if let Some(directory) = std::env::var_os("SHIJU_UI_REFRESH_32_SCREENSHOTS") {
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
struct Geometry {
    labels: Vec<(String, Rectangle)>,
    inputs: Vec<(String, Rectangle)>,
    scroll: Vec<(Rectangle, iced::Vector)>,
}

impl Operation for Geometry {
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
        state: &mut dyn operation::TextInput,
    ) {
        self.inputs.push((state.text().into(), bounds));
    }

    fn scrollable(
        &mut self,
        _: Option<&Id>,
        bounds: Rectangle,
        _: Rectangle,
        translation: iced::Vector,
        _: &mut dyn operation::Scrollable,
    ) {
        self.scroll.push((bounds, translation));
    }
}

fn geometry(ui: &mut Ui<'_>, renderer: &Renderer) -> Geometry {
    let mut geometry = Geometry::default();
    ui.operate(renderer, &mut geometry);
    geometry
}

fn label(geometry: &Geometry, value: &str) -> Rectangle {
    geometry
        .labels
        .iter()
        .find(|(label, _)| label == value)
        .unwrap_or_else(|| panic!("missing {value}: {:?}", geometry.labels))
        .1
}

fn assert_fits(geometry: &Geometry, size: Size<u32>) {
    for (value, bounds) in geometry.labels.iter().chain(&geometry.inputs) {
        assert!(bounds.width > 0.0 && bounds.height > 0.0, "{value}: {bounds:?}");
        assert!(bounds.x >= 0.0 && bounds.y >= 0.0, "{value}: {bounds:?}");
        assert!(bounds.x + bounds.width <= size.width as f32 + 0.1, "{value}: {bounds:?}");
        assert!(bounds.y + bounds.height <= size.height as f32 + 0.1, "{value}: {bounds:?}");
    }
}

fn find_layout<'a>(
    layout: Layout<'a>,
    matches: impl Fn(Rectangle) -> bool + Copy,
) -> Option<Layout<'a>> {
    if matches(layout.bounds()) {
        return Some(layout);
    }
    layout.children().find_map(|child| find_layout(child, matches))
}

fn assert_control_alignment(
    mut view: Element<'_, Message>,
    renderer: &Renderer,
    size: Size<u32>,
    input: Option<Rectangle>,
) -> Point {
    let mut tree = Tree::new(view.as_widget());
    let limits = layout::Limits::new(Size::ZERO, Size::new(size.width as f32, size.height as f32));
    let node = view.as_widget_mut().layout(&mut tree, renderer, &limits);
    let layout = Layout::new(&node);
    let close = find_layout(layout, |bounds| bounds.width == 28.0 && bounds.height == 28.0)
        .expect("close button");
    let icon = find_layout(close, |bounds| bounds.width == 12.0 && bounds.height == 12.0)
        .expect("close icon");
    assert!((close.bounds().center_y() - icon.bounds().center_y()).abs() < 0.5);
    if let Some(input) = input {
        let field = find_layout(layout, |bounds| bounds == input).expect("input field");
        let glyphs = field.children().next().expect("input text");
        assert!((glyphs.bounds().center_y() - input.center_y()).abs() < 0.5);
    }
    close.bounds().center()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn todo_continuation_status_draws_centered_in_both_themes() {
    use crate::host_module_5_tests::message_view;
    use crate::message_view::contracts::TODO_CONTINUATION_TEXT;
    let mut app = fixture();
    let cache = crate::render_cache::ChatRenderCacheEntry::default();
    let mut renderer = renderer().await;
    let size = Size::new(480, 64);
    for (locale, language) in
        [(DesktopLocale::SimplifiedChinese, "zh"), (DesktopLocale::English, "en")]
    {
        app.desktop_locale = locale;
        for (theme, mode) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
            let view = message_view(
                &app,
                0,
                ChatRole::System,
                TODO_CONTINUATION_TEXT,
                &[],
                None,
                None,
                &cache,
                false,
            );
            let mut ui = scene(view, &mut renderer, size);
            let bounds = geometry(&mut ui, &renderer);
            let text = label(&bounds, I18n::new(locale).__(TODO_CONTINUATION_TEXT));
            assert!((text.center().x - 240.0).abs() < 1.0);
            assert_fits(&bounds, size);
            let pixels = capture(
                &mut ui,
                &mut renderer,
                &theme,
                size,
                &format!("todo-continuation-{language}-{mode}"),
                mouse::Cursor::Unavailable,
            );
            assert!(pixels.chunks_exact(4).any(|pixel| pixel != &pixels[..4]));
        }
    }
}
