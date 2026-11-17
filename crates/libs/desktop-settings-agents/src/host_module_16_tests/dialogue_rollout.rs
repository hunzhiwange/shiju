use super::dialogue_inputs::{DialogueFixture, Message};
use config_ui::DesktopLocale;
use iced::advanced::clipboard;
use iced::advanced::renderer::{Headless, Style};
use iced::advanced::widget::{Id, Operation};
use iced::{Element, Event, Point, Rectangle, Renderer, Size, Theme, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};

pub(super) const MODES: [(DesktopLocale, Theme); 3] = [
    (DesktopLocale::SimplifiedChinese, Theme::Light),
    (DesktopLocale::English, Theme::Dark),
    (DesktopLocale::TraditionalChinese, Theme::Dark),
];
const SIZE: Size<u32> = Size::new(480, 1240);
type Ui<'a> = UserInterface<'a, Message, Theme, Renderer>;

#[derive(Default)]
struct Geometry {
    labels: Vec<(String, Rectangle)>,
    inputs: Vec<(String, Rectangle)>,
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
        state: &mut dyn iced::advanced::widget::operation::TextInput,
    ) {
        self.inputs.push((state.text().into(), bounds));
    }
}

pub(super) struct Scene<'a> {
    ui: Ui<'a>,
    renderer: &'a mut Renderer,
    app: &'a DialogueFixture,
    name: &'a str,
}

impl<'a> Scene<'a> {
    pub(super) fn new(
        app: &'a DialogueFixture,
        renderer: &'a mut Renderer,
        name: &'a str,
        page: Element<'a, Message>,
    ) -> Self {
        use iced::Length;
        use iced::widget::scrollable::{Direction, Scrollbar};
        use iced::widget::{container, scrollable};

        let frame = scrollable(container(page).padding([0, 7]).width(Length::Fill))
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
            .height(Length::Fill);
        let ui = UserInterface::build(
            frame,
            Size::from((SIZE.width, SIZE.height)),
            Cache::new(),
            renderer,
        );
        Self { ui, renderer, app, name }
    }

    fn geometry(&mut self) -> Geometry {
        let mut geometry = Geometry::default();
        self.ui.operate(self.renderer, &mut geometry);
        geometry
    }

    pub(super) fn label(&mut self, label: &str) -> Rectangle {
        let bounds = self
            .geometry()
            .labels
            .into_iter()
            .find(|(text, _)| text == label)
            .unwrap_or_else(|| panic!("{}: missing label {label:?}", self.name))
            .1;
        self.assert_fits(bounds);
        bounds
    }

    fn assert_fits(&self, bounds: Rectangle) {
        assert!(bounds.width > 0.0 && bounds.height > 0.0, "{bounds:?}");
        assert!(bounds.x >= 0.0 && bounds.x + bounds.width <= SIZE.width as f32, "{bounds:?}");
        assert!(bounds.y >= 0.0 && bounds.y + bounds.height <= SIZE.height as f32, "{bounds:?}");
    }

    fn send(&mut self, cursor: mouse::Cursor, events: &[Event]) -> Vec<Message> {
        let mut messages = Vec::new();
        self.ui.update(events, cursor, self.renderer, &mut clipboard::Null, &mut messages);
        messages.retain(|message| !matches!(message, Message::None));
        messages
    }

    pub(super) fn click(&mut self, point: Point) -> Vec<Message> {
        self.send(
            mouse::Cursor::Available(point),
            &[
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            ],
        )
    }

    fn key(&mut self, name: iced::keyboard::key::Named) -> Vec<Message> {
        use iced::keyboard::{self, Key};
        let key = Key::Named(name);
        self.send(
            mouse::Cursor::Unavailable,
            &[Event::Keyboard(keyboard::Event::KeyPressed {
                modified_key: key.clone(),
                key,
                physical_key: keyboard::key::Physical::Unidentified(
                    keyboard::key::NativeCode::Unidentified,
                ),
                location: keyboard::Location::Standard,
                modifiers: keyboard::Modifiers::empty(),
                text: None,
                repeat: false,
            })],
        )
    }

    fn frame(&mut self, tag: &str, cursor: mouse::Cursor) -> Vec<u8> {
        assert!(
            self.send(
                cursor,
                &[Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now(),))]
            )
            .is_empty()
        );
        let theme = &self.app.app_theme;
        self.ui.draw(self.renderer, theme, &Style { text_color: theme.palette().text }, cursor);
        let pixels = self.renderer.screenshot(SIZE, 1.0, theme.palette().background);
        if let Some(out) = std::env::var_os("SHIJU_ROLLOUT_17_OUTPUT") {
            let path = std::path::PathBuf::from(out)
                .join("screenshots")
                .join(format!("{}-{:?}-{tag}.png", self.name, self.app.desktop_locale,));
            ::image::save_buffer(path, &pixels, SIZE.width, SIZE.height, ::image::ColorType::Rgba8)
                .unwrap();
        }
        pixels
    }

    pub(super) fn help(&mut self, title: &str, description: &str) -> Point {
        assert!(
            self.geometry().labels.iter().all(|(text, _)| text != description),
            "{}: {title}: static explanation is still inline",
            self.name
        );
        let bounds = self.label(title);
        let away = mouse::Cursor::Unavailable;
        let idle = self.frame(&format!("{title}-idle"), away);
        let point = [
            Point::new(bounds.x + bounds.width + 15.0, bounds.center_y()),
            Point::new(bounds.x + 11.0, bounds.y + bounds.height + 15.0),
        ]
        .into_iter()
        .filter(|point| {
            Rectangle::with_size(Size::from((SIZE.width, SIZE.height))).contains(*point)
        })
        .find(|&position| {
            let cursor = mouse::Cursor::Available(position);
            assert!(
                self.send(cursor, &[Event::Mouse(mouse::Event::CursorMoved { position })])
                    .is_empty()
            );
            self.frame(&format!("{title}-hover"), cursor) != idle
        })
        .unwrap_or_else(|| panic!("{}: hovering beside {title} must open help", self.name));

        assert!(self.click(point).is_empty());
        let open = self.frame(&format!("{title}-open"), away);
        assert!(open != idle, "{title}: clicking must keep help open when the pointer leaves");
        assert!(self.key(iced::keyboard::key::Named::Escape).is_empty());
        let closed = self.frame(&format!("{title}-closed"), away);
        assert!(closed == idle, "{title}: Escape must restore the initial frame");
        point
    }

    pub(super) fn actions(&mut self, icon: Point, description: &str) {
        assert!(self.click(icon).is_empty());
        let body = Point::new(icon.x + 29.0, icon.y + 5.0);
        let line_height = iced::advanced::text::LineHeight::default().to_absolute(12.0.into());
        let item_height = f32::from(line_height) + 10.0;
        for index in [0, 3] {
            assert!(
                self.send(
                    mouse::Cursor::Available(body),
                    &[Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),]
                )
                .is_empty()
            );
            let item = Point::new(body.x + 24.0, body.y + 3.0 + item_height * (index as f32 + 0.5));
            let messages = self.click(item);
            assert!(
                match (index, messages.as_slice()) {
                    (0, [Message::CopyCode(value)]) => value == description,
                    (3, [Message::OpenUrlExternal(url)]) => {
                        *url == format!(
                            "https://www.google.com/search?q={}",
                            urlencoding::encode(description)
                        )
                    }
                    _ => false,
                },
                "unexpected help action: {messages:?}"
            );
        }
        assert!(self.key(iced::keyboard::key::Named::Escape).is_empty());
    }
}
