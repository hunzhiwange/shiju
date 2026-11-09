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

fn masked(value: &str) -> String {
    "•".repeat(value.len())
}

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
    scroll_frame(network::view(app))
}

fn scroll_frame(view: Element<'_, Message>) -> Element<'_, Message> {
    use scrollable::{Direction, Scrollbar};

    scrollable(container(view).padding([0, 7]).width(Length::Fill))
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
            assert!((input.center_y() - label.center_y()).abs() < 0.5, "{input:?} {label:?}");
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
    for value in [&masked(API_KEY), &masked("fixture-brave-key"), "8", "43", AGENT] {
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
    assert!(!positions.inputs.iter().any(|(value, _)| *value == masked(API_KEY)));
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
    let trigger = positions
        .focusables
        .iter()
        .copied()
        .find(|bounds| bounds.x > label.x && (bounds.center_y() - label.center_y()).abs() < 0.5)
        .expect("provider select must be keyboard focusable");
    assert_fits(trigger, size);
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

const HELP_LAYOUTS: [(DesktopLocale, Theme); 3] = [
    (DesktopLocale::SimplifiedChinese, Theme::Light),
    (DesktopLocale::English, Theme::Dark),
    (DesktopLocale::TraditionalChinese, Theme::Dark),
];

fn help_icon_candidates(
    ui: &mut Ui<'_>,
    renderer: &Renderer,
    title: &str,
    description: &str,
) -> Vec<Point> {
    let positions = geometry(ui, renderer);
    assert!(
        positions.labels.iter().all(|(label, _)| label != description),
        "{title}: static explanation is still shown inline"
    );
    let bounds = positions.labels.iter().rfind(|(label, _)| label == title).unwrap().1;
    let (viewport, translation) = positions.scroll.expect("network settings scroll");
    // Long translated labels can wrap the question mark onto the next line.
    [
        Point::new(bounds.x + bounds.width + 15.0, bounds.center_y()),
        Point::new(bounds.x + 11.0, bounds.y + bounds.height + 15.0),
    ]
    .into_iter()
    .map(move |point| point - translation)
    .filter(|point| viewport.contains(*point))
    .collect()
}

fn assert_help_frame(
    app: &App,
    ui: &mut Ui<'_>,
    renderer: &mut Renderer,
    size: Size<u32>,
    entry: &str,
    (title, description): (&str, &str),
) -> Point {
    let icons = help_icon_candidates(ui, renderer, title, description);
    let prefix = format!("rollout-16-{entry}-{:?}-{title}", app.desktop_locale);
    let theme = &app.app_theme;
    let away = mouse::Cursor::Unavailable;
    let idle = capture(ui, renderer, theme, size, &format!("{prefix}-idle"), away);
    let icon = icons
        .into_iter()
        .find(|&icon| {
            assert!(
                send(
                    ui,
                    renderer,
                    at(icon),
                    &[Event::Mouse(mouse::Event::CursorMoved { position: icon })]
                )
                .is_empty()
            );
            let hover = capture(ui, renderer, theme, size, &format!("{prefix}-hover"), at(icon));
            idle != hover
        })
        .unwrap_or_else(|| panic!("{title}: hovering must render help"));
    assert!(click(ui, renderer, icon).is_empty());
    let open = capture(ui, renderer, theme, size, &format!("{prefix}-open"), away);
    assert!(idle != open, "{title}: pinned help must render with the pointer away");
    assert!(key_at(ui, renderer, Named::Escape).is_empty());
    let closed = capture(ui, renderer, theme, size, &format!("{prefix}-closed"), away);
    assert!(idle == closed, "{title}: Escape must restore the initial frame");
    icon
}

fn assert_help_actions(ui: &mut Ui<'_>, renderer: &mut Renderer, icon: Point, description: &str) {
    assert!(click(ui, renderer, icon).is_empty());
    let body = Point::new(icon.x + 29.0, icon.y + 5.0);
    let line_height = iced::advanced::text::LineHeight::default().to_absolute(12.0.into());
    let item_height = f32::from(line_height) + 10.0;
    let search = format!("https://www.google.com/search?q={}", urlencoding::encode(description));
    for index in [0, 3] {
        assert!(
            send(
                ui,
                renderer,
                at(body),
                &[Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right))],
            )
            .is_empty()
        );
        let item = Point::new(body.x + 24.0, body.y + 3.0 + item_height * (index as f32 + 0.5));
        let messages = click(ui, renderer, item);
        assert!(
            match (index, messages.as_slice()) {
                (0, [Message::CopyCode(value)]) => value == description,
                (3, [Message::View(crate::app::message::ViewMessage::OpenUrlExternal(url))]) => {
                    url == &search
                }
                _ => false,
            },
            "unexpected contextual help action: {messages:?}"
        );
    }
    assert!(key_at(ui, renderer, Named::Escape).is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn help_rollout_16_renders_contextual_help() {
    if !isolated("help_rollout_16_renders_contextual_help") {
        return;
    }

    let mut renderer = renderer().await;
    let mut app = fixture();
    let size = Size::new(480, 1400);
    for (locale, theme) in HELP_LAYOUTS {
        app.desktop_locale = locale;
        app.app_theme = theme;
        app.system_settings_tab = SystemTab::WebSearch;
        app.web_search_settings.provider = "brave".into();
        let t = |key| tr(locale, key);
        let heading = (
            connection_text::text(locale, "网络配置"),
            connection_text::text(
                locale,
                "通过标签切换网页搜索、网络请求、浏览器、代理与隧道配置。",
            ),
        );
        let mut ui = scene(page(&app), &mut renderer, size);
        let heading_icon =
            assert_help_frame(&app, &mut ui, &mut renderer, size, "network", heading);
        for (title, description) in [
            ("基础行为", "控制开关与默认搜索后端。"),
            ("Brave Search", "配置专用密钥、备用密钥和兼容接口地址。"),
            ("Brave 密钥", "Brave 优先使用 `brave_api_key`；留空时会回退到 `api_key`。"),
            ("搜索后端", "选择网页搜索使用的后端。"),
            ("结果数量", "仅接受正整数；最终会限制在 1-10 之间。"),
        ] {
            assert_help_frame(
                &app,
                &mut ui,
                &mut renderer,
                size,
                "search",
                (t(title), t(description)),
            );
        }
        let positions = geometry(&mut ui, &renderer);
        positions.input("8");
        assert!(matches!(
            shorten(&mut ui, &mut renderer, positions.input(&masked(API_KEY))).as_slice(),
            [Message::Settings(SettingsMessage::WebSearch(Search::ApiKeyChanged(value)))]
                if value == "fixture-api-ke"
        ));
        let switch = positions.label(connection_text::text(locale, "网络请求")).center();

        // An application-driven switch (such as settings search) has no outside click.
        assert!(click(&mut ui, &mut renderer, heading_icon).is_empty());
        let cache = ui.into_cache();
        drop(crate::app::message::settings::update(
            &mut app,
            SettingsMessage::SystemTabSelected(SystemTab::HttpRequest),
        ));
        assert_eq!(app.system_settings_tab, SystemTab::HttpRequest);
        let mut switched = UserInterface::build(
            page(&app),
            Size::new(size.width as f32, size.height as f32),
            cache,
            &mut renderer,
        );
        let mut fresh = scene(page(&app), &mut renderer, size);
        let away = mouse::Cursor::Unavailable;
        assert!(
            capture(
                &mut switched,
                &mut renderer,
                &app.app_theme,
                size,
                "rollout-16-switched",
                away
            ) == capture(&mut fresh, &mut renderer, &app.app_theme, size, "rollout-16-fresh", away),
            "programmatic tab switch must reset the pinned network help"
        );
        assert!(matches!(
            click(&mut switched, &mut renderer, switch).as_slice(),
            [Message::Settings(SettingsMessage::SystemTabSelected(SystemTab::HttpRequest))]
        ));
        drop((switched, fresh));

        app.system_settings_tab = SystemTab::WebSearch;
        app.web_search_settings.provider = "duckduckgo".into();
        let mut ui = scene(page(&app), &mut renderer, size);
        assert_help_frame(
            &app,
            &mut ui,
            &mut renderer,
            size,
            "duckduckgo",
            ("DuckDuckGo", t("公开搜索页面模式，无需额外鉴权配置。")),
        );
        geometry(&mut ui, &renderer)
            .label(t("当前后端会直接使用 DuckDuckGo 公开搜索页面，无需 API Key。"));

        drop(ui);
        app.web_search_settings.provider = "google".into();
        let mut ui = scene(page(&app), &mut renderer, size);
        assert_help_frame(
            &app,
            &mut ui,
            &mut renderer,
            size,
            "search",
            (t("后端连接"), t("配置当前搜索后端的鉴权信息和兼容接口地址。")),
        );
        let mut ui = scene(
            scroll_frame(super::super::system_settings_web_search::view(&app)),
            &mut renderer,
            size,
        );
        assert_help_frame(
            &app,
            &mut ui,
            &mut renderer,
            size,
            "search-standalone",
            (t("网页搜索配置"), t("配置网页搜索工具的提供方、鉴权信息和请求参数。")),
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn help_rollout_16_http_preserves_permissions_and_errors() {
    if !isolated("help_rollout_16_http_preserves_permissions_and_errors") {
        return;
    }

    let mut renderer = renderer().await;
    let mut app = fixture();
    app.system_settings_tab = SystemTab::HttpRequest;
    app.http_request_settings.allowed_domains.clear();
    app.http_request_settings.save_error = Some("fixture request error".into());
    let size = Size::new(480, 1000);
    for (locale, theme) in HELP_LAYOUTS {
        app.desktop_locale = locale;
        app.app_theme = theme;
        let t = |key| tr(locale, key);
        let mut ui = scene(page(&app), &mut renderer, size);
        let positions = geometry(&mut ui, &renderer);
        for label in [
            "fixture request error",
            t("当前未配置允许域名；启用工具后仍会因为白名单为空而拒绝所有请求。"),
            t("默认的 `*` 允许所有公网域名。"),
            t("`0` 表示不限制响应体大小。"),
        ] {
            assert_fits(positions.label(label), size);
        }
        assert_fits(positions.input("37"), size);
        for (title, description) in [
            (
                "基础行为",
                "控制 http_request 工具的启用状态、超时、最大响应体大小和默认 User-Agent。",
            ),
            ("超时时间", "`0` 会在运行时回退到安全默认值 30 秒。"),
            ("新增域名", "支持精确域名、子域模式或 *。例如：example.com 或 *.example.org。"),
            ("User-Agent", "请求默认携带的 User-Agent。"),
        ] {
            let translated = (t(title), t(description));
            let icon = assert_help_frame(&app, &mut ui, &mut renderer, size, "http", translated);
            if title == "User-Agent" && locale == DesktopLocale::English {
                assert_help_actions(&mut ui, &mut renderer, icon, translated.1);
            }
        }
        assert!(matches!(
            click(&mut ui, &mut renderer, positions.label(t("添加")).center()).as_slice(),
            [Message::Settings(SettingsMessage::HttpRequest(Http::AddAllowedDomain))]
        ));
        let mut ui = scene(scroll_frame(super::view(&app)), &mut renderer, size);
        assert_help_frame(
            &app,
            &mut ui,
            &mut renderer,
            size,
            "http-standalone",
            (t("网络请求配置"), t("配置网络请求工具的开关、超时、响应大小限制和域名白名单。")),
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn help_rollout_16_browser_preserves_native_and_sidecar_controls() {
    if !isolated("help_rollout_16_browser_preserves_native_and_sidecar_controls") {
        return;
    }

    let mut renderer = renderer().await;
    let mut app = fixture();
    app.system_settings_tab = SystemTab::Browser;
    let browser = &mut app.browser_settings;
    browser.backend = "native".into();
    browser.native_chrome_path_input = "/fixture/浏览器".into();
    browser.computer_use_api_key_input = API_KEY.into();
    let size = Size::new(480, 1900);
    for (locale, theme) in HELP_LAYOUTS {
        app.desktop_locale = locale;
        app.app_theme = theme;
        let i18n = crate::app::i18n::I18n::new(locale);
        let mut ui = scene(page(&app), &mut renderer, size);
        let positions = geometry(&mut ui, &renderer);
        assert_fits(positions.input("/fixture/浏览器"), size);
        assert_fits(positions.input(&masked(API_KEY)), size);
        for label in [
            "默认的 `*` 允许所有公网域名。列表为空时拒绝所有站点。",
            "当前运行时完整支持 default；其余选项会先写入配置。",
            "仅在明确可信时允许 sidecar 部署在公网。",
        ] {
            assert_fits(positions.label(i18n.__(label)), size);
        }
        for (title, description) in [
            ("基础行为", "域名范围、窗口行为与运行后端。"),
            (
                "后端实现",
                "默认使用代理浏览器；原生浏览器需要可用 WebDriver；系统控制需要 sidecar。",
            ),
            ("Rust Native 后端", "当后端为 native 时，使用 WebDriver 驱动浏览器自动化。"),
            ("WebDriver 地址", "连接本地或远程 WebDriver 服务。"),
            ("系统级控制", "配置本地 sidecar 端点，用于鼠标、键盘、截图等系统级浏览器操作。"),
            ("访问密钥", "为 sidecar 请求附带可选的 Bearer Token。"),
        ] {
            assert_help_frame(
                &app,
                &mut ui,
                &mut renderer,
                size,
                "browser",
                (i18n.__(title), i18n.__(description)),
            );
        }
        assert!(matches!(
            shorten(&mut ui, &mut renderer, positions.input(&masked(API_KEY))).as_slice(),
            [Message::Settings(SettingsMessage::Browser(
                crate::app::message::settings::BrowserMessage::ComputerUseApiKeyChanged(value)
            ))] if value == "fixture-api-ke"
        ));
        let mut ui = scene(
            scroll_frame(super::super::system_settings_browser::view(&app)),
            &mut renderer,
            size,
        );
        assert_help_frame(
            &app,
            &mut ui,
            &mut renderer,
            size,
            "browser-standalone",
            (i18n.__("浏览器配置"), i18n.__("配置允许站点、打开方式与浏览器自动化后端。")),
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn settings_help_gap_08_brave_credentials_and_result_range() {
    if !isolated("settings_help_gap_08_brave_credentials_and_result_range") {
        return;
    }

    let mut renderer = renderer().await;
    let mut app = fixture();
    app.system_settings_tab = SystemTab::WebSearch;
    for ((locale, theme), width) in HELP_LAYOUTS.into_iter().zip([420, 720, 420]) {
        app.desktop_locale = locale;
        app.app_theme = theme;
        let t = |key| tr(locale, key);
        let size = Size::new(width, 1400);
        let mut ui = scene(page(&app), &mut renderer, size);
        for (title, body) in [
            ("Brave 密钥", "Brave 优先使用 `brave_api_key`；留空时会回退到 `api_key`。"),
            ("接口地址", "留空时使用默认接口；适合代理层或自托管兼容端点。"),
            ("结果数量", "仅接受正整数；最终会限制在 1-10 之间。"),
        ] {
            let icon = assert_help_frame(
                &app,
                &mut ui,
                &mut renderer,
                size,
                "gap-08-brave",
                (t(title), t(body)),
            );
            if locale == DesktopLocale::English && title == "Brave 密钥" {
                assert_help_actions(&mut ui, &mut renderer, icon, t(body));
            }
        }
        let positions = geometry(&mut ui, &renderer);
        let brave_key = "fixture-brave-key";
        assert!(!positions.inputs.iter().any(|(value, _)| value == brave_key));
        assert!(matches!(
            shorten(&mut ui, &mut renderer, positions.input(&masked(brave_key))).as_slice(),
            [Message::Settings(SettingsMessage::WebSearch(Search::BraveApiKeyChanged(value)))]
                if value == "fixture-brave-ke"
        ));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn settings_help_gap_08_http_limits_and_permissions() {
    if !isolated("settings_help_gap_08_http_limits_and_permissions") {
        return;
    }

    let mut renderer = renderer().await;
    let mut app = fixture();
    app.system_settings_tab = SystemTab::HttpRequest;
    app.http_request_settings.timeout_secs = 0;
    app.http_request_settings.max_response_size = 0;
    app.http_request_settings.allowed_domains.clear();
    for ((locale, theme), width) in HELP_LAYOUTS.into_iter().zip([420, 720, 420]) {
        app.desktop_locale = locale;
        app.app_theme = theme;
        let t = |key| tr(locale, key);
        let size = Size::new(width, 1000);
        let mut ui = scene(page(&app), &mut renderer, size);
        for (title, body) in [
            ("超时时间", "`0` 会在运行时回退到安全默认值 30 秒。"),
            ("响应大小上限", "较大的响应会增加内存占用与模型上下文成本。"),
            ("新增域名", "支持精确域名、子域模式或 *。例如：example.com 或 *.example.org。"),
        ] {
            assert_help_frame(
                &app,
                &mut ui,
                &mut renderer,
                size,
                "gap-08-http",
                (t(title), t(body)),
            );
        }
        let positions = geometry(&mut ui, &renderer);
        for label in [
            "`0` 表示不限制响应体大小。",
            "默认的 `*` 允许所有公网域名。",
            "当前未配置允许域名；启用工具后仍会因为白名单为空而拒绝所有请求。",
        ] {
            assert_fits(positions.label(t(label)), size);
        }
        assert_eq!(positions.inputs.iter().filter(|(value, _)| value == "0").count(), 2);
        assert!(matches!(
            click(&mut ui, &mut renderer, positions.label(t("添加")).center()).as_slice(),
            [Message::Settings(SettingsMessage::HttpRequest(Http::AddAllowedDomain))]
        ));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn settings_help_gap_08_browser_backend_and_domain_help() {
    if !isolated("settings_help_gap_08_browser_backend_and_domain_help") {
        return;
    }

    let mut renderer = renderer().await;
    let mut app = fixture();
    app.system_settings_tab = SystemTab::Browser;
    for ((locale, theme), width) in HELP_LAYOUTS.into_iter().zip([420, 720, 420]) {
        app.desktop_locale = locale;
        app.app_theme = theme;
        let i18n = crate::app::i18n::I18n::new(locale);
        for (backend, dependency) in [
            ("native", "原生浏览器需要可用的 WebDriver。"),
            ("computer_use", "系统控制需要 sidecar。"),
            ("agent_browser", ""),
        ] {
            app.browser_settings.backend = backend.into();
            let size = Size::new(width, 1900);
            let mut ui = scene(page(&app), &mut renderer, size);
            for (title, body) in [
                (
                    "后端实现",
                    "默认使用代理浏览器；原生浏览器需要可用 WebDriver；系统控制需要 sidecar。",
                ),
                ("允许域名", "每行一个域名，或使用逗号分隔，例: example.com\n*.example.org"),
            ] {
                assert_help_frame(
                    &app,
                    &mut ui,
                    &mut renderer,
                    size,
                    "gap-08-browser",
                    (i18n.__(title), i18n.__(body)),
                );
            }
            let positions = geometry(&mut ui, &renderer);
            if !dependency.is_empty() {
                assert_fits(positions.label(i18n.__(dependency)), size);
            }
            assert_fits(
                positions.label(i18n.__("默认的 `*` 允许所有公网域名。列表为空时拒绝所有站点。")),
                size,
            );
            assert_fits(
                positions.label(i18n.__("当前运行时完整支持 default；其余选项会先写入配置。")),
                size,
            );
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn settings_help_gap_08_tunnel_examples_and_restart() {
    use super::super::system_settings_tunnel;
    use crate::app::message::settings::TunnelMessage;

    if !isolated("settings_help_gap_08_tunnel_examples_and_restart") {
        return;
    }

    let mut renderer = renderer().await;
    let mut app = fixture();
    app.system_settings_tab = SystemTab::Tunnel;
    app.tunnel_settings.provider = "tailscale".into();
    app.tunnel_settings.tailscale_hostname = "fixture.example.invalid".into();
    for ((locale, theme), width) in HELP_LAYOUTS.into_iter().zip([420, 720, 420]) {
        app.desktop_locale = locale;
        app.app_theme = theme;
        let t = |key| tr(locale, key);
        for funnel in [false, true] {
            app.tunnel_settings.tailscale_funnel = funnel;
            let size = Size::new(width, 960);
            let mut ui = scene(page(&app), &mut renderer, size);
            let positions = geometry(&mut ui, &renderer);
            let old = t(
                "保存配置后重启网关。公网 funnel 常见入口是 https://<主机名>/，tailnet-only 常见入口是 https://<主机名>:8443/。",
            );
            assert!(
                positions.labels.iter().all(|(label, _)| label != old),
                "Tailscale URLs are still inline"
            );
            assert_help_frame(
                &app,
                &mut ui,
                &mut renderer,
                size,
                "gap-08-tunnel",
                (
                    t("访问方式"),
                    t(
                        "公网 funnel 常见入口是 https://<主机名>/，tailnet-only 常见入口是 https://<主机名>:8443/。",
                    ),
                ),
            );
            assert_fits(positions.label(t("保存配置后重启网关。")), size);
            assert_fits(positions.label(t("serve 仅 tailnet 内可访问，funnel 才会暴露到公网。访问地址以启动日志为准，通常不要使用本地监听端口。")), size);
            assert!(matches!(
                click(&mut ui, &mut renderer, positions.label(t("启用 funnel（关闭时使用 serve）")).center()).as_slice(),
                [Message::Settings(SettingsMessage::Tunnel(TunnelMessage::TailscaleFunnelToggled(value)))]
                    if *value != funnel
            ));
        }
        let size = Size::new(width, 960);
        let mut ui = scene(scroll_frame(system_settings_tunnel::view(&app)), &mut renderer, size);
        assert_help_frame(
            &app,
            &mut ui,
            &mut renderer,
            size,
            "gap-08-tunnel-standalone",
            (t("隧道配置"), t("配置网关的暴露方式与 provider 细节。")),
        );
        assert_fits(geometry(&mut ui, &renderer).label(t("保存配置后重启网关。")), size);
    }
}
