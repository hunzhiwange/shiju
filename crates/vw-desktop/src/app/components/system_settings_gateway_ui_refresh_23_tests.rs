#![cfg(not(target_arch = "wasm32"))]

use super::*;
use iced::advanced::{
    clipboard,
    renderer::{Headless, Style},
};
use iced::{Event, Font, Renderer, Size, Theme, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
use vw_config_types::ui::{DesktopLocale, GatewayClientSystemSettingsConfig};
fn isolated(test: &str) -> bool {
    const CHILD: &str = "SHIJU_UI_REFRESH_23_CHILD";
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
    if let Some(directory) = std::env::var_os("SHIJU_UI_REFRESH_23_SCREENSHOTS") {
        image::save_buffer(
            std::path::PathBuf::from(directory).join(format!("{name}.png")),
            &pixels,
            size.width,
            size.height,
            image::ColorType::Rgba8,
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

const DRAFT_NAME: &str = "测试 · 測試 · fixture key";
const SAVE_ERROR: &str = "fixture: configuration could not be saved";
const NODE_TOKEN: &str = "fictional-node-token";

fn fixture() -> App {
    assert_eq!(crate::app::config::gateway_client_endpoint().normalized_host(), "[");
    let mut app = App::new().0;
    let s = &mut app.gateway_settings;
    s.host_input = format!("{}.example.invalid", "fictional-long-gateway-".repeat(14));
    s.node_control_auth_token_input = NODE_TOKEN.into();
    s.node_control_allowed_node_ids_input = "/fixture/测试/測試/".repeat(16);
    s.port = 42167;
    s.auth_enabled = true;
    s.node_control_enabled = false;
    s.new_skey_calendar_month = "2030-02".into();
    s.new_skey_name_input = DRAFT_NAME.into();
    s.new_skey_expires_at_input = "2030-02-14".into();
    s.skeys.clear();
    app
}

fn assert_within(bounds: iced::Rectangle, width: u32) {
    assert!(bounds.x >= 0.0 && bounds.x + bounds.width <= width as f32, "{bounds:?}");
}

fn assert_centered(left: iced::Rectangle, right: iced::Rectangle) {
    assert!((left.center_y() - right.center_y()).abs() < 0.5, "{left:?} vs. {right:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_23_layout_and_interaction() {
    if !isolated("ui_refresh_23_layout_and_interaction") {
        return;
    }
    let mut renderer = renderer().await;
    let mut app = fixture();
    for (locale, theme, width, suffix) in [
        (DesktopLocale::SimplifiedChinese, Theme::Light, 640, "zh-cn-light"),
        (DesktopLocale::TraditionalChinese, Theme::Dark, 640, "zh-tw-dark"),
        (DesktopLocale::English, Theme::Light, 960, "en-light"),
        (DesktopLocale::English, Theme::Dark, 640, "en-dark"),
    ] {
        app.desktop_locale = locale;
        app.gateway_settings.active_tab = GatewaySettingsTab::Config;
        let size = Size::new(width, 2100);
        let mut ui = scene(view(&app), &mut renderer, size);
        let positions = geometry(&mut ui, &renderer);
        assert_within(positions.input(&app.gateway_settings.host_input), width);
        assert_centered(positions.input("42167"), positions.label("42167"));
        assert_within(
            positions.input(&app.gateway_settings.node_control_allowed_node_ids_input),
            width,
        );
        capture(
            &mut ui,
            &mut renderer,
            &theme,
            size,
            &format!("config-{suffix}"),
            mouse::Cursor::Unavailable,
        );
        drop(ui);

        app.gateway_settings.active_tab = GatewaySettingsTab::Skeys;
        app.gateway_settings.new_skey_calendar_open = true;
        snapshot(
            view(&app),
            &mut renderer,
            &theme,
            Size::new(width, 720),
            &format!("calendar-{suffix}"),
        );
        app.gateway_settings.new_skey_calendar_open = false;

        app.gateway_settings.pending_service_command_confirmation = Some("stop".into());
        snapshot(
            view_overlays(&app, view(&app)),
            &mut renderer,
            &theme,
            Size::new(420, 520),
            &format!("confirm-{suffix}"),
        );
        app.gateway_settings.pending_service_command_confirmation = None;
        app.gateway_settings.show_help_modal = true;
        snapshot(
            view_overlays(&app, view(&app)),
            &mut renderer,
            &theme,
            Size::new(420, 560),
            &format!("help-{suffix}"),
        );
        app.gateway_settings.show_help_modal = false;
    }

    app.desktop_locale = DesktopLocale::English;
    app.gateway_settings.active_tab = GatewaySettingsTab::Skeys;
    let size = Size::new(640, 720);
    let mut ui = scene(view(&app), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    let generate = positions.label(connection_text(app.desktop_locale, "生成 skey"));
    assert_centered(positions.input(DRAFT_NAME), generate);
    let generate = generate.center();
    assert!(matches!(
        click(&mut ui, &mut renderer, generate).as_slice(),
        [Message::Settings(SettingsMessage::Gateway(GatewayMessage::AddSkey))]
    ));
    drop(ui);
    app.gateway_settings.new_skey_calendar_open = true;
    let mut ui = scene(view(&app), &mut renderer, size);
    capture(
        &mut ui,
        &mut renderer,
        &Theme::Dark,
        size,
        "calendar-open",
        mouse::Cursor::Unavailable,
    );
    assert!(matches!(
        click(&mut ui, &mut renderer, generate).as_slice(),
        [Message::Settings(SettingsMessage::Gateway(GatewayMessage::NewSkeyCalendarClosed))]
    ));
    drop(ui);
    let mut ui = scene(skey_calendar_popover(&app), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    let day = positions.label("21");
    assert!((day.center_x() - positions.label("Thu").center_x()).abs() < 0.5);
    assert!((day.center_y() - positions.label("20").center_y()).abs() < 0.5);
    drop(ui);

    let mut copy = service_command_copy_button("restart", None);
    let mut tree = iced::advanced::widget::Tree::new(&copy);
    let limits = iced::advanced::layout::Limits::new(Size::ZERO, Size::new(108.0, 40.0));
    let node = copy.as_widget_mut().layout(&mut tree, &renderer, &limits);
    let content =
        iced::advanced::Layout::new(&node).children().next().unwrap().children().next().unwrap();
    let text = content.children().next().unwrap().bounds();
    let icon = content.children().last().unwrap().bounds();
    assert_centered(text, icon);
    assert_within(icon, 108);

    // Start the picker at the origin, where there is enough room to open below.
    let mut picker = skey_date_picker(&app);
    let mut tree = iced::advanced::widget::Tree::new(&picker);
    let bounds = iced::advanced::layout::Limits::new(Size::ZERO, Size::new(640.0, 720.0));
    let trigger = picker.as_widget_mut().layout(&mut tree, &renderer, &bounds);
    let shift = iced::Vector::new(0.0, trigger.size().height + 6.0);
    let mut ui = scene(picker, &mut renderer, size);
    let resting = capture(
        &mut ui,
        &mut renderer,
        &Theme::Dark,
        size,
        "picker-resting",
        mouse::Cursor::Unavailable,
    );
    let mut hovered = Vec::new();
    for label in ["20", "21"] {
        let point = positions.label(label).center() + shift;
        let cursor = mouse::Cursor::Available(point);
        assert!(
            send(
                &mut ui,
                &mut renderer,
                cursor,
                &[Event::Mouse(mouse::Event::CursorMoved { position: point })]
            )
            .is_empty()
        );
        hovered.push(capture(
            &mut ui,
            &mut renderer,
            &Theme::Dark,
            size,
            &format!("picker-hover-{label}"),
            cursor,
        ));
    }
    assert_ne!(resting, hovered[0]);
    assert_ne!(hovered[0], hovered[1]);
    let away = iced::Point::new(620.0, 550.0);
    let cursor = mouse::Cursor::Available(away);
    assert!(
        send(
            &mut ui,
            &mut renderer,
            cursor,
            &[Event::Mouse(mouse::Event::CursorMoved { position: away })]
        )
        .is_empty()
    );
    assert_eq!(
        resting,
        capture(&mut ui, &mut renderer, &Theme::Dark, size, "picker-hover-left", cursor)
    );
    assert!(matches!(
        click(&mut ui, &mut renderer, day.center() + shift).as_slice(),
        [Message::Settings(SettingsMessage::Gateway(GatewayMessage::NewSkeyExpiresDateSelected(value)))]
            if value == "2030-02-21"
    ));
    assert!(matches!(
        click(&mut ui, &mut renderer, positions.label(">").center() + shift).as_slice(),
        [Message::Settings(SettingsMessage::Gateway(GatewayMessage::NewSkeyExpiresMonthChanged(
            1
        )))]
    ));
    assert!(matches!(
        click(&mut ui, &mut renderer, positions.label("Never expires").center() + shift).as_slice(),
        [Message::Settings(SettingsMessage::Gateway(GatewayMessage::NewSkeyExpiresAtCleared))]
    ));
    drop(ui);

    app.show_system_settings = true;
    app.system_settings_tab = crate::app::components::system_settings::SystemTab::Gateway;
    snapshot(
        crate::app::components::system_settings::view(&app),
        &mut renderer,
        &Theme::Dark,
        Size::new(640, 800),
        "calendar-in-narrow-window",
    );
    app.gateway_settings.active_tab = GatewaySettingsTab::Config;
    snapshot(
        crate::app::components::system_settings::view(&app),
        &mut renderer,
        &Theme::Light,
        Size::new(640, 800),
        "config-in-narrow-window",
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_23_preserves_values_and_disabled_states() {
    if !isolated("ui_refresh_23_preserves_values_and_disabled_states") {
        return;
    }
    use iced::keyboard::key::Named;
    let mut renderer = renderer().await;
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::English;
    app.gateway_settings.service_action_running = Some("status".into());
    drop(crate::app::message::settings::update(
        &mut app,
        SettingsMessage::AgentConfigSaved { tag: "gateway", result: Err(SAVE_ERROR.into()) },
    ));
    assert_eq!(app.gateway_settings.new_skey_name_input, DRAFT_NAME);
    assert_eq!(app.gateway_settings.new_skey_expires_at_input, "2030-02-14");
    let error = app.gateway_settings.save_error.as_ref().unwrap().display(app.desktop_locale);
    assert!(error.contains(SAVE_ERROR));
    let size = Size::new(640, 2100);
    let mut ui = scene(view(&app), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    let label = |key| positions.label(connection_text(app.desktop_locale, key)).center();
    let capture_hover = mouse::Cursor::Available(label("启动服务"));
    capture(&mut ui, &mut renderer, &Theme::Dark, size, "busy-config-save-error", capture_hover);
    assert!(click(&mut ui, &mut renderer, label("启动服务")).is_empty());
    assert!(
        matches!(click(&mut ui, &mut renderer, positions.label("shiju server start").center()).as_slice(),
        [Message::Settings(SettingsMessage::Gateway(GatewayMessage::CopyServiceCommand(command)))]
            if command == "shiju server start")
    );
    assert!(matches!(
        click(&mut ui, &mut renderer, label("通过 Authorization: Bearer 校验 skey")).as_slice(),
        [Message::Settings(SettingsMessage::Gateway(GatewayMessage::AuthEnabledToggled(false)))]
    ));
    assert!(matches!(
        click(&mut ui, &mut renderer, label("启用实验性的 node-control API")).as_slice(),
        [Message::Settings(SettingsMessage::Gateway(GatewayMessage::NodeControlEnabledToggled(
            true
        )))]
    ));
    let token = mouse::Cursor::Available(positions.input(&"•".repeat(NODE_TOKEN.len())).center());
    assert!(
        send(
            &mut ui,
            &mut renderer,
            token,
            &[Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)), key(Named::End),]
        )
        .is_empty()
    );
    assert!(matches!(send(&mut ui, &mut renderer, token, &[key(Named::Backspace)]).as_slice(),
        [Message::Settings(SettingsMessage::Gateway(GatewayMessage::NodeControlAuthTokenChanged(value)))]
            if value == "fictional-node-toke"));
    assert!(
        send(&mut ui, &mut renderer, token, &[key(Named::Enter), key(Named::Escape)]).is_empty()
    );
    let port = positions.input("42167").center();
    assert!(click(&mut ui, &mut renderer, port).is_empty());
    let cursor = mouse::Cursor::Available(port);
    assert!(send(&mut ui, &mut renderer, cursor, &[key(Named::End)]).is_empty());
    assert!(matches!(
        send(&mut ui, &mut renderer, cursor, &[key(Named::Backspace)]).as_slice(),
        [Message::Settings(SettingsMessage::Gateway(GatewayMessage::PortChanged(4216)))]
    ));
    drop(ui);

    app.gateway_settings.active_tab = GatewaySettingsTab::Skeys;
    let size = Size::new(640, 720);
    let mut ui = scene(view(&app), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    positions.label(&error);
    assert_within(positions.input(DRAFT_NAME), size.width);
    capture(
        &mut ui,
        &mut renderer,
        &Theme::Light,
        size,
        "skey-save-error",
        mouse::Cursor::Unavailable,
    );
    drop(ui);

    app.gateway_settings.pending_service_command_confirmation = Some("stop".into());
    let mut ui = scene(view_overlays(&app, view(&app)), &mut renderer, Size::new(420, 520));
    let positions = geometry(&mut ui, &renderer);
    let cancel = positions.label(connection_text(app.desktop_locale, "取消"));
    let confirm = owned_text(
        app.desktop_locale,
        "确认%s",
        &[service_display_label(app.desktop_locale, "stop").into()],
    );
    let confirm = positions.labels.iter().rev().find(|(value, _)| value == &confirm).unwrap().1;
    assert!(matches!(
        click(&mut ui, &mut renderer, cancel.center()).as_slice(),
        [Message::Settings(SettingsMessage::Gateway(GatewayMessage::CancelDangerServiceCommand))]
    ));
    assert!(matches!(click(&mut ui, &mut renderer, confirm.center()).as_slice(),
        [Message::Settings(SettingsMessage::Gateway(GatewayMessage::ConfirmDangerServiceCommand(command)))]
            if command == "stop"));
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_23_scrolls_skeys_and_keeps_row_indices() {
    if !isolated("ui_refresh_23_scrolls_skeys_and_keeps_row_indices") {
        return;
    }
    let mut renderer = renderer().await;
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::English;
    let s = &mut app.gateway_settings;
    s.active_tab = GatewaySettingsTab::Skeys;
    s.last_created_skey = Some(format!("fixture-{}", "A".repeat(76)));
    s.skeys = (0..15)
        .map(|index| vw_config_types::gateway::GatewaySkey {
            enabled: true,
            skey: None,
            skey_hash: format!("fictional-hash-{index:02}"),
            masked_skey: format!("sk-fixture**************{index:02}"),
            name: if index == 0 {
                "测试測試".repeat(20)
            } else {
                format!("fixture key {index:02}")
            },
            expires_at: (index == 0).then_some("2030-02-14T23:59:59Z".into()),
        })
        .collect();

    let size = Size::new(640, 1050);
    let mut ui = scene(view(&app), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    assert!(matches!(
        click(&mut ui, &mut renderer, positions.label("Copy").center()).as_slice(),
        [Message::Settings(SettingsMessage::Gateway(GatewayMessage::CopyLastCreatedSkey))]
    ));
    let (viewport, translation) = positions.scroll.unwrap();
    assert_eq!(translation.y, 0.0);
    let tail = positions.label("fixture key 14");
    assert!(tail.y > viewport.y + viewport.height);
    let delete = |positions: &Geometry| {
        positions.labels.iter().rev().find(|(label, _)| label == "Delete").unwrap().1
    };
    let initial = capture(
        &mut ui,
        &mut renderer,
        &Theme::Dark,
        size,
        "skeys-list",
        mouse::Cursor::Unavailable,
    );
    assert!(
        send(
            &mut ui,
            &mut renderer,
            mouse::Cursor::Available(viewport.center()),
            &[Event::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -900.0 }
            })],
        )
        .is_empty()
    );
    let positions = geometry(&mut ui, &renderer);
    let (viewport, translation) = positions.scroll.unwrap();
    let point = delete(&positions).center() - translation;
    assert!(viewport.contains(point));
    assert!(matches!(
        click(&mut ui, &mut renderer, point).as_slice(),
        [Message::Settings(SettingsMessage::Gateway(GatewayMessage::RemoveSkey(14)))]
    ));
    assert_ne!(
        initial,
        capture(
            &mut ui,
            &mut renderer,
            &Theme::Dark,
            size,
            "skeys-list-scrolled",
            mouse::Cursor::Unavailable
        )
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn help_rollout_18_renders_contextual_help() {
    use crate::app::components::system_settings::{self, SystemTab};
    use crate::app::components::system_settings_proxy::system_settings_proxy_tests::rollout::{
        MODES, Scene,
    };

    if !isolated("help_rollout_18_renders_contextual_help") {
        return;
    }
    let mut renderer = renderer().await;
    let mut app = fixture();
    app.show_system_settings = true;
    app.system_settings_tab = SystemTab::Gateway;
    app.gateway_settings.save_error = Some(SAVE_ERROR.into());
    app.gateway_settings.service_action_output = Some("Fixture: service status".into());

    for (locale, theme) in MODES {
        app.desktop_locale = locale;
        app.app_theme = theme;
        let t = |key| connection_text(locale, key);
        app.gateway_settings.active_tab = GatewaySettingsTab::Config;

        let mut settings = Scene::new(&app, &mut renderer, "gateway-settings", view(&app));
        settings.label(SAVE_ERROR);
        settings.label("Fixture: service status");
        settings.label(t("若 host 非本地地址，建议仅在可信反向代理或隧道环境下使用。"));
        for (title, description) in [
            (
                "服务端网关配置",
                "配置 Gateway 的监听地址、skey 鉴权、限流和实验性 node-control 行为。",
            ),
            ("服务管理", "通过本机 gateway 调用 shiju service 管理 OS 服务生命周期。"),
            ("主机地址", "控制服务端 Gateway 的监听地址。"),
            ("安全开关", "控制 skey 鉴权、公网绑定与反向代理头信任策略。"),
        ] {
            settings.help(t(title), t(description));
        }
        let port_help = t("控制服务端 Gateway 的监听端口。");
        let port_icon = settings.help(t("端口"), port_help);
        settings.actions(port_icon, port_help);
        assert!(matches!(
            settings.edit(&app.gateway_settings.host_input).as_slice(),
            [Message::Settings(SettingsMessage::Gateway(GatewayMessage::HostChanged(value)))]
                if Some(value.as_str()) == app.gateway_settings.host_input.strip_suffix('d')
        ));
        drop(settings);

        let size = Size::new(640, 1700);
        let mut shell = scene(system_settings::view(&app), &mut renderer, size);
        for (index, (title, description)) in [
            ("速率限制", "限制 /webhook 请求频率，并约束网关内部 key map 规模。"),
            ("Webhook速率限制/分钟", "限制 /webhook 每分钟可接受的请求数。"),
            ("速率限制最大键数", "约束网关内部 key map 规模。"),
            ("幂等性", "控制 webhook 幂等 key 的存活时间与内存上限。"),
            ("幂等性TTL(秒)", "控制 webhook 幂等 key 的存活时间。"),
            ("幂等性最大键数", "控制幂等缓存允许保留的 key 数量。"),
            ("Node Control", "配置实验性的 node-control 协议，包括额外鉴权和允许的远端节点 ID。"),
        ]
        .into_iter()
        .enumerate()
        {
            assert_help_frames(
                &mut shell,
                &mut renderer,
                &app.app_theme,
                size,
                &format!("gateway-shell-{locale:?}-{index}"),
                t(title),
                t(description),
            );
        }
        let skey_tab = geometry(&mut shell, &renderer).label(t("skey 管理")).center();
        assert!(matches!(
            click(&mut shell, &mut renderer, skey_tab).as_slice(),
            [Message::Settings(SettingsMessage::Gateway(GatewayMessage::TabSelected(
                GatewaySettingsTab::Skeys
            )))]
        ));
        drop(shell);

        app.gateway_settings.active_tab = GatewaySettingsTab::Skeys;
        let mut skeys = Scene::new(&app, &mut renderer, "gateway-skey", view(&app));
        skeys.help(t("skey 名称"), t("用于列表展示和识别该 skey。"));
        skeys.label(t("可手写 YYYY-MM-DD，也可点日历选择；为空表示永不过期。"));
        let generate = skeys.label(t("生成 skey")).center();
        assert!(matches!(
            skeys.click(generate).as_slice(),
            [Message::Settings(SettingsMessage::Gateway(GatewayMessage::AddSkey))]
        ));
    }
}

fn assert_help_frames(
    ui: &mut Ui<'_>,
    renderer: &mut Renderer,
    theme: &Theme,
    size: Size<u32>,
    name: &str,
    title: &str,
    description: &str,
) {
    let positions = geometry(ui, renderer);
    assert!(
        positions.labels.iter().all(|(text, _)| text != description),
        "{title}: static explanation is still inline"
    );
    let label = positions.label(title);
    let away = mouse::Cursor::Unavailable;
    let idle = capture(ui, renderer, theme, size, &format!("{name}-idle"), away);
    let icon = [
        iced::Point::new(label.x + label.width + 15.0, label.center_y()),
        iced::Point::new(label.x + 11.0, label.y + label.height + 15.0),
    ]
    .into_iter()
    .filter(|point| {
        iced::Rectangle::with_size(Size::new(size.width as f32, size.height as f32))
            .contains(*point)
    })
    .find(|&position| {
        let cursor = mouse::Cursor::Available(position);
        assert!(
            send(ui, renderer, cursor, &[Event::Mouse(mouse::Event::CursorMoved { position })])
                .is_empty()
        );
        capture(ui, renderer, theme, size, &format!("{name}-hover"), cursor) != idle
    })
    .unwrap_or_else(|| panic!("{title}: hovering beside the label must open help"));
    assert!(click(ui, renderer, icon).is_empty());
    assert!(
        capture(ui, renderer, theme, size, &format!("{name}-open"), away) != idle,
        "{title}: clicking must keep help open when the pointer leaves"
    );
    assert!(
        send(ui, renderer, away, &[key(iced::keyboard::key::Named::Escape)]).is_empty(),
        "{title}: Escape must be consumed by the help card"
    );
    assert!(
        capture(ui, renderer, theme, size, &format!("{name}-closed"), away) == idle,
        "{title}: Escape must restore the initial frame"
    );
}
