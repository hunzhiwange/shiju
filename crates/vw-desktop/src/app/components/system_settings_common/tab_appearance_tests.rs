//! Compare the real settings views with the rendered General tab strip.

use crate::app::components::system_settings_general;
use crate::app::i18n::I18n;
use crate::app::message::settings::SettingsMessage;
use crate::app::state::GeneralSettingsTab;
use crate::app::{App, Message};
use iced::advanced::{
    clipboard,
    renderer::{Headless, Style},
    widget::{Id, Operation, operation::Scrollable},
};
use iced::{Color, Element, Event, Font, Rectangle, Renderer, Size, Theme, Vector, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
use image::{Rgba, RgbaImage};
use vw_config_types::ui::{DesktopLocale, GatewayClientSystemSettingsConfig};

type Ui<'a> = UserInterface<'a, Message, Theme, Renderer>;

fn isolated(test: &str) -> bool {
    const CHILD: &str = "SHIJU_CORE_TABS_TEST_CHILD";
    if std::env::var_os(CHILD).is_some() {
        return true;
    }
    let home = tempfile::tempdir().unwrap();
    let config = GatewayClientSystemSettingsConfig {
        host: "[".into(),
        servers: Vec::new(),
        ..Default::default()
    };
    std::fs::write(
        home.path().join("gateway-client-bootstrap.json"),
        serde_json::to_vec(&config).unwrap(),
    )
    .unwrap();
    let test = test.split_once("::").unwrap().1;
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", test])
        .env(CHILD, "1")
        .env("HOME", home.path())
        .env("VIBEWINDOW_CONFIG_DIR", home.path())
        .env("VIBEWINDOW_WORKSPACE", home.path())
        .current_dir(home.path())
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

#[derive(Default)]
pub(in crate::app::components) struct Labels {
    text: Vec<(String, Rectangle)>,
    scrolls: Vec<(Rectangle, Rectangle, Vector)>,
}

impl Operation for Labels {
    fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) {
        children(self);
    }

    fn text(&mut self, _: Option<&Id>, bounds: Rectangle, label: &str) {
        self.text.push((label.into(), bounds));
    }
    fn scrollable(
        &mut self,
        _: Option<&Id>,
        bounds: Rectangle,
        content: Rectangle,
        translation: Vector,
        _: &mut dyn Scrollable,
    ) {
        self.scrolls.push((bounds, content, translation));
    }
}

impl Labels {
    pub(in crate::app::components) fn contains(&self, label: &str) -> bool {
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

pub(in crate::app::components) struct TabAppearance<'a> {
    renderer: &'a mut Renderer,
    theme: Theme,
    width: u32,
}

impl TabAppearance<'_> {
    pub(in crate::app::components) fn labels(&mut self, element: Element<'_, Message>) -> Labels {
        let mut ui = self.scene(element);
        self.geometry(&mut ui)
    }

    pub(in crate::app::components) fn check(
        &mut self,
        app: &App,
        element: Element<'_, Message>,
        names: &[&str],
        target: usize,
    ) -> SettingsMessage {
        self.check_with_padding(app, element, names, target, 14)
    }

    pub(in crate::app::components) fn check_with_padding(
        &mut self,
        app: &App,
        element: Element<'_, Message>,
        names: &[&str],
        target: usize,
        padding: u32,
    ) -> SettingsMessage {
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

    pub(in crate::app::components) fn click(
        &mut self,
        element: Element<'_, Message>,
        label: &str,
    ) -> SettingsMessage {
        let mut ui = self.scene(element);
        let position = self.geometry(&mut ui).bounds(label).center();
        let cursor = mouse::Cursor::Available(position);
        assert!(self.send(&mut ui, cursor, press()).is_empty());
        self.release(&mut ui, cursor)
    }

    fn release(&mut self, ui: &mut Ui<'_>, cursor: mouse::Cursor) -> SettingsMessage {
        let mut messages =
            self.send(ui, cursor, Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)));
        assert_eq!(messages.len(), 1);
        match messages.pop().unwrap() {
            Message::Settings(message) => message,
            message => panic!("expected settings message, got {message:?}"),
        }
    }

    pub(in crate::app::components) fn check_modal_scroll(
        &mut self,
        element: Element<'_, Message>,
        header: &str,
        footer: &str,
    ) {
        let mut ui = self.scene_with_height(element, 460.0);
        let before = self.geometry(&mut ui);
        let (viewport, _, _) = before
            .scrolls
            .iter()
            .copied()
            .find(|(viewport, content, _)| content.height > viewport.height)
            .expect("long modal body must scroll");
        assert!(before.bounds(header).y < viewport.y);
        assert!(before.bounds(footer).y > viewport.y + viewport.height);
        assert!(before.bounds(footer).y + before.bounds(footer).height <= 460.0);
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
        assert_eq!(before.bounds(footer), after.bounds(footer));
        let _ = self.capture(&mut ui, cursor);
    }

    fn scene<'a>(&mut self, element: Element<'a, Message>) -> Ui<'a> {
        self.scene_with_height(element, 760.0)
    }

    fn scene_with_height<'a>(&mut self, element: Element<'a, Message>, height: f32) -> Ui<'a> {
        UserInterface::build(
            element,
            Size::new(self.width as f32, height),
            Cache::new(),
            self.renderer,
        )
    }

    fn geometry(&self, ui: &mut Ui<'_>) -> Labels {
        let mut labels = Labels::default();
        ui.operate(self.renderer, &mut labels);
        labels
    }

    fn send(&mut self, ui: &mut Ui<'_>, cursor: mouse::Cursor, event: Event) -> Vec<Message> {
        let mut messages = Vec::new();
        ui.update(&[event], cursor, self.renderer, &mut clipboard::Null, &mut messages);
        messages.retain(|message| !matches!(message, Message::None));
        messages
    }

    fn capture(&mut self, ui: &mut Ui<'_>, cursor: mouse::Cursor) -> RgbaImage {
        self.capture_on(ui, cursor, self.theme.palette().background)
    }

    fn capture_on(
        &mut self,
        ui: &mut Ui<'_>,
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

    fn general_reference(&mut self, app: &App, surface: Color) -> TabPixels {
        let i18n = I18n::new(app.desktop_locale);
        let mut ui = self.scene(system_settings_general::view(app));
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

pub(in crate::app::components) async fn run(
    test: &str,
    fixture: impl Fn() -> App,
    check: impl Fn(&mut TabAppearance<'_>, &mut App),
) {
    run_at_least(test, 320, fixture, check).await;
}

pub(in crate::app::components) async fn run_at_least(
    test: &str,
    min_width: u32,
    fixture: impl Fn() -> App,
    check: impl Fn(&mut TabAppearance<'_>, &mut App),
) {
    if !isolated(test) {
        return;
    }
    assert_eq!(crate::app::config::gateway_client_endpoint().normalized_host(), "[");
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    let mut renderer =
        Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
            .await
            .expect("tiny-skia renderer");
    for locale in [
        DesktopLocale::SimplifiedChinese,
        DesktopLocale::TraditionalChinese,
        DesktopLocale::English,
    ] {
        for theme in [Theme::Light, Theme::Dark] {
            let mut app = fixture();
            app.desktop_locale = locale;
            app.app_theme = theme.clone();
            app.general_settings_tab = GeneralSettingsTab::Interface;
            let width = if locale == DesktopLocale::English { min_width } else { 640 };
            let mut scene = TabAppearance { renderer: &mut renderer, theme, width };
            check(&mut scene, &mut app);
        }
    }
}
