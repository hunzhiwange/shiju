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
    const CHILD: &str = "SHIJU_UI_REFRESH_21_CHILD";
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
    if let Some(directory) = std::env::var_os("SHIJU_UI_REFRESH_21_SCREENSHOTS") {
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
    app.runtime_settings.kind = "docker".into();
    app.runtime_settings.docker_image = "fixture/alpine:1.0".into();
    app.runtime_settings.docker_network = "none".into();
    app.runtime_settings.docker_cpu_limit_input = "1.5".into();
    app.runtime_settings.docker_read_only_rootfs = true;
    app.runtime_settings.docker_mount_workspace = false;
    app.runtime_settings.docker_allowed_workspace_roots_input =
        "/workspace/虚构/測試/long-fixture-directory/only-this-root".into();
    app.reliability_settings.provider_retries = 3;
    app.reliability_settings.provider_backoff_ms = 60_000;
    app.reliability_settings.channel_initial_backoff_secs = 12;
    app.reliability_settings.channel_max_backoff_secs = 60;
    app.reliability_settings.scheduler_poll_secs = 3600;
    app.reliability_settings.scheduler_retries = 0;
    app
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_21_renders_pages_and_narrow_help() {
    use crate::app::components::system_settings_runtime;
    if !isolated("ui_refresh_21_renders_pages_and_narrow_help") {
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
        let size = Size::new(width, 1320);
        snapshot(
            system_settings_runtime::view(&app),
            &mut renderer,
            &theme,
            size,
            &format!("runtime-{tag}"),
        );
        snapshot(view(&app), &mut renderer, &theme, size, &format!("reliability-{tag}"));
        app.reliability_settings.show_help_modal = true;
        snapshot(
            view_overlays(&app, view(&app)),
            &mut renderer,
            &theme,
            Size::new(420, 560),
            &format!("help-{tag}"),
        );
        use crate::app::components::{system_settings::SystemTab, system_settings_help};
        snapshot(
            system_settings_help::with_help_modal(
                &app,
                system_settings_runtime::view(&app),
                SystemTab::Runtime,
                Some(SystemTab::Runtime),
            ),
            &mut renderer,
            &theme,
            Size::new(420, 560),
            &format!("runtime-help-{tag}"),
        );
    }
    use crate::app::components::system_settings;
    app.show_system_settings = true;
    app.system_settings_tab = system_settings::SystemTab::Runtime;
    app.runtime_settings.save_error = Some("Docker CPU 限制 必须大于 0".into());
    snapshot(
        system_settings::view(&app),
        &mut renderer,
        &Theme::Dark,
        Size::new(960, 720),
        "runtime-error-window",
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_21_runtime_select_is_reachable_from_keyboard() {
    use crate::app::components::system_settings::{self, SELECT_FOCUS_SCOPE, SystemTab};
    use crate::app::state::AgentConfigTab;
    use iced::advanced::widget::{
        Id,
        operation::{Operation, Outcome, focusable, scope},
    };
    use iced::keyboard::key::Named;

    if !isolated("ui_refresh_21_runtime_select_is_reachable_from_keyboard") {
        return;
    }
    let mut renderer = renderer().await;
    let mut app = fixture();
    app.show_system_settings = true;
    app.runtime_settings.kind = "native".into();
    app.agents_settings.active_config_tab = AgentConfigTab::Runtime;
    for tab in [SystemTab::Runtime, SystemTab::Agents] {
        app.system_settings_tab = tab;
        let mut ui = scene(system_settings::view(&app), &mut renderer, Size::new(960, 950));
        let cursor = mouse::Cursor::Unavailable;
        let mut focus = scope(Id::from(SELECT_FOCUS_SCOPE), focusable::focus_next::<()>());
        ui.operate(&renderer, &mut focus);
        let Outcome::Chain(mut next) = focus.finish() else {
            panic!("expected focus traversal to run in two passes");
        };
        ui.operate(&renderer, next.as_mut());
        assert!(send(&mut ui, &mut renderer, cursor, &[key(Named::Enter)]).is_empty());
        assert!(send(&mut ui, &mut renderer, cursor, &[key(Named::ArrowDown)]).is_empty());
        assert!(
            matches!(
                send(&mut ui, &mut renderer, cursor, &[key(Named::Enter)]).as_slice(),
                [Message::Settings(message::SettingsMessage::Runtime(message::settings::RuntimeMessage::KindChanged(value)))] if value == "docker"
            ),
            "Tab focus must reach the first runtime select in {tab:?}"
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
async fn ui_refresh_21_layout_and_interaction() {
    use crate::app::components::system_settings_runtime;
    use crate::app::message::settings::{RuntimeMessage, SettingsMessage};
    use iced::{Point, keyboard::key::Named};

    if !isolated("ui_refresh_21_layout_and_interaction") {
        return;
    }
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::SimplifiedChinese;
    app.runtime_settings.kind = "native".into();
    let mut renderer = renderer().await;
    let size = Size::new(640, 460);
    let mut ui = scene(system_settings_runtime::view(&app), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    let trigger = positions.focusable[0];
    assert!((trigger.center_y() - positions.label("运行时类型").center_y()).abs() < 0.5);
    assert!((trigger.height - 38.0).abs() < 0.5);
    assert!(positions.inputs.is_empty());
    assert!(click(&mut ui, &mut renderer, trigger.center()).is_empty());

    let mut screenshots = Vec::new();
    for (index, point) in [
        Point::new(trigger.center_x(), trigger.y + trigger.height + 20.0),
        Point::new(trigger.center_x(), trigger.y + trigger.height + 56.0),
        Point::new(5.0, 5.0),
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
        // Draw rebuilds the overlay after the cursor event, as in the Iced runtime.
        screenshots.push(capture(
            &mut ui,
            &mut renderer,
            &Theme::Light,
            size,
            &format!("runtime-hover-{index}"),
            cursor,
        ));
    }
    assert_ne!(screenshots[0], screenshots[1]);
    assert!(screenshots[..2].iter().all(|hovered| hovered != &screenshots[2]));

    let cursor = mouse::Cursor::Unavailable;
    assert!(send(&mut ui, &mut renderer, cursor, &[key(Named::Escape)]).is_empty());
    assert!(send(&mut ui, &mut renderer, cursor, &[key(Named::Enter)]).is_empty());
    assert!(send(&mut ui, &mut renderer, cursor, &[key(Named::ArrowDown)]).is_empty());
    let selected = send(&mut ui, &mut renderer, cursor, &[key(Named::Enter)]);
    let [Message::Settings(settings @ SettingsMessage::Runtime(RuntimeMessage::KindChanged(kind)))] =
        selected.as_slice()
    else {
        panic!("expected exactly one runtime kind change");
    };
    assert_eq!(kind, "docker");
    drop(ui);
    drop(crate::app::message::settings::update(&mut app, settings.clone()));
    assert_eq!(app.runtime_settings.kind, "docker");

    // The stored wasm value remains visible but is not a supported desktop selection.
    app.runtime_settings.kind = "wasm".into();
    let mut ui = scene(system_settings_runtime::view(&app), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    assert!(positions.inputs.is_empty());
    assert!(click(&mut ui, &mut renderer, positions.focusable[0].center()).is_empty());
    assert!(send(&mut ui, &mut renderer, cursor, &[key(Named::End)]).is_empty());
    let selected = send(&mut ui, &mut renderer, cursor, &[key(Named::Enter)]);
    assert!(matches!(
        selected.as_slice(),
        [Message::Settings(SettingsMessage::Runtime(RuntimeMessage::KindChanged(kind)))] if kind == "docker"
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_21_preserves_values_and_disabled_states() {
    use crate::app::components::system_settings_runtime;
    use crate::app::message::settings::{RuntimeMessage, SettingsMessage};
    use iced::{Point, keyboard::key::Named};

    if !isolated("ui_refresh_21_preserves_values_and_disabled_states") {
        return;
    }
    let mut app = fixture();
    let mut renderer = renderer().await;
    app.desktop_locale = DesktopLocale::SimplifiedChinese;
    let size = Size::new(640, 1320);
    let mut ui = scene(system_settings_runtime::view(&app), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    let network = positions.input("none");
    assert!((network.center_y() - positions.label("网络模式 (network)").center_y()).abs() < 0.5);
    let checkboxes: Vec<_> = positions.labels.iter().filter(|(label, _)| label == "启用").collect();
    assert_eq!(checkboxes.len(), 2);
    let mount = checkboxes[1].1;
    let mount_label = positions.label("挂载工作区 (mount_workspace)");
    assert!((mount.center_y() - mount_label.center_y()).abs() < 0.5);
    assert!(matches!(
        click(&mut ui, &mut renderer, checkboxes[0].1.center()).as_slice(),
        [Message::Settings(SettingsMessage::Runtime(RuntimeMessage::DockerReadOnlyRootfsToggled(
            false
        )))]
    ));
    assert!(matches!(
        click(&mut ui, &mut renderer, mount.center()).as_slice(),
        [Message::Settings(SettingsMessage::Runtime(RuntimeMessage::DockerMountWorkspaceToggled(
            true
        )))]
    ));
    let path = &app.runtime_settings.docker_allowed_workspace_roots_input;
    assert!(positions.input(path).width > 500.0);
    let point = positions.input(path).center();
    assert!(click(&mut ui, &mut renderer, point).is_empty());
    assert!(matches!(
        send(&mut ui, &mut renderer, mouse::Cursor::Available(point), &[
            key(Named::End),
            Event::InputMethod(iced::advanced::input_method::Event::Commit("/子目錄".into())),
        ]).as_slice(),
        [Message::Settings(SettingsMessage::Runtime(RuntimeMessage::DockerAllowedWorkspaceRootsChanged(value)))] if value == &format!("{path}/子目錄")
    ));
    assert!(
        send(
            &mut ui,
            &mut renderer,
            mouse::Cursor::Unavailable,
            &[key(Named::Enter), key(Named::Escape)]
        )
        .is_empty()
    );
    drop(ui);

    let mut ui = scene(view(&app), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    let badge = positions.label("60 s");
    positions.assert_centered(badge, "频道最大退避", "退避算法允许增长到的最大等待时间。");
    let point = Point::new(badge.x - 70.0, badge.center_y());
    let press = send(
        &mut ui,
        &mut renderer,
        mouse::Cursor::Available(point),
        &[Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))],
    );
    assert!(matches!(
        press.as_slice(),
        [Message::Settings(SettingsMessage::ReliabilityChannelMaxBackoffSecsChanged(value))] if (12..=3600).contains(value)
    ));
    for (x, expected) in [(10.0, 12), (630.0, 3600)] {
        let point = Point::new(x, badge.center_y());
        assert!(matches!(
            send(&mut ui, &mut renderer, mouse::Cursor::Available(point), &[
                Event::Mouse(mouse::Event::CursorMoved { position: point }),
            ]).as_slice(),
            [Message::Settings(SettingsMessage::ReliabilityChannelMaxBackoffSecsChanged(value))] if *value == expected
        ));
    }
    assert!(
        send(
            &mut ui,
            &mut renderer,
            mouse::Cursor::Unavailable,
            &[Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),]
        )
        .is_empty()
    );
    drop(ui);

    // Help overlays keep the underlying form inactive, including its sliders.
    app.reliability_settings.show_help_modal = true;
    let size = Size::new(420, 560);
    let mut ui = scene(view_overlays(&app, view(&app)), &mut renderer, size);
    let cursor = mouse::Cursor::Available(Point::new(200.0, 380.0));
    let before = capture(&mut ui, &mut renderer, &Theme::Dark, size, "help-before-scroll", cursor);
    assert!(
        send(
            &mut ui,
            &mut renderer,
            cursor,
            &[Event::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -380.0 }
            }),]
        )
        .is_empty()
    );
    let after = capture(&mut ui, &mut renderer, &Theme::Dark, size, "help-after-scroll", cursor);
    let header_bytes = 60 * size.width as usize * 4;
    assert_eq!(&before[..header_bytes], &after[..header_bytes], "help actions must not scroll");
    assert_ne!(&before[header_bytes..], &after[header_bytes..]);
    assert!(click(&mut ui, &mut renderer, Point::new(200.0, 380.0)).is_empty());
    assert!(matches!(
        click(&mut ui, &mut renderer, Point::new(382.0, 36.0)).as_slice(),
        [Message::Settings(SettingsMessage::ReliabilityHelpClose)]
    ));
}
