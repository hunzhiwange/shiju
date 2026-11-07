//! Exercise the real network settings widgets with isolated state and tiny-skia.
#![cfg(not(target_arch = "wasm32"))]

use super::super::{system_settings::SystemTab, system_settings_network as network};
use super::{App, HttpRequestMessage as Http, Message, SettingsMessage};
use crate::app::i18n::{settings_extensions::text as tr, settings_models::connection_text};
use crate::app::message::settings::WebSearchMessage as Search;
use iced::keyboard::key::Named;
use iced::widget::{container, scrollable};
use iced::{Element, Length, Point, Rectangle};

use iced::advanced::{
    clipboard,
    renderer::{Headless, Style},
};
use iced::{Event, Font, Renderer, Size, Theme, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
use vw_config_types::ui::{DesktopLocale, GatewayClientSystemSettingsConfig};

fn isolated(test: &str) -> bool {
    const CHILD: &str = "SHIJU_UI_REFRESH_25_CHILD";
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
    if let Some(directory) = std::env::var_os("SHIJU_UI_REFRESH_25_SCREENSHOTS") {
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

fn key(name: iced::keyboard::key::Named) -> Event {
    use iced::keyboard::{self, Key};
    let key = Key::Named(name);
    Event::Keyboard(keyboard::Event::KeyPressed {
        modified_key: key.clone(),
        key,
        physical_key: keyboard::key::Physical::Unidentified(
            keyboard::key::NativeCode::Unidentified,
        ),
        location: keyboard::Location::Standard,
        modifiers: keyboard::Modifiers::empty(),
        text: None,
        repeat: false,
    })
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

impl Geometry {
    fn label(&self, value: &str) -> iced::Rectangle {
        self.labels
            .iter()
            .find(|(label, _)| label == value)
            .unwrap_or_else(|| panic!("missing label {value}: {:?}", self.labels))
            .1
    }

    fn input(&self, value: &str) -> iced::Rectangle {
        self.inputs
            .iter()
            .find(|(input, _)| input == value)
            .unwrap_or_else(|| panic!("missing fixture input {value}"))
            .1
    }
}

fn geometry(ui: &mut Ui<'_>, renderer: &Renderer) -> Geometry {
    let mut geometry = Geometry::default();
    ui.operate(renderer, &mut geometry);
    geometry
}

fn click(ui: &mut Ui<'_>, renderer: &mut Renderer, point: iced::Point) -> Vec<Message> {
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

const AGENT: &str = "Fixture-Agent/2";
const DOMAIN: &str = "*.api.example.invalid";
const LONG_DOMAIN: &str = "service.service.service.service.service.service.service.service.service.service.service.service.example.invalid";
const API_KEY: &str = "fixture-api-key";

fn fixture() -> App {
    assert_eq!(crate::app::config::gateway_client_endpoint().normalized_host(), "[");
    let mut app = App::new().0;
    let http = &mut app.http_request_settings;
    http.enabled = false;
    http.timeout_secs = 37;
    http.max_response_size = 524_288;
    http.user_agent = AGENT.into();
    http.new_allowed_domain_input = DOMAIN.into();
    http.allowed_domains = vec!["api.example.invalid".into(), LONG_DOMAIN.into()];
    let search = &mut app.web_search_settings;
    search.enabled = false;
    search.provider = "brave".into();
    search.api_key_input = API_KEY.into();
    search.brave_api_key_input = "fixture-brave-key".into();
    search.api_url_input = format!("https://api.example.invalid/{}search", "long-path/".repeat(14));
    search.max_results_input = "8".into();
    search.timeout_secs_input = "43".into();
    search.user_agent = AGENT.into();
    app
}

fn at(point: Point) -> mouse::Cursor {
    mouse::Cursor::Available(point)
}

fn key_at(ui: &mut Ui<'_>, renderer: &mut Renderer, name: Named) -> Vec<Message> {
    send(ui, renderer, mouse::Cursor::Unavailable, &[key(name)])
}

fn shorten(ui: &mut Ui<'_>, renderer: &mut Renderer, bounds: Rectangle) -> Vec<Message> {
    assert!(click(ui, renderer, bounds.center()).is_empty());
    assert!(key_at(ui, renderer, Named::End).is_empty());
    key_at(ui, renderer, Named::Backspace)
}

fn assert_fits(bounds: Rectangle, size: Size<u32>) {
    assert!(bounds.width > 0.0 && bounds.height > 0.0, "{bounds:?}");
    assert!(bounds.x >= 0.0 && bounds.y >= 0.0, "{bounds:?}");
    assert!(bounds.x + bounds.width <= size.width as f32 + 0.1, "{bounds:?}");
    assert!(bounds.y + bounds.height <= size.height as f32 + 0.1, "{bounds:?}");
}

fn page(app: &App) -> Element<'_, Message> {
    use scrollable::{Direction, Scrollbar};

    scrollable(container(network::view(app)).padding([0, 7]).width(Length::Fill))
        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
        .height(Length::Fill)
        .into()
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_25_layout_and_interaction() {
    if !isolated("ui_refresh_25_layout_and_interaction") {
        return;
    }

    let mut renderer = renderer().await;
    let mut app = fixture();
    for (locale, width, tag) in [
        (DesktopLocale::English, 640, "en-640"),
        (DesktopLocale::SimplifiedChinese, 960, "zh-cn-960"),
        (DesktopLocale::TraditionalChinese, 640, "zh-tw-640"),
    ] {
        app.desktop_locale = locale;
        for (tab, name) in [(SystemTab::WebSearch, "search"), (SystemTab::HttpRequest, "http")] {
            app.system_settings_tab = tab;
            let size = Size::new(width, 1450);
            let mut ui = scene(page(&app), &mut renderer, size);
            let positions = geometry(&mut ui, &renderer);
            let input = positions.input(AGENT);
            assert_fits(input, size);
            let label = positions.label("User-Agent");
            let hint = positions.label(tr(
                locale,
                if tab == SystemTab::HttpRequest {
                    "请求默认携带的 User-Agent。"
                } else {
                    "搜索请求携带的 User-Agent。"
                },
            ));
            let description_center = (label.y + hint.y + hint.height) / 2.0;
            assert!(
                (input.center_y() - description_center).abs() < 0.5,
                "{input:?} {label:?} {hint:?}"
            );
            for (theme, mode) in [(&Theme::Light, "light"), (&Theme::Dark, "dark")] {
                capture(
                    &mut ui,
                    &mut renderer,
                    theme,
                    size,
                    &format!("{name}-{tag}-{mode}"),
                    mouse::Cursor::Unavailable,
                );
            }

            for (target, label) in [
                (SystemTab::WebSearch, "网页搜索"),
                (SystemTab::HttpRequest, "网络请求"),
                (SystemTab::Browser, "浏览器"),
                (SystemTab::Proxy, "代理"),
                (SystemTab::Tunnel, "隧道"),
            ] {
                let bounds = positions.label(connection_text::text(locale, label));
                assert_fits(bounds, size);
                assert!(matches!(
                    click(&mut ui, &mut renderer, bounds.center()).as_slice(),
                    [Message::Settings(SettingsMessage::SystemTabSelected(actual))]
                        if *actual == target
                ));
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_25_preserves_values_and_disabled_states() {
    if !isolated("ui_refresh_25_preserves_values_and_disabled_states") {
        return;
    }

    let mut renderer = renderer().await;
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::English;
    let size = Size::new(640, 1450);
    app.system_settings_tab = SystemTab::HttpRequest;
    let mut ui = scene(page(&app), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    let long_domain = positions.label(LONG_DOMAIN);
    assert_fits(long_domain, size);
    assert!(long_domain.height > positions.label("api.example.invalid").height);
    assert!(matches!(
        click(&mut ui, &mut renderer, positions.label(tr(app.desktop_locale, "添加")).center())
            .as_slice(),
        [Message::Settings(SettingsMessage::HttpRequest(Http::AddAllowedDomain))]
    ));
    let delete = positions
        .labels
        .iter()
        .filter(|(label, _)| label == tr(app.desktop_locale, "删除"))
        .nth(1)
        .unwrap()
        .1;
    assert!(matches!(
        click(&mut ui, &mut renderer, delete.center()).as_slice(),
        [Message::Settings(SettingsMessage::HttpRequest(Http::RemoveAllowedDomain(1)))]
    ));
    assert!(matches!(
        shorten(&mut ui, &mut renderer, positions.input(DOMAIN)).as_slice(),
        [Message::Settings(SettingsMessage::HttpRequest(Http::NewAllowedDomainChanged(value)))]
            if value == "*.api.example.invali"
    ));
    assert!(matches!(
        shorten(&mut ui, &mut renderer, positions.input("37")).as_slice(),
        [Message::Settings(SettingsMessage::HttpRequest(Http::TimeoutSecsChanged(3)))]
    ));
    assert!(matches!(
        shorten(&mut ui, &mut renderer, positions.input(AGENT)).as_slice(),
        [Message::Settings(SettingsMessage::HttpRequest(Http::UserAgentChanged(value)))]
            if value == "Fixture-Agent/"
    ));
    drop(ui);

    app.http_request_settings.allowed_domains.clear();
    app.http_request_settings.save_error = Some("Fixture: save failed".into());
    let mut ui = scene(page(&app), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    assert_fits(positions.label("Fixture: save failed"), size);
    assert!(!positions.labels.iter().any(|(label, _)| label == "Delete"));
    capture(
        &mut ui,
        &mut renderer,
        &Theme::Dark,
        size,
        "http-empty-error",
        mouse::Cursor::Unavailable,
    );
    drop(ui);

    app.system_settings_tab = SystemTab::WebSearch;
    let mut ui = scene(page(&app), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    for value in [API_KEY, "fixture-brave-key", "8", "43", AGENT] {
        assert_fits(positions.input(value), size);
    }
    assert!(matches!(
        shorten(&mut ui, &mut renderer, positions.input("43")).as_slice(),
        [Message::Settings(SettingsMessage::WebSearch(Search::TimeoutSecsChanged(value)))]
            if value == "4"
    ));
    assert!(key_at(&mut ui, &mut renderer, Named::Enter).is_empty());
    drop(ui);

    app.web_search_settings.provider = "duckduckgo".into();
    let narrow = Size::new(420, 420);
    let mut ui = scene(page(&app), &mut renderer, narrow);
    let positions = geometry(&mut ui, &renderer);
    assert!(!positions.inputs.iter().any(|(value, _)| value == API_KEY));
    let viewport = positions.scroll.expect("network page scroll").0;
    assert!(
        send(
            &mut ui,
            &mut renderer,
            at(viewport.center()),
            &[Event::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -250.0 },
            })],
        )
        .is_empty()
    );
    assert!(geometry(&mut ui, &renderer).scroll.unwrap().1.y > 0.0);
    capture(
        &mut ui,
        &mut renderer,
        &Theme::Light,
        narrow,
        "search-duckduckgo-420-scrolled",
        mouse::Cursor::Unavailable,
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_25_provider_keyboard_and_cancel() {
    if !isolated("ui_refresh_25_provider_keyboard_and_cancel") {
        return;
    }

    let mut renderer = renderer().await;
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::English;
    app.system_settings_tab = SystemTab::WebSearch;
    let size = Size::new(420, 520);
    let mut ui = scene(page(&app), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    let label = positions.label("Search backend");
    let hint = positions.label(tr(app.desktop_locale, "选择网页搜索使用的后端。"));
    let trigger = positions
        .focusables
        .iter()
        .copied()
        .find(|bounds| bounds.x > label.x && (bounds.center_y() - label.center_y()).abs() < 34.0)
        .expect("provider select must be keyboard focusable");
    assert_fits(trigger, size);
    assert!((trigger.center_y() - (label.y + hint.y + hint.height) / 2.0).abs() < 0.5);
    assert!(click(&mut ui, &mut renderer, trigger.center()).is_empty());
    assert!(key_at(&mut ui, &mut renderer, Named::Escape).is_empty());
    assert!(key_at(&mut ui, &mut renderer, Named::Enter).is_empty());
    assert!(key_at(&mut ui, &mut renderer, Named::ArrowDown).is_empty());
    capture(
        &mut ui,
        &mut renderer,
        &Theme::Dark,
        size,
        "search-select-keyboard-420-dark",
        mouse::Cursor::Unavailable,
    );
    assert!(matches!(
        key_at(&mut ui, &mut renderer, Named::Enter).as_slice(),
        [Message::Settings(SettingsMessage::WebSearch(Search::ProviderChanged(id)))]
            if id == "serper"
    ));

    assert!(click(&mut ui, &mut renderer, trigger.center()).is_empty());
    let menu = geometry(&mut ui, &renderer).scroll.expect("provider menu scroll").0;
    assert_fits(menu, size);
    let first = Point::new(menu.center_x(), menu.y + 18.0);
    let second = Point::new(menu.center_x(), menu.y + 18.0 + 38.0);
    let mut images = Vec::new();
    for (point, name) in [
        (first, "search-select-first-hover"),
        (second, "search-select-second-hover"),
        (Point::ORIGIN, "search-select-no-hover"),
    ] {
        assert!(
            send(
                &mut ui,
                &mut renderer,
                at(point),
                &[Event::Mouse(mouse::Event::CursorMoved { position: point })],
            )
            .is_empty()
        );
        images.push(capture(&mut ui, &mut renderer, &Theme::Light, size, name, at(point)));
    }
    assert!(images[0] != images[1] && images[0] != images[2] && images[1] != images[2]);
    assert!(matches!(
        click(&mut ui, &mut renderer, first).as_slice(),
        [Message::Settings(SettingsMessage::WebSearch(Search::ProviderChanged(id)))]
            if id == "duckduckgo"
    ));
    assert!(click(&mut ui, &mut renderer, trigger.center()).is_empty());
    let underneath =
        positions.label(connection_text::text(app.desktop_locale, "网络请求")).center();
    assert!(
        click(&mut ui, &mut renderer, underneath).is_empty(),
        "closing the overlay must not select the tab underneath"
    );
}
