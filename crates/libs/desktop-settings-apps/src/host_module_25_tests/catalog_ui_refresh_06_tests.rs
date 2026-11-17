use super::page_fixture::{self as system_settings_apps, *};
use super::page_fixture::{drag, launch, lifecycle, preferences};
use crate::system_settings_apps::catalog::CatalogRow;
use desktop_i18n::I18n;
use iced::advanced::renderer::{Headless, Style};
use iced::advanced::{
    Layout, clipboard, layout,
    widget::{Tree, operation::Operation},
};
use iced::keyboard::{self, Key, key::Named};
use iced::{Element, Event, Font, Point, Rectangle, Renderer, Size, Theme, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
use vw_api_app_identity::{
    AppCatalogEntry, AppHostPreferences, AppId, AppInstallation, AppInstallationEnvironment,
    AppInstallationId, AppInstallationRuntimeStatus, AppInstallationSource, AppPreferencesResult,
    AppReadResponse, AppRuntimeKind, AppUserPreferences,
};
use vw_api_app_operations::{AppActionPreviewRequest, AppActionTarget, AppLifecycleAction};
use {gateway_http::AgentAppsReadError, vw_api_app_identity::*};

fn view(state: &AppsCatalogState) -> Element<'_, Message> {
    localized_view(state, I18n::new(Default::default()))
}

fn catalog_row(id: AppId, name: String, command: &str) -> CatalogRow {
    CatalogRow {
        entry: AppCatalogEntry {
            id,
            name,
            cli_command: Some(command.into()),
            runtime_kinds: vec![AppRuntimeKind::NativeCli],
        },
        observation: Ok(AppReadResponse { installations: vec![], desktop: None }),
    }
}

fn fixture() -> AppsCatalogState {
    let mut installed = catalog_row(
        AppId::Codex,
        format!("虚构应用・虛構應用 {}", "UnbrokenFixtureName".repeat(4)),
        "fixture-command",
    );
    installed.observation = Ok(AppReadResponse {
        installations: vec![AppInstallation {
            id: AppInstallationId("fixture-installation".into()),
            app_id: AppId::Codex,
            target_host_id: "fixture-host".into(),
            environment: AppInstallationEnvironment::Wsl { distribution: "測試發行版".into() },
            path: format!("/隔离/應用/{}/fixture", "long-directory".repeat(12)),
            real_path: Some("/private/虚构/fixture".into()),
            source: AppInstallationSource::Homebrew,
            is_path_default: true,
            version: Some("1.2.3-fixture".into()),
            runtime_status: AppInstallationRuntimeStatus::Runnable,
        }],
        desktop: None,
    });
    let mut unverified = catalog_row(AppId::Gemini, "Unverified fixture".into(), "fixture-failed");
    unverified.observation = Err(AgentAppsReadError::Transport("fixture offline".into()));
    let rows = vec![
        installed,
        catalog_row(AppId::Claude, "待安裝應用".into(), "fixture-missing"),
        unverified,
    ];
    AppsCatalogState {
        host: "fixture-host".into(),
        expanded: Some(AppId::Codex),
        rows,
        ..Default::default()
    }
}

async fn renderer() -> Renderer {
    static FONTS: std::sync::Once = std::sync::Once::new();
    FONTS.call_once(|| {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in desktop_assets::fonts::load_all() {
            fonts.load_font(font);
        }
    });
    let font =
        if cfg!(target_os = "macos") { Font::with_name("Noto Sans CJK SC") } else { Font::DEFAULT };
    Renderer::new(font, 14.0.into(), Some("tiny-skia")).await.unwrap()
}

fn screenshot(
    renderer: &mut Renderer,
    element: Element<'_, Message>,
    theme: &Theme,
    width: u32,
    height: u32,
    name: &str,
) {
    let mut ui = UserInterface::build(
        element,
        Size::new(width as f32, height as f32),
        Cache::new(),
        renderer,
    );
    let cursor = mouse::Cursor::Unavailable;
    ui.update(
        &[iced::Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now()))],
        cursor,
        renderer,
        &mut iced::advanced::clipboard::Null,
        &mut Vec::new(),
    );
    ui.draw(renderer, theme, &Style { text_color: theme.palette().text }, cursor);
    save_screenshot(renderer, theme, Size::new(width, height), name);
}

fn save_screenshot(renderer: &mut Renderer, theme: &Theme, size: Size<u32>, name: &str) {
    let pixels = renderer.screenshot(size, 1.0, theme.palette().background);
    if let Some(directory) = std::env::var_os("SHIJU_SETTINGS_PREVIEW_DIR") {
        let file = std::path::PathBuf::from(directory).join(format!("{name}.png"));
        image::save_buffer(file, &pixels, size.width, size.height, image::ColorType::Rgba8)
            .unwrap();
    }
}

#[tokio::test]
async fn ui_refresh_06_renders_catalog_states_and_narrow_connection() {
    let mut renderer = renderer().await;
    let mut state = fixture();
    for (theme, width, name) in [(Theme::Light, 960, "light"), (Theme::Dark, 640, "dark")] {
        screenshot(&mut renderer, view(&state), &theme, width, 960, &format!("catalog-{name}"));
    }
    state.query = "  FIXTURE-MISSING  ".into();
    screenshot(&mut renderer, view(&state), &Theme::Light, 640, 640, "filtered");
    state.query = "unmatched".into();
    screenshot(&mut renderer, view(&state), &Theme::Dark, 640, 640, "no-results");
    state = AppsCatalogState::default();
    screenshot(&mut renderer, view(&state), &Theme::Light, 960, 640, "empty");
    state.loading = true;
    screenshot(&mut renderer, view(&state), &Theme::Dark, 640, 640, "loading");
    state.loading = false;
    state.error = Some("测试远端离线，请重试。測試遠端離線。".into());
    screenshot(&mut renderer, view(&state), &Theme::Light, 640, 640, "error");
    state.connection_open = true;
    state.host = "fixture-host".into();
    for (theme, name) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
        screenshot(
            &mut renderer,
            view_overlays(&state, I18n::new(Default::default()), &iced::Theme::Light, view(&state)),
            &theme,
            420,
            480,
            &format!("connection-{name}"),
        );
    }
}

struct Scene<'a> {
    ui: UserInterface<'a, Message, Theme, Renderer>,
    node: layout::Node,
}

impl<'a> Scene<'a> {
    fn new(mut element: Element<'a, Message>, renderer: &mut Renderer, size: Size) -> Self {
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(
            &mut tree,
            renderer,
            &layout::Limits::new(Size::ZERO, size),
        );
        let ui = UserInterface::build(element, size, Cache::new(), renderer);
        Self { ui, node }
    }

    fn bounds(&self, path: &[usize]) -> Rectangle {
        path.iter()
            .fold(Layout::new(&self.node), |node, &index| {
                node.children().nth(index).unwrap_or_else(|| panic!("missing layout path {path:?}"))
            })
            .bounds()
    }

    fn label(&mut self, renderer: &Renderer, value: &str) -> Rectangle {
        let mut labels = CatalogLabels::default();
        self.ui.operate(renderer, &mut labels);
        labels
            .0
            .into_iter()
            .find(|(label, _)| label == value)
            .unwrap_or_else(|| panic!("missing label {value:?}"))
            .1
    }

    fn send(
        &mut self,
        renderer: &mut Renderer,
        events: &[Event],
        point: Option<Point>,
    ) -> Vec<Message> {
        let mut messages = Vec::new();
        self.ui.update(
            events,
            point.map_or(mouse::Cursor::Unavailable, mouse::Cursor::Available),
            renderer,
            &mut clipboard::Null,
            &mut messages,
        );
        messages
    }

    fn click(&mut self, renderer: &mut Renderer, point: Point) -> Vec<Message> {
        self.send(
            renderer,
            &[
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            ],
            Some(point),
        )
    }

    fn hover(&mut self, renderer: &mut Renderer, point: Point) -> Vec<Message> {
        self.send(
            renderer,
            &[Event::Mouse(mouse::Event::CursorMoved { position: point })],
            Some(point),
        )
    }

    fn draw(&mut self, renderer: &mut Renderer, theme: &Theme, point: Option<Point>) {
        let redraw =
            Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now()));
        assert!(self.send(renderer, &[redraw], point).is_empty());
        self.ui.draw(
            renderer,
            theme,
            &Style { text_color: theme.palette().text },
            point.map_or(mouse::Cursor::Unavailable, mouse::Cursor::Available),
        );
    }
}

fn key(key: Named) -> Event {
    let key = Key::Named(key);
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

fn ime(text: &str) -> Event {
    Event::InputMethod(iced::advanced::input_method::Event::Commit(text.into()))
}

fn catalog_messages(messages: Vec<Message>) -> Vec<AppsCatalogMessage> {
    messages
        .into_iter()
        .map(|message| match message {
            Message::Settings(SettingsMessage::AppsCatalog(value)) => value,
            other => panic!("unexpected message {other:?}"),
        })
        .collect()
}

const SEARCH: &[usize] = &[0, 2, 0, 0, 0, 1];
const REFRESH: &[usize] = &[0, 0, 1];
const SETTINGS: &[usize] = &[0, 0, 2];
const SUMMARY: &[usize] = &[0, 4, 1, 0, 0, 0];

#[tokio::test]
async fn ui_refresh_06_layout_and_interaction() {
    let mut renderer = renderer().await;
    let mut state = fixture();
    state.preferences.saved = Some(AppPreferencesResult {
        user: AppUserPreferences {
            app_order: vec!["codex".into(), "claude".into(), "gemini".into()],
            visible_apps: vec![AppId::Codex, AppId::Claude, AppId::Gemini],
        },
        host: AppHostPreferences::default(),
        restart_required: false,
        capabilities: None,
    });
    let mut scene = Scene::new(view(&state), &mut renderer, Size::new(640.0, 1800.0));
    let name = scene.bounds(&[SUMMARY, &[2, 0]].concat());
    let actions = scene.bounds(&[SUMMARY, &[3]].concat());
    let summary = scene.bounds(SUMMARY);
    assert!(name.height > 32.0, "long name should wrap over multiple lines");
    assert!(name.x + name.width < actions.x);
    assert!(actions.x + actions.width <= summary.x + summary.width);
    assert!((name.y - summary.y).abs() < 0.5);
    let launch = scene.label(&renderer, "启动");
    let brand = scene.bounds(&[SUMMARY, &[1]].concat());
    assert!((actions.center_y() - brand.center_y()).abs() < 0.5);
    let search = scene.bounds(SEARCH);
    let search_text = scene.bounds(&[SEARCH, &[0]].concat());
    let search_icon = scene.bounds(&[0, 2, 0, 0, 0, 0]);
    assert!((search.center_y() - search_text.center_y()).abs() < 0.5);
    assert!((search.center_y() - search_icon.center_y()).abs() < 0.5);

    let theme = Theme::Dark;
    scene.draw(&mut renderer, &theme, Some(search.center()));
    save_screenshot(&mut renderer, &theme, Size::new(640, 1800), "catalog-interaction");

    let launch_point = launch.center();
    assert!(scene.hover(&mut renderer, launch_point).is_empty());
    assert!(matches!(
        catalog_messages(scene.click(&mut renderer, launch_point)).as_slice(),
        [AppsCatalogMessage::Launch(launch::LaunchMessage::Open(AppId::Codex, id))]
            if id.0 == "fixture-installation"
    ));
    let toggle = scene.label(&renderer, "已选择").center();
    assert!(matches!(
        catalog_messages(scene.hover(&mut renderer, toggle)).as_slice(),
        [AppsCatalogMessage::Drag(drag::DragMessage::Hover(AppId::Codex))]
    ));
    assert!(scene.click(&mut renderer, toggle).is_empty());
    let grip = scene.bounds(&[SUMMARY, &[0]].concat()).center();
    assert!(scene.hover(&mut renderer, grip).is_empty());
    assert!(matches!(
        catalog_messages(scene.click(&mut renderer, grip)).as_slice(),
        [AppsCatalogMessage::Drag(drag::DragMessage::Start(AppId::Codex))]
    ));
    assert!(matches!(
        catalog_messages(scene.hover(&mut renderer, search.center())).as_slice(),
        [AppsCatalogMessage::Drag(drag::DragMessage::Leave(AppId::Codex))]
    ));
    let path =
        scene.label(&renderer, &state.rows[0].observation.as_ref().unwrap().installations[0].path);
    let update = scene.label(&renderer, "检查升级");
    assert!(path.height > 26.0 && path.y + path.height < update.y);
    assert!(matches!(
        catalog_messages(scene.click(&mut renderer, update.center())).as_slice(),
        [AppsCatalogMessage::Lifecycle(LifecycleMessage::Preview(AppActionPreviewRequest {
            app_id: AppId::Codex,
            action: AppLifecycleAction::Update,
            target: AppActionTarget::Installation { installation_id },
        }))] if installation_id.0 == "fixture-installation"
    ));
    assert!(scene.hover(&mut renderer, search.center()).is_empty());
    assert!(scene.click(&mut renderer, search.center()).is_empty());
    assert!(matches!(
        catalog_messages(scene.send(&mut renderer, &[ime("測試篩選")], Some(search.center()))).as_slice(),
        [AppsCatalogMessage::QueryInput(value)] if value == "測試篩選"
    ));
    assert!(
        scene
            .send(&mut renderer, &[key(Named::Enter), key(Named::Escape)], Some(search.center()))
            .is_empty()
    );

    drop(scene);
    state.query = "  FIXTURE-MISSING  ".into();
    let mut scene = Scene::new(view(&state), &mut renderer, Size::new(640.0, 640.0));
    let content = Layout::new(&scene.node).children().next().unwrap();
    assert_eq!(content.children().count(), 6, "only one status group matches the command");
    assert!(matches!(
        catalog_messages(scene.click(&mut renderer, scene.bounds(&[0, 4, 1, 0, 0, 0, 3]).center()))
            .as_slice(),
        [AppsCatalogMessage::ToggleDetails(AppId::Claude)]
    ));
    drop(scene);

    state.expanded = Some(AppId::Claude);
    let mut scene = Scene::new(view(&state), &mut renderer, Size::new(640.0, 1200.0));
    let install = scene.label(&renderer, "Linux 安装预检").center();
    assert!(matches!(
        catalog_messages(scene.click(&mut renderer, install)).as_slice(),
        [AppsCatalogMessage::Lifecycle(LifecycleMessage::Preview(AppActionPreviewRequest {
            app_id: AppId::Claude,
            action: AppLifecycleAction::Install,
            target: AppActionTarget::Environment { environment: AppInstallationEnvironment::Linux },
        }))]
    ));
    drop(scene);

    state.lifecycle.busy = true;
    let mut scene = Scene::new(view(&state), &mut renderer, Size::new(640.0, 1200.0));
    let install = scene.label(&renderer, "Linux 安装预检").center();
    assert!(scene.click(&mut renderer, install).is_empty());
}

#[tokio::test]
async fn ui_refresh_06_preserves_values_and_disabled_states() {
    let mut renderer = renderer().await;
    let mut state = fixture();
    let size = Size::new(640.0, 960.0);
    let mut scene = Scene::new(view(&state), &mut renderer, size);
    assert!(matches!(
        catalog_messages(scene.click(&mut renderer, scene.bounds(REFRESH).center())).as_slice(),
        [AppsCatalogMessage::Refresh]
    ));
    drop(scene);
    state.loading = true;
    let mut scene = Scene::new(view(&state), &mut renderer, size);
    for target in [REFRESH, SETTINGS] {
        assert!(scene.click(&mut renderer, scene.bounds(target).center()).is_empty());
    }

    drop(scene);
    state.loading = false;
    state.drag.source = Some(AppId::Codex);
    let mut scene = Scene::new(view(&state), &mut renderer, size);
    for target in [REFRESH, SETTINGS] {
        assert!(scene.click(&mut renderer, scene.bounds(target).center()).is_empty());
    }
    let primary = scene.label(&renderer, "启动").center();
    let toggle = scene.label(&renderer, "已选择").center();
    assert!(scene.hover(&mut renderer, primary).is_empty());
    assert!(scene.click(&mut renderer, primary).is_empty());
    assert!(matches!(
        catalog_messages(scene.hover(&mut renderer, toggle)).as_slice(),
        [AppsCatalogMessage::Drag(drag::DragMessage::Hover(AppId::Codex))]
    ));
    assert!(scene.click(&mut renderer, toggle).is_empty());

    drop(scene);
    state.drag = Default::default();
    // Legacy connection state must not expose a separate credentials form.
    state.connection_open = true;
    for (size, theme) in
        [(Size::new(420.0, 960.0), Theme::Dark), (Size::new(960.0, 640.0), Theme::Light)]
    {
        let mut scene = Scene::new(
            view_overlays(&state, I18n::new(Default::default()), &theme, view(&state)),
            &mut renderer,
            size,
        );
        let mut labels = CatalogLabels::default();
        scene.ui.operate(&renderer, &mut labels);
        assert!(
            !labels.0.iter().any(|(label, _)| matches!(
                label.as_str(),
                "高级连接" | "选择执行主机" | "加载应用"
            ))
        );
        let search = scene.bounds(SEARCH).center();
        assert!(scene.click(&mut renderer, search).is_empty());
        assert!(matches!(
            catalog_messages(scene.send(&mut renderer, &[ime("測試篩選")], Some(search))).as_slice(),
            [AppsCatalogMessage::QueryInput(value)] if value == "測試篩選"
        ));
        scene.draw(&mut renderer, &theme, Some(search));
    }
}

#[derive(Default)]
struct CatalogLabels(Vec<(String, Rectangle)>);

impl Operation for CatalogLabels {
    fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) {
        children(self);
    }

    fn text(&mut self, _: Option<&iced::advanced::widget::Id>, bounds: Rectangle, value: &str) {
        self.0.push((value.into(), bounds));
    }
}

#[tokio::test]
async fn settings_help_gap_14_app_catalog() {
    use config_ui::DesktopLocale;

    let mut renderer = renderer().await;
    let mut state = fixture();
    state.expanded = None;
    for (locale, theme, width) in [
        (DesktopLocale::SimplifiedChinese, Theme::Dark, 420),
        (DesktopLocale::English, Theme::Light, 760),
        (DesktopLocale::TraditionalChinese, Theme::Dark, 760),
    ] {
        let i18n = I18n::new(locale);
        for (dragging, syncing, notice) in [
            (false, false, None),
            (true, false, Some("松开鼠标完成排序 · Esc 取消")),
            (false, true, Some("正在同步设置…")),
        ] {
            state.drag.source = dragging.then_some(AppId::Codex);
            state.preferences.busy = syncing;
            let size = Size::new(width, 960);
            let mut scene = Scene::new(
                localized_view(&state, i18n),
                &mut renderer,
                Size::new(size.width as f32, size.height as f32),
            );
            scene.draw(&mut renderer, &theme, None);
            let mut labels = CatalogLabels::default();
            scene.ui.operate(&renderer, &mut labels);
            assert!(labels.0.iter().any(|(value, _)| value == "打开详情查看检测结果"));
            assert_eq!(labels.0.iter().filter(|(value, _)| value == "1").count(), 3);
            if let Some(text) = notice {
                assert!(labels.0.iter().any(|(value, _)| value == text));
            }
            for (title, explanation) in
                [("已安装", "拖动左侧手柄可调整顺序"), ("待安装", "发现更多适合你的工具")]
            {
                let title = i18n.__(title);
                let translated = i18n.__(explanation);
                if locale != DesktopLocale::SimplifiedChinese {
                    assert_ne!(translated, explanation, "{locale:?}: missing help translation");
                }
                assert!(
                    !labels.0.iter().any(|(value, _)| value == explanation || value == translated),
                    "{title}: explanation must be absent from the idle view"
                );
                let bounds = labels.0.iter().find(|(label, _)| label == title).unwrap().1;
                if dragging {
                    continue;
                }
                let icon = Point::new(bounds.x + bounds.width + 15.0, bounds.center_y());
                scene.draw(&mut renderer, &theme, None);
                let idle = renderer.screenshot(size, 1.0, theme.palette().background);
                assert!(scene.hover(&mut renderer, icon).is_empty());
                scene.draw(&mut renderer, &theme, Some(icon));
                assert!(idle != renderer.screenshot(size, 1.0, theme.palette().background));
                assert!(scene.click(&mut renderer, icon).is_empty());
                scene.draw(&mut renderer, &theme, None);
                assert!(idle != renderer.screenshot(size, 1.0, theme.palette().background));

                if notice.is_none() {
                    let menu = Point::new(icon.x + 29.0, icon.y + 5.0);
                    assert!(
                        scene
                            .send(
                                &mut renderer,
                                &[Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right))],
                                Some(menu),
                            )
                            .is_empty()
                    );
                    let item_height = f32::from(
                        iced::advanced::text::LineHeight::default().to_absolute(12.0.into()),
                    ) + 10.0;
                    let copy = Point::new(menu.x + 24.0, menu.y + 3.0 + item_height * 0.5);
                    assert!(matches!(
                        scene.click(&mut renderer, copy).as_slice(),
                        [Message::CopyCode(value)] if value == translated
                    ));
                }
                assert!(scene.send(&mut renderer, &[key(Named::Escape)], None).is_empty());
                scene.draw(&mut renderer, &theme, None);
                assert!(idle == renderer.screenshot(size, 1.0, theme.palette().background));
            }
            if notice.is_none() {
                assert!(matches!(
                    catalog_messages(scene.click(&mut renderer, scene.bounds(REFRESH).center()))
                        .as_slice(),
                    [AppsCatalogMessage::Refresh]
                ));
            }
        }
    }
}
