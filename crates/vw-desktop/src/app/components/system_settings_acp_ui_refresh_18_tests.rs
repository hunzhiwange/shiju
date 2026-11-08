#![cfg(not(target_arch = "wasm32"))]

use super::*;
use crate::app::components::system_settings_mcp as mcp;
use crate::app::state::{AcpDiagnosticStatus, McpNewServerTransport, McpSettingsTab};
use iced::advanced::{
    clipboard,
    renderer::{Headless, Style},
};
use iced::{Event, Font, Renderer, Size, Theme, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
use vw_config_types::ui::{DesktopLocale, GatewayClientSystemSettingsConfig};

fn isolated(test: &str) -> bool {
    const CHILD: &str = "SHIJU_UI_REFRESH_18_CHILD";
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

fn fixture() -> App {
    assert_eq!(crate::app::config::gateway_client_endpoint().normalized_host(), "[");
    let mut app = App::new().0;
    app.acp_settings.catalog.clear();
    app.acp_settings.catalog.insert(
        "fixture-agent".into(),
        AcpAgentConfig { command: "/示例路徑/bin/fixture-agent".into(), ..Default::default() },
    );
    app.acp_settings.new_agent_name_input = "fixture-agent".into();
    app.acp_settings.new_agent_command_input = "/示例路徑/bin/fixture-agent".into();
    app.acp_settings.editor_args = vec!["--label, preserve spaces".into()];
    app.acp_settings.editor_env = vec![("FIXTURE_TOKEN".into(), "虚构的秘密".into())];
    app.mcp_settings.editor = iced::widget::text_editor::Content::with_text(
        r#"[{"name":"示例伺服器 fixture-server","type":"stdio","command":"fixture-bin","env":[{"name":"NAME","value":"繁體值"}]}]"#,
    );
    app.mcp_settings.server_count = 1;
    app.mcp_settings.new_server.name_input = "fixture-server".into();
    app.mcp_settings.new_server.command_input = "fixture-bin".into();
    app.mcp_settings.new_server.args_input = "--label, /示例路徑".into();
    app.mcp_settings.new_server.url_input =
        format!("https://example.invalid/{}", "segment/".repeat(24));
    app
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
    if let Some(directory) = std::env::var_os("SHIJU_UI_REFRESH_18_SCREENSHOTS") {
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

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_18_renders_catalogs_and_forms() {
    if !isolated("ui_refresh_18_renders_catalogs_and_forms") {
        return;
    }
    let mut renderer = renderer().await;
    for (locale, width) in [
        (DesktopLocale::SimplifiedChinese, 640),
        (DesktopLocale::TraditionalChinese, 960),
        (DesktopLocale::English, 420),
    ] {
        for theme in [Theme::Light, Theme::Dark] {
            let mut app = fixture();
            app.desktop_locale = locale;
            let size = Size::new(width, 660);
            let stem = format!("{locale:?}-{theme:?}-{width}");
            snapshot(view(&app), &mut renderer, &theme, size, &format!("{stem}-acp-list"));
            snapshot(mcp::view(&app), &mut renderer, &theme, size, &format!("{stem}-mcp-list"));
            app.acp_settings.new_agent_expanded = true;
            app.acp_settings.diagnostic_status = Some(AcpDiagnosticStatus::HandshakeFailed);
            app.acp_settings.editing_agent = Some("fixture-agent".into());
            snapshot(view(&app), &mut renderer, &theme, size, &format!("{stem}-acp-draft"));
            app.acp_settings.save_error = Some("诊断失败，請檢查配置。".repeat(80));
            snapshot(view(&app), &mut renderer, &theme, size, &format!("{stem}-acp-edit"));
            app.mcp_settings.new_server_expanded = true;
            snapshot(
                mcp::view_overlays(&app, mcp::view(&app)),
                &mut renderer,
                &theme,
                size,
                &format!("{stem}-mcp-stdio"),
            );
            app.mcp_settings.new_server.transport = McpNewServerTransport::Http;
            app.mcp_settings.editing_server_index = Some(0);
            snapshot(
                mcp::view_overlays(&app, mcp::view(&app)),
                &mut renderer,
                &theme,
                size,
                &format!("{stem}-mcp-url"),
            );
            app.mcp_settings.saving = true;
            app.mcp_settings.save_error = Some("连接失败，請檢查配置。".repeat(80));
            snapshot(
                mcp::view_overlays(&app, mcp::view(&app)),
                &mut renderer,
                &theme,
                size,
                &format!("{stem}-mcp-http"),
            );
            app.mcp_settings.new_server_expanded = false;
            app.mcp_settings.saving = false;
            app.mcp_settings.save_error = None;
            app.mcp_settings.active_tab = McpSettingsTab::Json;
            snapshot(mcp::view(&app), &mut renderer, &theme, size, &format!("{stem}-mcp-json"));
            app.mcp_settings.active_tab = McpSettingsTab::Runtime;
            app.mcp_settings.inspect_loading = true;
            snapshot(
                mcp::view(&app),
                &mut renderer,
                &theme,
                size,
                &format!("{stem}-mcp-runtime-loading"),
            );
            app.acp_settings.new_agent_expanded = false;
            app.acp_settings.save_error = None;
            app.acp_settings.loading = true;
            app.acp_settings.catalog.clear();
            snapshot(view(&app), &mut renderer, &theme, size, &format!("{stem}-acp-empty-loading"));
        }
    }
}

#[derive(Default)]
struct Geometry {
    labels: Vec<(String, iced::Rectangle)>,
    inputs: Vec<(String, iced::Rectangle)>,
    focusable: Vec<iced::Rectangle>,
    scroll_offset: iced::Vector,
}

impl iced::advanced::widget::Operation for Geometry {
    fn scrollable(
        &mut self,
        _: Option<&iced::advanced::widget::Id>,
        _: iced::Rectangle,
        _: iced::Rectangle,
        translation: iced::Vector,
        _: &mut dyn iced::advanced::widget::operation::Scrollable,
    ) {
        self.scroll_offset = translation;
    }

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
            .unwrap_or_else(|| panic!("missing fixture input: {value}"))
            .1
    }
}

fn geometry(ui: &mut Ui<'_>, renderer: &Renderer) -> Geometry {
    let mut positions = Geometry::default();
    ui.operate(renderer, &mut positions);
    positions
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

fn edit(ui: &mut Ui<'_>, renderer: &mut Renderer, point: iced::Point, value: &str) -> Vec<Message> {
    assert!(click(ui, renderer, point).is_empty());
    send(
        ui,
        renderer,
        mouse::Cursor::Available(point),
        &[
            key(iced::keyboard::key::Named::End),
            Event::InputMethod(iced::advanced::input_method::Event::Commit(value.into())),
        ],
    )
}

fn scroll(ui: &mut Ui<'_>, renderer: &mut Renderer, point: iced::Point, delta: f32) {
    assert!(
        send(
            ui,
            renderer,
            mouse::Cursor::Available(point),
            &[Event::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Pixels { x: 0.0, y: delta },
            })],
        )
        .is_empty()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_18_layout_and_interaction() {
    if !isolated("ui_refresh_18_layout_and_interaction") {
        return;
    }
    use crate::app::message::settings::{self, McpMessage};
    use iced::keyboard::key::Named;
    let mut renderer = renderer().await;
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::SimplifiedChinese;
    let size = Size::new(420, 660);
    let mut ui = scene(mcp::view(&app), &mut renderer, size);
    let edit_server = geometry(&mut ui, &renderer).label("编辑").center();
    let messages = click(&mut ui, &mut renderer, edit_server);
    assert!(matches!(
        messages.as_slice(),
        [Message::Settings(SettingsMessage::Mcp(McpMessage::ServerEditRequested(0)))]
    ));
    drop(ui);
    drop(settings::update(&mut app, SettingsMessage::Mcp(McpMessage::ServerEditRequested(0))));
    assert_eq!(app.mcp_settings.editing_server_index, Some(0), "{:?}", app.mcp_settings.save_error);
    assert!(app.mcp_settings.new_server_expanded);
    let mut ui = scene(mcp::view_overlays(&app, mcp::view(&app)), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    let label = positions.label("类型");
    let trigger = *positions
        .focusable
        .iter()
        .find(|bounds| {
            bounds.y >= label.y + label.height && (bounds.y - (label.y + label.height)) < 12.0
        })
        .expect("transport select");
    let save = positions.label("保存修改");
    assert!(save.y + save.height < size.height as f32);
    assert!(click(&mut ui, &mut renderer, trigger.center()).is_empty());
    let no_cursor = mouse::Cursor::Unavailable;
    let normal =
        capture(&mut ui, &mut renderer, &Theme::Dark, size, "mcp-transport-open", no_cursor);
    let mut hovered = Vec::new();
    // Visit the first and second rows, then leave the menu. Draw after each event.
    for (index, y) in [30.0, 68.0].into_iter().enumerate() {
        let point = iced::Point::new(trigger.center_x(), trigger.y + trigger.height + y);
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
            &format!("mcp-transport-hover-{index}"),
            cursor,
        ));
    }
    assert_ne!(hovered[0], hovered[1]);
    assert_eq!(hovered[0], normal, "initial keyboard highlight is on stdio");
    let outside = iced::Point::new(5.0, 5.0);
    let cursor = mouse::Cursor::Available(outside);
    assert!(
        send(
            &mut ui,
            &mut renderer,
            cursor,
            &[Event::Mouse(mouse::Event::CursorMoved { position: outside })]
        )
        .is_empty()
    );
    let left = capture(&mut ui, &mut renderer, &Theme::Dark, size, "mcp-transport-left", cursor);
    assert!(hovered.iter().all(|pixels| *pixels != left));
    assert!(send(&mut ui, &mut renderer, no_cursor, &[key(Named::Escape)]).is_empty());
    assert!(send(&mut ui, &mut renderer, no_cursor, &[key(Named::Enter)]).is_empty());
    // The runtime only dispatches to the newly opened overlay on the next update.
    assert!(send(&mut ui, &mut renderer, no_cursor, &[key(Named::ArrowDown)]).is_empty());
    let messages = send(&mut ui, &mut renderer, no_cursor, &[key(Named::Enter)]);
    assert!(matches!(
        messages.as_slice(),
        [Message::Settings(SettingsMessage::Mcp(McpMessage::NewServerTransportChanged(
            McpNewServerTransport::Http
        )))]
    ));
    let messages = click(&mut ui, &mut renderer, save.center());
    assert!(matches!(
        messages.as_slice(),
        [Message::Settings(SettingsMessage::Mcp(McpMessage::SaveServerEdit))]
    ));
    let cancel = positions.label("取消编辑").center();
    assert!(matches!(
        click(&mut ui, &mut renderer, cancel).as_slice(),
        [Message::Settings(SettingsMessage::Mcp(McpMessage::ServerEditCanceled))]
    ));
    drop(ui);
    let saved_json = app.mcp_settings.editor.text();
    drop(settings::update(&mut app, SettingsMessage::Mcp(McpMessage::ServerEditCanceled)));
    assert_eq!(app.mcp_settings.editor.text(), saved_json);
    assert!(app.mcp_settings.editing_server_index.is_none());

    app.mcp_settings.new_server_expanded = false;
    app.mcp_settings.delete_pending = Some(0);
    let mut ui = scene(mcp::view_overlays(&app, mcp::view(&app)), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    let confirm = positions.label("确认删除").center();
    let cancel = positions.label("取消").center();
    assert!(matches!(
        click(&mut ui, &mut renderer, confirm).as_slice(),
        [Message::Settings(SettingsMessage::Mcp(McpMessage::ConfirmDelete(0)))]
    ));
    assert!(matches!(
        click(&mut ui, &mut renderer, cancel).as_slice(),
        [Message::Settings(SettingsMessage::Mcp(McpMessage::CancelDelete))]
    ));
    drop(ui);
    drop(settings::update(&mut app, SettingsMessage::Mcp(McpMessage::CancelDelete)));
    assert_eq!(app.mcp_settings.editor.text(), saved_json);

    app.mcp_settings.active_tab = McpSettingsTab::Json;
    let mut ui = scene(mcp::view(&app), &mut renderer, size);
    let mut positions = geometry(&mut ui, &renderer);
    positions.focusable.sort_by(|a, b| b.height.total_cmp(&a.height));
    let editor = positions.focusable[0];
    assert!(editor.height > 200.0);
    let point = iced::Point::new(editor.x + 30.0, editor.y + 30.0);
    let _ = click(&mut ui, &mut renderer, point);
    let mut messages = send(
        &mut ui,
        &mut renderer,
        mouse::Cursor::Available(point),
        &[Event::InputMethod(iced::advanced::input_method::Event::Commit("繁體值".into()))],
    );
    assert_eq!(messages.len(), 1);
    let Message::Settings(SettingsMessage::Mcp(McpMessage::EditorAction(action))) =
        messages.remove(0)
    else {
        panic!("expected JSON editor action")
    };
    assert!(action.is_edit());
    drop(ui);
    drop(settings::update(&mut app, SettingsMessage::Mcp(McpMessage::EditorAction(action))));
    assert_eq!(app.mcp_settings.editor.text().matches("繁體值").count(), 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_18_preserves_values_and_disabled_states() {
    if !isolated("ui_refresh_18_preserves_values_and_disabled_states") {
        return;
    }
    use crate::app::message::settings::{self, McpMessage};
    let mut renderer = renderer().await;
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::SimplifiedChinese;
    let size = Size::new(420, 660);
    let mut ui = scene(view(&app), &mut renderer, size);
    let test = geometry(&mut ui, &renderer).label("测试连接").center();
    assert!(
        matches!(click(&mut ui, &mut renderer, test).as_slice(), [Message::Settings(SettingsMessage::Acp(AcpMessage::TestAgent(agent)))] if agent == "fixture-agent")
    );
    drop(ui);
    app.acp_settings.new_agent_expanded = true;
    app.acp_settings.editing_agent = Some("fixture-agent".into());
    let mut ui = scene(view(&app), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    let name = positions.input("fixture-agent");
    assert!(edit(&mut ui, &mut renderer, name.center(), "禁止改名").is_empty());
    let command = positions.input("/示例路徑/bin/fixture-agent");
    assert!(
        matches!(edit(&mut ui, &mut renderer, command.center(), " 新值").as_slice(), [Message::Settings(SettingsMessage::Acp(AcpMessage::NewAgentCommandChanged(value)))] if value == "/示例路徑/bin/fixture-agent 新值")
    );
    let retry = positions.label("测试连接 / 重试").center();
    assert!(matches!(
        click(&mut ui, &mut renderer, retry).as_slice(),
        [Message::Settings(SettingsMessage::Acp(AcpMessage::StartDiagnostic))]
    ));
    let save = positions.label("保存");
    assert!(matches!(
        click(&mut ui, &mut renderer, save.center()).as_slice(),
        [Message::Settings(SettingsMessage::Acp(AcpMessage::AddAgent))]
    ));
    let viewport_point = iced::Point::new(210.0, 360.0);
    scroll(&mut ui, &mut renderer, viewport_point, -550.0);
    let scrolled = geometry(&mut ui, &renderer);
    assert_eq!(scrolled.label("编辑 ACP"), positions.label("编辑 ACP"));
    assert_eq!(scrolled.label("保存"), save);
    let env_key = scrolled.input("FIXTURE_TOKEN");
    let (masked, value) = scrolled
        .inputs
        .iter()
        .find(|(_, bounds)| bounds.x > env_key.x && bounds.y == env_key.y)
        .expect("masked environment value beside key");
    assert_ne!(masked, "虚构的秘密");
    assert!((env_key.center_y() - value.center_y()).abs() < 0.5);
    assert!(scrolled.scroll_offset.y > 0.0);
    let value_point = value.center() - scrolled.scroll_offset;
    assert!(
        matches!(edit(&mut ui, &mut renderer, value_point, "新值").as_slice(), [Message::Settings(SettingsMessage::Acp(AcpMessage::EnvironmentChanged(0, false, value)))] if value == "虚构的秘密新值")
    );
    let cancel = scrolled.label("取消").center();
    assert!(matches!(
        click(&mut ui, &mut renderer, cancel).as_slice(),
        [Message::Settings(SettingsMessage::Acp(AcpMessage::CancelEditor))]
    ));
    drop(ui);
    drop(settings::update(&mut app, SettingsMessage::Acp(AcpMessage::CancelEditor)));
    assert!(!app.acp_settings.new_agent_expanded);
    assert!(app.acp_settings.diagnostic_id.is_none());
    assert_eq!(app.acp_settings.catalog["fixture-agent"].command, "/示例路徑/bin/fixture-agent");

    app.acp_settings.adding_agent = true;
    app.acp_settings.new_agent_expanded = true;
    let mut ui = scene(view(&app), &mut renderer, size);
    let save = geometry(&mut ui, &renderer).label("保存中...").center();
    assert!(click(&mut ui, &mut renderer, save).is_empty());
    drop(ui);

    app.mcp_settings.new_server_expanded = true;
    app.mcp_settings.new_server.transport = McpNewServerTransport::Sse;
    let mut ui = scene(mcp::view_overlays(&app, mcp::view(&app)), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    let url = &app.mcp_settings.new_server.url_input;
    assert!(!positions.inputs.iter().any(|(value, _)| value == "fixture-bin"));
    assert!(
        matches!(edit(&mut ui, &mut renderer, positions.input(url).center(), "tail").as_slice(), [Message::Settings(SettingsMessage::Mcp(McpMessage::NewServerUrlChanged(value)))] if value == &format!("{url}tail"))
    );
    let add = positions.label("新增服务器").center();
    assert!(matches!(
        click(&mut ui, &mut renderer, add).as_slice(),
        [Message::Settings(SettingsMessage::Mcp(McpMessage::AddServer))]
    ));
    drop(ui);
    app.mcp_settings.saving = true;
    let mut ui = scene(mcp::view_overlays(&app, mcp::view(&app)), &mut renderer, size);
    assert!(click(&mut ui, &mut renderer, add).is_empty());
    let close = geometry(&mut ui, &renderer).label("收起").center();
    assert!(matches!(
        click(&mut ui, &mut renderer, close).as_slice(),
        [Message::Settings(SettingsMessage::Mcp(McpMessage::NewServerPanelToggled))]
    ));
}

fn assert_help_card(
    ui: &mut Ui<'_>,
    renderer: &mut Renderer,
    (size, theme, locale): (Size<u32>, &Theme, DesktopLocale),
    title: &'static str,
    description: &'static str,
) -> iced::Point {
    use iced::keyboard::key::Named;
    let i18n = I18n::new(locale);
    let positions = geometry(ui, renderer);
    let label = positions
        .labels
        .iter()
        .rev()
        .find(|(label, _)| label == i18n.__(title))
        .unwrap_or_else(|| panic!("missing help label {title}"))
        .1;
    let icon = iced::Point::new(label.x + label.width + 15.0, label.center_y());
    let frame = |ui: &mut Ui<'_>, renderer: &mut Renderer, suffix: &str, cursor| {
        capture(ui, renderer, theme, size, &format!("help-09-{locale:?}-{title}-{suffix}"), cursor)
    };
    assert!(
        iced::Rectangle::with_size(Size::new(size.width as f32, size.height as f32)).contains(icon)
    );
    let idle = frame(ui, renderer, "idle", mouse::Cursor::Unavailable);
    let cursor = mouse::Cursor::Available(icon);
    assert!(
        send(ui, renderer, cursor, &[Event::Mouse(mouse::Event::CursorMoved { position: icon })])
            .is_empty()
    );
    assert!(idle != frame(ui, renderer, "hover", cursor), "{title}: hovering must preview help");
    assert!(click(ui, renderer, icon).is_empty());
    let open = frame(ui, renderer, "open", mouse::Cursor::Unavailable);
    assert!(idle != open, "{title}: clicking the question mark must pin the help card");
    assert!(!positions.labels.iter().any(|(label, _)| label == i18n.__(description)));
    assert!(send(ui, renderer, mouse::Cursor::Unavailable, &[key(Named::Escape)]).is_empty());
    assert!(
        idle == frame(ui, renderer, "closed", mouse::Cursor::Unavailable),
        "{title}: Escape must dismiss the help card"
    );
    icon
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn help_rollout_09_renders_contextual_help() {
    if !isolated("help_rollout_09_renders_contextual_help") {
        return;
    }
    let mut app = fixture();
    let mut renderer = renderer().await;
    for (locale, theme, width) in [
        (DesktopLocale::SimplifiedChinese, Theme::Dark, 420),
        (DesktopLocale::English, Theme::Light, 640),
    ] {
        app.desktop_locale = locale;
        app.app_theme = theme.clone();
        let i18n = I18n::new(locale);
        let size = Size::new(width, 660);
        let context = (size, &theme, locale);
        app.acp_settings.new_agent_expanded = true;
        let mut form = scene(view(&app), &mut renderer, size);
        let positions = geometry(&mut form, &renderer);
        positions.label(i18n.__("诊断使用当前草稿，在网关主机执行，不发送消息或自动认证"));
        positions.input("/示例路徑/bin/fixture-agent");
        let description = "每项独立保存，保留空格与逗号。";
        let icon = assert_help_card(&mut form, &mut renderer, context, "参数", description);
        assert!(click(&mut form, &mut renderer, icon).is_empty());
        if locale == DesktopLocale::English {
            // Open the real text menu and check the messages at the OS integration boundary.
            let menu = iced::Point::new(icon.x + 29.0, icon.y + 5.0);
            let item_height =
                f32::from(iced::advanced::text::LineHeight::default().to_absolute(12.0.into()))
                    + 10.0;
            let description = i18n.__(description);
            let url =
                format!("https://www.google.com/search?q={}", urlencoding::encode(description));
            for index in [0.0, 3.0] {
                assert!(
                    send(
                        &mut form,
                        &mut renderer,
                        mouse::Cursor::Available(menu),
                        &[Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right))],
                    )
                    .is_empty()
                );
                let point =
                    iced::Point::new(menu.x + 24.0, menu.y + 3.0 + item_height * (index + 0.5));
                let messages = click(&mut form, &mut renderer, point);
                assert!(
                    match messages.as_slice() {
                        [Message::CopyCode(text)] if index == 0.0 => text == description,
                        [
                            Message::View(crate::app::message::ViewMessage::OpenUrlExternal(
                                actual,
                            )),
                        ] if index == 3.0 => actual == &url,
                        _ => false,
                    },
                    "{messages:?}"
                );
            }
        }
        assert!(
            send(
                &mut form,
                &mut renderer,
                mouse::Cursor::Unavailable,
                &[key(iced::keyboard::key::Named::Escape)],
            )
            .is_empty()
        );
        assert!(matches!(
            click(&mut form, &mut renderer, positions.label(i18n.__("取消")).center()).as_slice(),
            [Message::Settings(SettingsMessage::Acp(AcpMessage::CancelEditor))]
        ));
        drop(form);
        app.acp_settings.new_agent_expanded = false;
        app.acp_settings
            .diagnostic_results
            .insert("fixture-agent".into(), AcpDiagnosticStatus::AuthenticationRequired);
        let mut catalog = scene(view(&app), &mut renderer, size);
        for (title, description) in [
            ("ACP 配置", "查看可用 ACP 智能体，按需初始化本机环境，并通过网关更新启用状态。"),
            ("可用 ACP", "启用后会出现在聊天、任务和设计生成的 ACP 后端选择中。"),
            ("自定义 ACP", acp_description("fixture-agent")),
        ] {
            assert_help_card(&mut catalog, &mut renderer, context, title, description);
        }
        let positions = geometry(&mut catalog, &renderer);
        positions.label(i18n.__(setup_hint("fixture-agent")));
        positions.label(i18n.__(AcpDiagnosticStatus::AuthenticationRequired.description()));
        positions.label("/示例路徑/bin/fixture-agent");
        drop(catalog);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn help_rollout_09_mcp_server_and_policy_help() {
    if !isolated("help_rollout_09_mcp_server_and_policy_help") {
        return;
    }
    use crate::app::components::system_settings_agents;
    use crate::app::message::settings::McpMessage;
    use crate::app::state::{AGENT_DETAIL_MCP_TAB, AgentConfigTab, DelegateAgentSettingsEntry};
    let mut app = fixture();
    let mut renderer = renderer().await;
    for (locale, theme, width) in [
        (DesktopLocale::English, Theme::Dark, 420),
        (DesktopLocale::SimplifiedChinese, Theme::Light, 640),
        (DesktopLocale::TraditionalChinese, Theme::Dark, 520),
    ] {
        app.desktop_locale = locale;
        app.app_theme = theme.clone();
        let tr = |label| crate::app::i18n::settings_extensions::text(locale, label);
        let size = Size::new(width, 660);
        let context = (size, &theme, locale);
        app.mcp_settings.active_tab = McpSettingsTab::Servers;
        app.mcp_settings.new_server_expanded = true;
        app.mcp_settings.editing_server_index = Some(0);
        let mut edit = scene(mcp::view_overlays(&app, mcp::view(&app)), &mut renderer, size);
        assert_help_card(
            &mut edit,
            &mut renderer,
            context,
            "类型",
            "stdio · 命令 / 参数\nHTTP / SSE · URL",
        );
        let positions = geometry(&mut edit, &renderer);
        positions.input("fixture-bin");
        positions.label(tr("修改当前服务器；高级 JSON 字段会在传输类型不变时保留。"));
        assert!(matches!(
            click(&mut edit, &mut renderer, positions.label(tr("保存修改")).center()).as_slice(),
            [Message::Settings(SettingsMessage::Mcp(McpMessage::SaveServerEdit))]
        ));
        drop(edit);
        app.mcp_settings.editing_server_index = None;
        let mut add = scene(mcp::view_overlays(&app, mcp::view(&app)), &mut renderer, size);
        assert_help_card(
            &mut add,
            &mut renderer,
            context,
            "新增 MCP 服务器",
            "按服务器逐项添加；复杂字段可继续在高级 JSON 中编辑。",
        );
        assert_help_card(
            &mut add,
            &mut renderer,
            context,
            "类型",
            "stdio · 命令 / 参数\nHTTP / SSE · URL",
        );
        drop(add);

        app.mcp_settings.new_server_expanded = false;
        for (tab, title, description) in [
            (McpSettingsTab::Servers, "全局 MCP 服务器", "逐条启用、禁用和预览全局 mcpServers。"),
            (
                McpSettingsTab::Runtime,
                "运行时 MCP",
                "展示项目配置和插件声明合并后的 MCP 连接、工具、资源与 prompts。",
            ),
            (
                McpSettingsTab::Json,
                "高级 JSON",
                "支持 type=stdio/http/sse；可配置 headers、env、enabled、required、timeout、allowedTools、deniedTools。",
            ),
        ] {
            app.mcp_settings.active_tab = tab;
            let mut page = scene(mcp::view(&app), &mut renderer, size);
            if tab == McpSettingsTab::Servers {
                assert_help_card(
                    &mut page,
                    &mut renderer,
                    context,
                    "MCP 配置",
                    "配置全局 mcpServers；插件声明的 MCP 会随插件启用状态在运行时合并。",
                );
            }
            assert_help_card(&mut page, &mut renderer, context, title, description);
        }
        app.agents_settings.config_ready = true;
        app.agents_settings.active_config_tab = AgentConfigTab::Agents;
        app.agents_settings.selected_agent = "fixture-agent".into();
        app.agents_settings.profile_modal =
            Some(DelegateAgentSettingsEntry::from_config("fixture-agent", None));
        app.agents_settings.active_detail_tab = AGENT_DETAIL_MCP_TAB.into();
        app.mcp_settings.inspect_loading = true;
        let mut policy = scene(system_settings_agents::view(&app), &mut renderer, size);
        assert_help_card(
            &mut policy,
            &mut renderer,
            context,
            "默认 MCP",
            "继承当前会话工作区启用的服务；固定空列表不启用默认 MCP。这里只选择服务，不配置连接或凭据。",
        );
        let positions = geometry(&mut policy, &renderer);
        let title = positions.label(tr("默认 MCP"));
        scroll(&mut policy, &mut renderer, title.center(), -300.0);
        let positions = geometry(&mut policy, &renderer);
        positions.label(tr("正在加载服务目录；保留已有选择。"));
        let refresh = positions.label(tr("刷新服务目录")).center() - positions.scroll_offset;
        let messages = click(&mut policy, &mut renderer, refresh);
        assert!(
            matches!(
                messages.as_slice(),
                [Message::Settings(SettingsMessage::Mcp(McpMessage::InspectRefresh))]
            ),
            "{locale:?}: {messages:?}"
        );
    }
}
