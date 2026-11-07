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
    const CHILD: &str = "SHIJU_UI_REFRESH_20_CHILD";
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
    if let Some(directory) = std::env::var_os("SHIJU_UI_REFRESH_20_SCREENSHOTS") {
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

const DB_URL: &str = "postgres://fixture.invalid/資料庫/記憶/長目錄/long-fixture-database-name";
const QDRANT_KEY: &str = "fixture-secret";
const QDRANT_URL: &str = "https://fixture.invalid/qdrant";

fn fixture() -> App {
    assert_eq!(crate::app::config::gateway_client_endpoint().normalized_host(), "[");
    let mut app = App::new().0;
    app.memory_settings.active_tab = MemorySettingsTab::General;
    app.memory_settings.backend = "qdrant".into();
    app.memory_settings.qdrant_url_input = QDRANT_URL.into();
    app.memory_settings.qdrant_api_key_input = QDRANT_KEY.into();
    app.storage_settings.provider = "postgres".into();
    app.storage_settings.schema = "fixture_schema".into();
    app.storage_settings.table = "fixture_memories".into();
    app.storage_settings.tls = false;
    app.storage_settings.db_url_input = DB_URL.into();
    app.storage_settings.connect_timeout_secs_input = "30".into();
    app
}

fn render_pages(app: &mut App, renderer: &mut Renderer) {
    for (locale, theme, width, tag) in [
        (DesktopLocale::SimplifiedChinese, Theme::Light, 640, "zh-cn-light"),
        (DesktopLocale::TraditionalChinese, Theme::Dark, 420, "zh-tw-dark"),
        (DesktopLocale::English, Theme::Dark, 960, "en-dark"),
    ] {
        app.desktop_locale = locale;
        for (tab, name) in [
            (MemorySettingsTab::General, "general"),
            (MemorySettingsTab::RetentionAndCache, "retention"),
            (MemorySettingsTab::Embedding, "embedding"),
            (MemorySettingsTab::Storage, "storage"),
            (MemorySettingsTab::Qdrant, "qdrant"),
            (MemorySettingsTab::EmbeddingRoutes, "routes"),
        ] {
            app.memory_settings.active_tab = tab;
            snapshot(view(app), renderer, &theme, Size::new(width, 1024), &format!("{name}-{tag}"));
        }
    }
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

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_20_narrow_tabs_stay_reachable() {
    if !isolated("ui_refresh_20_narrow_tabs_stay_reachable") {
        return;
    }
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::TraditionalChinese;
    let mut renderer = renderer().await;
    let size = Size::new(420, 1024);
    let mut ui = scene(view(&app), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    let tab = positions.label(memory_tab_label(app.desktop_locale, MemorySettingsTab::Qdrant));
    assert!(
        tab.width >= 70.0 && tab.height < 30.0 && tab.x + tab.width <= size.width as f32,
        "Qdrant tab is clipped or wraps inside its button: {tab:?}"
    );
    assert!(matches!(
        click(&mut ui, &mut renderer, tab.center()).as_slice(),
        [Message::Settings(SettingsMessage::Memory(MemoryMessage::TabSelected(
            MemorySettingsTab::Qdrant
        )))]
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_20_layout_and_interaction() {
    if !isolated("ui_refresh_20_layout_and_interaction") {
        return;
    }
    use iced::keyboard::key::Named;
    let mut renderer = renderer().await;
    let mut app = fixture();
    render_pages(&mut app, &mut renderer);

    app.desktop_locale = DesktopLocale::SimplifiedChinese;
    app.memory_settings.active_tab = MemorySettingsTab::General;
    app.memory_settings.backend = "sqlite".into();
    let size = Size::new(640, 460);
    let mut ui = scene(view(&app), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    let qdrant_tab = memory_tab_label(app.desktop_locale, MemorySettingsTab::Qdrant);
    assert!(!positions.labels.iter().any(|(label, _)| label == qdrant_tab));

    let trigger = positions.focusable[0];
    let label = positions.label("存储后端");
    let hint = positions.label("选择记忆系统使用的持久化后端。");
    assert!((trigger.center_y() - (label.y + hint.y + hint.height) / 2.0).abs() < 0.5);
    assert!((trigger.height - 38.0).abs() < 0.5);
    assert!(click(&mut ui, &mut renderer, trigger.center()).is_empty());

    let mut hovered = Vec::new();
    for (index, offset) in [20.0, 56.0].into_iter().enumerate() {
        let point = iced::Point::new(trigger.center_x(), trigger.y + trigger.height + offset);
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
            &Theme::Light,
            size,
            &format!("backend-hover-{index}"),
            cursor,
        ));
    }
    let outside = iced::Point::new(4.0, 4.0);
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
    let unhovered =
        capture(&mut ui, &mut renderer, &Theme::Light, size, "backend-hover-left", cursor);
    assert_ne!(hovered[0], hovered[1]);
    assert!(hovered.iter().all(|pixels| pixels != &unhovered));

    let menu_point = iced::Point::new(trigger.center_x(), trigger.y + trigger.height + 56.0);
    assert!(
        send(
            &mut ui,
            &mut renderer,
            mouse::Cursor::Available(menu_point),
            &[Event::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -180.0 },
            })],
        )
        .is_empty()
    );
    assert_ne!(
        unhovered,
        capture(&mut ui, &mut renderer, &Theme::Light, size, "backend-menu-scrolled", cursor),
        "scroll should reveal the last backend"
    );

    let cursor = mouse::Cursor::Unavailable;
    assert!(send(&mut ui, &mut renderer, cursor, &[key(Named::Escape)]).is_empty());
    assert!(send(&mut ui, &mut renderer, cursor, &[key(Named::Enter)]).is_empty());
    assert!(
        send(&mut ui, &mut renderer, cursor, &[key(Named::ArrowDown), key(Named::ArrowDown)])
            .is_empty()
    );
    let messages = send(&mut ui, &mut renderer, cursor, &[key(Named::Enter)]);
    let [
        Message::Settings(settings @ SettingsMessage::Memory(MemoryMessage::BackendChanged(value))),
    ] = messages.as_slice()
    else {
        panic!("expected exactly one backend change");
    };
    assert_eq!(value, "qdrant");
    drop(ui);
    // The update is synchronous; drop the returned save task without executing it.
    drop(crate::app::message::settings::update(&mut app, settings.clone()));
    assert_eq!(app.memory_settings.backend, "qdrant");
    let mut ui = scene(view(&app), &mut renderer, size);
    let position = geometry(&mut ui, &renderer).label(qdrant_tab);
    assert!(matches!(
        click(&mut ui, &mut renderer, position.center()).as_slice(),
        [Message::Settings(SettingsMessage::Memory(MemoryMessage::TabSelected(
            MemorySettingsTab::Qdrant
        )))]
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_20_preserves_values_and_disabled_states() {
    if !isolated("ui_refresh_20_preserves_values_and_disabled_states") {
        return;
    }
    use crate::app::message::settings::{StorageMessage, update};
    use iced::keyboard::key::Named;
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::English;
    let size = Size::new(640, 800);
    let mut renderer = renderer().await;

    app.memory_settings.active_tab = MemorySettingsTab::Storage;
    let mut ui = scene(view(&app), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    let url = positions.input(DB_URL);
    assert!(url.width > 300.0, "{url:?}");
    let trigger = positions.focusable[0];
    assert!(click(&mut ui, &mut renderer, trigger.center()).is_empty());
    assert!(
        send(&mut ui, &mut renderer, mouse::Cursor::Unavailable, &[key(Named::ArrowDown)])
            .is_empty()
    );
    let messages = send(&mut ui, &mut renderer, mouse::Cursor::Unavailable, &[key(Named::Enter)]);
    let [
        Message::Settings(
            settings @ SettingsMessage::Storage(StorageMessage::ProviderChanged(value)),
        ),
    ] = messages.as_slice()
    else {
        panic!("expected exactly one provider change");
    };
    assert_eq!(value, "mariadb");
    drop(ui);
    drop(update(&mut app, settings.clone()));

    let mut ui = scene(view(&app), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    positions.input(DB_URL);
    positions.input("fixture_schema");
    positions.input("fixture_memories");
    let timeout = positions.input("30");
    let messages = edit(&mut ui, &mut renderer, timeout.center(), "x");
    let [
        Message::Settings(
            settings @ SettingsMessage::Storage(StorageMessage::ConnectTimeoutSecsChanged(value)),
        ),
    ] = messages.as_slice()
    else {
        panic!("expected exactly one timeout edit");
    };
    assert_eq!(value, "30x");
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
    drop(update(&mut app, settings.clone()));

    let mut ui = scene(view(&app), &mut renderer, size);
    let error =
        geometry(&mut ui, &renderer).label("connect_timeout_secs must be whole seconds or blank");
    assert!(error.width > 200.0 && error.y < 220.0);
    capture(
        &mut ui,
        &mut renderer,
        &Theme::Dark,
        size,
        "storage-validation-error",
        mouse::Cursor::Unavailable,
    );
    drop(ui);

    app.memory_settings.active_tab = MemorySettingsTab::RetentionAndCache;
    app.memory_settings.archive_after_days = 3650;
    let mut ui = scene(view(&app), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    let archive = positions.input("3650");
    assert!(
        (archive.center_y()
            - positions.label(runtime_text(app.desktop_locale, "归档天数")).center_y())
        .abs()
            < 0.5
    );
    assert!(matches!(
        edit(&mut ui, &mut renderer, archive.center(), "9").as_slice(),
        [Message::Settings(SettingsMessage::Memory(MemoryMessage::ArchiveAfterDaysChanged(3650)))]
    ));
    drop(ui);

    app.memory_settings.active_tab = MemorySettingsTab::Qdrant;
    let mut ui = scene(view(&app), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    positions.input(QDRANT_URL);
    assert!(matches!(
        edit(&mut ui, &mut renderer, positions.input(QDRANT_KEY).center(), "-updated").as_slice(),
        [Message::Settings(SettingsMessage::Memory(MemoryMessage::QdrantApiKeyChanged(value)))] if value == "fixture-secret-updated"
    ));
    drop(ui);

    drop(update(&mut app, SettingsMessage::Memory(MemoryMessage::BackendChanged("sqlite".into()))));
    let mut ui = scene(view(&app), &mut renderer, size);
    assert!(
        geometry(&mut ui, &renderer).inputs.is_empty(),
        "Qdrant fields must no longer be editable"
    );
    assert_eq!(app.memory_settings.qdrant_url_input, QDRANT_URL);
    assert_eq!(app.memory_settings.qdrant_api_key_input, QDRANT_KEY);
}

#[tokio::test(flavor = "multi_thread")]
async fn settings_tabs_core_appearance_memory() {
    use super::super::system_settings_common::tab_appearance_tests;

    tab_appearance_tests::run(
        concat!(module_path!(), "::settings_tabs_core_appearance_memory"),
        fixture,
        |scene, app| {
            let locale = app.desktop_locale;
            let qdrant_label = memory_tab_label(locale, MemorySettingsTab::Qdrant);
            app.memory_settings.backend = "sqlite".into();
            assert!(!scene.labels(view(app)).contains(qdrant_label));
            app.memory_settings.backend = "qdrant".into();
            let labels: Vec<_> = MEMORY_TABS
                .iter()
                .chain(QDRANT_TAB)
                .map(|tab| memory_tab_label(locale, tab.tab))
                .collect();
            let content = runtime_text(locale, "Qdrant 地址");
            assert!(!scene.labels(view(app)).contains(content));
            let message = scene.check(app, view(app), &labels, labels.len() - 1);
            assert!(matches!(
                message,
                SettingsMessage::Memory(MemoryMessage::TabSelected(MemorySettingsTab::Qdrant))
            ));
            let _ = crate::app::message::settings::update(app, message);
            assert_eq!(app.memory_settings.active_tab, MemorySettingsTab::Qdrant);
            assert!(scene.labels(view(app)).contains(content));
        },
    )
    .await;
}
