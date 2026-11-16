#![cfg(not(target_arch = "wasm32"))]
//! Channel form rendering and editing through the production domain update entry.
use crate::{
    basic_channels,
    channel_render_fixture::*,
    common::number_row,
    extended_channels,
    messages::{ChannelsMessage, Message, SettingsMessage},
    state::ChannelsSettingsTab,
    test_support::Fixture,
};
use config_ui::DesktopLocale;
use iced::advanced::renderer::{Headless, Style};
use iced::{Element, Event, Rectangle, Renderer, Size, Theme, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
fn isolated(test: &str) -> bool {
    const CHILD: &str = "SHIJU_CHANNEL_FORMS_CHILD";
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
fn update(app: &mut Fixture, message: ChannelsMessage) {
    let services = crate::update::Services {
        config: desktop_config::ConfigAccess::new(
            gateway_endpoint::GatewayEndpoint::new("127.0.0.1", 1),
            None,
        ),
        client: Err("fixture has no gateway".into()),
        project_choices: Vec::new(),
    };
    // Apply production state changes; do not execute persistence/network effects.
    drop(crate::update::updates::update(&mut app.channels_settings, &services, message));
}
fn fixture() -> Fixture {
    let mut app = Fixture::new().0;
    for channel in ["telegram", "webhook", "email", "nextcloud_talk"] {
        update(&mut app, ChannelsMessage::EnabledToggled(channel.into(), true));
    }
    let url = format!("https://example.invalid/{}", "webhook-segment/".repeat(20));
    for (key, value) in [
        ("telegram.bot_token", "fixture-token"),
        ("telegram.allowed_users", "fixture-one\n示例使用者\nfixture-two"),
        ("nextcloud_talk.base_url", url.as_str()),
        ("email.username", "fixture@example.invalid"),
        ("email.password", "fixture-password"),
        ("email.allowed_senders", "a@example.invalid\nb@example.invalid"),
    ] {
        update(&mut app, ChannelsMessage::TextChanged(key.into(), value.into()));
    }
    app.channels_settings.refresh_text_inputs();
    app
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
    let masked_secret = "•".repeat("fixture-webhook-key".len());
    {
        let size = Size::new(900, 1200);
        let mut ui = scene(basic_channels::webhook_panel(app.view()), &mut renderer, size);
        let geometry = geometry(&mut ui, &renderer);
        let label = geometry.label("Shared secret");
        let control = geometry.input(&masked_secret);
        assert!(label.x + label.width < control.x);
        assert!((label.center_y() - control.center_y()).abs() < 0.5);
        screenshot(
            &mut ui,
            &mut renderer,
            &Theme::Dark,
            size,
            "webhook-form-900",
            mouse::Cursor::Unavailable,
        );
    }

    let size = Size::new(640, 1200);
    let messages = {
        let mut ui = scene(
            crate::overview::view(
                app.view(),
                basic_channels::webhook_panel(app.view()),
                std::convert::identity,
            ),
            &mut renderer,
            size,
        );
        let initial = geometry(&mut ui, &renderer);
        let control = initial.input(&masked_secret);
        let label = initial.label("Shared secret");
        assert!(control.width >= 500.0);
        assert!(label.y + label.height < control.y);
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
        let mut ui = scene(extended_channels::email_panel(app.view()), &mut renderer, size);
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

    let mask_pixels = |app: &Fixture, renderer: &mut Renderer| {
        let mut ui = scene(extended_channels::email_panel(app.view()), renderer, size);
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
    let mut ui = scene(extended_channels::email_panel(app.view()), &mut renderer, size);
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
    let mut ui = scene(extended_channels::nextcloud_talk_panel(app.view()), &mut renderer, size);
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

    let mut ui = scene(basic_channels::telegram_panel(app.view()), &mut renderer, size);
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
    let fields = Layout::new(&node).children().next().unwrap();
    let row = fields.children().nth(1).unwrap();
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
