//! Render the dialogue and quick message forms and exercise their real controls.
#![cfg(not(target_arch = "wasm32"))]

use super::super::system_settings_agents as agents;
use super::*;
use crate::app::state::AgentConfigTab;
use config_ui::GatewayClientSystemSettingsConfig;
use config_ui::{DesktopLocale, QuickMessageConfig, QuickMessageGroup, QuickMessageKind};
use iced::advanced::{
    clipboard,
    renderer::{Headless, Style},
};
use iced::{Event, Font, Point, Renderer, Size, Theme, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};

fn isolated(test: &str) -> bool {
    const CHILD: &str = "SHIJU_UI_REFRESH_29_CHILD";
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
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", &format!("{}::{test}", module_path!().split_once("::").unwrap().1)])
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

async fn renderer() -> Renderer {
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
        .await
        .unwrap()
}

type Ui<'a> = UserInterface<'a, Message, Theme, Renderer>;

fn scene<'a>(view: Element<'a, Message>, renderer: &mut Renderer, size: Size<u32>) -> Ui<'a> {
    rebuild(view, renderer, size, Cache::new())
}

fn rebuild<'a>(
    view: Element<'a, Message>,
    renderer: &mut Renderer,
    size: Size<u32>,
    cache: Cache,
) -> Ui<'a> {
    UserInterface::build(view, Size::new(size.width as f32, size.height as f32), cache, renderer)
}

fn send(
    ui: &mut Ui<'_>,
    renderer: &mut Renderer,
    cursor: mouse::Cursor,
    events: &[Event],
) -> Vec<Message> {
    let mut messages = Vec::new();
    ui.update(events, cursor, renderer, &mut clipboard::Null, &mut messages);
    messages.retain(|message| !matches!(message, Message::None));
    messages
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
    if let Some(directory) = std::env::var_os("SHIJU_UI_REFRESH_29_SCREENSHOTS") {
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
    labels: Vec<(String, iced::Rectangle)>,
    inputs: Vec<(String, iced::Rectangle)>,
    focusables: Vec<iced::Rectangle>,
    scroll: Option<(iced::Rectangle, iced::Vector)>,
}

impl iced::advanced::widget::Operation for Geometry {
    fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn iced::advanced::widget::Operation)) {
        children(self);
    }

    fn text(
        &mut self,
        _: Option<&iced::advanced::widget::Id>,
        bounds: iced::Rectangle,
        value: &str,
    ) {
        self.labels.push((value.into(), bounds));
    }

    fn text_input(
        &mut self,
        _: Option<&iced::advanced::widget::Id>,
        bounds: iced::Rectangle,
        state: &mut dyn iced::advanced::widget::operation::TextInput,
    ) {
        self.inputs.push((state.text().into(), bounds));
    }
    fn focusable(
        &mut self,
        _: Option<&iced::advanced::widget::Id>,
        bounds: iced::Rectangle,
        _: &mut dyn iced::advanced::widget::operation::Focusable,
    ) {
        self.focusables.push(bounds);
    }

    fn scrollable(
        &mut self,
        _: Option<&iced::advanced::widget::Id>,
        bounds: iced::Rectangle,
        _: iced::Rectangle,
        translation: iced::Vector,
        _: &mut dyn iced::advanced::widget::operation::Scrollable,
    ) {
        self.scroll = Some((bounds, translation));
    }
}

fn geometry(ui: &mut Ui<'_>, renderer: &Renderer) -> Geometry {
    let mut geometry = Geometry::default();
    ui.operate(renderer, &mut geometry);
    geometry
}

fn at(point: Point) -> mouse::Cursor {
    mouse::Cursor::Available(point)
}

const TEMPLATE: &str = "檢查設定 /fixture/abcdefghijklmnopqrstuvwxyz/abcdefghijklmnopqrstuvwxyz/config.toml\r\n保留 `git status` 的結果。";

fn fixture() -> App {
    assert_eq!(crate::app::config::gateway_client_endpoint().normalized_host(), "[");
    App::new().0
}

fn templates() -> Vec<QuickMessageConfig> {
    vec![
        QuickMessageConfig {
            text: TEMPLATE.into(),
            group: QuickMessageGroup::General,
            kind: QuickMessageKind::FillInput,
        },
        QuickMessageConfig {
            text: "Create a new review session".into(),
            group: QuickMessageGroup::Git,
            kind: QuickMessageKind::CreateNewSession,
        },
    ]
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_29_host_quick_page_scrolls() {
    if !isolated("ui_refresh_29_host_quick_page_scrolls") {
        return;
    }
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::English;
    let mut renderer = renderer().await;
    app.agents_settings.active_config_tab = AgentConfigTab::QuickMessages;
    app.quick_messages = (0..4).flat_map(|_| templates()).collect();
    let size = Size::new(640, 420);
    let mut ui = scene(agents::view(&app), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    let (page, before) = positions.scroll.unwrap();
    assert!(
        send(
            &mut ui,
            &mut renderer,
            at(page.center()),
            &[Event::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Lines { x: 0.0, y: -8.0 }
            })]
        )
        .is_empty()
    );
    assert!(geometry(&mut ui, &renderer).scroll.unwrap().1.y > before.y);
    capture(
        &mut ui,
        &mut renderer,
        &Theme::Light,
        size,
        "agents-quick-scrolled-en-640",
        at(page.center()),
    );
    drop(ui);
}
