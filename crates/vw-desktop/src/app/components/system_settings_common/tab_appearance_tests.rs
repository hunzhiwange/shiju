//! Compare the real settings views with the rendered General tab strip.

use crate::app::components::system_settings_general;
use crate::app::i18n::I18n;
use crate::app::message::settings::SettingsMessage;
use crate::app::state::GeneralSettingsTab;
use crate::app::{App, Message};
use iced::advanced::{
    clipboard,
    renderer::{Headless, Style},
    widget::{Id, Operation},
};
use iced::{Element, Event, Font, Rectangle, Renderer, Size, Theme, mouse};
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
pub(in crate::app::components) struct Labels(Vec<(String, Rectangle)>);

impl Operation for Labels {
    fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) {
        children(self);
    }

    fn text(&mut self, _: Option<&Id>, bounds: Rectangle, label: &str) {
        self.0.push((label.into(), bounds));
    }
}

impl Labels {
    pub(in crate::app::components) fn contains(&self, label: &str) -> bool {
        self.0.iter().any(|(text, _)| text == label)
    }

    fn tabs(&self, names: &[&str]) -> Vec<Rectangle> {
        self.0
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
        let reference = self.general_reference(app);
        let mut ui = self.scene(element);
        let tabs = self.geometry(&mut ui).tabs(names);
        let selected = tabs[0];
        for (label, tab) in names.iter().zip(&tabs) {
            assert!(
                tab.x >= 14.0
                    && tab.x + tab.width + 14.0 <= self.width as f32 + 0.5
                    && tab.height < 30.0,
                "{label}: tab is clipped or wraps inside its button: {tab:?}"
            );
        }

        let idle = self.capture(&mut ui, mouse::Cursor::Unavailable);
        assert_eq!(fill(&idle, selected), reference.selected, "selected {}", names[0]);
        for (&label, &tab) in names.iter().zip(&tabs).skip(1) {
            assert_eq!(fill(&idle, tab), reference.idle, "idle {label}");
            assert_eq!(edge(&idle, tab), reference.edge, "outline {label}");
        }

        let position = tabs[target].center();
        let cursor = mouse::Cursor::Available(position);
        assert!(
            self.send(&mut ui, cursor, Event::Mouse(mouse::Event::CursorMoved { position }))
                .is_empty()
        );
        let hovered = self.capture(&mut ui, cursor);
        assert_eq!(fill(&hovered, tabs[target]), reference.hovered, "hovered");
        assert!(self.send(&mut ui, cursor, press()).is_empty());
        let pressed = self.capture(&mut ui, cursor);
        assert_eq!(fill(&pressed, tabs[target]), reference.pressed, "pressed");
        let mut messages = self.send(
            &mut ui,
            cursor,
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        );
        assert_eq!(messages.len(), 1);
        match messages.pop().unwrap() {
            Message::Settings(message) => message,
            message => panic!("expected settings message, got {message:?}"),
        }
    }

    fn scene<'a>(&mut self, element: Element<'a, Message>) -> Ui<'a> {
        UserInterface::build(
            element,
            Size::new(self.width as f32, 760.0),
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
        assert!(
            self.send(
                ui,
                cursor,
                Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now())),
            )
            .is_empty()
        );
        let background = self.theme.palette().background;
        ui.draw(
            self.renderer,
            &self.theme,
            &Style { text_color: self.theme.palette().text },
            cursor,
        );
        let pixels = self.renderer.screenshot(Size::new(self.width, 760), 1.0, background);
        RgbaImage::from_raw(self.width, 760, pixels).unwrap()
    }

    fn general_reference(&mut self, app: &App) -> TabPixels {
        let i18n = I18n::new(app.desktop_locale);
        let mut ui = self.scene(system_settings_general::view(app));
        let tabs = self
            .geometry(&mut ui)
            .tabs(&["界面", "终端", "编辑器", "主题"].map(|key| i18n.__(key)));
        let idle = self.capture(&mut ui, mouse::Cursor::Unavailable);
        let position = tabs[1].center();
        let cursor = mouse::Cursor::Available(position);
        assert!(
            self.send(&mut ui, cursor, Event::Mouse(mouse::Event::CursorMoved { position }))
                .is_empty()
        );
        let hovered = self.capture(&mut ui, cursor);
        assert!(self.send(&mut ui, cursor, press()).is_empty());
        let pressed = self.capture(&mut ui, cursor);
        let reference = TabPixels {
            selected: fill(&idle, tabs[0]),
            idle: fill(&idle, tabs[1]),
            edge: edge(&idle, tabs[1]),
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

fn edge(image: &RgbaImage, label: Rectangle) -> Rgba<u8> {
    *image.get_pixel(label.x as u32 - 14, label.center_y() as u32)
}

pub(in crate::app::components) async fn run(
    test: &str,
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
            let width = if locale == DesktopLocale::English { 320 } else { 640 };
            let mut scene = TabAppearance { renderer: &mut renderer, theme, width };
            check(&mut scene, &mut app);
        }
    }
}
