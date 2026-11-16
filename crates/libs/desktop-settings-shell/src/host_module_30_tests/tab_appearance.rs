#![allow(dead_code)] // Individual consumers exercise different shared assertion methods.
//! Shared real renderer, layout, pixels and interaction assertions for settings pages.
use config_ui::DesktopLocale;
use desktop_i18n::I18n;
use iced::advanced::{
    clipboard,
    renderer::{Headless, Style},
    widget::{
        Id, Operation,
        operation::{Scrollable, TextInput},
    },
};
use iced::{Color, Element, Event, Rectangle, Renderer, Size, Theme, Vector, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
use image::{Rgba, RgbaImage};
type Ui<'a, Message> = UserInterface<'a, Message, Theme, Renderer>;
pub trait AppearanceMessage: Clone + std::fmt::Debug {
    type Settings;
    fn none() -> Self;
    fn settings(self) -> Self::Settings;
}
pub trait AppearanceContext<Message> {
    fn locale(&self) -> DesktopLocale;
    fn general_view(&self) -> Element<'_, Message>;
}
#[derive(Default)]
pub struct Labels {
    text: Vec<(String, Rectangle)>,
    scrolls: Vec<(Rectangle, Rectangle, Vector)>,
    inputs: Vec<(String, Rectangle)>,
    select_input: Option<Rectangle>,
    offset: Vector,
    pending_scroll: Option<Vector>,
}

impl Operation for Labels {
    fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) {
        let parent_offset = self.offset;
        if let Some(translation) = self.pending_scroll.take() {
            self.offset -= translation;
        }
        children(self);
        self.offset = parent_offset;
    }

    fn text(&mut self, _: Option<&Id>, bounds: Rectangle, label: &str) {
        self.text.push((label.into(), self.translated(bounds)));
    }
    fn text_input(&mut self, _: Option<&Id>, bounds: Rectangle, state: &mut dyn TextInput) {
        let bounds = self.translated(bounds);
        if self.select_input == Some(bounds) {
            state.select_all();
        }
        self.inputs.push((state.text().into(), bounds));
    }
    fn scrollable(
        &mut self,
        _: Option<&Id>,
        bounds: Rectangle,
        content: Rectangle,
        translation: Vector,
        _: &mut dyn Scrollable,
    ) {
        self.scrolls.push((self.translated(bounds), self.translated(content), translation));
        // Iced exposes unscrolled child layout bounds to operations; pixels and pointer
        // events use the translated viewport. Keep assertions in that same coordinate space.
        self.pending_scroll = Some(translation);
    }
}

impl Labels {
    fn translated(&self, bounds: Rectangle) -> Rectangle {
        Rectangle { x: bounds.x + self.offset.x, y: bounds.y + self.offset.y, ..bounds }
    }

    pub fn contains(&self, label: &str) -> bool {
        self.text.iter().any(|(text, _)| text == label)
    }

    fn bounds(&self, label: &str) -> Rectangle {
        self.tabs(&[label])[0]
    }

    fn tabs(&self, names: &[&str]) -> Vec<Rectangle> {
        self.text
            .windows(names.len())
            .find(|labels| labels.iter().map(|(label, _)| label.as_str()).eq(names.iter().copied()))
            .unwrap_or_else(|| panic!("missing tab labels {names:?}"))
            .iter()
            .map(|(_, bounds)| *bounds)
            .collect()
    }
}

struct TabPixels {
    selected: Rgba<u8>,
    idle: Rgba<u8>,
    edge: Rgba<u8>,
    hovered: Rgba<u8>,
    pressed: Rgba<u8>,
}

pub struct TabAppearance<'a, Message> {
    pub message: std::marker::PhantomData<Message>,
    pub renderer: &'a mut Renderer,
    pub theme: Theme,
    pub width: u32,
}

impl<Message: AppearanceMessage + 'static> TabAppearance<'_, Message> {
    pub fn labels(&mut self, element: Element<'_, Message>) -> Labels {
        let mut ui = self.scene(element);
        self.geometry(&mut ui)
    }

    pub fn save_preview(
        &mut self,
        app: &impl AppearanceContext<Message>,
        element: Element<'_, Message>,
        name: &str,
    ) {
        if let Some(directory) = std::env::var_os("SHIJU_SETTINGS_PREVIEW_DIR") {
            let mut ui = self.scene(element);
            let preview = self.capture(&mut ui, mouse::Cursor::Unavailable);
            let filename = format!("{name}-{:?}-{}-{}.png", app.locale(), self.theme, self.width);
            preview.save(std::path::PathBuf::from(directory).join(filename)).unwrap();
        }
    }

    pub fn check_disabled(&mut self, element: Element<'_, Message>, label: &str) {
        let mut ui = self.scene(element);
        let cursor = mouse::Cursor::Available(self.geometry(&mut ui).bounds(label).center());
        assert!(self.send(&mut ui, cursor, press()).is_empty());
        let release = Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left));
        assert!(self.send(&mut ui, cursor, release).is_empty(), "{label} must be disabled");
    }

    pub fn check(
        &mut self,
        app: &impl AppearanceContext<Message>,
        element: Element<'_, Message>,
        names: &[&str],
        target: usize,
    ) -> Message::Settings {
        self.check_with_padding(app, element, names, target, 14)
    }

    pub fn check_with_padding(
        &mut self,
        app: &impl AppearanceContext<Message>,
        element: Element<'_, Message>,
        names: &[&str],
        target: usize,
        padding: u32,
    ) -> Message::Settings {
        settings_message(self.check_message(app, element, names, target, padding))
    }

    pub fn check_message(
        &mut self,
        app: &impl AppearanceContext<Message>,
        element: Element<'_, Message>,
        names: &[&str],
        target: usize,
        padding: u32,
    ) -> Message {
        let mut ui = self.scene(element);
        let tabs = self.geometry(&mut ui).tabs(names);
        let selected = tabs[0];
        for (label, tab) in names.iter().zip(&tabs) {
            assert!(
                tab.x >= padding as f32
                    && tab.x + tab.width + padding as f32 <= self.width as f32 + 0.5
                    && tab.height < 30.0,
                "{label}: tab is clipped or wraps inside its button: {tab:?}"
            );
        }

        let idle = self.capture(&mut ui, mouse::Cursor::Unavailable);
        // Cards and dialogs can have a different backdrop from the settings page.
        let surface = if selected.x >= (padding + 2) as f32 {
            let Rgba([r, g, b, _]) =
                *idle.get_pixel(selected.x as u32 - padding - 2, selected.center_y() as u32);
            Color::from_rgb8(r, g, b)
        } else {
            self.theme.palette().background
        };
        let reference = self.general_reference(app, surface);
        assert_pixel(fill(&idle, selected), reference.selected, names[0]);
        for (&label, &tab) in names.iter().zip(&tabs).skip(1) {
            assert_pixel(fill(&idle, tab), reference.idle, label);
            assert_pixel(edge(&idle, tab, padding), reference.edge, label);
        }

        let position = tabs[target].center();
        let cursor = mouse::Cursor::Available(position);
        assert!(
            self.send(&mut ui, cursor, Event::Mouse(mouse::Event::CursorMoved { position }))
                .is_empty()
        );
        let hovered = self.capture(&mut ui, cursor);
        assert_pixel(fill(&hovered, tabs[target]), reference.hovered, "hovered");
        assert!(self.send(&mut ui, cursor, press()).is_empty());
        let pressed = self.capture(&mut ui, cursor);
        assert_pixel(fill(&pressed, tabs[target]), reference.pressed, "pressed");
        self.release(&mut ui, cursor)
    }

    pub fn click(&mut self, element: Element<'_, Message>, label: &str) -> Message::Settings {
        settings_message(self.click_message(element, label))
    }

    pub fn click_message(&mut self, element: Element<'_, Message>, label: &str) -> Message {
        let mut ui = self.scene(element);
        let position = self.geometry(&mut ui).bounds(label).center();
        let cursor = mouse::Cursor::Available(position);
        assert!(self.send(&mut ui, cursor, press()).is_empty());
        self.release(&mut ui, cursor)
    }

    fn release(&mut self, ui: &mut Ui<'_, Message>, cursor: mouse::Cursor) -> Message {
        let mut messages =
            self.send(ui, cursor, Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)));
        assert_eq!(messages.len(), 1);
        messages.pop().unwrap()
    }

    pub fn check_selected_segment(&mut self, element: Element<'_, Message>, label: &str) {
        use iced::widget::{Space, button, row, text};

        let mut ui = self.scene(element);
        let bounds = self.geometry(&mut ui).bounds(label);
        let actual = self.capture(&mut ui, mouse::Cursor::Unavailable);
        let Rgba([r, g, b, _]) = *actual.get_pixel(bounds.x as u32 - 14, bounds.center_y() as u32);
        let reference = row![
            Space::new().width(24),
            button(text(label).size(12)).padding([8, 12]).on_press(Message::none()).style(
                |theme, status| {
                    desktop_ui_controls::system_settings_common::settings_segment_button_style(
                        theme, status, true,
                    )
                }
            ),
        ];
        let mut ui = self.scene(reference.into());
        let reference_bounds = self.geometry(&mut ui).bounds(label);
        let expected =
            self.capture_on(&mut ui, mouse::Cursor::Unavailable, Color::from_rgb8(r, g, b));
        assert_pixel(fill(&actual, bounds), fill(&expected, reference_bounds), label);
    }

    pub fn edit_modal_input(
        &mut self,
        element: Element<'_, Message>,
        current: &str,
        replacement: &str,
    ) -> Message {
        let mut ui = self.scene(element);
        let geometry = self.geometry(&mut ui);
        let (_, bounds) = geometry
            .inputs
            .iter()
            .find(|(value, _)| value == current)
            .unwrap_or_else(|| panic!("missing modal input {current:?}"));
        assert_eq!(
            bounds.height,
            desktop_ui_controls::system_settings_common::utils::SETTINGS_CONTROL_HEIGHT
        );
        assert!(
            geometry.scrolls.iter().any(|(viewport, _, _)| {
                bounds.x >= viewport.x + 7.0
                    && bounds.x + bounds.width <= viewport.x + viewport.width - 7.0
                    && bounds.y >= viewport.y
                    && bounds.y + bounds.height <= viewport.y + viewport.height
            }),
            "modal input must fit within the visible body gutters: {bounds:?}"
        );

        let cursor = mouse::Cursor::Available(bounds.center());
        let idle = self.capture(&mut ui, mouse::Cursor::Unavailable);
        assert!(self.send(&mut ui, cursor, press()).is_empty());
        let focused = self.capture(&mut ui, cursor);
        let border_x = bounds.x.round() as u32;
        let border_y = bounds.center_y().round() as u32;
        assert_ne!(idle.get_pixel(border_x, border_y), focused.get_pixel(border_x, border_y));

        ui.operate(self.renderer, &mut Labels { select_input: Some(*bounds), ..Labels::default() });
        let mut messages = self.send(
            &mut ui,
            cursor,
            Event::InputMethod(iced::advanced::input_method::Event::Commit(replacement.into())),
        );
        assert_eq!(messages.len(), 1);
        messages.pop().unwrap()
    }

    pub fn check_modal_scroll<'a>(
        &mut self,
        element: Element<'_, Message>,
        header: &str,
        footer: impl Into<Option<&'a str>>,
    ) {
        let footer = footer.into();
        let mut ui = self.scene_with_height(element, 460.0);
        let before = self.geometry(&mut ui);
        let (viewport, _, _) = before
            .scrolls
            .iter()
            .copied()
            .find(|(viewport, content, _)| content.height > viewport.height)
            .expect("long modal body must scroll");
        assert!(before.bounds(header).y < viewport.y);
        if let Some(footer) = footer {
            assert!(before.bounds(footer).y > viewport.y + viewport.height);
            assert!(before.bounds(footer).y + before.bounds(footer).height <= 460.0);
        }
        // Wheel over the body gutter so the nested text editor does not consume it.
        let cursor =
            mouse::Cursor::Available(iced::Point::new(viewport.x + 2.0, viewport.center_y()));
        assert!(
            self.send(
                &mut ui,
                cursor,
                Event::Mouse(mouse::Event::WheelScrolled {
                    delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -120.0 },
                }),
            )
            .is_empty()
        );
        let after = self.geometry(&mut ui);
        assert!(
            after
                .scrolls
                .iter()
                .any(|(bounds, _, offset)| { *bounds == viewport && offset.y != 0.0 })
        );
        assert_eq!(before.bounds(header), after.bounds(header));
        if let Some(footer) = footer {
            assert_eq!(before.bounds(footer), after.bounds(footer));
        }
        let _ = self.capture(&mut ui, cursor);
    }

    fn scene<'a>(&mut self, element: Element<'a, Message>) -> Ui<'a, Message> {
        self.scene_with_height(element, 760.0)
    }

    fn scene_with_height<'a>(
        &mut self,
        element: Element<'a, Message>,
        height: f32,
    ) -> Ui<'a, Message> {
        UserInterface::build(
            element,
            Size::new(self.width as f32, height),
            Cache::new(),
            self.renderer,
        )
    }

    fn geometry(&self, ui: &mut Ui<'_, Message>) -> Labels {
        let mut labels = Labels::default();
        ui.operate(self.renderer, &mut labels);
        labels
    }

    fn send(
        &mut self,
        ui: &mut Ui<'_, Message>,
        cursor: mouse::Cursor,
        event: Event,
    ) -> Vec<Message> {
        let mut messages = Vec::new();
        ui.update(&[event], cursor, self.renderer, &mut clipboard::Null, &mut messages);
        messages.retain(|message| !matches!(message, Message::none()));
        messages
    }

    fn capture(&mut self, ui: &mut Ui<'_, Message>, cursor: mouse::Cursor) -> RgbaImage {
        self.capture_on(ui, cursor, self.theme.palette().background)
    }

    fn capture_on(
        &mut self,
        ui: &mut Ui<'_, Message>,
        cursor: mouse::Cursor,
        background: Color,
    ) -> RgbaImage {
        assert!(
            self.send(
                ui,
                cursor,
                Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now())),
            )
            .is_empty()
        );
        ui.draw(
            self.renderer,
            &self.theme,
            &Style { text_color: self.theme.palette().text },
            cursor,
        );
        let pixels = self.renderer.screenshot(Size::new(self.width, 760), 1.0, background);
        RgbaImage::from_raw(self.width, 760, pixels).unwrap()
    }

    fn general_reference(
        &mut self,
        app: &impl AppearanceContext<Message>,
        surface: Color,
    ) -> TabPixels {
        let i18n = I18n::new(app.locale());
        let mut ui = self.scene(app.general_view());
        let tabs = self
            .geometry(&mut ui)
            .tabs(&["界面", "终端", "编辑器", "主题"].map(|key| i18n.__(key)));
        let idle = self.capture_on(&mut ui, mouse::Cursor::Unavailable, surface);
        let position = tabs[1].center();
        let cursor = mouse::Cursor::Available(position);
        assert!(
            self.send(&mut ui, cursor, Event::Mouse(mouse::Event::CursorMoved { position }))
                .is_empty()
        );
        let hovered = self.capture_on(&mut ui, cursor, surface);
        assert!(self.send(&mut ui, cursor, press()).is_empty());
        let pressed = self.capture_on(&mut ui, cursor, surface);
        let reference = TabPixels {
            selected: fill(&idle, tabs[0]),
            idle: fill(&idle, tabs[1]),
            edge: edge(&idle, tabs[1], 14),
            hovered: fill(&hovered, tabs[1]),
            pressed: fill(&pressed, tabs[1]),
        };
        assert_ne!(reference.idle, reference.hovered, "general hover must change the fill");
        assert_ne!(reference.hovered, reference.pressed, "general press must change the fill");
        reference
    }
}

fn settings_message<Message: AppearanceMessage>(message: Message) -> Message::Settings {
    message.settings()
}

fn press() -> Event {
    Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
}

// Sample at the vertical midpoint and within the horizontal padding.
fn fill(image: &RgbaImage, label: Rectangle) -> Rgba<u8> {
    *image.get_pixel(label.x as u32 - 9, label.center_y() as u32)
}

fn edge(image: &RgbaImage, label: Rectangle, padding: u32) -> Rgba<u8> {
    *image.get_pixel(label.x as u32 - padding, label.center_y() as u32)
}

fn assert_pixel(actual: Rgba<u8>, expected: Rgba<u8>, label: &str) {
    // Compositing over an already painted surface can round by one channel value.
    assert!(
        actual.0.iter().zip(expected.0).all(|(actual, expected)| actual.abs_diff(expected) <= 1),
        "{label}: expected {expected:?}, got {actual:?}"
    );
}
