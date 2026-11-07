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
    positions.assert_centered(
        positions.input(&app.security_settings.audit_log_path),
        "审计日志路径",
        "指定审计日志文件的写入位置。",
    );
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
            "自动审批",
            "选择要自动审批的工具。",
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
