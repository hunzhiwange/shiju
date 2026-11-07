#![cfg(not(target_arch = "wasm32"))]

use super::*;
use crate::app::message::settings;
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
use vw_config_types::ui::GatewayClientSystemSettingsConfig;

const CHANNELS: [&str; 21] = [
    "telegram",
    "discord",
    "slack",
    "mattermost",
    "webhook",
    "imessage",
    "signal",
    "whatsapp",
    "linq",
    "wati",
    "nextcloud_talk",
    "email",
    "irc",
    "lark",
    "feishu",
    "dingtalk",
    "qq",
    "wecom",
    "wechat",
    "nostr",
    "clawdtalk",
];

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

fn update(app: &mut App, message: ChannelsMessage) {
    // State and validation run synchronously. The resulting persistence/network task is dropped.
    drop(settings::update(app, SettingsMessage::Channels(message)));
}

fn fixture() -> App {
    assert_eq!(crate::app::config::gateway_client_endpoint().normalized_host(), "[");
    let mut app = App::new().0;
    for channel in CHANNELS {
        update(&mut app, ChannelsMessage::EnabledToggled(channel.into(), true));
    }
    let long_url = format!("https://example.invalid/{}", "webhook-segment/".repeat(20));
    for (key, value) in [
        ("telegram.bot_token", "fixture-token"),
        ("telegram.base_url", long_url.as_str()),
        ("telegram.allowed_users", "fixture-one\n示例使用者\nfixture-two"),
        ("mattermost.url", long_url.as_str()),
        ("nextcloud_talk.base_url", long_url.as_str()),
        ("email.username", "fixture@example.invalid"),
        ("email.password", "fixture-password"),
        ("email.allowed_senders", "a@example.invalid\nb@example.invalid"),
        ("feishu.app_id", "fixture-app"),
        ("feishu.app_secret", "fixture-secret"),
    ] {
        update(&mut app, ChannelsMessage::TextChanged(key.into(), value.into()));
    }
    app.channels_settings.refresh_text_inputs();
    app.channels_settings.project_dir_input = format!("/示例專案/{}/workspace", "path-".repeat(25));
    app
}

fn screenshot(
    ui: &mut UserInterface<'_, Message, Theme, Renderer>,
    renderer: &mut Renderer,
    theme: &Theme,
    size: Size<u32>,
    name: &str,
    cursor: mouse::Cursor,
) {
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
}

fn snapshot(
    view: Element<'_, Message>,
    renderer: &mut Renderer,
    theme: &Theme,
    size: Size<u32>,
    name: &str,
) {
    let mut ui = UserInterface::build(
        view,
        Size::new(size.width as f32, size.height as f32),
        Cache::new(),
        renderer,
    );
    screenshot(&mut ui, renderer, theme, size, name, mouse::Cursor::Unavailable);
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_17_renders_channel_forms() {
    if !isolated("ui_refresh_17_renders_channel_forms") {
        return;
    }
    let mut renderer = renderer().await;
    let mut app = fixture();
    for (width, theme, locale, suffix) in [
        (640, Theme::Light, DesktopLocale::SimplifiedChinese, "light-zh"),
        (960, Theme::Dark, DesktopLocale::TraditionalChinese, "dark-tw"),
        (640, Theme::Dark, DesktopLocale::English, "dark-en"),
    ] {
        app.desktop_locale = locale;
        for channel in std::iter::once("global").chain(CHANNELS) {
            app.channels_settings.active_tab = ChannelsSettingsTab::from_channel_id(channel)
                .unwrap_or(ChannelsSettingsTab::Global);
            snapshot(
                view(&app),
                &mut renderer,
                &theme,
                Size::new(width, 2000),
                &format!("{channel}-{suffix}"),
            );
        }
    }
    app.channels_settings.active_tab = ChannelsSettingsTab::Webhook;
    app.channels_settings.save_error = Some("Fixture channel validation error".into());
    snapshot(
        view(&app),
        &mut renderer,
        &Theme::Dark,
        Size::new(640, 1200),
        "channel-validation-error",
    );
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

fn layout(
    element: &mut Element<'_, Message>,
    renderer: &Renderer,
    size: Size,
) -> iced::advanced::layout::Node {
    let mut tree = iced::advanced::widget::Tree::new(element.as_widget());
    element.as_widget_mut().layout(
        &mut tree,
        renderer,
        &iced::advanced::layout::Limits::new(Size::ZERO, size),
    )
}

#[tokio::test]
async fn ui_refresh_17_channel_card_text_fits() {
    let mut renderer = renderer().await;
    let descriptor = channel_tab_descriptors()
        .into_iter()
        .find(|descriptor| descriptor.tab == ChannelsSettingsTab::DingTalk)
        .unwrap();
    let mut card =
        china_channel_card(DesktopLocale::English, &descriptor, ChannelsSettingsTab::Global);
    // 640px content, page gutters and two gaps between three cards.
    let width = (640.0 - 24.0 - 24.0) / 3.0;
    let node = layout(&mut card, &renderer, Size::new(width, 120.0));
    let mut ui = UserInterface::build(card, Size::new(width, 120.0), Cache::new(), &mut renderer);
    let geometry = geometry(&mut ui, &renderer);
    for (label, text_size) in [("DingTalk", 15), ("Configuration", 12)] {
        let bounds = geometry.label(label);
        let text_width =
            layout(&mut text(label).size(text_size).into(), &renderer, Size::new(640.0, 120.0))
                .size()
                .width;
        assert!(bounds.x + text_width <= node.size().width - 8.0, "{label}: {bounds:?}");
    }
}

#[tokio::test]
async fn ui_refresh_17_layout_and_interaction() {
    use iced::keyboard::key::Named;
    let mut renderer = renderer().await;
    let mut field = common::pick_row(
        DesktopLocale::English,
        "群聊回复模式",
        "all_messages",
        &common::GROUP_REPLY_MODE_OPTIONS,
        "telegram.group_reply.mode",
    );
    let size = Size::new(420.0, 200.0);
    let node = layout(&mut field, &renderer, size);
    let trigger = iced::advanced::Layout::new(&node).children().nth(1).unwrap();
    for item in trigger.children().next().unwrap().children() {
        assert!((item.bounds().center_y() - trigger.bounds().center_y()).abs() < 0.5);
    }
    let point = trigger.bounds().center();
    let cursor = mouse::Cursor::Available(point);
    let mut ui = UserInterface::build(field, size, Cache::new(), &mut renderer);
    assert!(click(&mut ui, &mut renderer, point).is_empty());
    let menu_y = trigger.bounds().y + trigger.bounds().height + 12.0;
    let screenshot_size = Size::new(420, 200);
    for (index, theme) in [Theme::Light, Theme::Dark].into_iter().enumerate() {
        for row in 0..2 {
            let position = iced::Point::new(point.x, menu_y + 18.0 + 38.0 * row as f32);
            let hover = mouse::Cursor::Available(position);
            assert!(
                send(
                    &mut ui,
                    &mut renderer,
                    hover,
                    &[Event::Mouse(mouse::Event::CursorMoved { position })],
                )
                .is_empty()
            );
            screenshot(
                &mut ui,
                &mut renderer,
                &theme,
                screenshot_size,
                &format!("select-{index}-hover-{row}"),
                hover,
            );
        }
        assert!(
            send(
                &mut ui,
                &mut renderer,
                mouse::Cursor::Unavailable,
                &[Event::Mouse(mouse::Event::CursorLeft)]
            )
            .is_empty()
        );
        screenshot(
            &mut ui,
            &mut renderer,
            &theme,
            screenshot_size,
            &format!("select-{index}-left"),
            mouse::Cursor::Unavailable,
        );
    }
    // Reopening through the keyboard starts navigation at the current selection.
    assert!(send(&mut ui, &mut renderer, cursor, &[key(Named::Escape)]).is_empty());
    assert!(send(&mut ui, &mut renderer, cursor, &[key(Named::Enter)]).is_empty());
    let messages =
        send(&mut ui, &mut renderer, cursor, &[key(Named::ArrowDown), key(Named::Enter)]);
    assert!(
        matches!(
            messages.as_slice(),
            [Message::Settings(SettingsMessage::Channels(ChannelsMessage::ReceiveModeChanged(id, value)))]
                if id == "telegram.group_reply.mode" && value == "mention_only"
        ),
        "{messages:?}"
    );
    assert!(send(&mut ui, &mut renderer, cursor, &[key(Named::Enter)]).is_empty());
    assert!(send(&mut ui, &mut renderer, cursor, &[key(Named::Escape)]).is_empty());
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
async fn ui_refresh_17_preserves_values_and_disabled_states() {
    if !isolated("ui_refresh_17_preserves_values_and_disabled_states") {
        return;
    }
    use iced::keyboard::key::Named;
    let mut renderer = renderer().await;
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::English;
    app.channels_settings.active_tab = ChannelsSettingsTab::Webhook;
    app.channels_settings.webhook.as_mut().unwrap().secret = Some("fixture-webhook-key".into());

    let size = Size::new(640, 1200);
    let messages = {
        let mut ui = scene(view(&app), &mut renderer, size);
        let initial = geometry(&mut ui, &renderer);
        assert!(initial.input(&"•".repeat("fixture-webhook-key".len())).width >= 500.0);
        let email = initial.label("Email").center();
        click(&mut ui, &mut renderer, email)
    };
    assert!(
        matches!(
            messages.as_slice(),
            [Message::Settings(SettingsMessage::Channels(ChannelsMessage::TabSelected(
                ChannelsSettingsTab::Email
            )))]
        ),
        "{messages:?}"
    );
    update(&mut app, ChannelsMessage::TabSelected(ChannelsSettingsTab::Email));

    let size = Size::new(420, 1200);
    let messages = {
        let mut ui = scene(extended_channels::email_panel(&app), &mut renderer, size);
        let bounds = geometry(&mut ui, &renderer).input(&"•".repeat("fixture-password".len()));
        assert!(bounds.width >= 350.0);
        screenshot(
            &mut ui,
            &mut renderer,
            &Theme::Dark,
            size,
            "email-form-420",
            mouse::Cursor::Unavailable,
        );
        enter_text(&mut ui, &mut renderer, bounds, "x")
    };
    let [Message::Settings(SettingsMessage::Channels(ChannelsMessage::TextChanged(field, value)))] =
        messages.as_slice()
    else {
        panic!("expected one password edit, got {messages:?}");
    };
    assert_eq!(field, "email.password");
    assert_eq!(value, "fixture-passwordx");
    update(&mut app, ChannelsMessage::TextChanged(field.clone(), value.clone()));
    assert_eq!(
        app.channels_settings.webhook.as_ref().unwrap().secret.as_deref(),
        Some("fixture-webhook-key")
    );

    let mask_pixels = |app: &App, renderer: &mut Renderer| {
        let mut ui = scene(extended_channels::email_panel(app), renderer, size);
        ui.draw(
            renderer,
            &Theme::Light,
            &Style { text_color: Theme::Light.palette().text },
            mouse::Cursor::Unavailable,
        );
        renderer.screenshot(size, 1.0, Theme::Light.palette().background)
    };
    let masked = mask_pixels(&app, &mut renderer);
    update(
        &mut app,
        ChannelsMessage::TextChanged("email.password".into(), "Z".repeat(value.len())),
    );
    assert!(masked == mask_pixels(&app, &mut renderer), "password leaked through rendering");
    update(&mut app, ChannelsMessage::TextChanged("email.password".into(), "short".into()));
    assert!(masked != mask_pixels(&app, &mut renderer), "password field was absent");

    update(&mut app, ChannelsMessage::EnabledToggled("email".into(), false));
    let mut ui = scene(extended_channels::email_panel(&app), &mut renderer, size);
    let disabled = geometry(&mut ui, &renderer);
    assert!(disabled.inputs.is_empty() && disabled.focusable.is_empty());
    assert!(
        send(
            &mut ui,
            &mut renderer,
            mouse::Cursor::Unavailable,
            &[key(Named::Enter), key(Named::Escape), commit("stale")]
        )
        .is_empty()
    );
    let point = disabled.label("Enabled").center();
    assert!(matches!(
        click(&mut ui, &mut renderer, point).as_slice(),
        [Message::Settings(SettingsMessage::Channels(ChannelsMessage::EnabledToggled(id, true)))]
            if id == "email"
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_17_edits_long_inputs_and_number_controls() {
    if !isolated("ui_refresh_17_edits_long_inputs_and_number_controls") {
        return;
    }
    use iced::advanced::Layout;
    use iced::keyboard::key::Named;
    let mut renderer = renderer().await;
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::TraditionalChinese;
    let size = Size::new(420, 1000);
    let mut ui = scene(extended_channels::nextcloud_talk_panel(&app), &mut renderer, size);
    let url = &app.channels_settings.nextcloud_talk.as_ref().unwrap().base_url;
    let bounds = geometry(&mut ui, &renderer).input(url);
    assert!(bounds.width >= 350.0);
    assert!(matches!(
        enter_text(&mut ui, &mut renderer, bounds, "tail").as_slice(),
        [Message::Settings(SettingsMessage::Channels(ChannelsMessage::TextChanged(key, value)))]
            if key == "nextcloud_talk.base_url" && *value == format!("{url}tail")
    ));
    screenshot(
        &mut ui,
        &mut renderer,
        &Theme::Light,
        size,
        "webhook-url-420",
        mouse::Cursor::Available(bounds.center()),
    );

    let mut ui = scene(basic_channels::telegram_panel(&app), &mut renderer, size);
    let geometry = geometry(&mut ui, &renderer);
    let editor = geometry.focusable.iter().find(|bounds| bounds.height == 92.0).unwrap();
    assert!(editor.width >= 350.0);
    let cursor = mouse::Cursor::Available(editor.center());
    let focus = click(&mut ui, &mut renderer, editor.center());
    assert!(focus.iter().all(|message| matches!(
        message, Message::Settings(SettingsMessage::Channels(ChannelsMessage::TextEditorAction(
            key, action
        ))) if key == "telegram.allowed_users" && !matches!(action, iced::widget::text_editor::Action::Edit(_))
    )), "{focus:?}");
    let edited = send(&mut ui, &mut renderer, cursor, &[commit("示例")]);
    assert!(
        matches!(
            edited.as_slice(),
            [Message::Settings(SettingsMessage::Channels(ChannelsMessage::TextEditorAction(
                key, iced::widget::text_editor::Action::Edit(_)
            )))] if key == "telegram.allowed_users"
        ),
        "{edited:?}"
    );
    screenshot(&mut ui, &mut renderer, &Theme::Dark, size, "allowlist-420", cursor);

    let mut field = number_row(DesktopLocale::English, "Port", 8787, 1, 65535, "", "webhook.port");
    let limits = Size::new(420.0, 140.0);
    let node = layout(&mut field, &renderer, limits);
    let row = Layout::new(&node).children().nth(1).unwrap();
    let [input, badge]: [_; 2] =
        row.children().map(|node| node.bounds()).collect::<Vec<_>>().try_into().unwrap();
    assert!((input.center_y() - badge.center_y()).abs() < 0.5);
    let mut ui = UserInterface::build(field, limits, Cache::new(), &mut renderer);
    let point = input.center();
    let cursor = mouse::Cursor::Available(point);
    assert!(click(&mut ui, &mut renderer, point).is_empty());
    let messages = send(&mut ui, &mut renderer, cursor, &[key(Named::Backspace)]);
    assert!(
        matches!(
            messages.as_slice(),
            [Message::Settings(SettingsMessage::Channels(ChannelsMessage::NumberChanged(key, 878)))]
                if key == "webhook.port"
        ),
        "{messages:?}"
    );
    assert!(
        send(&mut ui, &mut renderer, cursor, &[key(Named::Escape), key(Named::Enter)]).is_empty()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_17_preserves_project_binding_and_qr_actions() {
    if !isolated("ui_refresh_17_preserves_project_binding_and_qr_actions") {
        return;
    }
    use vw_gateway_client::ChannelQrStatusDto;
    let mut renderer = renderer().await;
    let mut app = fixture();
    let size = Size::new(420, 400);
    for channel in CHANNELS {
        let instance = format!("{channel}:default");
        let state = &mut app.channels_settings;
        let generation = state.begin_project_choices("tenant", &instance);
        assert!(state.select_default_project("tenant", &instance, Some("project-1")).is_err());
        assert!(state.finish_project_choices(
            "tenant",
            &instance,
            generation,
            Ok(vec![("project-1".into(), "示例專案".into())]),
        ));
        assert!(state.select_default_project("tenant", &instance, Some("unknown")).is_err());
        state.select_default_project("tenant", &instance, Some("project-1")).unwrap();
        assert_eq!(state.instance_default_projects[&instance], "project-1");
    }

    app.desktop_locale = DesktopLocale::TraditionalChinese;
    let instance = "feishu:default";
    let state = &mut app.channels_settings;
    let generation = state.begin_project_choices("tenant", instance);
    snapshot(
        common::default_project_selector(&app, instance.into()),
        &mut renderer,
        &Theme::Light,
        size,
        "projects-loading",
    );
    app.channels_settings.finish_project_choices(
        "tenant",
        instance,
        generation,
        Err("fixture error".into()),
    );
    snapshot(
        common::default_project_selector(&app, instance.into()),
        &mut renderer,
        &Theme::Light,
        size,
        "projects-error",
    );
    let generation = app.channels_settings.begin_project_choices("tenant", instance);
    app.channels_settings.finish_project_choices("tenant", instance, generation, Ok(Vec::new()));
    snapshot(
        common::default_project_selector(&app, instance.into()),
        &mut renderer,
        &Theme::Dark,
        size,
        "projects-empty",
    );
    let projects = (1..=30)
        .map(|i| (format!("project-{i}"), format!("示例專案 {i} · {}", "很長的顯示名稱".repeat(6))))
        .collect::<Vec<_>>();
    let generation = app.channels_settings.begin_project_choices("tenant", instance);
    app.channels_settings.finish_project_choices(
        "tenant",
        instance,
        generation,
        Ok(projects.clone()),
    );

    {
        use iced::{Point, advanced::Layout};
        let mut field = common::default_project_selector(&app, instance.into());
        let bounds = Size::new(420.0, 400.0);
        let node = layout(&mut field, &renderer, bounds);
        let field_layout = Layout::new(&node).children().next().unwrap();
        let picker = field_layout.children().nth(1).unwrap().children().next().unwrap().bounds();
        let mut ui = UserInterface::build(field, bounds, Cache::new(), &mut renderer);
        let messages = click(&mut ui, &mut renderer, picker.center());
        assert!(
            matches!(
                messages.as_slice(),
                [Message::Settings(SettingsMessage::Channels(ChannelsMessage::ProjectChoicesRequested(id)))]
                    if id == instance
            ),
            "{messages:?}"
        );
        let cursor = mouse::Cursor::Available(Point::new(200.0, picker.y + picker.height + 65.0));
        let press = Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
        assert!(send(&mut ui, &mut renderer, cursor, &[press]).is_empty());
        let cache = ui.into_cache();
        let generation = app.channels_settings.begin_project_choices("tenant", instance);
        let ui = UserInterface::build(
            common::default_project_selector(&app, instance.into()),
            bounds,
            cache,
            &mut renderer,
        );
        let cache = ui.into_cache();
        assert!(app.channels_settings.finish_project_choices(
            "tenant",
            instance,
            generation,
            Ok(projects),
        ));
        let mut ui = UserInterface::build(
            common::default_project_selector(&app, instance.into()),
            bounds,
            cache,
            &mut renderer,
        );
        let release = Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left));
        assert!(send(&mut ui, &mut renderer, cursor, &[release]).is_empty());
        let scroll = Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Lines { x: 0.0, y: -8.0 },
        });
        assert!(send(&mut ui, &mut renderer, cursor, &[scroll]).is_empty());
        screenshot(&mut ui, &mut renderer, &Theme::Dark, size, "projects-menu-scrolled", cursor);
        use iced::keyboard::key::Named;
        let messages = send(
            &mut ui,
            &mut renderer,
            mouse::Cursor::Unavailable,
            &[key(Named::End), key(Named::Enter)],
        );
        assert!(
            matches!(
                messages.as_slice(),
                [Message::Settings(SettingsMessage::Channels(ChannelsMessage::DefaultProjectSelected {
                    instance: id, project: Some(project),
                }))] if id == instance && project == "project-30"
            ),
            "{messages:?}"
        );
        assert!(matches!(
            click(&mut ui, &mut renderer, picker.center()).as_slice(),
            [Message::Settings(SettingsMessage::Channels(ChannelsMessage::ProjectChoicesRequested(id)))]
                if id == instance
        ));
        let mut other = UserInterface::build(
            common::default_project_selector(&app, "qq:default".into()),
            bounds,
            ui.into_cache(),
            &mut renderer,
        );
        assert!(
            send(
                &mut other,
                &mut renderer,
                mouse::Cursor::Unavailable,
                &[key(Named::End), key(Named::Enter)]
            )
            .is_empty()
        );
    }

    app.desktop_locale = DesktopLocale::English;
    let size = Size::new(420, 1900);
    app.channels_settings.qr_flow.channel = Some("feishu".into());
    for (status, name) in [(None, "qr-loading"), (Some(ChannelQrStatusDto::Failed), "qr-failed")] {
        app.channels_settings.qr_flow.status = status;
        let mut ui = scene(enterprise_channels::feishu_panel(&app), &mut renderer, size);
        let geometry = geometry(&mut ui, &renderer);
        assert!(
            !geometry
                .labels
                .iter()
                .any(|(label, _)| label == tr(app.desktop_locale, "保存授权配置"))
        );
        screenshot(&mut ui, &mut renderer, &Theme::Dark, size, name, mouse::Cursor::Unavailable);
    }
    app.channels_settings.qr_flow.status = Some(ChannelQrStatusDto::Authorized);
    app.channels_settings.qr_flow.credentials_ready = true;
    // Match the displayed size so tiny-skia does not round the offset in scaled image pixels.
    let pixels = (0..220)
        .flat_map(|y| {
            (0..220).flat_map(move |x| {
                let shade = ((x / 22 + y / 22) % 2) as u8 * 255;
                [shade, shade, shade, 255]
            })
        })
        .collect::<Vec<_>>();
    app.channels_settings.qr_flow.qrcode_image =
        Some(iced::widget::image::Handle::from_rgba(220, 220, pixels));
    let mut ui = scene(enterprise_channels::feishu_panel(&app), &mut renderer, size);
    let geometry = geometry(&mut ui, &renderer);
    screenshot(
        &mut ui,
        &mut renderer,
        &Theme::Light,
        size,
        "qr-authorized",
        mouse::Cursor::Unavailable,
    );
    let close = geometry.label(tr(app.desktop_locale, "关闭")).center();
    let save = geometry.label(tr(app.desktop_locale, "保存授权配置")).center();
    assert!(matches!(
        click(&mut ui, &mut renderer, close).as_slice(),
        [Message::Settings(SettingsMessage::Channels(ChannelsMessage::QrClosed))]
    ));
    assert!(matches!(
        click(&mut ui, &mut renderer, save).as_slice(),
        [Message::Settings(SettingsMessage::Channels(ChannelsMessage::QrSaveConfirmed))]
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
