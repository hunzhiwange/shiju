//! Headless rendering and event regression for the network settings forms.
#![cfg(not(target_arch = "wasm32"))]

use super::super::{system_settings::SystemTab, system_settings_network as network};
use super::{App, Message, SettingsMessage};
use crate::app::i18n::settings_extensions::text as tr;
use crate::app::message::settings::TunnelMessage;
use iced::advanced::{
    clipboard,
    renderer::{Headless, Style},
};
use iced::keyboard::key::Named;
use iced::widget::{container, scrollable};
use iced::{Element, Event, Font, Length, Renderer, Size, Theme, mouse};
use iced::{Point, Rectangle};
use iced_runtime::user_interface::{Cache, UserInterface};
use vw_config_types::ui::{DesktopLocale, GatewayClientSystemSettingsConfig};

fn isolated(test: &str) -> bool {
    const CHILD: &str = "SHIJU_UI_REFRESH_26_CHILD";
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
    if let Some(directory) = std::env::var_os("SHIJU_UI_REFRESH_26_SCREENSHOTS") {
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

const PROXY: &str = "socks5h://fixture:secret@proxy.example.invalid:7891";
const SCOPE: &str = "provider.fixture, tool.http_request";
const WEBDRIVER: &str = "http://127.0.0.1:9515";
const TOKEN: &str = "fixture-token";

fn fixture() -> App {
    assert_eq!(crate::app::config::gateway_client_endpoint().normalized_host(), "[");
    let mut app = App::new().0;
    let browser = &mut app.browser_settings;
    browser.enabled = false;
    browser.backend = "native".into();
    browser.browser_open = "default".into();
    browser.native_webdriver_url = WEBDRIVER.into();
    browser.native_chrome_path_input = format!("/tmp/{}/浏览器", "long-path/".repeat(12));
    browser.computer_use_api_key_input = TOKEN.into();
    browser.computer_use_timeout_ms_input = "12345".into();
    let proxy = &mut app.proxy_settings;
    proxy.enabled = false;
    proxy.all_proxy = PROXY.into();
    proxy.services_input = SCOPE.into();
    let tunnel = &mut app.tunnel_settings;
    tunnel.provider = "none".into();
    tunnel.cloudflare_token = TOKEN.into();
    tunnel.ngrok_auth_token = TOKEN.into();
    tunnel.custom_start_command = "fixture local {port} --to {host}".into();
    app
}

fn page(app: &App) -> Element<'_, Message> {
    use scrollable::{Direction, Scrollbar};

    scrollable(container(network::view(app)).padding([0, 7]).width(Length::Fill))
        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
        .height(Length::Fill)
        .into()
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_26_layout_and_interaction() {
    if !isolated("ui_refresh_26_layout_and_interaction") {
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
        for (tab, name) in [
            (SystemTab::Browser, "browser"),
            (SystemTab::Proxy, "proxy"),
            (SystemTab::Tunnel, "tunnel"),
        ] {
            app.system_settings_tab = tab;
            app.tunnel_settings.provider = "tailscale".into();
            let size = Size::new(width, 2100);
            let mut ui = scene(page(&app), &mut renderer, size);
            let t = |source| tr(locale, source);
            let positions = geometry(&mut ui, &renderer);
            match tab {
                SystemTab::Browser => {
                    assert_input_center(&positions, WEBDRIVER, t("WebDriver 地址"), None, size)
                }
                SystemTab::Proxy => assert_input_center(
                    &positions,
                    SCOPE,
                    t("服务选择器"),
                    Some(t("仅在范围为指定服务时生效。")),
                    size,
                ),
                SystemTab::Tunnel => {
                    let trigger = select_bounds(
                        &positions,
                        t("提供者"),
                        t("选择自动暴露网关到公网或私有网络的方式。"),
                    );
                    assert_fits(trigger, size);
                    assert_fits(positions.label(t(EXPOSURE)), size);
                }
                _ => unreachable!(),
            }
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
        }
    }

    app.proxy_settings.show_help_modal = true;
    app.system_settings_tab = SystemTab::Proxy;
    let size = Size::new(420, 520);
    let mut ui = scene(
        super::super::system_settings_proxy::view_overlays(&app, page(&app)),
        &mut renderer,
        size,
    );
    for (theme, mode) in [(&Theme::Light, "light"), (&Theme::Dark, "dark")] {
        capture(
            &mut ui,
            &mut renderer,
            theme,
            size,
            &format!("proxy-help-420-{mode}"),
            mouse::Cursor::Unavailable,
        );
    }
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

fn select_bounds(geometry: &Geometry, label: &str, hint: &str) -> Rectangle {
    let label = geometry.label(label);
    let hint = geometry.label(hint);
    let center = (label.y + hint.y + hint.height) / 2.0;
    let trigger = geometry
        .focusables
        .iter()
        .copied()
        .find(|bounds| bounds.x > label.x && (bounds.center_y() - center).abs() < 35.0)
        .expect("provider must be a keyboard focusable settings select");
    assert!((trigger.center_y() - center).abs() < 0.5, "{trigger:?} {label:?} {hint:?}");
    trigger
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_26_tunnel_keyboard_cancel_and_confirm() {
    if !isolated("ui_refresh_26_tunnel_keyboard_cancel_and_confirm") {
        return;
    }

    let mut renderer = renderer().await;
    let mut app = fixture();
    let locale = DesktopLocale::TraditionalChinese;
    app.desktop_locale = locale;
    app.system_settings_tab = SystemTab::Tunnel;
    let size = Size::new(420, 460);
    let mut ui = scene(page(&app), &mut renderer, size);
    let trigger = select_bounds(
        &geometry(&mut ui, &renderer),
        tr(locale, "提供者"),
        tr(locale, "选择自动暴露网关到公网或私有网络的方式。"),
    );
    assert_fits(trigger, size);
    assert!(click(&mut ui, &mut renderer, trigger.center()).is_empty());
    assert!(key_at(&mut ui, &mut renderer, Named::ArrowDown).is_empty());
    assert!(key_at(&mut ui, &mut renderer, Named::Escape).is_empty());
    assert!(key_at(&mut ui, &mut renderer, Named::Enter).is_empty());
    assert!(key_at(&mut ui, &mut renderer, Named::ArrowDown).is_empty());
    assert!(matches!(
        key_at(&mut ui, &mut renderer, Named::Enter).as_slice(),
        [Message::Settings(SettingsMessage::Tunnel(TunnelMessage::ProviderChanged(value)))]
            if value == "cloudflare"
    ));
    let size = Size::new(420, 320);
    let mut ui = scene(page(&app), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    let viewport = positions.scroll.unwrap().0;
    wheel(&mut ui, &mut renderer, viewport.center(), -85.0);
    let positions = geometry(&mut ui, &renderer);
    let translation = positions.scroll.unwrap().1;
    let trigger = select_bounds(
        &positions,
        tr(locale, "提供者"),
        tr(locale, "选择自动暴露网关到公网或私有网络的方式。"),
    );
    assert!(translation.y > 0.0);
    assert!(click(&mut ui, &mut renderer, trigger.center() - translation).is_empty());
    let (menu, initial) = geometry(&mut ui, &renderer).scroll.expect("provider menu");
    assert_fits(menu, size);
    wheel(&mut ui, &mut renderer, menu.center(), -150.0);
    assert!(geometry(&mut ui, &renderer).scroll.unwrap().1.y > initial.y);
    capture(
        &mut ui,
        &mut renderer,
        &Theme::Light,
        size,
        "tunnel-menu-scrolled-420",
        mouse::Cursor::Unavailable,
    );
    assert!(key_at(&mut ui, &mut renderer, Named::Escape).is_empty());
}

fn wheel(ui: &mut Ui<'_>, renderer: &mut Renderer, point: Point, y: f32) {
    assert!(
        send(
            ui,
            renderer,
            at(point),
            &[Event::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Pixels { x: 0.0, y },
            })],
        )
        .is_empty()
    );
}

fn assert_input_center(
    positions: &Geometry,
    value: &str,
    label: &str,
    hint: Option<&str>,
    size: Size<u32>,
) {
    let input = positions.input(value);
    let label = positions.label(label);
    assert_fits(input, size);
    assert!(label.x + label.width < input.x);
    let center = if let Some(hint) = hint {
        let hint = positions.label(hint);
        assert_fits(hint, size);
        (label.y + hint.y + hint.height) / 2.0
    } else {
        label.center_y()
    };
    assert!((input.center_y() - center).abs() < 0.5);
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_26_preserves_values_and_disabled_states() {
    use crate::app::message::settings::BrowserMessage as Browser;

    if !isolated("ui_refresh_26_preserves_values_and_disabled_states") {
        return;
    }

    let mut renderer = renderer().await;
    let mut app = fixture();
    let locale = DesktopLocale::English;
    app.desktop_locale = locale;
    let t = |source| tr(locale, source);
    let size = Size::new(640, 2100);
    app.system_settings_tab = SystemTab::Browser;
    let mut ui = scene(page(&app), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    let backend = positions.label(t("后端实现"));
    let backend_trigger = positions
        .focusables
        .iter()
        .copied()
        .filter(|bounds| bounds.x > backend.x)
        .min_by(|a, b| {
            (a.center_y() - backend.center_y())
                .abs()
                .total_cmp(&(b.center_y() - backend.center_y()).abs())
        })
        .unwrap();
    assert!(click(&mut ui, &mut renderer, backend_trigger.center()).is_empty());
    assert!(key_at(&mut ui, &mut renderer, Named::ArrowDown).is_empty());
    let messages = key_at(&mut ui, &mut renderer, Named::Enter);
    let [Message::Settings(settings @ SettingsMessage::Browser(Browser::BackendChanged(id)))] =
        messages.as_slice()
    else {
        panic!("expected one backend edit: {messages:?}");
    };
    assert_eq!(id, "computer_use");
    assert!(matches!(
        shorten(&mut ui, &mut renderer, positions.input("12345")).as_slice(),
        [Message::Settings(SettingsMessage::Browser(Browser::ComputerUseTimeoutMsChanged(value)))]
            if value == "1234"
    ));
    let editor = positions.focusables.iter().find(|bounds| bounds.height >= 96.0).unwrap();
    assert!(click(&mut ui, &mut renderer, editor.center()).iter().all(|message| matches!(
        message,
        Message::Settings(SettingsMessage::Browser(Browser::AllowedDomainsEditorAction(_)))
    )));
    assert!(matches!(
        key_at(&mut ui, &mut renderer, Named::Enter).as_slice(),
        [Message::Settings(SettingsMessage::Browser(Browser::AllowedDomainsEditorAction(
            iced::widget::text_editor::Action::Edit(_),
        )))]
    ));
    drop(ui);

    // Force local validation to fail, before any gateway task could be created.
    app.browser_settings.computer_use_timeout_ms_input = "0".into();
    drop(crate::app::message::settings::update(&mut app, settings.clone()));
    assert_eq!(app.browser_settings.backend, "computer_use");
    assert!(app.browser_settings.save_error.is_some());
    let mut ui = scene(page(&app), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    assert!(!positions.inputs.iter().any(|(value, _)| value == WEBDRIVER));
    assert_fits(positions.input(&"•".repeat(TOKEN.len())), size);
    let error = app
        .browser_settings
        .save_error_translation
        .as_ref()
        .unwrap()
        .render(crate::app::i18n::I18n::new(locale))
        .unwrap();
    assert_fits(positions.label(&error), size);
    capture(
        &mut ui,
        &mut renderer,
        &Theme::Dark,
        size,
        "browser-computer-use-error",
        mouse::Cursor::Unavailable,
    );
    drop(ui);

    app.system_settings_tab = SystemTab::Proxy;
    let mut ui = scene(page(&app), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    for (old, expected) in [(PROXY, &PROXY[..PROXY.len() - 1]), (SCOPE, &SCOPE[..SCOPE.len() - 1])]
    {
        let messages = shorten(&mut ui, &mut renderer, positions.input(old));
        match (old, messages.as_slice()) {
            (PROXY, [Message::Settings(SettingsMessage::ProxyAllChanged(value))])
            | (SCOPE, [Message::Settings(SettingsMessage::ProxyServicesChanged(value))]) => {
                assert_eq!(value, expected);
            }
            _ => panic!("unexpected proxy edit: {messages:?}"),
        }
        assert!(key_at(&mut ui, &mut renderer, Named::Enter).is_empty());
    }
    drop(ui);

    app.system_settings_tab = SystemTab::Tunnel;
    for provider in ["none", "cloudflare", "tailscale", "ngrok", "custom", "unsupported"] {
        app.tunnel_settings.provider = provider.into();
        let size = Size::new(640, 720);
        let mut ui = scene(page(&app), &mut renderer, size);
        let positions = geometry(&mut ui, &renderer);
        match provider {
            "none" | "unsupported" => {
                assert!(positions.inputs.is_empty());
                assert_fits(
                    positions.label(t("当前仅监听本地网关地址，不会主动创建公网或 tailnet 暴露。")),
                    size,
                );
                assert!(key_at(&mut ui, &mut renderer, Named::Enter).is_empty());
            }
            "cloudflare" => assert!(matches!(
                shorten(&mut ui, &mut renderer, positions.input(TOKEN)).as_slice(),
                [Message::Settings(SettingsMessage::Tunnel(TunnelMessage::CloudflareTokenChanged(value)))]
                    if value == "fixture-toke"
            )),
            "ngrok" => assert!(matches!(
                shorten(&mut ui, &mut renderer, positions.input(TOKEN)).as_slice(),
                [Message::Settings(SettingsMessage::Tunnel(TunnelMessage::NgrokAuthTokenChanged(value)))]
                    if value == "fixture-toke"
            )),
            "tailscale" => {
                assert_fits(positions.label(t(EXPOSURE)), size);
                assert!(matches!(
                    click(
                        &mut ui,
                        &mut renderer,
                        positions.label(t("启用 funnel（关闭时使用 serve）")).center()
                    )
                    .as_slice(),
                    [Message::Settings(SettingsMessage::Tunnel(
                        TunnelMessage::TailscaleFunnelToggled(true)
                    ))]
                ));
            }
            "custom" => assert!(matches!(
                shorten(&mut ui, &mut renderer, positions.input("fixture local {port} --to {host}")).as_slice(),
                [Message::Settings(SettingsMessage::Tunnel(TunnelMessage::CustomStartCommandChanged(value)))]
                    if value == "fixture local {port} --to {host"
            )),
            _ => unreachable!(),
        }
        capture(
            &mut ui,
            &mut renderer,
            &Theme::Light,
            size,
            &format!("tunnel-{provider}"),
            mouse::Cursor::Unavailable,
        );
    }
}

const EXPOSURE: &str = "serve 仅 tailnet 内可访问，funnel 才会暴露到公网。访问地址以启动日志为准，通常不要使用本地监听端口。";

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_26_proxy_scope_hover_and_help() {
    use super::super::system_settings_proxy;

    if !isolated("ui_refresh_26_proxy_scope_hover_and_help") {
        return;
    }

    let mut renderer = renderer().await;
    let mut app = fixture();
    let locale = DesktopLocale::TraditionalChinese;
    app.desktop_locale = locale;
    app.system_settings_tab = SystemTab::Proxy;
    let size = Size::new(420, 520);
    let mut ui = scene(page(&app), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    let trigger = select_bounds(
        &positions,
        tr(locale, "生效范围"),
        tr(locale, "决定代理应用到环境、全局流量或指定服务。"),
    );
    assert!(click(&mut ui, &mut renderer, trigger.center()).is_empty());
    let menu = geometry(&mut ui, &renderer).scroll.expect("scope menu").0;
    assert_fits(menu, size);
    let mut images = Vec::new();
    for (point, name) in [
        (Point::new(menu.center_x(), menu.y + 18.0), "first"),
        (Point::new(menu.center_x(), menu.y + 56.0), "second"),
        (Point::ORIGIN, "left"),
    ] {
        assert!(
            send(
                &mut ui,
                &mut renderer,
                at(point),
                &[Event::Mouse(mouse::Event::CursorMoved { position: point })]
            )
            .is_empty()
        );
        images.push(capture(
            &mut ui,
            &mut renderer,
            &Theme::Dark,
            size,
            &format!("proxy-scope-{name}"),
            at(point),
        ));
    }
    assert!(images[0] != images[1] && images[0] != images[2] && images[1] != images[2]);
    let third = Point::new(menu.center_x(), menu.y + 94.0);
    assert!(matches!(
        click(&mut ui, &mut renderer, third).as_slice(),
        [Message::Settings(SettingsMessage::ProxyScopeTextChanged(value))]
            if value == "指定服务"
    ));
    assert!(click(&mut ui, &mut renderer, trigger.center()).is_empty());
    let underneath = positions
        .label(crate::app::i18n::settings_models::connection_text::text(locale, "浏览器"))
        .center();
    assert!(click(&mut ui, &mut renderer, underneath).is_empty());
    assert!(matches!(
        click(&mut ui, &mut renderer, underneath).as_slice(),
        [Message::Settings(SettingsMessage::SystemTabSelected(SystemTab::Browser))]
    ));
    let help = positions.label(tr(locale, "代理设置")).center_y();
    assert!(matches!(
        click(&mut ui, &mut renderer, Point::new(size.width as f32 - 20.0, help)).as_slice(),
        [Message::Settings(SettingsMessage::ProxyHelpOpen)]
    ));
    drop(ui);

    app.proxy_settings.show_help_modal = true;
    let mut ui = scene(system_settings_proxy::view_overlays(&app, page(&app)), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    let title = positions.label("Proxy 設定說明");
    let (body, offset) = positions.scroll.expect("help body");
    wheel(&mut ui, &mut renderer, body.center(), -250.0);
    let scrolled = geometry(&mut ui, &renderer);
    assert!(scrolled.scroll.unwrap().1.y > offset.y);
    assert_eq!(scrolled.label("Proxy 設定說明"), title);
    capture(
        &mut ui,
        &mut renderer,
        &Theme::Dark,
        size,
        "proxy-help-420-scrolled",
        mouse::Cursor::Unavailable,
    );
    let close = Point::new(size.width as f32 - 24.0 - 14.0, title.center_y());
    assert!(matches!(
        click(&mut ui, &mut renderer, close).as_slice(),
        [Message::Settings(SettingsMessage::ProxyHelpClose)]
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn browser_install_help_renders_chinese_in_light_and_dark_themes() {
    if !isolated("browser_install_help_renders_chinese_in_light_and_dark_themes") {
        return;
    }
    let mut renderer = renderer().await;
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::SimplifiedChinese;
    app.browser_settings.backend = "agent_browser".into();
    app.system_settings_tab = SystemTab::Browser;
    let size = Size::new(1000, 800);
    for (theme, name) in
        [(Theme::Light, "browser-install-light"), (Theme::Dark, "browser-install-dark")]
    {
        app.app_theme = theme.clone();
        let help = super::super::system_settings_help::with_help_modal(
            &app,
            page(&app),
            SystemTab::Browser,
            Some(SystemTab::Browser),
        );
        let mut ui = scene(help, &mut renderer, size);
        let pixels =
            capture(&mut ui, &mut renderer, &theme, size, name, mouse::Cursor::Unavailable);
        assert_eq!(pixels.len(), size.width as usize * size.height as usize * 4);
    }
}
