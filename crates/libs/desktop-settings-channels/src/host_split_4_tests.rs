#![cfg(not(target_arch = "wasm32"))]
//! Channel permission copy and form interactions without desktop App initialization.
use crate::{
    enterprise_channels, extended_channels,
    messages::{ChannelsMessage, Message, SettingsMessage},
    state::ChannelsSettingsTab,
    test_support::Fixture,
};
use config_ui::DesktopLocale;
use desktop_i18n::settings_extensions::text as tr;
use iced::advanced::{
    clipboard,
    renderer::{Headless, Style},
    widget::{
        Id, Operation,
        operation::{Focusable, TextInput},
    },
};
use iced::{Element, Event, Font, Rectangle, Renderer, Size, Theme, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
fn isolated(test: &str) -> bool {
    const CHILD: &str = "SHIJU_CHANNEL_PERMISSIONS_CHILD";
    if std::env::var_os(CHILD).is_some() {
        return true;
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", &format!("{}::{test}", module_path!().split_once("::").unwrap().1)])
        .env(CHILD, "1")
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
fn fixture() -> Fixture {
    let config = serde_json::from_value(serde_json::json!({
        "feishu": {"app_id":"fixture-app", "app_secret":"fixture-secret"},
        "dingtalk": {"client_id":"", "client_secret":""},
        "lark": {"app_id":"", "app_secret":""},
        "email": config_channel_services::EmailConfig::default()
    }))
    .unwrap();
    let mut app = Fixture::new().0;
    app.channels_settings = crate::app_init::settings_builders::build_channels_settings(&config);
    app
}
fn view(app: &Fixture) -> Element<'_, Message> {
    match app.channels_settings.active_tab {
        ChannelsSettingsTab::Feishu => enterprise_channels::feishu_panel(app.view()),
        ChannelsSettingsTab::DingTalk => enterprise_channels::dingtalk_panel(app.view()),
        ChannelsSettingsTab::Email => extended_channels::email_panel(app.view()),
        _ => unreachable!("permission fixture only selects tested panels"),
    }
}
async fn renderer() -> Renderer {
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in desktop_assets::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
        .await
        .unwrap()
}

fn screenshot(
    ui: &mut UserInterface<'_, Message, Theme, Renderer>,
    renderer: &mut Renderer,
    theme: &Theme,
    size: Size<u32>,
    name: &str,
    cursor: mouse::Cursor,
) -> Vec<u8> {
    let mut messages = Vec::new();
    ui.update(
        &[Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now()))],
        cursor,
        renderer,
        &mut clipboard::Null,
        &mut messages,
    );
    assert!(messages.iter().all(|message| matches!(message, Message::None)));
    ui.draw(renderer, theme, &Style { text_color: theme.palette().text }, cursor);
    let pixels = renderer.screenshot(size, 1.0, theme.palette().background);
    if let Some(directory) = std::env::var_os("SHIJU_UI_REFRESH_17_SCREENSHOTS") {
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

fn send(
    ui: &mut UserInterface<'_, Message, Theme, Renderer>,
    renderer: &mut Renderer,
    cursor: mouse::Cursor,
    events: &[Event],
) -> Vec<Message> {
    let mut messages = Vec::new();
    ui.update(events, cursor, renderer, &mut clipboard::Null, &mut messages);
    messages.retain(|message| !matches!(message, Message::None));
    messages
}

fn click(
    ui: &mut UserInterface<'_, Message, Theme, Renderer>,
    renderer: &mut Renderer,
    point: iced::Point,
) -> Vec<Message> {
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
    labels: Vec<(String, Rectangle)>,
    inputs: Vec<(String, Rectangle)>,
    focusable: Vec<Rectangle>,
}

impl Operation for Geometry {
    fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) {
        children(self);
    }

    fn text(&mut self, _: Option<&Id>, bounds: Rectangle, text: &str) {
        self.labels.push((text.into(), bounds));
    }

    fn text_input(&mut self, _: Option<&Id>, bounds: Rectangle, state: &mut dyn TextInput) {
        self.inputs.push((state.text().into(), bounds));
    }

    fn focusable(&mut self, _: Option<&Id>, bounds: Rectangle, _: &mut dyn Focusable) {
        self.focusable.push(bounds);
    }
}

impl Geometry {
    fn input(&self, value: &str) -> Rectangle {
        self.inputs.iter().find(|(text, _)| text == value).expect("missing fixture input").1
    }

    fn label(&self, name: &str) -> Rectangle {
        self.labels
            .iter()
            .find(|(label, _)| label == name)
            .unwrap_or_else(|| panic!("missing {name} in {:?}", self.labels))
            .1
    }
}

fn geometry(ui: &mut UserInterface<'_, Message, Theme, Renderer>, renderer: &Renderer) -> Geometry {
    let mut geometry = Geometry::default();
    ui.operate(renderer, &mut geometry);
    geometry
}

fn scene<'a>(
    element: Element<'a, Message>,
    renderer: &mut Renderer,
    size: Size<u32>,
) -> UserInterface<'a, Message, Theme, Renderer> {
    UserInterface::build(
        element,
        Size::new(size.width as f32, size.height as f32),
        Cache::new(),
        renderer,
    )
}

fn commit(value: &str) -> Event {
    Event::InputMethod(iced::advanced::input_method::Event::Commit(value.into()))
}

const HELP_GAP_06_PORT_HELP: &str = "WebSocket 模式会忽略端口。";
const HELP_GAP_06_PORT_HINT: &str = "Webhook 模式需要公网 HTTPS 入口。";
const HELP_GAP_06_STREAM: &str = "钉钉 Stream 模式鉴权。";
const HELP_GAP_06_EMPTY: &str = "白名单为空时拒绝所有用户。";
const HELP_GAP_06_USERS: &str = "* 表示允许所有用户。";
const HELP_GAP_06_SENDERS: &str = "* 表示允许所有发送者。";
const HELP_GAP_06_FORMAT: &str = "每行一个，亦支持逗号分隔，可使用 *";

fn help_gap_06_contexts() -> [(u32, Theme, DesktopLocale); 3] {
    [
        (420, Theme::Light, DesktopLocale::SimplifiedChinese),
        (960, Theme::Dark, DesktopLocale::TraditionalChinese),
        (420, Theme::Dark, DesktopLocale::English),
    ]
}

fn assert_channel_text_absent(
    positions: &Geometry,
    locale: DesktopLocale,
    explanations: &[&'static str],
) {
    for explanation in explanations {
        let localized = tr(locale, explanation);
        assert!(
            positions.labels.iter().all(|(label, _)| !label.contains(localized)),
            "{locale:?}: {localized} is visible in the closed form"
        );
    }
}

fn snapshot_panel(
    ui: &mut UserInterface<'_, Message, Theme, Renderer>,
    renderer: &mut Renderer,
    theme: &Theme,
    size: Size<u32>,
) {
    screenshot(ui, renderer, theme, size, "permissions", mouse::Cursor::Unavailable);
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn settings_help_gap_06_feishu_modes_keep_port_and_qr_controls() {
    if !isolated("settings_help_gap_06_feishu_modes_keep_port_and_qr_controls") {
        return;
    }
    use config_channel_services::LarkReceiveMode;
    use iced::keyboard::key::Named;

    let mut renderer = renderer().await;
    let mut app = fixture();
    for (width, theme, locale) in help_gap_06_contexts() {
        app.desktop_locale = locale;
        app.app_theme = theme.clone();
        app.channels_settings.active_tab = ChannelsSettingsTab::Feishu;
        for mode in [LarkReceiveMode::Webhook, LarkReceiveMode::Websocket] {
            let cfg = app.channels_settings.feishu.as_mut().unwrap();
            cfg.receive_mode = mode;
            cfg.port = Some(4321);
            let size = Size::new(width, 2000);
            let mut ui = scene(view(&app), &mut renderer, size);
            snapshot_panel(&mut ui, &mut renderer, &theme, size);
            let positions = geometry(&mut ui, &renderer);
            assert_channel_text_absent(
                &positions,
                locale,
                &[
                    "Webhook 模式需要公网 HTTPS 入口；WebSocket 模式会忽略 port。",
                    HELP_GAP_06_PORT_HELP,
                ],
            );
            let hint = positions.label(tr(locale, HELP_GAP_06_PORT_HINT));
            let port = positions.input("4321").center();
            assert!(hint.y + hint.height < size.height as f32);
            assert!((hint.center_y() - port.y).abs() < 65.0);
            assert!(click(&mut ui, &mut renderer, port).is_empty());
            assert!(matches!(
                send(&mut ui, &mut renderer, mouse::Cursor::Available(port), &[key(Named::Backspace)]).as_slice(),
                [Message::Settings(SettingsMessage::Channels(ChannelsMessage::NumberChanged(key, 432)))]
                    if key == "feishu.port"
            ));
            positions.label(tr(locale, "使用手机端扫描二维码；授权成功后请明确保存。"));
            assert!(matches!(
                click(&mut ui, &mut renderer, positions.label(tr(locale, "获取二维码")).center()).as_slice(),
                [Message::Settings(SettingsMessage::Channels(ChannelsMessage::QrRequested(id)))]
                    if id == "feishu"
            ));
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn settings_help_gap_06_dingtalk_keeps_allowlist_consequences() {
    if !isolated("settings_help_gap_06_dingtalk_keeps_allowlist_consequences") {
        return;
    }
    let mut renderer = renderer().await;
    let mut app = fixture();
    for (width, theme, locale) in help_gap_06_contexts() {
        app.desktop_locale = locale;
        app.app_theme = theme.clone();
        for named in [false, true] {
            app.channels_settings.dingtalk_instances.clear();
            app.channels_settings.dingtalk = fixture().channels_settings.dingtalk;
            if named {
                app.channels_settings.dingtalk_instances.push(
                    config_channel_services::DingTalkInstanceConfig {
                        id: "ops".into(),
                        config: app.channels_settings.dingtalk.clone().unwrap(),
                    },
                );
                app.channels_settings.dingtalk = None;
                app.channels_settings.dingtalk_instances[0].id = "ops".into();
                app.channels_settings.refresh_text_inputs();
            }
            let size = Size::new(width, 1700);
            let mut ui = scene(view(&app), &mut renderer, size);
            snapshot_panel(&mut ui, &mut renderer, &theme, size);
            let positions = geometry(&mut ui, &renderer);
            assert_channel_text_absent(
                &positions,
                locale,
                &[
                    "钉钉 Stream 模式鉴权；白名单为空时拒绝所有用户，* 表示允许所有用户。",
                    HELP_GAP_06_STREAM,
                    "填写 * 表示允许所有用户，推荐使用。",
                    HELP_GAP_06_FORMAT,
                ],
            );
            positions.label(tr(locale, HELP_GAP_06_EMPTY));
            positions.label(tr(locale, HELP_GAP_06_USERS));
            let editor = positions.focusable.iter().find(|bounds| bounds.height == 92.0).unwrap();
            assert!(editor.width >= width as f32 - 150.0);
            click(&mut ui, &mut renderer, editor.center());
            let field =
                if named { "dingtalk_instances.0.allowed_users" } else { "dingtalk.allowed_users" };
            assert!(matches!(
                send(&mut ui, &mut renderer, mouse::Cursor::Available(editor.center()), &[commit("*")]).as_slice(),
                [Message::Settings(SettingsMessage::Channels(ChannelsMessage::TextEditorAction(key, iced::widget::text_editor::Action::Edit(_))))]
                    if key == field
            ));
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn settings_help_gap_06_ordinary_and_wildcard_lists_and_disabled_form() {
    if !isolated("settings_help_gap_06_ordinary_and_wildcard_lists_and_disabled_form") {
        return;
    }
    let mut renderer = renderer().await;
    let mut app = fixture();
    for (width, theme, locale) in help_gap_06_contexts() {
        app.desktop_locale = locale;
        app.app_theme = theme.clone();
        let size = Size::new(width, 1700);

        {
            let mut ui = scene(extended_channels::email_panel(app.view()), &mut renderer, size);
            snapshot_panel(&mut ui, &mut renderer, &theme, size);
            let positions = geometry(&mut ui, &renderer);
            assert_channel_text_absent(
                &positions,
                locale,
                &[HELP_GAP_06_SENDERS, HELP_GAP_06_FORMAT],
            );
            let label = positions.label(tr(locale, "允许发送者"));
            let icon = iced::Point::new(label.x + label.width + 15.0, label.center_y());
            let idle = screenshot(
                &mut ui,
                &mut renderer,
                &theme,
                size,
                "gap-06-email-list",
                mouse::Cursor::Unavailable,
            );
            assert!(click(&mut ui, &mut renderer, icon).is_empty());
            assert!(
                idle == screenshot(
                    &mut ui,
                    &mut renderer,
                    &theme,
                    size,
                    "gap-06-email-no-help",
                    mouse::Cursor::Unavailable
                )
            );
            let editor = positions.focusable.iter().find(|bounds| bounds.height == 92.0).unwrap();
            click(&mut ui, &mut renderer, editor.center());
            assert!(matches!(
                send(&mut ui, &mut renderer, mouse::Cursor::Available(editor.center()), &[commit("fixture@example.invalid")]).as_slice(),
                [Message::Settings(SettingsMessage::Channels(ChannelsMessage::TextEditorAction(key, iced::widget::text_editor::Action::Edit(_))))]
                    if key == "email.allowed_senders"
            ));
        }
        {
            let mut ui = scene(enterprise_channels::lark_panel(app.view()), &mut renderer, size);
            snapshot_panel(&mut ui, &mut renderer, &theme, size);
            let positions = geometry(&mut ui, &renderer);
            assert_channel_text_absent(
                &positions,
                locale,
                &["填写 * 表示允许所有发送者，推荐使用。", HELP_GAP_06_FORMAT],
            );
            positions.label(tr(locale, HELP_GAP_06_SENDERS));
        }

        app.channels_settings.active_tab = ChannelsSettingsTab::Email;
        app.channels_settings.email = None;
        let mut ui = scene(view(&app), &mut renderer, size);
        snapshot_panel(&mut ui, &mut renderer, &theme, size);
        let positions = geometry(&mut ui, &renderer);
        assert_channel_text_absent(&positions, locale, &[HELP_GAP_06_DISABLED, "允许发送者"]);
        drop(ui);
        let mut ui = scene(extended_channels::email_panel(app.view()), &mut renderer, size);
        snapshot_panel(&mut ui, &mut renderer, &theme, size);
        let enabled = geometry(&mut ui, &renderer).label(tr(locale, "启用")).center();
        assert!(matches!(
            click(&mut ui, &mut renderer, enabled).as_slice(),
            [Message::Settings(SettingsMessage::Channels(ChannelsMessage::EnabledToggled(id, true)))]
                if id == "email"
        ));
        drop(ui);
        app.channels_settings.email = fixture().channels_settings.email;
        app.channels_settings.refresh_text_inputs();
    }
}
