#![cfg(not(target_arch = "wasm32"))]

use super::super::system_settings_gateway_client as gateway;
use super::*;
use crate::app::{App, state::GatewayClientServerDraft};
use vw_gateway_client::vw_api_types::agent_apps::CustomIconEntry;

#[path = "system_settings_agent_apps/ui_refresh_24_rename_tests.rs"]
mod rename_tests;
use iced::advanced::{
    clipboard,
    renderer::{Headless, Style},
};
use iced::{Event, Font, Renderer, Size, Theme, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
use vw_config_types::ui::{DesktopLocale, GatewayClientSystemSettingsConfig};

fn isolated(test: &str) -> bool {
    const CHILD: &str = "SHIJU_UI_REFRESH_24_CHILD";
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
    if let Some(directory) = std::env::var_os("SHIJU_UI_REFRESH_24_SCREENSHOTS") {
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

fn snapshot(
    view: Element<'_, Message>,
    renderer: &mut Renderer,
    theme: &Theme,
    size: Size<u32>,
    name: &str,
) {
    let mut ui = scene(view, renderer, size);
    capture(&mut ui, renderer, theme, size, name, mouse::Cursor::Unavailable);
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

const HOST: &str = "fixture.gateway.invalid";
const SECRET: &str = "fictional-skey";
const NAME: &str = "工作图标 · 測試 · Icon";

fn fixture() -> App {
    assert_eq!(crate::app::config::gateway_client_endpoint().normalized_host(), "[");
    let mut app = App::new().0;
    let s = &mut app.gateway_client_settings;
    s.name_input = "示例网关 · 測試".into();
    s.host_input = HOST.into();
    s.port = 42167;
    s.platform_id_input = "20000".into();
    s.company_id_input = "200000001".into();
    s.skey_input = SECRET.into();
    s.selected_server_id = "fixture".into();
    s.servers = vec![GatewayClientServerDraft {
        id: s.selected_server_id.clone(),
        name: s.name_input.clone(),
        host: s.host_input.clone(),
        port: s.port,
        platform_id: s.platform_id_input.clone(),
        company_id: s.company_id_input.clone(),
        skey: s.skey_input.clone(),
    }];
    app.agent_apps.icons.push(CustomIconEntry {
        id: "icon-fixture".into(),
        name: NAME.into(),
        data: String::new(),
        created_at: 0,
    });
    app
}

fn assert_fits(bounds: iced::Rectangle, size: Size<u32>) {
    assert!(bounds.width > 0.0 && bounds.height > 0.0, "{bounds:?}");
    assert!(bounds.x >= 0.0 && bounds.y >= 0.0, "{bounds:?}");
    assert!(bounds.x + bounds.width <= size.width as f32, "{bounds:?}");
    assert!(bounds.y + bounds.height <= size.height as f32, "{bounds:?}");
}

fn cursor(point: iced::Point) -> mouse::Cursor {
    mouse::Cursor::Available(point)
}

fn move_mouse(ui: &mut Ui<'_>, renderer: &mut Renderer, point: iced::Point) {
    assert!(
        send(
            ui,
            renderer,
            cursor(point),
            &[Event::Mouse(mouse::Event::CursorMoved { position: point })]
        )
        .is_empty()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_24_layout_and_interaction() {
    if !isolated("ui_refresh_24_layout_and_interaction") {
        return;
    }
    use crate::app::i18n::settings_models::connection_text::text as tr;
    use crate::app::message::{gateway_health::server_health_key, settings::GatewayClientMessage};

    let mut renderer = renderer().await;
    let mut app = fixture();
    let remote = GatewayClientServerDraft {
        id: "remote-fixture".into(),
        name: "遠端測試網關 Gateway ".repeat(4),
        host: format!("{}.invalid", "very-long-fixture-".repeat(13)),
        ..app.gateway_client_settings.servers[0].clone()
    };
    let key = server_health_key(&app.gateway_client_settings.servers[0]).unwrap();
    app.gateway_client_settings.health.insert(key, true);
    app.gateway_client_settings.servers.push(remote.clone());
    for (locale, width, tag) in [
        (DesktopLocale::English, 640, "en-640"),
        (DesktopLocale::SimplifiedChinese, 960, "zh-cn-960"),
        (DesktopLocale::TraditionalChinese, 640, "zh-tw-640"),
    ] {
        app.desktop_locale = locale;
        let size = Size::new(width, 1140);
        let mut ui = scene(gateway::view(&app), &mut renderer, size);
        let positions = geometry(&mut ui, &renderer);
        let host = positions.input(HOST);
        assert_fits(host, size);
        assert!(host.width >= 240.0, "{host:?}");
        let label = positions.label(tr(locale, "目标主机"));
        assert!((host.center_y() - label.center_y()).abs() < 0.5, "{host:?} {label:?}");
        let target = positions.label(&remote.name);
        assert_fits(target, size);
        for (theme, mode) in [(&Theme::Light, "light"), (&Theme::Dark, "dark")] {
            let path = format!("gateway-{tag}-{mode}");
            let resting =
                capture(&mut ui, &mut renderer, theme, size, &path, mouse::Cursor::Unavailable);
            move_mouse(&mut ui, &mut renderer, target.center());
            let hover = capture(
                &mut ui,
                &mut renderer,
                theme,
                size,
                &format!("{path}-hover"),
                cursor(target.center()),
            );
            assert!(resting != hover, "server row should show hover");
            let other = positions.label(&app.gateway_client_settings.servers[0].name).center();
            move_mouse(&mut ui, &mut renderer, other);
            let other_hover = capture(
                &mut ui,
                &mut renderer,
                theme,
                size,
                &format!("{path}-first"),
                cursor(other),
            );
            assert!(hover != other_hover, "hover should follow the cursor");
            move_mouse(&mut ui, &mut renderer, iced::Point::ORIGIN);
            assert!(
                resting
                    == capture(
                        &mut ui,
                        &mut renderer,
                        theme,
                        size,
                        &path,
                        mouse::Cursor::Unavailable
                    )
            );
        }
        assert!(matches!(
            click(&mut ui, &mut renderer, target.center()).as_slice(),
            [Message::Settings(SettingsMessage::GatewayClient(GatewayClientMessage::SelectServer(id)))]
                if id == "remote-fixture"
        ));
        assert!(matches!(
            click(&mut ui, &mut renderer, positions.label(tr(locale, "刷新状态")).center())
                .as_slice(),
            [Message::GatewayHealthTick]
        ));
        assert!(matches!(
            click(&mut ui, &mut renderer, positions.label(tr(locale, "新增网关")).center())
                .as_slice(),
            [Message::Settings(SettingsMessage::GatewayClient(GatewayClientMessage::AddServer))]
        ));
        assert!(matches!(
            click(&mut ui, &mut renderer, positions.label(tr(locale, "删除当前")).center()).as_slice(),
            [Message::Settings(SettingsMessage::GatewayClient(GatewayClientMessage::RemoveServerRequested(id)))]
                if id == "fixture"
        ));
    }

    app.gateway_client_settings.pending_remove_server_id = Some(remote.id);
    let size = Size::new(420, 540);
    let gateway_base =
        || container(gateway::view(&app)).width(Length::Fill).height(Length::Fill).into();
    let mut ui = scene(gateway::view_overlays(&app, gateway_base()), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    let cancel = positions.label(tr(app.desktop_locale, "取消"));
    let confirm = positions.label(tr(app.desktop_locale, "确认删除"));
    assert_fits(cancel, size);
    assert_fits(confirm, size);
    assert!(matches!(
        click(&mut ui, &mut renderer, cancel.center()).as_slice(),
        [Message::Settings(SettingsMessage::GatewayClient(
            GatewayClientMessage::RemoveServerCanceled
        ))]
    ));
    assert!(matches!(
        click(&mut ui, &mut renderer, confirm.center()).as_slice(),
        [Message::Settings(SettingsMessage::GatewayClient(GatewayClientMessage::RemoveServerConfirmed(id)))]
            if id == "remote-fixture"
    ));
    capture(
        &mut ui,
        &mut renderer,
        &Theme::Dark,
        size,
        "gateway-remove-420-dark",
        mouse::Cursor::Unavailable,
    );
    drop(ui);

    app.gateway_client_settings.pending_remove_server_id = None;
    app.gateway_client_settings.show_help_modal = true;
    let size = Size::new(420, 540);
    let mut ui = scene(
        gateway::view_overlays(
            &app,
            container(gateway::view(&app)).width(Length::Fill).height(Length::Fill).into(),
        ),
        &mut renderer,
        size,
    );
    assert_fits(
        geometry(&mut ui, &renderer).label(tr(app.desktop_locale, "客户端网关配置帮助")),
        size,
    );
    capture(
        &mut ui,
        &mut renderer,
        &Theme::Light,
        size,
        "gateway-help-420-light",
        mouse::Cursor::Unavailable,
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_24_preserves_values_and_disabled_states() {
    if !isolated("ui_refresh_24_preserves_values_and_disabled_states") {
        return;
    }
    use crate::app::message::settings::GatewayClientMessage as Gateway;
    use iced::keyboard::key::Named;

    let mut renderer = renderer().await;
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::English;
    let size = Size::new(640, 960);
    let mut ui = scene(gateway::view(&app), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    let delete = positions.label("Delete selected").center();
    move_mouse(&mut ui, &mut renderer, delete);
    assert!(click(&mut ui, &mut renderer, delete).is_empty());
    let masked = capture(
        &mut ui,
        &mut renderer,
        &Theme::Dark,
        size,
        "gateway-secret-dark",
        mouse::Cursor::Unavailable,
    );

    fn shorten(ui: &mut Ui<'_>, renderer: &mut Renderer, point: iced::Point) -> Vec<Message> {
        assert!(click(ui, renderer, point).is_empty());
        assert!(send(ui, renderer, cursor(point), &[key(Named::End)]).is_empty());
        send(ui, renderer, cursor(point), &[key(Named::Backspace)])
    }

    assert!(matches!(
        shorten(&mut ui, &mut renderer, positions.input("示例网关 · 測試").center()).as_slice(),
        [Message::Settings(SettingsMessage::GatewayClient(Gateway::NameChanged(name)))]
            if name == "示例网关 · 測"
    ));
    assert!(matches!(
        shorten(&mut ui, &mut renderer, positions.input(HOST).center()).as_slice(),
        [Message::Settings(SettingsMessage::GatewayClient(Gateway::HostChanged(host)))]
            if host == "fixture.gateway.invali"
    ));
    assert!(matches!(
        shorten(&mut ui, &mut renderer, positions.input("42167").center()).as_slice(),
        [Message::Settings(SettingsMessage::GatewayClient(Gateway::PortChanged(4216)))]
    ));
    assert!(matches!(
        shorten(&mut ui, &mut renderer, positions.input("20000").center()).as_slice(),
        [Message::Settings(SettingsMessage::GatewayClient(Gateway::PlatformIdChanged(id)))]
            if id == "2000"
    ));
    assert!(matches!(
        shorten(&mut ui, &mut renderer, positions.input("200000001").center()).as_slice(),
        [Message::Settings(SettingsMessage::GatewayClient(Gateway::CompanyIdChanged(id)))]
            if id == "20000000"
    ));
    assert!(matches!(
        shorten(&mut ui, &mut renderer, positions.input(&"•".repeat(SECRET.len())).center()).as_slice(),
        [Message::Settings(SettingsMessage::GatewayClient(Gateway::SkeyChanged(skey)))]
            if skey == "fictional-ske"
    ));
    drop(ui);

    app.gateway_client_settings.skey_input = "X".repeat(SECRET.len());
    let mut ui = scene(gateway::view(&app), &mut renderer, size);
    assert!(
        masked
            == capture(
                &mut ui,
                &mut renderer,
                &Theme::Dark,
                size,
                "gateway-secret-alternate-dark",
                mouse::Cursor::Unavailable
            ),
        "different secrets of equal length should have identical masked rendering"
    );
    drop(ui);

    app.gateway_client_settings.servers.clear();
    let mut ui = scene(gateway::view(&app), &mut renderer, size);
    assert_fits(geometry(&mut ui, &renderer).label("No Gateways"), size);
    capture(
        &mut ui,
        &mut renderer,
        &Theme::Light,
        size,
        "gateway-empty-light",
        mouse::Cursor::Unavailable,
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn help_rollout_19_renders_contextual_help() {
    use crate::app::components::system_settings_proxy::system_settings_proxy_tests::rollout::{
        MODES, Scene,
    };
    use crate::app::i18n::settings_models::connection_text::text as tr;
    use crate::app::message::settings::GatewayClientMessage;

    if !isolated("help_rollout_19_renders_contextual_help") {
        return;
    }
    let mut renderer = renderer().await;
    let mut app = fixture();
    for (locale, theme) in MODES {
        app.desktop_locale = locale;
        app.app_theme = theme;
        let mut settings = Scene::new(&app, &mut renderer, "gateway-client", gateway::view(&app));
        settings.label("fixture.gateway.invalid:42167");
        for (title, description) in [
            (
                "客户端网关",
                "配置 Desktop 作为客户端访问一个或多个 Gateway 时使用的地址与认证信息。",
            ),
            ("网关服务", "可以维护多个客户端网关，当前选中的网关会用于 Desktop 的 Gateway 请求。"),
            ("网关名称", "用于区分多个客户端网关。"),
            ("目标端口", "桌面端请求所连接的 Gateway 端口。"),
            ("平台 ID", "当前网关请求所属的平台租户。"),
            ("公司 ID", "当前网关请求所属的公司租户。"),
            ("skey", "可选 skey，会作为 Authorization: Bearer 发送。"),
        ] {
            settings.help(tr(locale, title), tr(locale, description));
        }
        let description = tr(locale, "桌面端请求所连接的 Gateway 主机。");
        let host_help = settings.help(tr(locale, "目标主机"), description);
        settings.actions(host_help, description);
        assert!(matches!(
            settings.edit(HOST).as_slice(),
            [Message::Settings(SettingsMessage::GatewayClient(GatewayClientMessage::HostChanged(host)))]
                if host == "fixture.gateway.invali"
        ));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn help_rollout_19_icon_library() {
    use crate::app::components::system_settings_proxy::system_settings_proxy_tests::rollout::{
        MODES, Scene,
    };

    if !isolated("help_rollout_19_icon_library") {
        return;
    }
    let mut renderer = renderer().await;
    let mut app = fixture();
    let error = "Fixture: icon service unavailable";
    app.agent_apps.error = Some(error.into());
    for (locale, theme) in MODES {
        app.desktop_locale = locale;
        app.app_theme = theme;
        let i18n = I18n::new(locale);
        let mut settings = Scene::new(
            &app,
            &mut renderer,
            "gateway-icons",
            view(&app.agent_apps, i18n, &app.app_theme),
        );
        settings.label(error);
        let description = i18n.__("读取当前网关与公司的图标库");
        let help = settings.help(i18n.__("我的图标"), description);
        settings.actions(help, description);
        let retry = settings.label("重试").center();
        assert!(matches!(
            settings.click(retry).as_slice(),
            [Message::Settings(SettingsMessage::AgentApps(AgentAppsMessage::Refresh))]
        ));
    }
}
