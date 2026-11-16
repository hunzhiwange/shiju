//! Host contracts: settings tab routing, ACP directory projection, profile overlay and host message dispatch.
use super::AGENT_CONFIG_TABS;
use crate::app::i18n::settings_extensions::text as tr;
use crate::app::message::settings::{
    AgentConfigTabMessage, AgentsMessage as A, SettingsMessage as S,
};
use crate::app::state::{AgentConfigTab, DelegateAgentSettingsEntry};
use crate::app::{App, Message};
use config_agent_profile::AgentRuntime;
use config_ui::{DesktopLocale, GatewayClientSystemSettingsConfig};
use iced::advanced::{
    clipboard,
    renderer::{Headless, Style},
    widget::{
        Id, Operation,
        operation::{Focusable, Scrollable, TextInput},
    },
};
use iced::widget::text_editor;
use iced::{Element, Event, Font, Point, Rectangle, Renderer, Size, Theme, Vector, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};

const KEY: &str = "fixture-agent";
const BACKEND: &str = "fixture-backend";
const LOCALES: [(DesktopLocale, &str); 3] = [
    (DesktopLocale::SimplifiedChinese, "zh-CN"),
    (DesktopLocale::TraditionalChinese, "zh-TW"),
    (DesktopLocale::English, "en"),
];

fn isolated(test: &str) -> bool {
    const CHILD: &str = "SHIJU_UI_REFRESH_15_CHILD";
    if std::env::var_os(CHILD).is_some() {
        return true;
    }
    let home = tempfile::tempdir().unwrap();
    // App startup reads configuration synchronously. An invalid URL keeps those reads offline.
    let gateway = GatewayClientSystemSettingsConfig {
        host: "[".into(),
        servers: Vec::new(),
        ..Default::default()
    };
    std::fs::write(
        home.path().join("gateway-client-bootstrap.json"),
        serde_json::to_vec(&gateway).unwrap(),
    )
    .unwrap();
    let (_, module) = module_path!().split_once("::").unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", &format!("{module}::{test}"), "--test-threads=1"])
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
        "{}\n{}",
        stdout,
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
        .expect("tiny-skia renderer")
}

fn fixture() -> (App, tempfile::TempDir, DelegateAgentSettingsEntry) {
    assert_eq!(crate::app::config::gateway_client_endpoint().normalized_host(), "[");
    let root = tempfile::tempdir().unwrap();
    let mut app = App::new().0;
    app.navigation_tabs.project_path = Some(root.path().to_string_lossy().into_owned());
    app.agents_settings.config_ready = true;
    app.agents_settings.active_config_tab = AgentConfigTab::Agents;
    app.agents_settings.loading = false;
    app.agents_settings.selected_agent = KEY.into();
    app.acp_agents = (0..30).map(|index| format!("fixture-engine-{index:02}")).collect();
    app.acp_agents.push(BACKEND.into());

    let mut draft = DelegateAgentSettingsEntry::from_config(KEY, None);
    draft.enabled = true;
    draft.label = "示例智慧體 UnbrokenAgentDisplayNameForNarrowCards".into();
    draft.description = format!("只使用測試資料。 /{}/fixture", "long-test-directory".repeat(6));
    draft.description_editor = text_editor::Content::with_text(&draft.description);
    draft.recommended_prompts = vec!["第一项 / first".into(), "第二項 / second".into()];
    (app, root, draft)
}

#[derive(Default)]
struct Geometry {
    labels: Vec<(String, Rectangle)>,
    inputs: Vec<(String, Rectangle)>,
    focused: Vec<Rectangle>,
    focusable: Vec<Rectangle>,
    scrolls: Vec<(Rectangle, Rectangle, Vector)>,
}

impl Operation for Geometry {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn text(&mut self, _id: Option<&Id>, bounds: Rectangle, value: &str) {
        self.labels.push((value.into(), bounds));
    }

    fn text_input(&mut self, _id: Option<&Id>, bounds: Rectangle, state: &mut dyn TextInput) {
        self.inputs.push((state.text().into(), bounds));
    }

    fn focusable(&mut self, _id: Option<&Id>, bounds: Rectangle, state: &mut dyn Focusable) {
        self.focusable.push(bounds);
        if state.is_focused() {
            self.focused.push(bounds);
        }
    }

    fn scrollable(
        &mut self,
        _id: Option<&Id>,
        bounds: Rectangle,
        content: Rectangle,
        translation: Vector,
        _state: &mut dyn Scrollable,
    ) {
        self.scrolls.push((bounds, content, translation));
    }
}

impl Geometry {
    fn label(&self, value: &str) -> Rectangle {
        self.labels
            .iter()
            .find(|(label, _)| label == value)
            .unwrap_or_else(|| panic!("missing visible control label {value:?}"))
            .1
    }
}

struct Scene<'a> {
    ui: UserInterface<'a, Message, Theme, Renderer>,
    size: Size<u32>,
}

impl<'a> Scene<'a> {
    fn new(
        element: Element<'a, Message>,
        renderer: &mut Renderer,
        width: u32,
        height: u32,
    ) -> Self {
        let size = Size::new(width, height);
        let ui = UserInterface::build(
            element,
            Size::new(width as f32, height as f32),
            Cache::new(),
            renderer,
        );
        Self { ui, size }
    }

    fn geometry(&mut self, renderer: &Renderer) -> Geometry {
        let mut geometry = Geometry::default();
        self.ui.operate(renderer, &mut geometry);
        geometry
    }

    fn send(
        &mut self,
        renderer: &mut Renderer,
        events: &[Event],
        cursor: mouse::Cursor,
    ) -> Vec<Message> {
        let mut messages = Vec::new();
        self.ui.update(events, cursor, renderer, &mut clipboard::Null, &mut messages);
        messages.retain(|message| !matches!(message, Message::None));
        messages
    }

    fn click(&mut self, renderer: &mut Renderer, point: Point) -> Vec<Message> {
        self.send(
            renderer,
            &[
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            ],
            mouse::Cursor::Available(point),
        )
    }

    fn screenshot(
        &mut self,
        renderer: &mut Renderer,
        theme: &Theme,
        cursor: mouse::Cursor,
        name: &str,
    ) -> Vec<u8> {
        assert!(
            self.send(
                renderer,
                &[Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now()))],
                cursor,
            )
            .is_empty()
        );
        self.ui.draw(renderer, theme, &Style { text_color: theme.palette().text }, cursor);
        let pixels = renderer.screenshot(self.size, 1.0, theme.palette().background);
        if let Some(directory) = std::env::var_os("SHIJU_UI_REFRESH_15_SCREENSHOTS") {
            let directory = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&directory).unwrap();
            image::save_buffer(
                directory.join(format!("{name}.png")),
                &pixels,
                self.size.width,
                self.size.height,
                image::ColorType::Rgba8,
            )
            .unwrap();
        }
        pixels
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_15_host_tabs_and_errors() {
    if !isolated("ui_refresh_15_host_tabs_and_errors") {
        return;
    }
    let mut renderer = renderer().await;
    let (mut app, _root, mut draft) = fixture();
    draft.runtime = AgentRuntime::Acp { backend: BACKEND.into() };
    app.agents_settings.entries = vec![draft.clone()];
    for index in 0..12 {
        let mut other = draft.clone();
        other.key = format!("fixture-{index}");
        other.label = format!("样例智慧體 {index}");
        app.agents_settings.entries.push(other);
    }

    for (theme, theme_name) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
        for (locale, locale_name) in LOCALES {
            app.desktop_locale = locale;
            for width in [640, 960] {
                let mut page = Scene::new(super::view(&app), &mut renderer, width, 840);
                let geometry = page.geometry(&renderer);
                for tab in AGENT_CONFIG_TABS {
                    let label = geometry.label(tr(locale, tab.label));
                    assert!(label.x >= 0.0 && label.x + label.width <= width as f32);
                    assert!(matches!(
                        page.click(&mut renderer, label.center()).as_slice(),
                        [Message::Settings(S::AgentConfigTab(AgentConfigTabMessage::Selected(actual)))]
                        if *actual == tab.tab
                    ));
                }
            }
        }
    }

    for tab in AGENT_CONFIG_TABS {
        let _ = crate::app::message::settings::update(
            &mut app,
            S::AgentConfigTab(AgentConfigTabMessage::Selected(tab.tab)),
        );
        assert_eq!(app.agents_settings.active_config_tab, tab.tab);
        let mut page = Scene::new(super::view(&app), &mut renderer, 640, 840);
        page.screenshot(
            &mut renderer,
            &Theme::Light,
            mouse::Cursor::Unavailable,
            &format!("tab-{:?}", tab.tab),
        );
    }

    app.agents_settings.config_ready = false;
    app.agents_settings.save_error = Some("fixture load failed".into());
    let mut unavailable = Scene::new(super::view(&app), &mut renderer, 640, 840);
    let geometry = unavailable.geometry(&renderer);
    geometry.label("fixture load failed");
    assert!(geometry.inputs.is_empty());
    unavailable.screenshot(
        &mut renderer,
        &Theme::Light,
        mouse::Cursor::Unavailable,
        "catalog-unavailable",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agent_profile_modal_preserves_catalog_scroll() {
    if !isolated("agent_profile_modal_preserves_catalog_scroll") {
        return;
    }
    use crate::app::components::system_settings::{self, SystemTab};
    use crate::app::message::settings::update;

    let mut renderer = renderer().await;
    let (mut app, _root, draft) = fixture();
    app.desktop_locale = DesktopLocale::SimplifiedChinese;
    app.navigation_panels.show_system_settings = true;
    app.navigation_panels.system_settings_tab = SystemTab::Agents;
    app.agents_settings.entries = (0..16)
        .map(|index| DelegateAgentSettingsEntry {
            key: format!("fixture-{index}"),
            ..draft.clone()
        })
        .collect();

    let size = Size::new(1200.0, 740.0);
    let mut cache = Cache::new();
    let catalog_scroll = |geometry: &Geometry| {
        *geometry.scrolls.iter().find(|(bounds, _, _)| bounds.width > 500.0).unwrap()
    };
    let before = {
        let mut scene = Scene {
            ui: UserInterface::build(system_settings::view(&app), size, cache, &mut renderer),
            size: Size::new(1200, 740),
        };
        let (bounds, _, _) = catalog_scroll(&scene.geometry(&renderer));
        scene.send(
            &mut renderer,
            &[Event::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -200.0 },
            })],
            mouse::Cursor::Available(bounds.center()),
        );
        let geometry = scene.geometry(&renderer);
        let before = catalog_scroll(&geometry);
        assert!(before.2.y > 0.0);
        let messages = scene.click(&mut renderer, geometry.label("新增智能体").center());
        cache = scene.ui.into_cache();
        assert!(matches!(messages.as_slice(), [Message::Settings(S::Agents(A::AddAgentOpened))]));
        before
    };
    let _ = update(&mut app, S::Agents(A::AddAgentOpened));
    assert!(app.agents_settings.profile_entering_key);
    let _ = update(&mut app, S::Agents(A::AddAgentKeyChanged("new-profile".into())));
    {
        let mut scene = Scene {
            ui: UserInterface::build(system_settings::view(&app), size, cache, &mut renderer),
            size: Size::new(1200, 740),
        };
        let geometry = scene.geometry(&renderer);
        assert_eq!(geometry.inputs.len(), 1);
        assert_eq!(geometry.inputs[0].0, "new-profile");
        assert!((geometry.inputs[0].1.center().x - size.width / 2.0).abs() < 1.0);
        assert!(matches!(
            scene.click(&mut renderer, geometry.label("下一步").center()).as_slice(),
            [Message::Settings(S::Agents(A::AddAgentRequested))]
        ));
        scene.screenshot(&mut renderer, &Theme::Light, mouse::Cursor::Unavailable, "new-agent");
        cache = scene.ui.into_cache();
    }
    let _ = update(&mut app, S::Agents(A::AddAgentRequested));
    assert!(!app.agents_settings.profile_entering_key);
    assert_eq!(app.agents_settings.profile_modal.as_ref().unwrap().key, "new-profile");
    for theme in [Theme::Light, Theme::Dark] {
        let mut scene = Scene {
            ui: UserInterface::build(system_settings::view(&app), size, cache, &mut renderer),
            size: Size::new(1200, 740),
        };
        let geometry = scene.geometry(&renderer);
        let title = geometry.label("智能体配置");
        assert!(title.y < before.0.y);
        let save = geometry.label("保存配置");
        assert!(save.y > title.y && save.y + save.height <= size.height);
        scene.screenshot(&mut renderer, &theme, mouse::Cursor::Unavailable, "new-agent-profile");
        cache = scene.ui.into_cache();
    }
    let _ = update(&mut app, S::Agents(A::ProfileCancel));
    let mut scene = Scene {
        ui: UserInterface::build(system_settings::view(&app), size, cache, &mut renderer),
        size: Size::new(1200, 740),
    };
    assert_eq!(catalog_scroll(&scene.geometry(&renderer)), before);
}
