#![cfg(not(target_arch = "wasm32"))]

use super::*;
use config_ui::{DesktopLocale, GatewayClientSystemSettingsConfig};
use iced::advanced::{
    clipboard,
    renderer::{Headless, Style},
};
use iced::{Event, Font, Renderer, Size, Theme, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
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
async fn ui_refresh_22_host_security_viewport() {
    use crate::app::state::SecuritySettingsTab;

    if !isolated("ui_refresh_22_host_security_viewport") {
        return;
    }
    let mut renderer = renderer().await;
    let mut app = fixture();
    use crate::app::components::system_settings::{self, SystemTab};
    app.desktop_locale = DesktopLocale::English;
    app.navigation_panels.show_system_settings = true;
    app.navigation_panels.system_settings_tab = SystemTab::Security;
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
    let hint = positions.label("Force disable prevents selecting other backends.");
    assert!(
        hint.height <= 85.0,
        "narrow layout should fit the policy hint in five lines: {hint:?}"
    );
    drop(narrow);
    app.navigation_panels.system_settings_tab = SystemTab::Autonomy;
    app.autonomy_settings.active_tab = AutonomySettingsTab::Permissions;
    snapshot(
        system_settings::view(&app),
        &mut renderer,
        &Theme::Dark,
        Size::new(640, 800),
        "autonomy-narrow-window",
    );
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
}

fn geometry(ui: &mut Ui<'_>, renderer: &Renderer) -> Geometry {
    let mut geometry = Geometry::default();
    ui.operate(renderer, &mut geometry);
    geometry
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
    app.navigation_panels.show_system_settings = true;
    app.navigation_panels.system_settings_tab = SystemTab::Security;
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

#[test]
fn ui_refresh_22_tool_menu_hover_scroll_and_add_keep_original_ids() {
    assert!(
        matches!(map(Local::AutonomyListDraftChanged(AutonomyListKind::AutoApprove, "fixture_tool_27".into())),
        Message::Settings(message::SettingsMessage::AutonomyListDraftChanged(AutonomyListKind::AutoApprove, id)) if id == "fixture_tool_27")
    );
    assert!(matches!(
        map(Local::AutonomyListAdd(AutonomyListKind::AutoApprove)),
        Message::Settings(message::SettingsMessage::AutonomyListAdd(AutonomyListKind::AutoApprove))
    ));
}

#[test]
fn settings_help_gap_11_permissions_explanations() {
    use crate::app::message::SettingsMessage;
    for level in [AutonomyLevel::ReadOnly, AutonomyLevel::Supervised, AutonomyLevel::Full] {
        assert!(matches!(map(Local::AutonomyLevelChanged(level)),
            Message::Settings(SettingsMessage::AutonomyLevelChanged(value)) if value == level));
    }
    assert!(matches!(
        map(Local::AutonomyRequireApprovalForMediumRiskToggled(true)),
        Message::Settings(SettingsMessage::AutonomyRequireApprovalForMediumRiskToggled(true))
    ));
    assert!(matches!(
        map(Local::AutonomyBlockHighRiskCommandsToggled(true)),
        Message::Settings(SettingsMessage::AutonomyBlockHighRiskCommandsToggled(true))
    ));
    assert!(matches!(
        map(Local::AutonomyShellRedirectPolicyChanged(ShellRedirectPolicy::Strip)),
        Message::Settings(SettingsMessage::AutonomyShellRedirectPolicyChanged(
            ShellRedirectPolicy::Strip
        ))
    ));
    assert!(matches!(
        map(Local::AutonomyNonCliNaturalLanguageApprovalModeChanged(
            NonCliNaturalLanguageApprovalMode::RequestConfirm
        )),
        Message::Settings(SettingsMessage::AutonomyNonCliNaturalLanguageApprovalModeChanged(
            NonCliNaturalLanguageApprovalMode::RequestConfirm
        ))
    ));
    assert!(
        matches!(map(Local::CopyCode("fixture help".into())), Message::CopyCode(value) if value == "fixture help")
    );
}

#[test]
fn settings_help_gap_11_list_editor_explanations() {
    assert!(matches!(
        map(Local::AutonomyListAdd(AutonomyListKind::AllowedCommands)),
        Message::Settings(message::SettingsMessage::AutonomyListAdd(
            AutonomyListKind::AllowedCommands
        ))
    ));
    assert!(matches!(
        map(Local::AutonomyListRemoveConfirmed(AutonomyListKind::AllowedCommands, 0)),
        Message::Settings(message::SettingsMessage::AutonomyListRemoveConfirmed(
            AutonomyListKind::AllowedCommands,
            0
        ))
    ));
}

#[test]
fn settings_help_gap_11_channel_formats() {
    assert!(
        matches!(map(Local::AutonomyNonCliNaturalLanguageApprovalModeByChannelChanged("fixture-channel:disable".into())),
        Message::Settings(message::SettingsMessage::AutonomyNonCliNaturalLanguageApprovalModeByChannelChanged(value)) if value == "fixture-channel:disable")
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
