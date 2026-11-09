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
    const CHILD: &str = "SHIJU_UI_REFRESH_22_CHILD";
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
    if let Some(directory) = std::env::var_os("SHIJU_UI_REFRESH_22_SCREENSHOTS") {
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

fn fixture() -> App {
    assert_eq!(crate::app::config::gateway_client_endpoint().normalized_host(), "[");
    let mut app = App::new().0;
    app.autonomy_settings.allowed_commands = vec!["fixture-tool --inspect".into()];
    app.autonomy_settings.forbidden_paths = vec![
            "/workspace/虚构/測試/long-fixture-directory/another-long-folder/deny-only-this-root"
                .into();
            12
        ];
    app.agents_settings.available_tools =
        (0..28).map(|index| format!("fixture_tool_{index:02}")).collect();
    app.security_settings.sandbox_enabled_input = "true".into();
    app.security_settings.sandbox_backend_input = "firejail".into();
    app.security_settings.bash.enabled = false;
    app.security_settings.otp_gated_domains = vec!["fixture.example".into()];
    app.security_settings.audit_log_path =
        "/workspace/虚构/測試/long-fixture-directory/audit.log".into();
    app
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_22_layout_and_interaction() {
    use crate::app::components::system_settings_security as security;
    use crate::app::state::SecuritySettingsTab;

    if !isolated("ui_refresh_22_layout_and_interaction") {
        return;
    }
    let mut renderer = renderer().await;
    let mut app = fixture();
    for (locale, theme, width, tag) in [
        (DesktopLocale::SimplifiedChinese, Theme::Light, 640, "zh-cn-light"),
        (DesktopLocale::TraditionalChinese, Theme::Dark, 640, "zh-tw-dark"),
        (DesktopLocale::English, Theme::Dark, 960, "en-dark"),
    ] {
        app.desktop_locale = locale;
        for (tab, name, height) in [
            (AutonomySettingsTab::Permissions, "permissions", 780),
            (AutonomySettingsTab::Budget, "budget", 420),
            (AutonomySettingsTab::ToolsAndPaths, "tools", 1500),
            (AutonomySettingsTab::NonCli, "non-cli", 740),
        ] {
            app.autonomy_settings.active_tab = tab;
            app.autonomy_settings.list_editor.request_remove(AutonomyListKind::AllowedCommands, 0);
            snapshot(
                view(&app),
                &mut renderer,
                &theme,
                Size::new(width, height),
                &format!("{name}-{tag}"),
            );
        }
        for (tab, name, height) in [
            (SecuritySettingsTab::Bash, "bash", 2250),
            (SecuritySettingsTab::SandboxAndResources, "sandbox", 1100),
            (SecuritySettingsTab::Audit, "audit", 620),
            (SecuritySettingsTab::OtpAndEstop, "otp-estop", 1570),
            (SecuritySettingsTab::Syscall, "syscall", 1200),
            (SecuritySettingsTab::Guard, "guard", 640),
        ] {
            app.security_settings.active_tab = tab;
            snapshot(
                security::view(&app),
                &mut renderer,
                &theme,
                Size::new(width, height),
                &format!("{name}-{tag}"),
            );
        }
        app.autonomy_settings.show_help_modal = true;
        app.security_settings.show_help_modal = true;
        snapshot(
            view_overlays(&app, view(&app)),
            &mut renderer,
            &theme,
            Size::new(420, 560),
            &format!("autonomy-help-{tag}"),
        );
        snapshot(
            security::view_overlays(&app, security::view(&app)),
            &mut renderer,
            &theme,
            Size::new(420, 560),
            &format!("security-help-{tag}"),
        );
        app.autonomy_settings.show_help_modal = false;
        app.security_settings.show_help_modal = false;
    }
    use crate::app::components::system_settings::{self, SystemTab};
    app.desktop_locale = DesktopLocale::English;
    app.show_system_settings = true;
    app.system_settings_tab = SystemTab::Security;
    app.security_settings.active_tab = SecuritySettingsTab::SandboxAndResources;
    app.security_settings.sandbox_enabled_input = "false".into();
    app.security_settings.sandbox_backend_input = "none".into();
    snapshot(
        system_settings::view(&app),
        &mut renderer,
        &Theme::Light,
        Size::new(640, 800),
        "security-narrow-window",
    );
    let mut narrow = scene(system_settings::view(&app), &mut renderer, Size::new(640, 800));
    let positions = geometry(&mut narrow, &renderer);
    let hint =
        positions.label("Choose automatic detection, forced activation or forced deactivation.");
    assert!(
        hint.height <= 85.0,
        "narrow layout should fit the policy hint in five lines: {hint:?}"
    );
    drop(narrow);
    app.system_settings_tab = SystemTab::Autonomy;
    app.autonomy_settings.active_tab = AutonomySettingsTab::Permissions;
    snapshot(
        system_settings::view(&app),
        &mut renderer,
        &Theme::Dark,
        Size::new(640, 800),
        "autonomy-narrow-window",
    );

    app.desktop_locale = DesktopLocale::SimplifiedChinese;
    let mut ui = scene(security::view(&app), &mut renderer, Size::new(640, 1100));
    let positions = geometry(&mut ui, &renderer);
    positions.assert_centered(
        positions.focusable[0],
        "沙箱启用",
        "选择自动检测、强制启用或强制禁用沙箱。",
    );
    positions.assert_centered(
        positions.focusable[1],
        "沙箱后端",
        "根据启用策略收窄可选值，避免互相矛盾的配置。",
    );
    drop(ui);
    app.security_settings.active_tab = SecuritySettingsTab::Audit;
    let mut ui = scene(security::view(&app), &mut renderer, Size::new(640, 620));
    let positions = geometry(&mut ui, &renderer);
    let input = positions.input(&app.security_settings.audit_log_path);
    let label = positions.label("审计日志路径");
    assert!((input.center_y() - label.center_y()).abs() < 0.5);
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
    focusable: Vec<iced::Rectangle>,
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
        self.focusable.push(bounds);
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

    fn assert_centered(&self, control: iced::Rectangle, label: &str, hint: &str) {
        let hint = self.label(hint);
        let center = (self.label(label).y + hint.y + hint.height) / 2.0;
        assert!((control.center_y() - center).abs() < 0.5, "{label}: {control:?}, center={center}");
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

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_22_sandbox_select_keeps_policy_values_from_keyboard() {
    use crate::app::components::system_settings::{self, SELECT_FOCUS_SCOPE, SystemTab};
    use crate::app::message::SettingsMessage;
    use crate::app::state::SecuritySettingsTab;
    use iced::advanced::widget::{
        Id,
        operation::{Operation, Outcome, focusable, scope},
    };
    use iced::keyboard::key::Named;

    if !isolated("ui_refresh_22_sandbox_select_keeps_policy_values_from_keyboard") {
        return;
    }
    let mut renderer = renderer().await;
    let mut app = fixture();
    app.show_system_settings = true;
    app.system_settings_tab = SystemTab::Security;
    app.security_settings.active_tab = SecuritySettingsTab::SandboxAndResources;
    let mut ui = scene(system_settings::view(&app), &mut renderer, Size::new(960, 800));
    // The first Tab focuses navigation search; the second reaches the current form.
    for _ in 0..2 {
        let mut focus = scope(Id::from(SELECT_FOCUS_SCOPE), focusable::focus_next::<()>());
        ui.operate(&renderer, &mut focus);
        let Outcome::Chain(mut next) = focus.finish() else {
            panic!("expected focus traversal to run in two passes");
        };
        ui.operate(&renderer, next.as_mut());
    }
    let cursor = mouse::Cursor::Unavailable;
    assert!(send(&mut ui, &mut renderer, cursor, &[key(Named::Enter)]).is_empty());
    assert!(send(&mut ui, &mut renderer, cursor, &[key(Named::ArrowDown)]).is_empty());
    assert!(send(&mut ui, &mut renderer, cursor, &[key(Named::Escape)]).is_empty());
    assert!(send(&mut ui, &mut renderer, cursor, &[key(Named::Enter)]).is_empty());
    assert!(send(&mut ui, &mut renderer, cursor, &[key(Named::ArrowDown)]).is_empty());
    assert!(
        matches!(
            send(&mut ui, &mut renderer, cursor, &[key(Named::Enter)]).as_slice(),
            [Message::Settings(SettingsMessage::SecuritySandboxEnabledChanged(value))] if value == "false"
        ),
        "keyboard must reach the sandbox policy select and submit its stable value"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_22_preserves_values_and_disabled_states() {
    use crate::app::components::system_settings_security as security;
    use crate::app::message::settings::{SettingsMessage, update};
    use crate::app::state::{SecurityListKind, SecuritySettingsTab};
    use iced::keyboard::key::Named;

    if !isolated("ui_refresh_22_preserves_values_and_disabled_states") {
        return;
    }
    let mut renderer = renderer().await;
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::SimplifiedChinese;
    app.autonomy_settings.level = AutonomyLevel::ReadOnly;
    app.autonomy_settings.active_tab = AutonomySettingsTab::Permissions;
    let mut ui = scene(view(&app), &mut renderer, Size::new(640, 780));
    let positions = geometry(&mut ui, &renderer);
    for (label, expected) in [
        ("监督", AutonomyLevel::Supervised),
        ("完全", AutonomyLevel::Full),
        ("只读", AutonomyLevel::ReadOnly),
    ] {
        assert!(matches!(
            click(&mut ui, &mut renderer, positions.label(label).center()).as_slice(),
            [Message::Settings(SettingsMessage::AutonomyLevelChanged(level))] if *level == expected
        ));
    }
    drop(ui);

    let allowed = AutonomyListKind::AllowedCommands;
    app.autonomy_settings.active_tab = AutonomySettingsTab::ToolsAndPaths;
    let size = Size::new(640, 1500);
    let mut ui = scene(view(&app), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    assert!(click(&mut ui, &mut renderer, positions.label("添加").center()).is_empty());
    let request = click(&mut ui, &mut renderer, positions.label("删除").center());
    let [
        Message::Settings(
            settings @ SettingsMessage::AutonomyListRemoveRequested(
                AutonomyListKind::AllowedCommands,
                0,
            ),
        ),
    ] = request.as_slice()
    else {
        panic!("remove must request confirmation with the original list and index");
    };
    drop(ui);
    assert_eq!(update(&mut app, settings.clone()).units(), 0);
    assert_eq!(app.autonomy_settings.allowed_commands, ["fixture-tool --inspect"]);
    let mut ui = scene(view(&app), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    let canceled = click(&mut ui, &mut renderer, positions.label("取消").center());
    assert!(matches!(
        canceled.as_slice(),
        [Message::Settings(SettingsMessage::AutonomyListRemoveCanceled)]
    ));
    assert!(matches!(
        click(&mut ui, &mut renderer, positions.label("确认删除").center()).as_slice(),
        [Message::Settings(SettingsMessage::AutonomyListRemoveConfirmed(
            AutonomyListKind::AllowedCommands,
            0
        ))]
    ));
    drop(ui);
    let Message::Settings(cancel) = canceled.into_iter().next().unwrap() else { unreachable!() };
    assert_eq!(update(&mut app, cancel).units(), 0);
    assert_eq!(app.autonomy_settings.list_editor.pending_remove_index(allowed), None);
    assert_eq!(app.autonomy_settings.allowed_commands, ["fixture-tool --inspect"]);
    assert_eq!(app.autonomy_settings.forbidden_paths.len(), 12);

    // Enter sends the original list action, but the validation path schedules no save.
    let env = AutonomyListKind::ShellEnvPassthrough;
    app.autonomy_settings.list_editor.set_draft(env, "invalid-name".into());
    let mut ui = scene(view(&app), &mut renderer, size);
    let input = geometry(&mut ui, &renderer).input("invalid-name").center();
    assert!(click(&mut ui, &mut renderer, input).is_empty());
    let messages =
        send(&mut ui, &mut renderer, mouse::Cursor::Available(input), &[key(Named::Enter)]);
    let [
        Message::Settings(
            settings @ SettingsMessage::AutonomyListAdd(AutonomyListKind::ShellEnvPassthrough),
        ),
    ] = messages.as_slice()
    else {
        panic!("Enter must use the same list action as Add");
    };
    drop(ui);
    let previous = app.autonomy_settings.list(env).to_vec();
    assert_eq!(update(&mut app, settings.clone()).units(), 0);
    assert_eq!(app.autonomy_settings.list(env), previous);
    assert!(app.autonomy_settings.save_error.is_some());
    snapshot(view(&app), &mut renderer, &Theme::Light, size, "invalid-autonomy-list");

    app.security_settings.active_tab = SecuritySettingsTab::Bash;
    app.security_settings.bash.policy.read_only = true;
    let mut ui = scene(security::view(&app), &mut renderer, Size::new(640, 2400));
    let positions = geometry(&mut ui, &renderer);
    let disabled: Vec<_> =
        positions.labels.iter().filter(|(label, _)| label == "已保存，当前不生效").collect();
    assert!(!disabled.is_empty());
    for (_, bounds) in disabled {
        assert!(click(&mut ui, &mut renderer, bounds.center()).is_empty());
    }
    drop(ui);
    assert!(app.security_settings.bash.policy.read_only);

    // An exact duplicate in a security list must not change the saved list or schedule a save.
    let domains = SecurityListKind::OtpGatedDomains;
    app.security_settings.active_tab = SecuritySettingsTab::OtpAndEstop;
    app.security_settings.list_editor.set_draft(domains, "fixture.example".into());
    let mut ui = scene(security::view(&app), &mut renderer, Size::new(640, 1600));
    let positions = geometry(&mut ui, &renderer);
    let point = positions.input("fixture.example").center();
    assert!(click(&mut ui, &mut renderer, point).is_empty());
    let messages =
        send(&mut ui, &mut renderer, mouse::Cursor::Available(point), &[key(Named::Enter)]);
    let [
        Message::Settings(
            settings @ SettingsMessage::SecurityListAdd(SecurityListKind::OtpGatedDomains),
        ),
    ] = messages.as_slice()
    else {
        panic!("expected the original OTP domain list action");
    };
    drop(ui);
    assert_eq!(update(&mut app, settings.clone()).units(), 0);
    assert_eq!(app.security_settings.otp_gated_domains, ["fixture.example"]);
    assert!(app.security_settings.save_error.is_some());
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_22_tool_menu_hover_scroll_and_add_keep_original_ids() {
    use crate::app::message::SettingsMessage;
    use iced::{Point, keyboard::key::Named};

    if !isolated("ui_refresh_22_tool_menu_hover_scroll_and_add_keep_original_ids") {
        return;
    }
    let mut renderer = renderer().await;
    let app = fixture();
    let locale = DesktopLocale::SimplifiedChinese;
    let kind = AutonomyListKind::AutoApprove;
    let size = Size::new(640, 460);
    let saved = ["fixture_tool_00".to_owned(), "fixture_tool_03".to_owned()];
    let editor = |draft| {
        list_editor(
            locale,
            kind,
            settings_field_label("自动审批").into(),
            Some("这些工具无需额外审批即可执行。"),
            &saved,
            draft,
            None,
            &app.agents_settings.available_tools,
        )
    };
    let mut ui = scene(editor("fixture_tool_01"), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    let trigger = positions.focusable[0];
    let add = positions.label("添加");
    assert!((trigger.center_y() - add.center_y()).abs() < 0.5);
    assert!(click(&mut ui, &mut renderer, trigger.center()).is_empty());

    let mut pixels = Vec::new();
    for (index, point) in [
        Point::new(trigger.center_x(), trigger.y + trigger.height + 30.0),
        Point::new(trigger.center_x(), trigger.y + trigger.height + 68.0),
        Point::new(2.0, 2.0),
    ]
    .into_iter()
    .enumerate()
    {
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
        pixels.push(capture(
            &mut ui,
            &mut renderer,
            &Theme::Dark,
            size,
            &format!("tool-menu-hover-{index}"),
            cursor,
        ));
    }
    assert_ne!(pixels[0], pixels[1]);
    assert_ne!(pixels[1], pixels[2]);
    let cursor =
        mouse::Cursor::Available(Point::new(trigger.center_x(), trigger.y + trigger.height + 68.0));
    assert!(
        send(
            &mut ui,
            &mut renderer,
            cursor,
            &[Event::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -192.0 },
            })]
        )
        .is_empty()
    );
    let scrolled =
        capture(&mut ui, &mut renderer, &Theme::Dark, size, "tool-menu-scrolled", cursor);
    assert_ne!(scrolled, pixels[1]);
    // Outside press dismisses the overlay and must not activate Add underneath it.
    assert!(click(&mut ui, &mut renderer, add.center()).is_empty());
    assert!(click(&mut ui, &mut renderer, trigger.center()).is_empty());
    let cursor = mouse::Cursor::Unavailable;
    assert!(send(&mut ui, &mut renderer, cursor, &[key(Named::End)]).is_empty());
    assert!(matches!(
        send(&mut ui, &mut renderer, cursor, &[key(Named::Enter)]).as_slice(),
        [Message::Settings(SettingsMessage::AutonomyListDraftChanged(AutonomyListKind::AutoApprove, id))]
            if id == "fixture_tool_27"
    ));
    drop(ui);

    let mut ui = scene(editor("fixture_tool_27"), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    for id in &saved {
        positions.label(id);
    }
    assert!(matches!(
        click(&mut ui, &mut renderer, positions.label("添加").center()).as_slice(),
        [Message::Settings(SettingsMessage::AutonomyListAdd(AutonomyListKind::AutoApprove))]
    ));
}

fn assert_contextual_help(
    app: &App,
    element: Element<'_, Message>,
    renderer: &mut Renderer,
    height: u32,
    persistent: &[&str],
    help: &[(&str, usize, &str)],
) {
    let locale = app.desktop_locale;
    let theme = &app.app_theme;
    let width = if locale == DesktopLocale::English { 680 } else { 480 };
    let size = Size::new(width, height);
    let mut ui = scene(element, renderer, size);
    let positions = geometry(&mut ui, renderer);
    for label in persistent {
        positions.label(label);
    }
    for &(title, occurrence, explanation) in help {
        assert!(
            positions.labels.iter().all(|(label, _)| label != explanation),
            "{title}: static explanation is visible before opening help"
        );
        let bounds = positions
            .labels
            .iter()
            .filter(|(label, _)| label == title)
            .nth(occurrence)
            .unwrap_or_else(|| panic!("missing help label {title}"))
            .1;
        let icon = iced::Point::new(bounds.x + bounds.width + 15.0, bounds.center_y());
        let prefix = format!("rollout-14-{locale:?}-{title}");
        let cursor = mouse::Cursor::Unavailable;
        let idle = capture(&mut ui, renderer, theme, size, &format!("{prefix}-idle"), cursor);
        let hover = mouse::Cursor::Available(icon);
        assert!(
            send(
                &mut ui,
                renderer,
                hover,
                &[Event::Mouse(mouse::Event::CursorMoved { position: icon })]
            )
            .is_empty()
        );
        let preview = capture(&mut ui, renderer, theme, size, &format!("{prefix}-hover"), hover);
        assert!(idle != preview, "{title}: hover preview must render");
        assert!(click(&mut ui, renderer, icon).is_empty());
        let open = capture(&mut ui, renderer, theme, size, &format!("{prefix}-open"), cursor);
        assert!(idle != open, "{title}: pinned help must render away from the cursor");

        if locale == DesktopLocale::English
            && title == crate::app::i18n::I18n::new(locale).__("自治配置")
        {
            assert_help_menu(&mut ui, renderer, icon, explanation);
        }

        assert!(
            send(&mut ui, renderer, cursor, &[key(iced::keyboard::key::Named::Escape)]).is_empty()
        );
        let closed = capture(&mut ui, renderer, theme, size, &format!("{prefix}-closed"), cursor);
        assert!(idle == closed, "{title}: Escape must restore the initial frame");
    }
}

fn assert_help_menu(
    ui: &mut Ui<'_>,
    renderer: &mut Renderer,
    icon: iced::Point,
    explanation: &str,
) {
    let body = iced::Point::new(icon.x + 29.0, icon.y + 5.0);
    let line_height = iced::advanced::text::LineHeight::default().to_absolute(12.0.into());
    let item_height = f32::from(line_height) + 10.0;
    let search = format!("https://www.google.com/search?q={}", urlencoding::encode(explanation));
    for (index, expected) in [
        (0.0, Message::CopyCode(explanation.into())),
        (3.0, Message::View(crate::app::message::ViewMessage::OpenUrlExternal(search))),
    ] {
        assert!(
            send(
                ui,
                renderer,
                mouse::Cursor::Available(body),
                &[Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right))],
            )
            .is_empty()
        );
        let item = iced::Point::new(body.x + 24.0, body.y + 3.0 + item_height * (index + 0.5));
        let messages = click(ui, renderer, item);
        assert!(
            match (messages.as_slice(), expected) {
                ([Message::CopyCode(value)], Message::CopyCode(expected)) => *value == expected,
                (
                    [Message::View(crate::app::message::ViewMessage::OpenUrlExternal(url))],
                    Message::View(crate::app::message::ViewMessage::OpenUrlExternal(expected)),
                ) => *url == expected,
                _ => false,
            },
            "unexpected contextual help action: {messages:?}"
        );
    }
}

fn assert_gap_11_help(
    app: &App,
    ui: &mut Ui<'_>,
    renderer: &mut Renderer,
    size: Size<u32>,
    title: &str,
    old_description: &str,
    body: &str,
) {
    let positions = geometry(ui, renderer);
    assert!(
        positions.labels.iter().all(|(label, _)| label != old_description),
        "{title}: static explanation is visible before opening help"
    );
    let bounds = positions.label(title);
    let prefix = format!("gap-11-{:?}-{}-{title}", app.desktop_locale, size.width);
    let theme = &app.app_theme;
    let cursor = mouse::Cursor::Unavailable;
    let idle = capture(ui, renderer, theme, size, &format!("{prefix}-idle"), cursor);
    let icon = [
        iced::Point::new(bounds.x + bounds.width + 15.0, bounds.center_y()),
        // The shared help label can wrap its icon below long labels.
        iced::Point::new(bounds.x + 11.0, bounds.y + bounds.height + 15.0),
    ]
    .into_iter()
    .find(|&icon| {
        let hover = mouse::Cursor::Available(icon);
        assert!(
            send(
                ui,
                renderer,
                hover,
                &[Event::Mouse(mouse::Event::CursorMoved { position: icon })]
            )
            .is_empty()
        );
        idle != capture(ui, renderer, theme, size, &format!("{prefix}-hover"), hover)
    })
    .unwrap_or_else(|| panic!("{title}: hover help did not render at {bounds:?}"));
    assert!(click(ui, renderer, icon).is_empty());
    assert!(idle != capture(ui, renderer, theme, size, &format!("{prefix}-pinned"), cursor));
    assert_eq!(geometry(ui, renderer).inputs, positions.inputs);

    let side = if icon.x < size.width as f32 / 2.0 { 29.0 } else { -29.0 };
    let point = iced::Point::new(icon.x + side, icon.y + 5.0);
    assert!(
        send(
            ui,
            renderer,
            mouse::Cursor::Available(point),
            &[Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right))]
        )
        .is_empty()
    );
    let line_height = iced::advanced::text::LineHeight::default().to_absolute(12.0.into());
    let item_height = f32::from(line_height) + 10.0;
    let menu_height = 6.0 + 5.0 * item_height;
    let top = if point.y + menu_height > size.height as f32 {
        (point.y - menu_height).max(0.0)
    } else {
        point.y
    };
    let item = iced::Point::new(
        point.x.min(size.width as f32 - 140.0) + 24.0,
        top + 3.0 + item_height * 0.5,
    );
    let messages = click(ui, renderer, item);
    assert!(
        matches!(messages.as_slice(), [Message::CopyCode(value)] if value == body),
        "{title}: unexpected help text: {messages:?}"
    );

    assert!(send(ui, renderer, cursor, &[key(iced::keyboard::key::Named::Escape)]).is_empty());
    assert!(idle == capture(ui, renderer, theme, size, &format!("{prefix}-closed"), cursor));
}

const GAP_11_LOCALE_THEMES: [(DesktopLocale, Theme); 3] = [
    (DesktopLocale::SimplifiedChinese, Theme::Light),
    (DesktopLocale::TraditionalChinese, Theme::Dark),
    (DesktopLocale::English, Theme::Dark),
];

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn settings_help_gap_11_permissions_explanations() {
    if !isolated("settings_help_gap_11_permissions_explanations") {
        return;
    }
    let mut app = fixture();
    app.autonomy_settings.active_tab = AutonomySettingsTab::Permissions;
    app.autonomy_settings.require_approval_for_medium_risk = false;
    app.autonomy_settings.block_high_risk_commands = false;
    let mut renderer = renderer().await;
    for (locale, theme) in GAP_11_LOCALE_THEMES {
        app.desktop_locale = locale;
        app.app_theme = theme;
        let t = |key| I18n::new(locale).__(key);
        for width in [420, 760] {
            let size = Size::new(width, 1100);
            let mut ui = scene(view(&app), &mut renderer, size);
            for (title, description) in [
                ("重定向策略", "控制 shell 命令中的输出重定向如何处理。"),
                ("级别", "决定代理默认可执行的动作范围。"),
                ("自然语言审批", "控制非 CLI 场景下自然语言请求的执行方式。"),
            ] {
                assert_gap_11_help(
                    &app,
                    &mut ui,
                    &mut renderer,
                    size,
                    t(title),
                    t(description),
                    t(description),
                );
            }
            let positions = geometry(&mut ui, &renderer);
            for hint in [
                "对中风险动作触发审批，而不是直接执行。",
                "对高风险命令直接阻断，不进入执行链路。",
                "仅允许在工作区路径内操作，除非显式加入 allowed_roots。",
            ] {
                positions.label(t(hint));
            }
            for (label, level) in [
                ("只读", AutonomyLevel::ReadOnly),
                ("监督", AutonomyLevel::Supervised),
                ("完全", AutonomyLevel::Full),
            ] {
                assert!(matches!(
                    click(&mut ui, &mut renderer, positions.label(t(label)).center()).as_slice(),
                    [Message::Settings(message::SettingsMessage::AutonomyLevelChanged(value))]
                        if *value == level
                ));
            }
            use crate::app::message::SettingsMessage;
            let mut click_label =
                |label| click(&mut ui, &mut renderer, positions.label(t(label)).center());
            assert!(matches!(
                click_label("中风险需审批").as_slice(),
                [Message::Settings(SettingsMessage::AutonomyRequireApprovalForMediumRiskToggled(
                    true
                ))]
            ));
            assert!(matches!(
                click_label("阻断高风险").as_slice(),
                [Message::Settings(SettingsMessage::AutonomyBlockHighRiskCommandsToggled(true))]
            ));
            assert!(matches!(
                click_label("移除").as_slice(),
                [Message::Settings(SettingsMessage::AutonomyShellRedirectPolicyChanged(
                    ShellRedirectPolicy::Strip
                ))]
            ));
            assert!(matches!(
                click_label("请求确认").as_slice(),
                [Message::Settings(
                    SettingsMessage::AutonomyNonCliNaturalLanguageApprovalModeChanged(
                        NonCliNaturalLanguageApprovalMode::RequestConfirm
                    )
                )]
            ));
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn settings_help_gap_11_list_editor_explanations() {
    if !isolated("settings_help_gap_11_list_editor_explanations") {
        return;
    }
    let mut app = fixture();
    app.autonomy_settings.active_tab = AutonomySettingsTab::ToolsAndPaths;
    let kind = AutonomyListKind::AllowedCommands;
    app.autonomy_settings.list_editor.set_draft(kind, "fixture-command".into());
    app.autonomy_settings.list_editor.request_remove(kind, 0);
    let mut renderer = renderer().await;
    for (locale, theme) in GAP_11_LOCALE_THEMES {
        app.desktop_locale = locale;
        app.app_theme = theme;
        let t = |key| I18n::new(locale).__(key);
        for width in [420, 760] {
            let size = Size::new(width, 1300);
            let mut ui = scene(view(&app), &mut renderer, size);
            for (title, old_description, help) in [
                (
                    "允许命令",
                    "Shell 白名单，每次添加一个命令。",
                    "Shell 白名单，每次添加一个命令。",
                ),
                (
                    "环境变量透传",
                    "允许透传到 shell 的环境变量名，每次添加一个。",
                    "允许透传到 shell 的环境变量名，每次添加一个。",
                ),
                (
                    "允许根目录",
                    "额外允许访问的目录，每次添加一个。",
                    "额外允许访问的目录，每次添加一个。",
                ),
                (
                    "自动审批",
                    "从现有工具中选择无需额外审批即可执行的工具。",
                    "在下拉列表中选择工具，再点击添加。",
                ),
                (
                    "始终询问",
                    "从现有工具中选择始终要求确认的工具。",
                    "在下拉列表中选择工具，再点击添加。",
                ),
            ] {
                assert_gap_11_help(
                    &app,
                    &mut ui,
                    &mut renderer,
                    size,
                    t(title),
                    t(old_description),
                    t(help),
                );
            }
            let positions = geometry(&mut ui, &renderer);
            for hint in [
                "确认删除该列表项？",
                "限制代理访问这些路径，即使在工作区外也保持拒绝。",
                "这些工具无需额外审批即可执行。",
                "这些工具始终要求确认。",
            ] {
                positions.label(t(hint));
            }
            assert!(matches!(
                click(&mut ui, &mut renderer, positions.label(t("添加")).center()).as_slice(),
                [Message::Settings(message::SettingsMessage::AutonomyListAdd(
                    AutonomyListKind::AllowedCommands
                ))]
            ));
            assert!(matches!(
                click(&mut ui, &mut renderer, positions.label(t("确认删除")).center()).as_slice(),
                [Message::Settings(message::SettingsMessage::AutonomyListRemoveConfirmed(
                    AutonomyListKind::AllowedCommands,
                    0
                ))]
            ));
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn settings_help_gap_11_channel_formats() {
    if !isolated("settings_help_gap_11_channel_formats") {
        return;
    }
    let mut app = fixture();
    app.autonomy_settings.active_tab = AutonomySettingsTab::NonCli;
    app.autonomy_settings.non_cli_approval_approvers_input = "fixture-channel:private-user".into();
    app.autonomy_settings.non_cli_natural_language_approval_mode_by_channel_input =
        "fixture-channel:disabled".into();
    app.autonomy_settings.save_error = Some("fixture channel validation error".into());
    let mut renderer = renderer().await;
    for (locale, theme) in GAP_11_LOCALE_THEMES {
        app.desktop_locale = locale;
        app.app_theme = theme;
        let t = |key| I18n::new(locale).__(key);
        for width in [420, 760] {
            let size = Size::new(width, 1000);
            let mut ui = scene(view(&app), &mut renderer, size);
            assert_gap_11_help(
                &app,
                &mut ui,
                &mut renderer,
                size,
                t("非 CLI 排除工具"),
                t("从现有工具中选择在非 CLI 通道里默认禁用的工具。"),
                t("在下拉列表中选择工具，再点击添加。"),
            );
            for (title, description, example) in [
                (
                    "审批人列表",
                    "格式为 channel:user，支持逗号或换行分隔。",
                    "telegram:alice, *:bob",
                ),
                (
                    "按通道模式",
                    "格式为 channel:mode；列表字段支持逗号或换行分隔。",
                    "telegram:direct, discord:request_confirm",
                ),
            ] {
                assert_gap_11_help(
                    &app,
                    &mut ui,
                    &mut renderer,
                    size,
                    t(title),
                    t(description),
                    &format!("{}\n{example}", t(description)),
                );
            }
            let positions = geometry(&mut ui, &renderer);
            for hint in ["这些工具在非 CLI 通道中默认禁用。", "按通道填写用户", "按通道填写模式"]
            {
                positions.label(t(hint));
            }
            positions.label("fixture channel validation error");
            let input = positions.input("fixture-channel:disabled").center();
            assert!(click(&mut ui, &mut renderer, input).is_empty());
            assert!(matches!(
                send(
                    &mut ui, &mut renderer, mouse::Cursor::Available(input),
                    &[key(iced::keyboard::key::Named::Backspace)]
                ).as_slice(),
                [Message::Settings(message::SettingsMessage::AutonomyNonCliNaturalLanguageApprovalModeByChannelChanged(value))]
                    if value.len() + 1 == "fixture-channel:disabled".len()
            ));
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn help_rollout_14_renders_contextual_help() {
    use crate::app::i18n::I18n;
    if !isolated("help_rollout_14_renders_contextual_help") {
        return;
    }
    let mut renderer = renderer().await;
    let mut app = fixture();
    let error = "fixture autonomy save failed";
    app.autonomy_settings.save_error = Some(error.into());
    app.autonomy_settings.level = AutonomyLevel::Full;
    app.autonomy_settings.list_editor.request_remove(AutonomyListKind::AllowedCommands, 0);

    for (locale, theme) in [
        (DesktopLocale::SimplifiedChinese, Theme::Light),
        (DesktopLocale::TraditionalChinese, Theme::Dark),
        (DesktopLocale::English, Theme::Dark),
    ] {
        app.desktop_locale = locale;
        app.app_theme = theme;
        let i18n = I18n::new(locale);
        let t = |key: &'static str| i18n.__(key);
        for (tab, persistent, help) in [
            (
                AutonomySettingsTab::Permissions,
                &[
                    "完全",
                    "仅允许在工作区路径内操作，除非显式加入 allowed_roots。",
                    "对中风险动作触发审批，而不是直接执行。",
                    "直接执行",
                ][..],
                &[
                    ("自治配置", 0, "配置代理的权限边界、预算限制与审批规则。"),
                    ("权限与审批", 1, "核心执行级别、审批阈值与重定向策略。"),
                ][..],
            ),
            (
                AutonomySettingsTab::Budget,
                &[][..],
                &[
                    ("预算", 1, "限制单位时间的动作量与成本。"),
                    ("每小时动作上限", 0, "限制自治过程中每小时可执行的动作数量。"),
                    ("每日成本上限(分)", 0, "限制自治行为每天可消耗的成本预算。"),
                ][..],
            ),
            (
                AutonomySettingsTab::ToolsAndPaths,
                &["确认删除该列表项？", "取消", "确认删除", "这些工具无需额外审批即可执行。"][..],
                &[("工具与路径", 1, "为 shell 命令、路径访问和自动审批设置边界。")][..],
            ),
            (
                AutonomySettingsTab::NonCli,
                &["按通道填写用户"][..],
                &[("非 CLI 暴露", 0, "控制外部通道中的工具暴露与审批人。")][..],
            ),
        ] {
            app.autonomy_settings.active_tab = tab;
            let persistent: Vec<_> =
                [error].into_iter().chain(persistent.iter().map(|s| t(s))).collect();
            let help: Vec<_> =
                help.iter().map(|&(title, index, body)| (t(title), index, t(body))).collect();
            assert_contextual_help(&app, view(&app), &mut renderer, 1300, &persistent, &help);
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn help_rollout_14_security_renders_contextual_help() {
    use crate::app::components::system_settings_security as security;
    use crate::app::i18n::settings_security::text;
    use crate::app::state::SecuritySettingsTab;

    if !isolated("help_rollout_14_security_renders_contextual_help") {
        return;
    }
    let mut renderer = renderer().await;
    let mut app = fixture();
    let error = "fixture security save failed";
    app.security_settings.save_error = Some(error.into());
    app.security_settings.sandbox_enabled_input = "false".into();
    app.security_settings.sandbox_backend_input = "none".into();

    for (locale, theme) in [
        (DesktopLocale::SimplifiedChinese, Theme::Light),
        (DesktopLocale::TraditionalChinese, Theme::Dark),
        (DesktopLocale::English, Theme::Dark),
    ] {
        app.desktop_locale = locale;
        app.app_theme = theme;
        let t = |key: &'static str| text(locale, key);
        for (tab, persistent, help) in [
            (
                SecuritySettingsTab::Bash,
                &[
                    "优先级为总开关 → 分组开关 → 单项开关；关闭父级不会改写已保存的子项。",
                    "统一控制命令、路径、风险、重定向与 Shell 语法审计；完全访问始终绕过。",
                    "关闭后保留各单项值，但命令、路径、风险和重定向策略均不生效。",
                    "已保存，当前不生效",
                ][..],
                &[
                    ("安全配置", 0, "配置沙箱、资源限制、审计与安全门控策略。"),
                    ("策略检查", 0, "命令、路径、风险与重定向策略。"),
                    ("Shell 语法检测", 0, "按 validator 独立控制 Shell 语法检查。"),
                ][..],
            ),
            (
                SecuritySettingsTab::SandboxAndResources,
                &[
                    "选择自动检测、强制启用或强制禁用沙箱。",
                    "不使用任何沙箱。仅在你明确接受更低隔离级别时使用。",
                ][..],
                &[
                    ("沙箱", 0, "控制是否启用沙箱以及选择具体后端。"),
                    ("资源限制", 0, "约束进程的内存、CPU 与子进程数量。"),
                ][..],
            ),
            (
                SecuritySettingsTab::Audit,
                &["为审计事件添加签名，提高完整性。"][..],
                &[
                    ("审计日志", 0, "记录并限制安全事件日志。"),
                    ("审计日志路径", 0, "指定审计日志文件的写入位置。"),
                ][..],
            ),
            (
                SecuritySettingsTab::OtpAndEstop,
                &[
                    "启用敏感动作的二次验证。",
                    "支持 totp、pairing、cli-prompt。",
                    "从 E-Stop 恢复时要求额外 OTP 验证。",
                ][..],
                &[
                    ("紧急停止", 0, "配置 E-Stop 状态与恢复要求。"),
                    ("E-Stop 状态文件", 0, "指定紧急停止状态文件位置。"),
                ][..],
            ),
            (
                SecuritySettingsTab::Syscall,
                &["使用更严格的异常检测规则。"][..],
                &[
                    ("系统调用异常检测", 0, "监控系统调用行为并限制告警节奏。"),
                    ("系统调用日志路径", 0, "指定系统调用异常日志的写入位置。"),
                ][..],
            ),
            (
                SecuritySettingsTab::Guard,
                &["启用蜜罐令牌与外泄检测。"][..],
                &[
                    ("防护", 1, "配置 Canary Token 与语义注入防护。"),
                    ("语义集合", 0, "指定用于语义分析的数据集合。"),
                ][..],
            ),
        ] {
            app.security_settings.active_tab = tab;
            let persistent: Vec<_> =
                [error].into_iter().chain(persistent.iter().map(|s| t(s))).collect();
            let help: Vec<_> =
                help.iter().map(|&(title, index, body)| (t(title), index, t(body))).collect();
            assert_contextual_help(
                &app,
                security::view(&app),
                &mut renderer,
                1450,
                &persistent,
                &help,
            );
        }
    }
}
