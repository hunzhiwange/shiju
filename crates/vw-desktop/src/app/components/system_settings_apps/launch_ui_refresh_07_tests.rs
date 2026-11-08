use super::super::{catalog::CatalogRow, preferences};
use crate::app::i18n::I18n;
use crate::app::{
    Message,
    message::settings::{AppsCatalogMessage, SettingsMessage, apps_lifecycle::LifecycleMessage},
    state::AppsCatalogState,
};
use iced::advanced::renderer::{Headless, Style};
use iced::advanced::widget::operation::{Operation, Scrollable};
use iced::advanced::{Layout, Shell, clipboard, layout, widget::Tree};
use iced::keyboard::{self, Key, key::Named};
use iced::{Element, Event, Font, Length, Point, Rectangle, Renderer, Size, Theme, Vector, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
use vw_gateway_client::vw_api_types::agent_apps::*;

fn default_i18n() -> I18n {
    I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese)
}

fn fixture() -> AppsCatalogState {
    let mut state =
        AppsCatalogState { host: "fixture-虚构-測試-host".into(), ..Default::default() };
    state.rows = [(AppId::Codex, "测试应用"), (AppId::Hermes, "虛構應用 Hermes")]
        .into_iter()
        .map(|(id, name)| CatalogRow {
            entry: AppCatalogEntry {
                id,
                name: name.into(),
                cli_command: None,
                runtime_kinds: vec![AppRuntimeKind::NativeCli],
            },
            observation: Ok(AppReadResponse { installations: vec![], desktop: None }),
        })
        .collect();
    state.preferences.open = true;
    state.preferences.draft = Some(AppPreferencesResult {
        user: AppUserPreferences {
            app_order: vec!["codex".into(), "hermes".into()],
            visible_apps: vec![AppId::Codex],
        },
        host: AppHostPreferences {
            preferred_terminal: Some("terminal-fixture".into()),
            wsl_shells: vec![AppWslPreference {
                app_id: AppId::Codex,
                shell: Some("fixture-shell".into()),
                flag: Some("-lc".into()),
            }],
        },
        restart_required: false,
        capabilities: Some(AppPreferencesCapabilities {
            terminals: vec!["terminal-fixture".into(), "terminal-測試".into()],
            wsl_preferences: true,
        }),
    });
    state
}

fn base() -> Element<'static, Message> {
    iced::widget::button(iced::widget::text("Underlying application catalog"))
        .width(Length::Fill)
        .height(Length::Fill)
        .on_press(super::super::message(AppsCatalogMessage::Refresh))
        .into()
}

async fn renderer() -> Renderer {
    static FONTS: std::sync::Once = std::sync::Once::new();
    FONTS.call_once(|| {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    });
    Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
        .await
        .unwrap()
}

fn at<'a>(node: &'a layout::Node, path: &[usize]) -> Layout<'a> {
    path.iter().fold(Layout::new(node), |node, &index| {
        node.children().nth(index).unwrap_or_else(|| panic!("missing layout path {path:?}"))
    })
}

fn capture(element: Element<'_, Message>, renderer: &mut Renderer, size: Size<u32>, name: &str) {
    let mut ui = UserInterface::build(
        element,
        Size::new(size.width as f32, size.height as f32),
        Cache::new(),
        renderer,
    );
    for (theme, suffix) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
        let cursor = mouse::Cursor::Unavailable;
        ui.update(
            &[Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now()))],
            cursor,
            renderer,
            &mut clipboard::Null,
            &mut Vec::new(),
        );
        ui.draw(renderer, &theme, &Style { text_color: theme.palette().text }, cursor);
        save(renderer, &theme, size, &format!("{name}-{suffix}"));
    }
}

fn save(renderer: &mut Renderer, theme: &Theme, size: Size<u32>, name: &str) {
    let pixels = renderer.screenshot(size, 1.0, theme.palette().background);
    if let Some(directory) = std::env::var_os("SHIJU_SETTINGS_PREVIEW_DIR") {
        image::save_buffer(
            std::path::PathBuf::from(directory).join(format!("{name}.png")),
            &pixels,
            size.width,
            size.height,
            image::ColorType::Rgba8,
        )
        .unwrap();
    }
}

#[tokio::test]
async fn ui_refresh_07_busy_terminal_does_not_open_a_menu() {
    let mut renderer = renderer().await;
    let mut state = fixture();
    state.preferences.busy = true;
    capture(
        preferences::overlay(&state, default_i18n(), &Theme::Light, base()),
        &mut renderer,
        Size::new(420, 280),
        "busy-preferences",
    );
    let size = Size::new(640.0, 640.0);
    let viewport = Rectangle::with_size(size);
    let mut element = preferences::overlay(&state, default_i18n(), &Theme::Light, base());
    let mut tree = Tree::new(element.as_widget());
    let widget = element.as_widget_mut();
    let node = widget.layout(&mut tree, &renderer, &layout::Limits::new(Size::ZERO, size));
    let bounds = at(&node, &[1, 1, 0, 0, 0, 1, 0, 0, 4, 1, 1]).bounds();
    let mut messages = Vec::new();
    for event in [
        mouse::Event::ButtonPressed(mouse::Button::Left),
        mouse::Event::ButtonReleased(mouse::Button::Left),
    ] {
        widget.update(
            &mut tree,
            &Event::Mouse(event),
            Layout::new(&node),
            mouse::Cursor::Available(bounds.center()),
            &renderer,
            &mut clipboard::Null,
            &mut Shell::new(&mut messages),
            &viewport,
        );
    }
    assert!(messages.is_empty());
    assert!(
        widget.overlay(&mut tree, Layout::new(&node), &renderer, &viewport, Vector::ZERO).is_none(),
        "terminal must not open its menu while saving"
    );
}

fn launch_fixture() -> AppsCatalogState {
    let mut state = AppsCatalogState { host: "fixture-测试主機".into(), ..Default::default() };
    let identity = LaunchIdentity {
        kind: LaunchIdentityKind::Provider,
        id: "provider-fixture".into(),
        label: "虚构供應商 Fixture".into(),
    };
    state.launch.request = Some(LaunchRequest {
        app_id: AppId::Codex,
        installation_id: AppInstallationId("fixture-instance".into()),
        cwd: format!("/隔离/測試/{}", "long-directory/".repeat(16)),
        args: vec![],
        skip_permissions: false,
    });
    state.launch.selected = Some(super::LaunchChoice::Identity(identity.clone()));
    state.launch.identities = vec![
        identity,
        LaunchIdentity {
            kind: LaunchIdentityKind::OfficialAccount,
            id: "account-fixture".into(),
            label: "測試帳號 Fixture".into(),
        },
    ];
    state.launch.identities_ready = true;
    state.launch.args = vec!["--literal=简体/繁體".into(), "argument with spaces".into()];
    state
}

fn lifecycle_fixture(action: AppLifecycleAction) -> AppsCatalogState {
    let mut state =
        AppsCatalogState { host: "fixture-虚构-測試-host".into(), ..Default::default() };
    let request = AppActionPreviewRequest {
        app_id: AppId::Hermes,
        action,
        target: if action == AppLifecycleAction::Install {
            AppActionTarget::Environment { environment: AppInstallationEnvironment::Linux }
        } else {
            AppActionTarget::Installation {
                installation_id: AppInstallationId("fixture-instance".into()),
            }
        },
    };
    state.lifecycle.request = Some(request.clone());
    state.lifecycle.target_label =
        format!("目标路径：/隔离/測試/{}\n来源：fixture", "long-path/".repeat(16));
    state.lifecycle.preview = Some(AppActionPreview {
        preview_id: AppActionPreviewId("preview-fixture".into()),
        request,
        source: AppInstallationSource::System,
        confirmation_required: true,
        summary: "虚构预检摘要\nVerify the target and source.\n請核對目標。".repeat(6),
        display_command: None,
    });
    state
}

#[tokio::test]
async fn ui_refresh_07_renders_modal_states() {
    let mut renderer = renderer().await;
    let mut launch = launch_fixture();
    for (width, height) in [(420, 280), (640, 640), (960, 640)] {
        capture(
            super::overlay(&launch, default_i18n(), &Theme::Light, base()),
            &mut renderer,
            Size::new(width, height),
            &format!("launch-{width}"),
        );
    }
    launch.launch.identities_loading = true;
    launch.launch.identities_ready = false;
    launch.launch.selected = None;
    launch.launch.identities.clear();
    capture(
        super::overlay(&launch, default_i18n(), &Theme::Light, base()),
        &mut renderer,
        Size::new(420, 350),
        "launch-loading",
    );
    launch.launch.identities_loading = false;
    launch.launch.identities_ready = true;
    launch.launch.error = Some("虚构错误：无法读取目录；請重新選擇。 Fixture failure. ".repeat(8));
    capture(
        super::overlay(&launch, default_i18n(), &Theme::Light, base()),
        &mut renderer,
        Size::new(420, 350),
        "launch-error",
    );

    let mut preferences = fixture();
    capture(
        super::super::preferences::overlay(&preferences, default_i18n(), &Theme::Light, base()),
        &mut renderer,
        Size::new(420, 400),
        "preferences",
    );
    preferences.rows.clear();
    preferences.preferences.draft.as_mut().unwrap().capabilities = None;
    capture(
        super::super::preferences::overlay(&preferences, default_i18n(), &Theme::Light, base()),
        &mut renderer,
        Size::new(640, 400),
        "preferences-empty",
    );

    for action in
        [AppLifecycleAction::Install, AppLifecycleAction::Update, AppLifecycleAction::Uninstall]
    {
        let state = lifecycle_fixture(action);
        capture(
            super::super::lifecycle::overlay(&state.lifecycle, &state.host, base()),
            &mut renderer,
            Size::new(420, 280),
            &format!("{action:?}"),
        );
    }
}

const CARD: &[usize] = &[1, 1, 0, 0];

struct Scene<'a> {
    ui: UserInterface<'a, Message, Theme, Renderer>,
    node: layout::Node,
    size: Size,
}

impl<'a> Scene<'a> {
    fn new(mut element: Element<'a, Message>, renderer: &mut Renderer, size: Size) -> Self {
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(
            &mut tree,
            renderer,
            &layout::Limits::new(Size::ZERO, size),
        );
        Self { ui: UserInterface::build(element, size, Cache::new(), renderer), node, size }
    }

    fn bounds(&self, path: &[usize]) -> Rectangle {
        at(&self.node, &[CARD, path].concat()).bounds()
    }

    fn send(
        &mut self,
        renderer: &mut Renderer,
        events: &[Event],
        point: Point,
    ) -> Vec<AppsCatalogMessage> {
        let mut messages = Vec::new();
        self.ui.update(
            events,
            mouse::Cursor::Available(point),
            renderer,
            &mut clipboard::Null,
            &mut messages,
        );
        messages
            .into_iter()
            .map(|message| match message {
                Message::Settings(SettingsMessage::AppsCatalog(message)) => message,
                other => panic!("unexpected message {other:?}"),
            })
            .collect()
    }

    fn click(&mut self, renderer: &mut Renderer, path: &[usize]) -> Vec<AppsCatalogMessage> {
        self.send(
            renderer,
            &[
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            ],
            self.bounds(path).center(),
        )
    }

    fn append_text(
        &mut self,
        renderer: &mut Renderer,
        point: Point,
        suffix: &str,
    ) -> Vec<AppsCatalogMessage> {
        let click = [
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        ];
        assert!(self.send(renderer, &click, point).is_empty());
        let input = Event::InputMethod(iced::advanced::input_method::Event::Commit(suffix.into()));
        self.send(renderer, &[key(Named::End), input], point)
    }

    fn hover(&mut self, renderer: &mut Renderer, point: Point) {
        assert!(
            self.send(
                renderer,
                &[Event::Mouse(mouse::Event::CursorMoved { position: point })],
                point
            )
            .is_empty()
        );
    }

    fn draw(&mut self, renderer: &mut Renderer, theme: &Theme, point: Point, name: &str) {
        assert!(
            self.send(
                renderer,
                &[Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now()))],
                point
            )
            .is_empty()
        );
        self.ui.draw(
            renderer,
            theme,
            &Style { text_color: theme.palette().text },
            mouse::Cursor::Available(point),
        );
        save(renderer, theme, Size::new(self.size.width as u32, self.size.height as u32), name);
    }
}

fn key(named: Named) -> Event {
    let key = Key::Named(named);
    Event::Keyboard(keyboard::Event::KeyPressed {
        key: key.clone(),
        modified_key: key,
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
        assert!(bounds.height < content.height, "short window must overflow only the body");
        self.0 = translation.y;
    }
}

#[tokio::test]
async fn ui_refresh_07_layout_and_interaction() {
    use super::super::lifecycle;
    let mut renderer = renderer().await;
    let size = Size::new(420.0, 280.0);
    for action in
        [AppLifecycleAction::Install, AppLifecycleAction::Update, AppLifecycleAction::Uninstall]
    {
        let state = lifecycle_fixture(action);
        let mut scene = Scene::new(
            lifecycle::overlay(&state.lifecycle, &state.host, base()),
            &mut renderer,
            size,
        );
        let card = scene.bounds(&[]);
        assert_eq!(card.width, size.width - 24.0);
        assert!((card.center_x() - size.width / 2.0).abs() < 0.5);
        assert!((card.center_y() - size.height / 2.0).abs() < 0.5);
        let header = scene.bounds(&[0, 0]);
        let close = scene.bounds(&[0, 0, 1]);
        assert!((header.center_y() - close.center_y()).abs() < 0.5);
        let scroll = scene.bounds(&[0, 1]);
        let footer = scene.bounds(&[0, 2]);
        let content = scene.bounds(&[0, 1, 0, 0]);
        assert!(header.y + header.height < scroll.y);
        assert!(scroll.y + scroll.height < footer.y);
        assert!(footer.y + footer.height < card.y + card.height);
        assert!((content.x - scroll.x - 7.0).abs() < 0.5);
        assert!(scroll.x + scroll.width - content.x - content.width >= 7.0);
        let mut offset = BodyOffset::default();
        scene.ui.operate(&renderer, &mut offset);
        assert_eq!(offset.0, 0.0);
        let scroll_point = scroll.center();
        let wheel = Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -160.0 },
        });
        assert!(scene.send(&mut renderer, &[wheel], scroll_point).is_empty());
        scene.ui.operate(&renderer, &mut offset);
        assert!(offset.0 > 0.0, "wheel must scroll the preview body");
        scene.hover(&mut renderer, scene.bounds(&[0, 2, 2]).center());
        scene.draw(&mut renderer, &Theme::Dark, scroll_point, &format!("{action:?}-scrolled-dark"));
        assert!(matches!(
            scene.click(&mut renderer, &[0, 2, 1]).as_slice(),
            [AppsCatalogMessage::Lifecycle(LifecycleMessage::Close)]
        ));
        assert!(matches!(
            scene.click(&mut renderer, &[0, 2, 2]).as_slice(),
            [AppsCatalogMessage::Lifecycle(LifecycleMessage::Confirm)]
        ));
    }
}

#[tokio::test]
async fn ui_refresh_07_preserves_launch_identity_and_literal_parameters() {
    use super::LaunchMessage as Launch;
    let mut renderer = renderer().await;
    let mut state = launch_fixture();
    let size = Size::new(640.0, 640.0);
    let mut scene = Scene::new(
        super::overlay(&state, default_i18n(), &Theme::Light, base()),
        &mut renderer,
        size,
    );
    let select_path = &[0, 1, 0, 0, 2, 1];
    let select = scene.bounds(select_path);
    let center = select.center();
    let icon = scene.bounds(&[select_path.as_slice(), &[0, 1]].concat());
    assert!((select.center_y() - icon.center_y()).abs() < 0.5);
    let arg = &[0, 1, 0, 0, 5, 1, 0];
    let remove = &[0, 1, 0, 0, 5, 1, 1];
    assert!((scene.bounds(arg).center_y() - scene.bounds(remove).center_y()).abs() < 0.5);

    assert!(scene.click(&mut renderer, select_path).is_empty());
    // Each call rebuilds the menu overlay, as the runtime does between update and draw.
    let first = Point::new(select.center_x(), select.y + select.height + 18.0);
    let second = Point::new(first.x, first.y + 36.0);
    scene.hover(&mut renderer, first);
    scene.draw(&mut renderer, &Theme::Light, first, "launch-menu-hover-first");
    scene.hover(&mut renderer, second);
    scene.draw(&mut renderer, &Theme::Dark, second, "launch-menu-hover-second");
    scene.hover(&mut renderer, Point::ORIGIN);
    assert!(scene.send(&mut renderer, &[key(Named::Escape)], center).is_empty());
    assert!(scene.send(&mut renderer, &[key(Named::Enter)], center).is_empty());
    assert!(matches!(
        scene.send(&mut renderer, &[key(Named::End), key(Named::Enter)], center).as_slice(),
        [AppsCatalogMessage::Launch(Launch::Select(super::LaunchChoice::Identity(identity)))]
            if identity.id == "account-fixture" && identity.kind == LaunchIdentityKind::OfficialAccount
    ));

    let cwd = scene.bounds(&[0, 1, 0, 0, 3, 1]).center();
    assert!(matches!(
        scene.append_text(&mut renderer, cwd, "子目录").as_slice(),
        [AppsCatalogMessage::Launch(Launch::Cwd(value))]
            if value == &format!("{}子目录", state.launch.request.as_ref().unwrap().cwd)
    ));
    assert!(matches!(
        scene.append_text(&mut renderer, scene.bounds(arg).center(), " '測試' ").as_slice(),
        [AppsCatalogMessage::Launch(Launch::Args(0, value))] if value == "--literal=简体/繁體 '測試' "
    ));
    assert!(matches!(
        scene.click(&mut renderer, remove).as_slice(),
        [AppsCatalogMessage::Launch(Launch::RemoveArg(0))]
    ));
    assert!(matches!(
        scene.click(&mut renderer, &[0, 2, 1, 1]).as_slice(),
        [AppsCatalogMessage::Launch(Launch::Close)]
    ));
    assert!(matches!(
        scene.click(&mut renderer, &[0, 2, 1, 2]).as_slice(),
        [AppsCatalogMessage::Launch(Launch::Submit)]
    ));
    drop(scene);

    state.launch.busy = true;
    let mut scene = Scene::new(
        super::overlay(&state, default_i18n(), &Theme::Light, base()),
        &mut renderer,
        size,
    );
    assert!(scene.click(&mut renderer, &[0, 2, 1, 2]).is_empty());
}

#[tokio::test]
async fn ui_refresh_07_preserves_preferences_and_disabled_states() {
    use preferences::PreferencesMessage as Prefs;
    let mut renderer = renderer().await;
    let mut state = fixture();
    let size = Size::new(640.0, 640.0);
    let mut scene = Scene::new(
        preferences::overlay(&state, default_i18n(), &Theme::Light, base()),
        &mut renderer,
        size,
    );
    let select_path = &[0, 1, 0, 0, 3, 1, 1];
    assert!(scene.click(&mut renderer, select_path).is_empty());
    let select = scene.bounds(select_path).center();
    assert!(matches!(
        scene.send(&mut renderer, &[key(Named::Home), key(Named::Enter)], select).as_slice(),
        [AppsCatalogMessage::Preferences(Prefs::Terminal(value))] if value.is_empty()
    ));
    assert!(scene.click(&mut renderer, select_path).is_empty());
    assert!(matches!(
        scene.send(&mut renderer, &[key(Named::End), key(Named::Enter)], select).as_slice(),
        [AppsCatalogMessage::Preferences(Prefs::Terminal(value))] if value == "terminal-測試"
    ));
    assert!(matches!(
        scene.click(&mut renderer, &[0, 1, 0, 0, 1, 2, 1]).as_slice(),
        [AppsCatalogMessage::Preferences(Prefs::MoveUp(id))] if id == "hermes"
    ));
    assert!(matches!(
        scene.click(&mut renderer, &[0, 1, 0, 0, 1, 2, 0]).as_slice(),
        [AppsCatalogMessage::Preferences(Prefs::Visible(AppId::Hermes, true))]
    ));
    let scroll = scene.bounds(&[0, 1]).center();
    let wheel = Event::Mouse(mouse::Event::WheelScrolled {
        delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -240.0 },
    });
    assert!(scene.send(&mut renderer, &[wheel], scroll).is_empty());
    let mut offset = BodyOffset::default();
    scene.ui.operate(&renderer, &mut offset);
    scene.draw(&mut renderer, &Theme::Dark, scroll, "preferences-wsl-scrolled");
    let shell = scene.bounds(&[0, 1, 0, 0, 3, 3, 0, 1, 1]).center() - Vector::new(0.0, offset.0);
    let flag = scene.bounds(&[0, 1, 0, 0, 3, 3, 0, 2, 1]).center() - Vector::new(0.0, offset.0);
    assert!(matches!(
        scene.append_text(&mut renderer, shell, "-測試").as_slice(),
        [AppsCatalogMessage::Preferences(Prefs::Shell(AppId::Codex, value))] if value == "fixture-shell-測試"
    ));
    assert!(matches!(
        scene.append_text(&mut renderer, flag, "-fixture").as_slice(),
        [AppsCatalogMessage::Preferences(Prefs::Flag(AppId::Codex, value))] if value == "-lc-fixture"
    ));
    assert!(matches!(
        scene.click(&mut renderer, &[0, 2, 1]).as_slice(),
        [AppsCatalogMessage::Preferences(Prefs::Close)]
    ));
    assert!(matches!(
        scene.click(&mut renderer, &[0, 2, 2]).as_slice(),
        [AppsCatalogMessage::Preferences(Prefs::SaveUser)]
    ));
    assert!(matches!(
        scene.click(&mut renderer, &[0, 2, 3]).as_slice(),
        [AppsCatalogMessage::Preferences(Prefs::SaveHost)]
    ));
    drop(scene);

    state.preferences.busy = true;
    let mut scene = Scene::new(
        preferences::overlay(&state, default_i18n(), &Theme::Light, base()),
        &mut renderer,
        size,
    );
    for index in 1..=3 {
        assert!(scene.click(&mut renderer, &[0, 2, index]).is_empty());
    }

    let mut state = lifecycle_fixture(AppLifecycleAction::Uninstall);
    state.lifecycle.busy = true;
    let mut scene = Scene::new(
        super::super::view_overlays(&state, default_i18n(), &Theme::Light, base()),
        &mut renderer,
        size,
    );
    assert!(scene.click(&mut renderer, &[0, 2, 2]).is_empty());
    drop(scene);
    state.lifecycle.busy = false;
    state.lifecycle.preview.as_mut().unwrap().source = AppInstallationSource::Unknown;
    let mut scene = Scene::new(
        super::super::view_overlays(&state, default_i18n(), &Theme::Light, base()),
        &mut renderer,
        size,
    );
    assert!(scene.click(&mut renderer, &[0, 2, 2]).is_empty());
}

#[tokio::test]
async fn help_rollout_04_launch_renders_contextual_help() {
    use super::super::preferences_tests::{HELP_LOCALES, verify_help};

    let mut state = launch_fixture();
    state.launch.error = Some("Fixture launch error".into());
    for mode in &HELP_LOCALES {
        let messages = verify_help(
            super::super::view_overlays(&state, I18n::new(mode.0), &mode.1, base()),
            "launch-help",
            mode,
            &[("附加参数", "每项一个参数，原样传递；不解释 shell。")],
            &[
                "默认保留权限检查；此表单不提供跳过权限。",
                "工作目录（由执行主机验证）",
                "Fixture launch error",
            ],
            "关闭",
        )
        .await;
        assert!(matches!(
            messages.as_slice(),
            [Message::Settings(SettingsMessage::AppsCatalog(AppsCatalogMessage::Launch(
                super::LaunchMessage::Close
            )))]
        ));
    }
}
