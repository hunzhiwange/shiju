use super::*;
use crate::app::message::{SettingsMessage, ViewMessage};
use crate::app::state::ModelConfigTab;
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, Shell, clipboard, layout, overlay, renderer, widget::Tree};
use iced::keyboard::{self, Key, key::Named};
use iced::{Event, Font, Point, Rectangle, Renderer, Size, Vector, mouse};
use vw_config_types::ui::DesktopLocale;

fn fixture() -> App {
    let mut app = None;
    crate::app::config::with_editor_settings_gateway(|fixture, _, _| app = Some(fixture));
    let mut app = app.expect("isolated settings fixture");
    app.show_system_settings = true;
    app.system_settings_tab = SystemTab::General;
    app.general_settings_tab = crate::app::state::GeneralSettingsTab::Interface;
    app
}

const HEADER: &[usize] = &[0, 0, 0, 0, 0];
const SIDEBAR: &[usize] = &[0, 0, 0, 0, 1, 0];
const PANEL: &[usize] = &[0, 0, 0, 0, 1, 1];
const HELP_CARD: &[usize] = &[1, 1, 0];

struct Scene<'a> {
    element: Element<'a, Message>,
    tree: Tree,
    node: layout::Node,
    renderer: Renderer,
    viewport: Rectangle,
}

impl<'a> Scene<'a> {
    async fn new(app: &'a App, width: u32, height: u32) -> Self {
        static FONTS: std::sync::Once = std::sync::Once::new();
        FONTS.call_once(|| {
            let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
            for font in crate::fonts::load_all() {
                fonts.load_font(font);
            }
        });
        let font = Font::with_name(if cfg!(target_os = "macos") {
            "Noto Sans CJK SC"
        } else {
            "JetBrains Mono"
        });
        let renderer = Renderer::new(font, 16.0.into(), Some("tiny-skia")).await.unwrap();
        let viewport = Rectangle::with_size(Size::new(width as f32, height as f32));
        let mut element = view(app);
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, viewport.size()),
        );
        Self { element, tree, node, renderer, viewport }
    }

    fn bounds(&self, parent: &[usize], children: &[usize]) -> Rectangle {
        parent
            .iter()
            .chain(children)
            .fold(Layout::new(&self.node), |node, &index| node.children().nth(index).unwrap())
            .bounds()
    }

    fn dispatch(&mut self, event: Event, mut cursor: mouse::Cursor) -> Vec<Message> {
        let root = Layout::new(&self.node);
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);
        if let Some(layer) = self.element.as_widget_mut().overlay(
            &mut self.tree,
            root,
            &self.renderer,
            &self.viewport,
            Vector::ZERO,
        ) {
            let mut layer = overlay::Nested::new(layer);
            let node = layer.layout(&self.renderer, self.viewport.size());
            let layout = Layout::new(&node);
            let interaction = layer.mouse_interaction(layout, cursor, &self.renderer);
            layer.update(&event, layout, cursor, &self.renderer, &mut clipboard::Null, &mut shell);
            if interaction != mouse::Interaction::None {
                cursor = mouse::Cursor::Unavailable;
            }
        }
        if !shell.is_event_captured() {
            self.element.as_widget_mut().update(
                &mut self.tree,
                &event,
                root,
                cursor,
                &self.renderer,
                &mut clipboard::Null,
                &mut shell,
                &self.viewport,
            );
        }
        messages
    }

    fn overlay_bounds(&mut self) -> Option<Rectangle> {
        let layer = self.element.as_widget_mut().overlay(
            &mut self.tree,
            Layout::new(&self.node),
            &self.renderer,
            &self.viewport,
            Vector::ZERO,
        )?;
        let node = overlay::Nested::new(layer).layout(&self.renderer, self.viewport.size());
        Layout::new(&node).children().next().map(|layout| layout.bounds())
    }

    fn click(&mut self, point: Point) -> Vec<Message> {
        [
            mouse::Event::ButtonPressed(mouse::Button::Left),
            mouse::Event::ButtonReleased(mouse::Button::Left),
        ]
        .into_iter()
        .flat_map(|event| self.dispatch(Event::Mouse(event), mouse::Cursor::Available(point)))
        .collect()
    }

    fn key(&mut self, key: Key, text: Option<&str>) -> Vec<Message> {
        self.dispatch(
            Event::Keyboard(keyboard::Event::KeyPressed {
                modified_key: key.clone(),
                key,
                physical_key: keyboard::key::Physical::Unidentified(
                    keyboard::key::NativeCode::Unidentified,
                ),
                location: keyboard::Location::Standard,
                modifiers: keyboard::Modifiers::empty(),
                text: text.map(Into::into),
                repeat: false,
            }),
            mouse::Cursor::Unavailable,
        )
    }

    fn snapshot(&mut self, theme: &Theme, cursor: mouse::Cursor, name: &str) -> Vec<u8> {
        use iced::advanced::Renderer as _;

        assert!(
            self.dispatch(
                Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now())),
                cursor,
            )
            .is_empty()
        );
        self.renderer.reset(self.viewport);
        let defaults = renderer::Style { text_color: theme.palette().text };
        let root = Layout::new(&self.node);
        self.element.as_widget().draw(
            &self.tree,
            &mut self.renderer,
            theme,
            &defaults,
            root,
            cursor,
            &self.viewport,
        );
        // Rebuild the overlay after event dispatch, as the native UI does.
        if let Some(layer) = self.element.as_widget_mut().overlay(
            &mut self.tree,
            root,
            &self.renderer,
            &self.viewport,
            Vector::ZERO,
        ) {
            let mut layer = overlay::Nested::new(layer);
            let node = layer.layout(&self.renderer, self.viewport.size());
            layer.draw(&mut self.renderer, theme, &defaults, Layout::new(&node), cursor);
        }
        let size = Size::new(self.viewport.width as u32, self.viewport.height as u32);
        let pixels = self.renderer.screenshot(size, 1.0, theme.palette().background);
        if !name.is_empty()
            && let Some(directory) = std::env::var_os("SHIJU_SETTINGS_PREVIEW_DIR")
        {
            let file = std::path::PathBuf::from(directory).join(format!("{name}.png"));
            image::save_buffer(file, &pixels, size.width, size.height, image::ColorType::Rgba8)
                .unwrap();
        }
        pixels
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_04_layout_and_interaction() {
    let mut app = fixture();
    for (theme, theme_name) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
        for (locale, locale_name) in [
            (DesktopLocale::SimplifiedChinese, "zh"),
            (DesktopLocale::TraditionalChinese, "zh-hant"),
            (DesktopLocale::English, "en"),
        ] {
            app.desktop_locale = locale;
            app.app_theme = theme.clone();
            for (window_width, content_width) in
                [(420, 218.0), (800, 502.0), (950, 640.0), (1270, 960.0)]
            {
                let mut scene = Scene::new(&app, window_width, 760).await;
                scene.snapshot(
                    &theme,
                    mouse::Cursor::Unavailable,
                    &format!("shell-{locale_name}-{theme_name}-{window_width}"),
                );
                let sidebar = scene.bounds(SIDEBAR, &[]);
                let panel = scene.bounds(PANEL, &[]);
                let body = scene.bounds(PANEL, &[0, 0, 0]);
                assert!(sidebar.x + sidebar.width <= panel.x);
                assert!(panel.width >= window_width as f32 * 0.6);
                assert!((body.width - content_width).abs() < 0.5, "{body:?}");
                assert_eq!(sidebar.y, panel.y);
                let back = scene.bounds(HEADER, &[0]);
                let close = scene.bounds(HEADER, &[2, 1]);
                assert!((back.center_y() - close.center_y()).abs() < 0.5);
            }
        }
    }

    app.desktop_locale = DesktopLocale::English;
    let mut scene = Scene::new(&app, 950, 410).await;
    let first = scene.bounds(SIDEBAR, &[0, 2, 0, 0, 0]).center();
    let second = scene.bounds(SIDEBAR, &[0, 2, 0, 0, 1]).center();
    let idle = scene.snapshot(&Theme::Dark, mouse::Cursor::Unavailable, "");
    for (index, point) in [first, second].into_iter().enumerate() {
        let cursor = mouse::Cursor::Available(point);
        assert!(
            scene
                .dispatch(Event::Mouse(mouse::Event::CursorMoved { position: point }), cursor)
                .is_empty()
        );
        let hovered = scene.snapshot(&Theme::Dark, cursor, "");
        if index == 1 {
            assert_ne!(hovered, idle, "inactive entry must highlight on hover");
        }
    }
    assert!(
        scene
            .dispatch(Event::Mouse(mouse::Event::CursorLeft), mouse::Cursor::Unavailable)
            .is_empty()
    );
    assert_eq!(scene.snapshot(&Theme::Dark, mouse::Cursor::Unavailable, ""), idle);

    assert!(matches!(
        scene.click(first).as_slice(),
        [Message::Settings(SettingsMessage::SystemSearchTargetSelected(
            SystemSettingsSearchTarget::TopLevel(SystemTab::General)
        ))]
    ));
    assert!(
        scene
            .dispatch(
                Event::Mouse(mouse::Event::WheelScrolled {
                    delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -180.0 },
                }),
                mouse::Cursor::Available(first)
            )
            .is_empty()
    );
    assert!(
        !matches!(
            scene.click(first).as_slice(),
            [Message::Settings(SettingsMessage::SystemSearchTargetSelected(
                SystemSettingsSearchTarget::TopLevel(SystemTab::General)
            ))]
        ),
        "wheel must move the result list under the pointer"
    );

    for action in [scene.bounds(HEADER, &[0]), scene.bounds(HEADER, &[2, 1])] {
        assert!(matches!(
            scene.click(action.center()).as_slice(),
            [Message::View(ViewMessage::ToggleSystemSettings)]
        ));
    }
    let help = scene.bounds(HEADER, &[2, 0]);
    assert!(matches!(
        scene.click(help.center()).as_slice(),
        [Message::Settings(SettingsMessage::SystemHelpOpen(SystemTab::General))]
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_04_preserves_values_and_disabled_states() {
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::English;
    app.system_settings_query = "OC".into();
    let mut scene = Scene::new(&app, 950, 760).await;
    let input = scene.bounds(SIDEBAR, &[0, 1]);
    assert!(scene.click(Point::new(input.x + 45.0, input.center_y())).is_empty());
    assert!(scene.key(Key::Named(Named::End), None).is_empty());
    let typed = scene.key(Key::Character("R".into()), Some("R"));
    assert!(matches!(typed.as_slice(), [Message::Settings(
        SettingsMessage::SystemTabQueryChanged(value)
    )] if value == "OCR"));
    assert!(scene.key(Key::Named(Named::Enter), None).is_empty());
    assert!(scene.key(Key::Named(Named::Escape), None).is_empty());
    drop(scene);
    let Message::Settings(query) = typed.into_iter().next().unwrap() else { unreachable!() };
    let _ = message::settings::update(&mut app, query);

    let mut scene = Scene::new(&app, 950, 760).await;
    scene.snapshot(&Theme::Light, mouse::Cursor::Unavailable, "search-ocr");
    let result = scene.bounds(SIDEBAR, &[0, 2, 0, 0, 0]);
    let selected = scene.click(result.center());
    assert!(matches!(
        selected.as_slice(),
        [Message::Settings(SettingsMessage::SystemSearchTargetSelected(
            SystemSettingsSearchTarget::Models(ModelConfigTab::Multimodal)
        ))]
    ));
    drop(scene);
    let Message::Settings(selected) = selected.into_iter().next().unwrap() else { unreachable!() };
    let _ = message::settings::update(&mut app, selected);
    assert_eq!(app.system_settings_tab, SystemTab::Models);
    assert_eq!(app.model_settings.active_tab, ModelConfigTab::Multimodal);

    app.system_settings_query = "unmatched-fixture-entry".into();
    let mut scene = Scene::new(&app, 950, 760).await;
    scene.snapshot(&Theme::Light, mouse::Cursor::Unavailable, "search-empty");
    assert!(scene.click(result.center()).is_empty());
    drop(scene);

    app.system_settings_query.clear();
    app.system_settings_tab = SystemTab::Apps;
    app.apps_catalog.loading = true;
    let mut scene = Scene::new(&app, 950, 760).await;
    scene.snapshot(&Theme::Light, mouse::Cursor::Unavailable, "apps-loading");
    let refresh_path = &[0, 0, 0, 0, 0, 0, 1];
    let refresh = scene.bounds(PANEL, refresh_path);
    let preferences = scene.bounds(PANEL, &[0, 0, 0, 0, 0, 0, 2]);
    assert!(scene.click(refresh.center()).is_empty());
    assert!(scene.click(preferences.center()).is_empty());
    drop(scene);

    app.apps_catalog.loading = false;
    app.apps_catalog.error = Some(format!(
        "Fixture could not load /tmp/{}/sample-app",
        "settings-navigation/".repeat(10)
    ));
    let mut scene = Scene::new(&app, 950, 760).await;
    scene.snapshot(&Theme::Light, mouse::Cursor::Unavailable, "apps-error");
    let retry = scene.bounds(PANEL, refresh_path);
    assert!(matches!(
        scene.click(retry.center()).as_slice(),
        [Message::Settings(SettingsMessage::AppsCatalog(
            message::settings::AppsCatalogMessage::Refresh
        ))]
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_04_renders_each_page_and_preserves_help_close() {
    let mut app = fixture();
    let mut scene = Scene::new(&app, 1270, 980).await;
    for (index, tab) in SystemTab::all().into_iter().enumerate() {
        let entry = scene.bounds(SIDEBAR, &[0, 2, 0, 0, index]);
        assert!(matches!(scene.click(entry.center()).as_slice(), [
            Message::Settings(SettingsMessage::SystemSearchTargetSelected(
                SystemSettingsSearchTarget::TopLevel(target)
            ))
        ] if *target == tab));
    }
    drop(scene);

    let tabs = SystemTab::all().into_iter().chain([
        SystemTab::Themes,
        SystemTab::Editor,
        SystemTab::Channels,
        SystemTab::Runtime,
        SystemTab::Proxy,
        SystemTab::Tunnel,
        SystemTab::HttpRequest,
        SystemTab::Browser,
    ]);
    for tab in tabs {
        app.system_settings_tab = tab;
        Scene::new(&app, 1270, 760).await.snapshot(&Theme::Light, mouse::Cursor::Unavailable, "");
    }

    app.system_settings_tab = SystemTab::General;
    app.system_settings_help_tab = Some(SystemTab::General);
    app.app_theme = Theme::Dark;
    for (locale, name) in [
        (DesktopLocale::SimplifiedChinese, "zh"),
        (DesktopLocale::TraditionalChinese, "zh-hant"),
        (DesktopLocale::English, "en"),
    ] {
        app.desktop_locale = locale;
        let mut scene = Scene::new(&app, 420, 500).await;
        scene.snapshot(&Theme::Dark, mouse::Cursor::Unavailable, &format!("help-{name}-420"));
        let card = scene.bounds(HELP_CARD, &[]);
        let close = scene.bounds(HELP_CARD, &[0, 0, 1, 1]);
        assert!(card.x >= 0.0 && card.x + card.width <= 420.5);
        assert!(scene.key(Key::Named(Named::Escape), None).is_empty());
        assert!(matches!(
            scene.click(close.center()).as_slice(),
            [Message::Settings(SettingsMessage::SystemHelpClose)]
        ));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_04_keeps_general_select_overlay_interactive() {
    use crate::app::message::GeneralSettingsMessage;

    let mut app = fixture();
    app.desktop_locale = DesktopLocale::SimplifiedChinese;
    let mut scene = Scene::new(&app, 950, 760).await;
    let trigger = scene.bounds(PANEL, &[0, 0, 0, 0, 2, 1, 0, 4, 0, 1, 0]).center();
    assert!(scene.click(trigger).is_empty());
    let menu = scene.overlay_bounds().expect("locale menu opens");
    assert!(menu.x >= 0.0 && menu.x + menu.width <= scene.viewport.width + 0.5);
    assert!(menu.y >= 0.0 && menu.y + menu.height <= scene.viewport.height + 0.5);
    scene.snapshot(&Theme::Light, mouse::Cursor::Unavailable, "locale-menu");
    assert!(scene.key(Key::Named(Named::Escape), None).is_empty());
    assert!(scene.overlay_bounds().is_none());

    assert!(scene.click(trigger).is_empty());
    assert!(scene.key(Key::Named(Named::End), None).is_empty());
    assert!(matches!(
        scene.key(Key::Named(Named::Enter), None).as_slice(),
        [Message::Settings(SettingsMessage::General(GeneralSettingsMessage::LocaleSelected(
            DesktopLocale::English
        )))]
    ));
    assert!(scene.overlay_bounds().is_none());
}
