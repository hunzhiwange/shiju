//! Headless layout, screenshots and message regressions for observability and hooks.
#![cfg(not(target_arch = "wasm32"))]

use super::super::{system_settings, system_settings_observability as observability};
use super::*;
use crate::app::message::SettingsMessage;
use iced::advanced::{
    clipboard,
    renderer::{Headless, Style},
    widget::{
        Id, Operation,
        operation::{Outcome, focusable, scope},
    },
};
use iced::keyboard::key::Named;
use iced::{Event, Font, Point, Rectangle, Renderer, Size, Theme, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
use vw_config_types::ui::GatewayClientSystemSettingsConfig;

fn isolated(test: &str) -> bool {
    const CHILD: &str = "SHIJU_UI_REFRESH_28_CHILD";
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
    if let Some(directory) = std::env::var_os("SHIJU_UI_REFRESH_28_SCREENSHOTS") {
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
    focusables: Vec<iced::Rectangle>,
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
    fn focusable(
        &mut self,
        _: Option<&iced::advanced::widget::Id>,
        bounds: iced::Rectangle,
        _: &mut dyn iced::advanced::widget::operation::Focusable,
    ) {
        self.focusables.push(bounds);
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

fn at(point: Point) -> mouse::Cursor {
    mouse::Cursor::Available(point)
}

fn key_at(ui: &mut Ui<'_>, renderer: &mut Renderer, name: Named) -> Vec<Message> {
    send(ui, renderer, mouse::Cursor::Unavailable, &[key(name)])
}

fn assert_fits(bounds: Rectangle, size: Size<u32>) {
    assert!(bounds.width > 0.0 && bounds.height > 0.0, "{bounds:?}");
    assert!(bounds.x >= 0.0 && bounds.y >= 0.0, "{bounds:?}");
    assert!(bounds.x + bounds.width <= size.width as f32 + 0.1, "{bounds:?}");
    assert!(bounds.y + bounds.height <= size.height as f32 + 0.1, "{bounds:?}");
}

fn frame(content: Element<'_, Message>) -> Element<'_, Message> {
    container(content).width(Length::Fill).height(Length::Fill).into()
}

fn wheel(ui: &mut Ui<'_>, renderer: &mut Renderer, point: Point) -> Vec<Message> {
    send(
        ui,
        renderer,
        at(point),
        &[Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Lines { x: 0.0, y: -8.0 },
        })],
    )
}

const ENDPOINT: &str = "https://telemetry.invalid/otlp";
const TRACE_PATH: &str =
    "/fixture/追蹤資料/abcdefghijklmnopqrstuvwxyz/abcdefghijklmnopqrstuvwxyz/trace.jsonl";
const COMMAND: &str = "/fixture/插件/abcdefghijklmnopqrstuvwxyz/abcdefghijklmnopqrstuvwxyz/review";
const PLUGIN_ID: &str = "fixture-plugin";
const HOOK_HASH: &str = "sha256:fixture";
const FAILURE: &str = "Fixture request failed / 範例請求失敗";

fn fixture() -> App {
    assert_eq!(crate::app::config::gateway_client_endpoint().normalized_host(), "[");
    App::new().0
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_28_layout_and_interaction() {
    if !isolated("ui_refresh_28_layout_and_interaction") {
        return;
    }
    let mut renderer = renderer().await;
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::English;
    let state = &mut app.observability_settings;
    state.backend = "log".into();
    state.otel_endpoint_input = ENDPOINT.into();
    state.otel_service_name_input = "fixture-service".into();
    state.runtime_trace_mode = "rolling".into();
    state.runtime_trace_path_input = TRACE_PATH.into();
    state.runtime_trace_max_entries = 25_000;
    let size = Size::new(640, 840);
    let mut ui = scene(frame(observability::view(&app)), &mut renderer, size);
    capture(
        &mut ui,
        &mut renderer,
        &Theme::Light,
        size,
        "observability-en-640",
        mouse::Cursor::Unavailable,
    );
    let positions = geometry(&mut ui, &renderer);
    let endpoint = positions.input(ENDPOINT);
    assert_fits(endpoint, size);
    assert_fits(positions.input(TRACE_PATH), size);
    assert_fits(positions.label("25000"), size);
    assert!(click(&mut ui, &mut renderer, endpoint.center()).is_empty());
    assert!(key_at(&mut ui, &mut renderer, Named::End).is_empty());
    assert!(matches!(key_at(&mut ui, &mut renderer, Named::Backspace).as_slice(),
        [Message::Settings(SettingsMessage::ObservabilityOtelEndpointChanged(value))]
        if value == "https://telemetry.invalid/otl"));
    let mut selectors = positions
        .focusables
        .iter()
        .copied()
        .filter(|bounds| !positions.inputs.iter().any(|(_, input)| input == bounds));
    let backend = selectors.next().expect("the backend selector must be keyboard focusable");
    assert_fits(backend, size);
    assert!((backend.center_y() - positions.label("Backend").center_y()).abs() < 0.5);
    let endpoint_label = positions.label("OTEL endpoint");
    let endpoint_hint = positions.label("Applies only when backend=otel.");
    assert!(
        (endpoint.center_y() - (endpoint_label.y + endpoint_hint.y + endpoint_hint.height) / 2.0)
            .abs()
            < 0.5
    );
    assert!(click(&mut ui, &mut renderer, backend.center()).is_empty());
    let menu = geometry(&mut ui, &renderer).scroll.unwrap().0;
    assert_fits(menu, size);
    let mut hovers = Vec::new();
    for (name, offset) in [("none", 18.0), ("log", 56.0)] {
        let point = Point::new(menu.center_x(), menu.y + offset);
        assert!(
            send(
                &mut ui,
                &mut renderer,
                at(point),
                &[Event::Mouse(mouse::Event::CursorMoved { position: point })]
            )
            .is_empty()
        );
        hovers.push(capture(&mut ui, &mut renderer, &Theme::Light, size, name, at(point)));
    }
    assert_ne!(hovers[0], hovers[1]);
    assert!(
        send(
            &mut ui,
            &mut renderer,
            mouse::Cursor::Unavailable,
            &[Event::Mouse(mouse::Event::CursorLeft)]
        )
        .is_empty()
    );
    assert!(key_at(&mut ui, &mut renderer, Named::Escape).is_empty());
    assert!(geometry(&mut ui, &renderer).scroll.is_none());
    assert!(key_at(&mut ui, &mut renderer, Named::Enter).is_empty());
    assert!(key_at(&mut ui, &mut renderer, Named::ArrowDown).is_empty());
    assert!(matches!(key_at(&mut ui, &mut renderer, Named::Enter).as_slice(),
        [Message::Settings(SettingsMessage::ObservabilityBackendChanged(value))]
        if value == "prometheus"));

    let trace = selectors.next().expect("the trace selector must be keyboard focusable");
    assert_fits(trace, size);
    assert!(selectors.next().is_none());
    assert!(click(&mut ui, &mut renderer, trace.center()).is_empty());
    assert!(key_at(&mut ui, &mut renderer, Named::ArrowDown).is_empty());
    assert!(matches!(key_at(&mut ui, &mut renderer, Named::Enter).as_slice(),
        [Message::Settings(SettingsMessage::ObservabilityRuntimeTraceModeChanged(value))]
        if value == "full"));
    let badge = positions.label("25000");
    let slider = Point::new(badge.x - 40.0, badge.center_y());
    assert!(matches!(click(&mut ui, &mut renderer, slider).as_slice(),
        [Message::Settings(SettingsMessage::ObservabilityRuntimeTraceMaxEntriesChanged(value))]
        if *value > 25_000 && *value <= 100_000));
    drop(ui);

    app.show_system_settings = true;
    app.system_settings_tab = system_settings::SystemTab::Observability;
    app.desktop_locale = DesktopLocale::SimplifiedChinese;
    app.observability_settings.save_error = Some(FAILURE.into());
    let size = Size::new(960, 840);
    let mut ui = scene(system_settings::view(&app), &mut renderer, size);
    assert_fits(geometry(&mut ui, &renderer).label(FAILURE), size);
    // The first Tab focuses the settings search, the second focuses the backend.
    for _ in 0..2 {
        let mut focus =
            scope(Id::from(system_settings::SELECT_FOCUS_SCOPE), focusable::focus_next::<()>());
        ui.operate(&renderer, &mut focus);
        let Outcome::Chain(mut apply_focus) = focus.finish() else {
            panic!("expected focus traversal to return its second pass");
        };
        ui.operate(&renderer, apply_focus.as_mut());
    }
    assert!(key_at(&mut ui, &mut renderer, Named::Enter).is_empty());
    capture(
        &mut ui,
        &mut renderer,
        &Theme::Dark,
        size,
        "settings-zh-cn-960",
        mouse::Cursor::Unavailable,
    );
    assert!(matches!(key_at(&mut ui, &mut renderer, Named::Enter).as_slice(),
        [Message::Settings(SettingsMessage::ObservabilityBackendChanged(value))]
        if value == "log"));
    drop(ui);

    app.desktop_locale = DesktopLocale::TraditionalChinese;
    app.observability_settings.show_help_modal = true;
    let size = Size::new(420, 360);
    let base = frame(observability::view(&app));
    let mut ui = scene(observability::view_overlays(&app, base), &mut renderer, size);
    capture(
        &mut ui,
        &mut renderer,
        &Theme::Dark,
        size,
        "help-zh-tw-420",
        mouse::Cursor::Unavailable,
    );
    let title = crate::app::i18n::settings_runtime::text(app.desktop_locale, "可观测性配置帮助");
    let positions = geometry(&mut ui, &renderer);
    assert_fits(positions.label(title), size);
    let (scroll, before) = positions.scroll.unwrap();
    assert!(wheel(&mut ui, &mut renderer, scroll.center()).is_empty());
    let after = geometry(&mut ui, &renderer);
    assert!(after.scroll.unwrap().1.y > before.y);
    assert_eq!(after.label(title), positions.label(title));
    capture(
        &mut ui,
        &mut renderer,
        &Theme::Dark,
        size,
        "help-scrolled-zh-tw-420",
        mouse::Cursor::Unavailable,
    );
    let close = Point::new(scroll.x + scroll.width - 14.0, after.label(title).center_y());
    assert!(matches!(
        click(&mut ui, &mut renderer, close).as_slice(),
        [Message::Settings(SettingsMessage::ObservabilityHelpClose)]
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_28_preserves_values_and_disabled_states() {
    if !isolated("ui_refresh_28_preserves_values_and_disabled_states") {
        return;
    }
    let mut renderer = renderer().await;
    let mut app = fixture();
    app.hooks_settings.enabled = true;
    app.hooks_settings.command_logger = true;
    app.hooks_settings.plugins_enabled = false;
    let size = Size::new(640, 1500);
    for (loading, label) in [(false, "未发现插件 hooks"), (true, "正在加载插件 hooks…")]
    {
        app.hooks_settings.catalog_loading = loading;
        app.hooks_settings.audit_loading = loading;
        let mut ui = scene(frame(view(&app)), &mut renderer, size);
        let positions = geometry(&mut ui, &renderer);
        assert_fits(positions.label(tr(app.desktop_locale, label)), size);
        let refresh_label = tr(app.desktop_locale, if loading { "加载中…" } else { "刷新清单" });
        let messages = click(&mut ui, &mut renderer, positions.label(refresh_label).center());
        if loading {
            assert!(messages.is_empty());
        } else {
            assert!(matches!(
                messages.as_slice(),
                [Message::Settings(SettingsMessage::Hooks(HooksMessage::CatalogRefresh))]
            ));
        }
        capture(
            &mut ui,
            &mut renderer,
            &Theme::Light,
            size,
            if loading { "hooks-loading-zh-cn-640" } else { "hooks-empty-zh-cn-640" },
            mouse::Cursor::Unavailable,
        );
    }
    app.hooks_settings.catalog_loading = false;
    app.hooks_settings.audit_loading = false;
    app.hooks_settings.catalog_error = Some(FAILURE.into());
    app.hooks_settings.audit = (0..12)
        .map(|index| vw_gateway_client::HookAuditEntryDto {
            timestamp: format!("2026-01-01T00:00:{index:02}Z"),
            tool_name: "fixture-tool".into(),
            duration_ms: 8,
            success: false,
        })
        .collect();
    app.hooks_settings.catalog = vec![vw_gateway_client::HookCatalogEntryDto {
        plugin_id: PLUGIN_ID.into(),
        event: "before_tool_call".into(),
        command: COMMAND.into(),
        args: vec!["--fixture".into()],
        working_dir: TRACE_PATH.into(),
        mode: "blocking".into(),
        hook_hash: HOOK_HASH.into(),
        trusted: false,
    }];

    for (locale, width, theme, trusted, busy, name) in [
        (DesktopLocale::SimplifiedChinese, 640, Theme::Dark, false, true, "hooks-busy-zh-cn-640"),
        (
            DesktopLocale::TraditionalChinese,
            960,
            Theme::Dark,
            false,
            false,
            "hooks-review-zh-tw-960",
        ),
        (DesktopLocale::English, 640, Theme::Light, true, false, "hooks-trusted-en-640"),
    ] {
        app.desktop_locale = locale;
        app.hooks_settings.mutation_loading = busy;
        app.hooks_settings.catalog[0].trusted = trusted;
        let size = Size::new(width, 1660);
        let mut ui = scene(frame(view(&app)), &mut renderer, size);
        let positions = geometry(&mut ui, &renderer);
        for value in [COMMAND, TRACE_PATH, FAILURE] {
            assert_fits(positions.label(value), size);
        }
        let action = positions.label(tr(locale, if trusted { "撤销信任" } else { "信任" }));
        let messages = click(&mut ui, &mut renderer, action.center());
        if busy {
            assert!(messages.is_empty());
        } else {
            assert!(matches!((trusted, messages.as_slice()),
                (false, [Message::Settings(SettingsMessage::Hooks(HooksMessage::TrustRequested { plugin_id, hook_hash }))])
                | (true, [Message::Settings(SettingsMessage::Hooks(HooksMessage::RevokeRequested { plugin_id, hook_hash }))])
                if plugin_id == PLUGIN_ID && hook_hash == HOOK_HASH));
        }
        assert!(matches!(
            click(
                &mut ui,
                &mut renderer,
                positions.label(tr(locale, "开启运行时 hooks 执行")).center()
            )
            .as_slice(),
            [Message::Settings(SettingsMessage::Hooks(HooksMessage::EnabledToggled(false)))]
        ));
        assert!(matches!(
            click(
                &mut ui,
                &mut renderer,
                positions.label(tr(locale, "允许执行可信插件 hooks")).center()
            )
            .as_slice(),
            [Message::Settings(SettingsMessage::Hooks(HooksMessage::PluginHooksToggled(true)))]
        ));
        assert!(matches!(
            click(&mut ui, &mut renderer, positions.label(tr(locale, "刷新审计")).center())
                .as_slice(),
            [Message::Settings(SettingsMessage::Hooks(HooksMessage::AuditRefresh))]
        ));
        let (audit, translation) = positions.scroll.unwrap();
        assert!(wheel(&mut ui, &mut renderer, audit.center()).is_empty());
        assert!(geometry(&mut ui, &renderer).scroll.unwrap().1.y > translation.y);
        capture(&mut ui, &mut renderer, &theme, size, name, mouse::Cursor::Unavailable);
    }
}
