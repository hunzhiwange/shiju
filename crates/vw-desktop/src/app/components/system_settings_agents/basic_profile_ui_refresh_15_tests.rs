use super::super::{AGENT_CONFIG_TABS, catalog, profile_modal};
use crate::app::i18n::settings_extensions::text as tr;
use crate::app::message::settings::{
    AcpMessage, AgentConfigTabMessage, AgentsMessage as A, SettingsMessage as S,
};
use crate::app::state::{AgentConfigTab, DelegateAgentSettingsEntry};
use crate::app::{App, Message};
use iced::advanced::{
    clipboard, input_method,
    renderer::{Headless, Style},
    widget::{
        Id, Operation,
        operation::{Focusable, Scrollable, TextInput},
    },
};
use iced::keyboard::{self, Key, key::Named};
use iced::widget::{Space, text_editor};
use iced::{Element, Event, Font, Length, Point, Rectangle, Renderer, Size, Theme, Vector, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
use vw_config_types::agent::AgentRuntime;
use vw_config_types::ui::{DesktopLocale, GatewayClientSystemSettingsConfig};

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
    app.project_path = Some(root.path().to_string_lossy().into_owned());
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

fn key(named: Named) -> Event {
    let key = Key::Named(named);
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

fn profile(app: &App) -> Element<'_, Message> {
    profile_modal::overlay(app, Space::new().width(Length::Fill).height(Length::Fill).into())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_15_layout_and_interaction() {
    if !isolated("ui_refresh_15_layout_and_interaction") {
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
                let mut page = Scene::new(super::super::view(&app), &mut renderer, width, 840);
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

                let catalog_tab = geometry.label(tr(locale, "官方智能体"));
                assert!(matches!(
                    page.click(&mut renderer, catalog_tab.center()).as_slice(),
                    [Message::Settings(S::Agents(A::CatalogTabSelected(true)))]
                ));
                let add = geometry.label(tr(locale, "新建智能体"));
                assert!((add.center().y - geometry.inputs[1].1.center().y).abs() < 0.5);
                assert!(matches!(
                    page.click(&mut renderer, add.center()).as_slice(),
                    [Message::Settings(S::Agents(A::AddAgentRequested))]
                ));
                let edit = geometry.label(tr(locale, "编辑配置"));
                let start = geometry.label(tr(locale, "开始会话"));
                let copy = geometry.label(tr(locale, "复制"));
                let disable = geometry.label(tr(locale, "停用智能体"));
                let runtime = geometry.label(tr(locale, "运行设置"));
                assert!(matches!(
                    page.click(&mut renderer, copy.center()).as_slice(),
                    [Message::Settings(S::Agents(A::CatalogCopy(id)))] if id == KEY
                ));
                assert!(matches!(
                    page.click(&mut renderer, disable.center()).as_slice(),
                    [Message::Settings(S::Agents(A::CatalogToggle(id, false)))] if id == KEY
                ));
                assert!(matches!(
                    page.click(&mut renderer, runtime.center()).as_slice(),
                    [Message::Settings(S::Agents(A::CatalogRuntimeSettings(id)))] if id == BACKEND
                ));
                assert!(matches!(
                    page.click(&mut renderer, edit.center()).as_slice(),
                    [Message::Settings(S::Agents(A::ProfileOpen(id)))] if id == KEY
                ));
                assert!(matches!(
                    page.click(&mut renderer, start.center()).as_slice(),
                    [Message::Settings(S::Agents(A::CatalogLaunch(id)))] if id == KEY
                ));
                let (scroll, content, _) = geometry.scrolls[0];
                assert!(content.height > scroll.height);
                let cursor = mouse::Cursor::Available(edit.center());
                assert!(
                    page.send(
                        &mut renderer,
                        &[Event::Mouse(mouse::Event::CursorMoved { position: edit.center() })],
                        cursor
                    )
                    .is_empty()
                );
                page.screenshot(
                    &mut renderer,
                    &theme,
                    cursor,
                    &format!("catalog-{width}-{theme_name}-{locale_name}"),
                );
                assert!(
                    page.send(
                        &mut renderer,
                        &[Event::Mouse(mouse::Event::WheelScrolled {
                            delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -80.0 },
                        })],
                        cursor
                    )
                    .is_empty()
                );
                assert!(page.geometry(&renderer).scrolls[0].2.y > 0.0);
            }
        }
    }

    for tab in AGENT_CONFIG_TABS {
        let _ = crate::app::message::settings::update(
            &mut app,
            S::AgentConfigTab(AgentConfigTabMessage::Selected(tab.tab)),
        );
        assert_eq!(app.agents_settings.active_config_tab, tab.tab);
        let mut page = Scene::new(super::super::view(&app), &mut renderer, 640, 840);
        page.screenshot(
            &mut renderer,
            &Theme::Light,
            mouse::Cursor::Unavailable,
            &format!("tab-{:?}", tab.tab),
        );
    }

    app.agents_settings.active_config_tab = AgentConfigTab::Agents;
    app.agents_settings.catalog_query = "no-match".into();
    let mut empty = Scene::new(super::super::view(&app), &mut renderer, 640, 840);
    empty.geometry(&renderer).label("No matching agents");
    empty.screenshot(&mut renderer, &Theme::Dark, mouse::Cursor::Unavailable, "catalog-empty");
    drop(empty);

    app.agents_settings.config_ready = false;
    app.agents_settings.save_error = Some("fixture load failed".into());
    let mut unavailable = Scene::new(super::super::view(&app), &mut renderer, 640, 840);
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
async fn ui_refresh_15_preserves_values_and_disabled_states() {
    if !isolated("ui_refresh_15_preserves_values_and_disabled_states") {
        return;
    }
    let mut renderer = renderer().await;
    let (mut app, _root, draft) = fixture();
    app.desktop_locale = DesktopLocale::English;

    {
        let mut fields = Scene::new(super::view(&app, &draft), &mut renderer, 420, 760);
        let geometry = fields.geometry(&renderer);
        assert_eq!(geometry.inputs[0].0, draft.label);
        assert_eq!(geometry.inputs[1].0, draft.recommended_prompts[0]);
        let up = geometry.label("Move up");
        let down = geometry.label("Move down");
        assert_eq!(geometry.inputs[0].1.width, geometry.inputs[1].1.width);
        assert!(up.y > geometry.inputs[1].1.y + geometry.inputs[1].1.height);
        assert!((up.center().y - down.center().y).abs() < 0.5);
        let mut wide = Scene::new(super::view(&app, &draft), &mut renderer, 640, 760);
        let wide = wide.geometry(&renderer);
        for label in ["Move up", "Move down"] {
            assert!((wide.label(label).center().y - wide.inputs[1].1.center().y).abs() < 0.5);
        }
        assert!(fields.click(&mut renderer, up.center()).is_empty());
        assert!(matches!(
            fields.click(&mut renderer, down.center()).as_slice(),
            [Message::Settings(S::Agents(A::ProfilePromptMove(0, 1)))]
        ));
        let name = geometry.inputs[0].1.center();
        assert!(fields.click(&mut renderer, name).is_empty());
        assert!(matches!(
            fields.send(&mut renderer, &[key(Named::End), Event::InputMethod(input_method::Event::Commit("追加".into()))],
                mouse::Cursor::Available(name)).as_slice(),
            [Message::Settings(S::Agents(A::ProfileNameChanged(value)))] if *value == format!("{}追加", draft.label)
        ));

        let runtime = *geometry.focusable.last().unwrap();
        assert_eq!(runtime.height, 38.0);
        assert!(fields.click(&mut renderer, runtime.center()).is_empty());
        let overlay = fields.geometry(&renderer);
        let (menu, content, _) = *overlay.scrolls.last().unwrap();
        assert!(menu.y >= 0.0 && menu.y + menu.height <= 760.0);
        assert!(menu.x >= 0.0 && menu.x + menu.width <= 420.0);
        assert!(content.height > menu.height);
        assert!(overlay.focused.contains(&runtime));
        let mut hovers = Vec::new();
        for offset in [18.0, 54.0] {
            let point = Point::new(menu.center().x, menu.y + offset);
            let cursor = mouse::Cursor::Available(point);
            assert!(
                fields
                    .send(
                        &mut renderer,
                        &[Event::Mouse(mouse::Event::CursorMoved { position: point })],
                        cursor
                    )
                    .is_empty()
            );
            hovers.push(fields.screenshot(
                &mut renderer,
                &Theme::Dark,
                cursor,
                &format!("runtime-menu-hover-{offset}"),
            ));
        }
        assert!(hovers[0] != hovers[1], "hover must move between options");
        let outside = Point::new(4.0, 4.0);
        let cursor = mouse::Cursor::Available(outside);
        assert!(
            fields
                .send(
                    &mut renderer,
                    &[Event::Mouse(mouse::Event::CursorMoved { position: outside })],
                    cursor
                )
                .is_empty()
        );
        let cleared = fields.screenshot(&mut renderer, &Theme::Dark, cursor, "runtime-menu-clear");
        assert!(cleared != hovers[1], "moving off the menu must clear its hover");
        assert!(fields.send(&mut renderer, &[key(Named::Escape)], cursor).is_empty());
        assert!(fields.geometry(&renderer).scrolls.is_empty());
        assert!(fields.send(&mut renderer, &[key(Named::Enter)], cursor).is_empty());
        assert!(matches!(
            fields.send(&mut renderer, &[key(Named::End), key(Named::Enter)], cursor).as_slice(),
            [Message::Settings(S::Agents(A::ProfileRuntimeChanged(value)))] if value == BACKEND
        ));
        assert!(matches!(
            fields.click(&mut renderer, geometry.label("Refresh runtimes").center()).as_slice(),
            [Message::Settings(S::Acp(AcpMessage::Refresh))]
        ));
    }

    app.agents_settings.profile_modal = Some(draft.clone());
    for (theme, theme_name) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
        for (locale, locale_name) in LOCALES {
            app.desktop_locale = locale;
            let mut modal = Scene::new(profile(&app), &mut renderer, 420, 680);
            let geometry = modal.geometry(&renderer);
            let cancel = geometry.label(tr(locale, "取消"));
            let save = geometry.label(tr(locale, "保存配置"));
            let (body, content, _) = geometry.scrolls[0];
            assert!(content.height > body.height);
            assert!(body.y + body.height <= save.y);
            assert!(save.x + save.width <= 420.0);
            modal.screenshot(
                &mut renderer,
                &theme,
                mouse::Cursor::Unavailable,
                &format!("profile-new-{theme_name}-{locale_name}"),
            );
            assert!(
                modal
                    .send(
                        &mut renderer,
                        &[Event::Mouse(mouse::Event::WheelScrolled {
                            delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -120.0 },
                        })],
                        mouse::Cursor::Available(Point::new(body.x + 4.0, body.center().y))
                    )
                    .is_empty()
            );
            assert!(modal.geometry(&renderer).scrolls[0].2.y > 0.0);
            assert_eq!(modal.geometry(&renderer).label(tr(locale, "保存配置")), save);
            modal.screenshot(
                &mut renderer,
                &theme,
                mouse::Cursor::Unavailable,
                &format!("profile-scrolled-{theme_name}-{locale_name}"),
            );
            assert!(matches!(
                modal.click(&mut renderer, save.center()).as_slice(),
                [Message::Settings(S::Agents(A::ProfileSave))]
            ));
            assert!(matches!(
                modal.click(&mut renderer, cancel.center()).as_slice(),
                [Message::Settings(S::Agents(A::ProfileCancel))]
            ));
        }
    }

    app.agents_settings.profile_saving = true;
    app.agents_settings.catalog_busy = true;
    app.agents_settings.entries.push(draft.clone());
    let mut busy = Scene::new(profile(&app), &mut renderer, 420, 680);
    let geometry = busy.geometry(&renderer);
    for label in ["Cancel", "Delete agent", "Saving…"] {
        assert!(busy.click(&mut renderer, geometry.label(label).center()).is_empty());
    }
    busy.screenshot(&mut renderer, &Theme::Dark, mouse::Cursor::Unavailable, "profile-busy");
    drop(busy);
    let mut cards = Scene::new(catalog::grid(&app), &mut renderer, 640, 840);
    let geometry = cards.geometry(&renderer);
    for label in ["Edit", "Copy", "Disable", "Start session"] {
        assert!(cards.click(&mut renderer, geometry.label(label).center()).is_empty());
    }
    drop(cards);

    app.agents_settings.profile_saving = false;
    app.agents_settings.entries.clear();
    let mut builtin = DelegateAgentSettingsEntry::from_config("main", None);
    builtin.runtime = AgentRuntime::Acp { backend: BACKEND.into() };
    app.agents_settings.profile_modal = Some(builtin.clone());
    let mut fixed = Scene::new(super::view(&app, &builtin), &mut renderer, 420, 760);
    let geometry = fixed.geometry(&renderer);
    geometry.label("Fixed runtime: fixture-backend");
    assert!(!geometry.focusable.iter().any(|bounds| bounds.height == 38.0));
    fixed.screenshot(&mut renderer, &Theme::Light, mouse::Cursor::Unavailable, "profile-fixed");
    drop(fixed);

    app.acp_agents.clear();
    app.acp_settings.loading = true;
    let mut loading = Scene::new(super::view(&app, &draft), &mut renderer, 420, 760);
    let geometry = loading.geometry(&renderer);
    geometry.label("Loading the ACP runtime catalog…");
    assert!(loading.click(&mut renderer, geometry.label("Loading…").center()).is_empty());
    loading.screenshot(&mut renderer, &Theme::Dark, mouse::Cursor::Unavailable, "profile-loading");
    drop(loading);
    app.acp_settings.loading = false;
    app.acp_settings.save_error = Some("fixture runtime refresh failed".into());
    let mut failure = Scene::new(super::view(&app, &draft), &mut renderer, 420, 760);
    failure.geometry(&renderer).label("fixture runtime refresh failed");
    failure.screenshot(&mut renderer, &Theme::Light, mouse::Cursor::Unavailable, "profile-failed");
}
