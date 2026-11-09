use super::super::{drag, launch, view as localized_view, view_overlays};
use super::CatalogRow;
use crate::app::i18n::I18n;
use crate::app::{
    Message,
    message::settings::{AppsCatalogMessage, SettingsMessage, apps_lifecycle::LifecycleMessage},
    state::AppsCatalogState,
};
use iced::advanced::renderer::{Headless, Style};
use iced::advanced::{
    Layout, clipboard, layout,
    widget::{
        Tree,
        operation::{Operation, Outcome, Scrollable, focusable},
    },
};
use iced::keyboard::{self, Key, key::Named};
use iced::{Element, Event, Font, Point, Rectangle, Renderer, Size, Theme, Vector, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
use vw_gateway_client::{AgentAppsReadError, vw_api_types::agent_apps::*};

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
        for font in crate::fonts::load_all() {
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

    fn focus_next(&mut self, renderer: &Renderer) {
        let mut operation: Box<dyn Operation> = Box::new(focusable::focus_next());
        loop {
            self.ui.operate(renderer, operation.as_mut());
            match operation.finish() {
                Outcome::Chain(next) => operation = next,
                _ => break,
            }
        }
    }
}

#[derive(Default)]
struct BodyOffset(f32);

impl Operation for BodyOffset {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        _id: Option<&iced::advanced::widget::Id>,
        bounds: Rectangle,
        content: Rectangle,
        translation: Vector,
        _state: &mut dyn Scrollable,
    ) {
        assert!(bounds.height < content.height, "body must overflow the short viewport");
        self.0 = translation.y;
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

const SEARCH: &[usize] = &[0, 1, 0, 0, 0, 1];
const REFRESH: &[usize] = &[0, 0, 1];
const SETTINGS: &[usize] = &[0, 0, 2];
const SUMMARY: &[usize] = &[0, 2, 1, 0, 0, 0, 0];
const CONNECTION: &[usize] = &[1, 1, 0, 0];
const CONNECTION_FOOTER: &[usize] = &[1, 1, 0, 0, 0, 2];

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
    let mut scene = Scene::new(view(&state), &mut renderer, Size::new(640.0, 960.0));
    let name = scene.bounds(&[SUMMARY, &[2, 0]].concat());
    let actions = scene.bounds(&[SUMMARY, &[3]].concat());
    let summary = scene.bounds(SUMMARY);
    assert!(name.height > 32.0, "long name should wrap over multiple lines");
    assert!(name.x + name.width < actions.x);
    assert!(actions.x + actions.width <= summary.x + summary.width);
    assert!((name.y - summary.y).abs() < 0.5);
    let launch = scene.bounds(&[SUMMARY, &[3, 1]].concat());
    let brand = scene.bounds(&[SUMMARY, &[1]].concat());
    assert!((launch.center_y() - brand.center_y()).abs() < 0.5);
    let search = scene.bounds(SEARCH);
    let search_text = scene.bounds(&[SEARCH, &[0]].concat());
    let search_icon = scene.bounds(&[0, 1, 0, 0, 0, 0]);
    assert!((search.center_y() - search_text.center_y()).abs() < 0.5);
    assert!((search.center_y() - search_icon.center_y()).abs() < 0.5);

    let connection = scene.bounds(&[0, 1, 0, 0, 0, 2]).center();
    assert!(scene.hover(&mut renderer, connection).is_empty());
    let theme = Theme::Dark;
    scene.draw(&mut renderer, &theme, Some(connection));
    save_screenshot(&mut renderer, &theme, Size::new(640, 960), "connection-tooltip");
    assert!(scene.hover(&mut renderer, search.center()).is_empty());
    assert!(matches!(
        catalog_messages(scene.click(&mut renderer, connection)).as_slice(),
        [AppsCatalogMessage::ConnectionOpen]
    ));

    let launch_point = launch.center();
    assert!(matches!(
        catalog_messages(scene.hover(&mut renderer, launch_point)).as_slice(),
        [AppsCatalogMessage::Drag(drag::DragMessage::Hover(AppId::Codex))]
    ));
    assert!(matches!(
        catalog_messages(scene.click(&mut renderer, launch_point)).as_slice(),
        [AppsCatalogMessage::Launch(launch::LaunchMessage::Open(AppId::Codex, id))]
            if id.0 == "fixture-installation"
    ));
    let toggle = scene.bounds(&[SUMMARY, &[3, 2]].concat()).center();
    assert!(matches!(
        catalog_messages(scene.click(&mut renderer, toggle)).as_slice(),
        [AppsCatalogMessage::ToggleDetails(AppId::Codex)]
    ));
    let grip = scene.bounds(&[SUMMARY, &[0]].concat()).center();
    assert!(matches!(
        catalog_messages(scene.click(&mut renderer, grip)).as_slice(),
        [AppsCatalogMessage::Drag(drag::DragMessage::Start(AppId::Codex))]
    ));
    let details: &[usize] = &[0, 2, 1, 0, 0, 2, 0, 3];
    let path = scene.bounds(&[details, &[1, 1]].concat());
    let update = scene.bounds(&[details, &[3, 1]].concat());
    assert!(path.height > 26.0 && path.y + path.height < update.y);
    assert!(matches!(
        catalog_messages(scene.click(&mut renderer, update.center())).as_slice(),
        [AppsCatalogMessage::Lifecycle(LifecycleMessage::Preview(AppActionPreviewRequest {
            app_id: AppId::Codex,
            action: AppLifecycleAction::Update,
            target: AppActionTarget::Installation { installation_id },
        }))] if installation_id.0 == "fixture-installation"
    ));
    assert!(matches!(
        catalog_messages(scene.hover(&mut renderer, search.center())).as_slice(),
        [AppsCatalogMessage::Drag(drag::DragMessage::Leave(AppId::Codex))]
    ));
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
    assert_eq!(content.children().count(), 3, "only one status group matches the command");
    assert!(matches!(
        catalog_messages(
            scene.click(&mut renderer, scene.bounds(&[SUMMARY, &[3, 1]].concat()).center())
        )
        .as_slice(),
        [AppsCatalogMessage::ToggleDetails(AppId::Claude)]
    ));
    drop(scene);

    state.expanded = Some(AppId::Claude);
    let install_button = &[0, 2, 1, 0, 0, 2, 0, 2, 1];
    let mut scene = Scene::new(view(&state), &mut renderer, Size::new(640.0, 640.0));
    let install = scene.bounds(install_button).center();
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
    let mut scene = Scene::new(view(&state), &mut renderer, Size::new(640.0, 640.0));
    assert!(scene.click(&mut renderer, scene.bounds(install_button).center()).is_empty());
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
    let primary = scene.bounds(&[SUMMARY, &[3, 1]].concat()).center();
    let toggle = scene.bounds(&[SUMMARY, &[3, 2]].concat()).center();
    assert!(matches!(
        catalog_messages(scene.hover(&mut renderer, primary)).as_slice(),
        [AppsCatalogMessage::Drag(drag::DragMessage::Hover(AppId::Codex))]
    ));
    assert!(scene.click(&mut renderer, primary).is_empty());
    assert!(scene.click(&mut renderer, toggle).is_empty());

    drop(scene);
    state.drag = Default::default();
    state.connection_open = true;
    for size in [Size::new(420.0, 480.0), Size::new(960.0, 640.0)] {
        let scene = Scene::new(
            view_overlays(&state, I18n::new(Default::default()), &iced::Theme::Light, view(&state)),
            &mut renderer,
            size,
        );
        let card = scene.bounds(CONNECTION);
        assert!((card.width - (size.width - 24.0).min(520.0)).abs() < 0.5);
        assert!((card.center_x() - size.width / 2.0).abs() < 0.5);
        assert!((card.center_y() - size.height / 2.0).abs() < 0.5);
    }
    let modal_size = Size::new(420.0, 250.0);
    let mut scene = Scene::new(
        view_overlays(&state, I18n::new(Default::default()), &iced::Theme::Light, view(&state)),
        &mut renderer,
        modal_size,
    );
    let card = scene.bounds(CONNECTION);
    assert!(card.x >= 12.0 && card.x + card.width <= modal_size.width - 12.0);
    let scroll = scene.bounds(&[CONNECTION, &[0, 1]].concat());
    let footer = scene.bounds(CONNECTION_FOOTER);
    assert!(scroll.y + scroll.height < footer.y);
    let confirm = scene.bounds(&[CONNECTION_FOOTER, &[2]].concat()).center();
    let cancel = scene.bounds(&[CONNECTION_FOOTER, &[1]].concat()).center();
    assert!(scene.click(&mut renderer, confirm).is_empty(), "empty token disables loading");
    scene.focus_next(&renderer);
    assert!(matches!(
        catalog_messages(scene.send(&mut renderer, &[ime("-測試")], None)).as_slice(),
        [AppsCatalogMessage::HostInput(value)] if value == "fixture-host-測試"
    ));
    assert!(scene.send(&mut renderer, &[key(Named::Enter), key(Named::Escape)], None).is_empty());
    let mut offset = BodyOffset::default();
    scene.ui.operate(&renderer, &mut offset);
    assert_eq!(offset.0, 0.0);
    let scroll_point = Point::new(scroll.x + 3.0, scroll.center_y());
    assert!(scene.hover(&mut renderer, scroll_point).is_empty());
    let scroll_event = Event::Mouse(mouse::Event::WheelScrolled {
        delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -160.0 },
    });
    assert!(scene.send(&mut renderer, &[scroll_event], Some(scroll_point)).is_empty());
    scene.ui.operate(&renderer, &mut offset);
    assert!(offset.0 > 0.0, "wheel must move the modal body");
    let theme = Theme::Dark;
    scene.draw(&mut renderer, &theme, None);
    save_screenshot(&mut renderer, &theme, Size::new(420, 250), "connection-scrolled");
    assert!(matches!(
        catalog_messages(scene.click(&mut renderer, cancel)).as_slice(),
        [AppsCatalogMessage::ConnectionClose]
    ));
    let mut roomy = Scene::new(
        view_overlays(&state, I18n::new(Default::default()), &iced::Theme::Light, view(&state)),
        &mut renderer,
        Size::new(960.0, 640.0),
    );
    let refresh = roomy.bounds(&[&[0][..], REFRESH].concat()).center();
    assert!(!roomy.bounds(CONNECTION).contains(refresh));
    assert!(roomy.click(&mut renderer, refresh).is_empty());
    drop(roomy);
    let close = scene.bounds(&[CONNECTION, &[0, 0, 1]].concat()).center();
    assert!(matches!(
        catalog_messages(scene.click(&mut renderer, close)).as_slice(),
        [AppsCatalogMessage::ConnectionClose]
    ));

    drop(scene);
    state.token = "fake-fixture-token".into();
    let mut scene = Scene::new(
        view_overlays(&state, I18n::new(Default::default()), &iced::Theme::Light, view(&state)),
        &mut renderer,
        modal_size,
    );
    assert!(matches!(
        catalog_messages(scene.click(&mut renderer, confirm)).as_slice(),
        [AppsCatalogMessage::Refresh]
    ));
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
    use vw_config_types::ui::DesktopLocale;

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
