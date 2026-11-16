#![cfg(not(target_arch = "wasm32"))]

use super::*;
use crate::app::message::settings::{ChannelsMessage, SettingsMessage};
use config_ui::DesktopLocale;
use config_ui::GatewayClientSystemSettingsConfig;
use desktop_i18n::settings_extensions::text as tr;
use iced::advanced::{
    clipboard,
    renderer::{Headless, Style},
    widget::{
        Id, Operation,
        operation::{Focusable, TextInput},
    },
};
use iced::{Event, Font, Rectangle, Renderer, Size, Theme, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};

fn isolated(test: &str) -> bool {
    const CHILD: &str = "SHIJU_UI_REFRESH_17_CHILD";
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

fn fixture() -> App {
    App::new().0
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

fn enter_text(
    ui: &mut UserInterface<'_, Message, Theme, Renderer>,
    renderer: &mut Renderer,
    bounds: Rectangle,
    text: &str,
) -> Vec<Message> {
    use iced::keyboard::key::Named;
    let point = bounds.center();
    assert!(click(ui, renderer, point).is_empty());
    send(ui, renderer, mouse::Cursor::Available(point), &[key(Named::End), commit(text)])
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_17_gateway_client_edits_host_and_masked_key() {
    use crate::app::components::system_settings_gateway_client;
    use settings::GatewayClientMessage;

    if !isolated("ui_refresh_17_gateway_client_edits_host_and_masked_key") {
        return;
    }
    let mut renderer = renderer().await;
    let mut app = App::new().0;
    app.gateway_client_settings.name_input = "Fixture gateway".into();
    app.gateway_client_settings.host_input = "gateway.example.invalid".into();
    app.gateway_client_settings.platform_id_input = "10000".into();
    app.gateway_client_settings.company_id_input = "100000000".into();
    app.gateway_client_settings.skey_input = "test-key".into();

    for (width, theme, locale) in [
        (420, Theme::Light, DesktopLocale::SimplifiedChinese),
        (960, Theme::Dark, DesktopLocale::TraditionalChinese),
        (420, Theme::Dark, DesktopLocale::English),
    ] {
        app.desktop_locale = locale;
        let size = Size::new(width, 1100);
        let mut ui = scene(system_settings_gateway_client::view(&app), &mut renderer, size);
        let fields = geometry(&mut ui, &renderer);
        let host = fields.input("gateway.example.invalid");
        let secret = fields.input(&"•".repeat("test-key".len()));
        for bounds in [host, secret] {
            assert!(bounds.x >= 0.0 && bounds.x + bounds.width <= width as f32);
        }
        screenshot(
            &mut ui,
            &mut renderer,
            &theme,
            size,
            &format!("gateway-client-{locale:?}-{width}"),
            mouse::Cursor::Unavailable,
        );
        assert!(matches!(
            enter_text(&mut ui, &mut renderer, host, "x").as_slice(),
            [Message::Settings(SettingsMessage::GatewayClient(GatewayClientMessage::HostChanged(value)))]
                if value == "gateway.example.invalidx"
        ));
        assert!(matches!(
            enter_text(&mut ui, &mut renderer, secret, "x").as_slice(),
            [Message::Settings(SettingsMessage::GatewayClient(GatewayClientMessage::SkeyChanged(value)))]
                if value == "test-keyx"
        ));
    }

    let size = Size::new(420, 1100);
    let secret = {
        let mut ui = scene(system_settings_gateway_client::view(&app), &mut renderer, size);
        geometry(&mut ui, &renderer).input(&"•".repeat("test-key".len()))
    };
    app.gateway_client_settings.skey_input.clear();
    let mut ui = scene(system_settings_gateway_client::view(&app), &mut renderer, size);
    screenshot(
        &mut ui,
        &mut renderer,
        &Theme::Dark,
        size,
        "gateway-client-empty-key",
        mouse::Cursor::Unavailable,
    );
    assert!(matches!(
        enter_text(&mut ui, &mut renderer, secret, "new-key").as_slice(),
        [Message::Settings(SettingsMessage::GatewayClient(GatewayClientMessage::SkeyChanged(value)))]
            if value == "new-key"
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn settings_tabs_capabilities_appearance_channels() {
    use crate::app::components::system_settings_common::tab_appearance_tests;

    tab_appearance_tests::run_at_least(
        concat!(module_path!(), "::settings_tabs_capabilities_appearance_channels"),
        520,
        fixture,
        |scene, app| {
            app.channels_settings.active_tab = ChannelsSettingsTab::Global;
            app.channels_settings.save_error = Some("fixture channel error".into());
            let locale = app.desktop_locale;
            let webhook = scene.check_with_padding(
                app,
                view(app),
                &["全局", "Webhook", "Email"].map(|label| tr(locale, label)),
                1,
                12,
            );
            assert!(matches!(
                webhook,
                SettingsMessage::Channels(ChannelsMessage::TabSelected(
                    ChannelsSettingsTab::Webhook
                ))
            ));
            let _ = app.update(Message::Settings(webhook));
            assert_eq!(app.channels_settings.active_tab, ChannelsSettingsTab::Webhook);
            let labels = scene.labels(view(app));
            assert!(labels.contains("fixture channel error"));
            assert!(labels.contains(tr(locale, "配置")));

            let feishu = scene.click(view(app), tr(locale, "飞书"));
            assert!(matches!(
                feishu,
                SettingsMessage::Channels(ChannelsMessage::TabSelected(
                    ChannelsSettingsTab::Feishu
                ))
            ));
            let _ = app.update(Message::Settings(feishu));
            assert_eq!(app.channels_settings.active_tab, ChannelsSettingsTab::Feishu);
        },
    )
    .await;
}
