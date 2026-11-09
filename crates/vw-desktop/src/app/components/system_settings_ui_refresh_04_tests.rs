use super::*;
use crate::app::components::system_settings_search::SystemSettingsSearchTarget;
use crate::app::message::{SettingsMessage, ViewMessage};
use crate::app::state::ModelConfigTab;
use iced::advanced::renderer::Headless;
use iced::advanced::widget::{Id, Operation};
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

const SIDEBAR: &[usize] = &[0, 0, 0];
const ACTIONS: &[usize] = &[0, 0, 2, 0, 0];
const PANEL: &[usize] = &[0, 0, 2, 1];
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
        let mut layout = Layout::new(&node).children().next()?;
        while layout.bounds() == self.viewport {
            layout = layout.children().next()?;
        }
        Some(layout.bounds())
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
            for window_width in [420, 800, 950, 1270, 1720] {
                let mut scene = Scene::new(&app, window_width, 760).await;
                scene.snapshot(
                    &theme,
                    mouse::Cursor::Unavailable,
                    &format!("shell-{locale_name}-{theme_name}-{window_width}"),
                );
                let sidebar = scene.bounds(SIDEBAR, &[]);
                let panel = scene.bounds(PANEL, &[]);
                let body = scene.bounds(PANEL, &[0, 0, 0]);
                assert_eq!(sidebar.x, 0.0);
                assert_eq!(sidebar.y, 0.0);
                assert!((sidebar.width + 1.0 - panel.x).abs() < 0.5);
                assert_eq!(panel.x + panel.width, window_width as f32);
                assert!((body.center_x() - panel.center_x()).abs() < 0.5);
                let expected_width = (panel.width - 2.0 * f32::from(SETTINGS_SCROLL_GUTTER))
                    .min(SETTINGS_CONTENT_MAX_WIDTH);
                assert!((body.width - expected_width).abs() < 0.5, "{body:?}");
                let label = scene.bounds(PANEL, &[0, 0, 0, 0, 0, 2, 1, 0, 0, 0, 0]);
                assert!(label.width > 90.0, "settings row label is cramped: {label:?}");
            }
        }
    }

    app.desktop_locale = DesktopLocale::English;
    let mut scene = Scene::new(&app, 950, 410).await;
    let first = scene.bounds(SIDEBAR, &[0, 2, 0, 0, 0, 1, 0]).center();
    let second = scene.bounds(SIDEBAR, &[0, 2, 0, 0, 0, 1, 1]).center();
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

    for action in [scene.bounds(SIDEBAR, &[0, 0, 0, 0]), scene.bounds(ACTIONS, &[1])] {
        assert!(matches!(
            scene.click(action.center()).as_slice(),
            [Message::View(ViewMessage::ToggleSystemSettings)]
        ));
    }
    let help = scene.bounds(ACTIONS, &[0]);
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
    let input = scene.bounds(SIDEBAR, &[0, 1, 0, 0]);
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

    let mut scene = Scene::new(&app, 950, 760).await;
    let clear = scene.bounds(SIDEBAR, &[0, 1, 0, 1, 2, 0]).center();
    let cleared = scene.click(clear);
    assert!(matches!(cleared.as_slice(), [
        Message::Settings(SettingsMessage::SystemTabQueryChanged(query))
    ] if query.is_empty()));
    drop(scene);
    let Message::Settings(clear) = cleared.into_iter().next().unwrap() else { unreachable!() };
    let _ = message::settings::update(&mut app, clear);
    assert!(app.system_settings_query.is_empty());
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
    let refresh_path = &[0, 0, 0, 0, 0, 0, 0, 1];
    let refresh = scene.bounds(PANEL, refresh_path);
    let preferences = scene.bounds(PANEL, &[0, 0, 0, 0, 0, 0, 0, 2]);
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
    for (group, (_, tabs)) in navigation::GROUPS.iter().enumerate() {
        for (index, tab) in tabs.iter().enumerate() {
            let entry = scene.bounds(SIDEBAR, &[0, 2, 0, 0, group, 1, index]);
            assert!(matches!(scene.click(entry.center()).as_slice(), [
                Message::Settings(SettingsMessage::SystemSearchTargetSelected(
                    SystemSettingsSearchTarget::TopLevel(target)
                ))
            ] if target == tab));
        }
    }
    drop(scene);

    let tabs = SystemTab::all().chain([
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
    let trigger = scene.bounds(PANEL, &[0, 0, 0, 0, 0, 2, 1, 0, 4, 0, 1, 0, 0]).center();
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_04_search_opens_the_language_control_row() {
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::SimplifiedChinese;
    app.general_settings_tab = GeneralSettingsTab::Terminal;
    app.system_settings_query = "语言".into();
    let mut scene = Scene::new(&app, 420, 400).await;
    scene.snapshot(&Theme::Dark, mouse::Cursor::Unavailable, "search-language-dark");
    let result = scene.bounds(SIDEBAR, &[0, 2, 0, 0, 0]);
    let messages = scene.click(result.center());
    assert!(matches!(
        messages.as_slice(),
        [Message::Settings(SettingsMessage::SystemSearchTargetSelected(
            SystemSettingsSearchTarget::GeneralField(field)
        ))] if **field == general_fields::LANGUAGE
    ));
    drop(scene);
    let Message::Settings(selected) = messages.into_iter().next().unwrap() else { unreachable!() };
    let task = message::settings::update(&mut app, selected);
    assert_eq!(app.general_settings_tab, GeneralSettingsTab::Interface);

    // The help card leaves the language row higher on the page.
    let mut scene = Scene::new(&app, 420, 260).await;
    let row_path = &[0, 0, 0, 0, 0, 2, 1, 0, 4];
    let field = scene.bounds(PANEL, row_path);
    let trigger = scene.bounds(PANEL, &[0, 0, 0, 0, 0, 2, 1, 0, 4, 0, 1, 0, 0]).center();
    assert!(trigger.y > scene.viewport.height);

    scene.apply_scroll_task(task).await;

    scene.snapshot(&Theme::Light, mouse::Cursor::Unavailable, "search-language-positioned");
    let scroll_top = scene.bounds(PANEL, &[0]).y;
    let visible_trigger = Point::new(trigger.x, scroll_top + 12.0 + trigger.y - field.y);
    assert!(scene.click(visible_trigger).is_empty());
    assert!(scene.overlay_bounds().is_some(), "the revealed language picker opens");
}

#[tokio::test]
async fn help_rollout_27_quick_messages_renders_contextual_help() {
    use iced::advanced::widget::{Id, Operation};

    struct Heading {
        title: &'static str,
        description: &'static str,
        bounds: Option<Rectangle>,
    }

    impl Operation for Heading {
        fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) {
            children(self);
        }

        fn text(&mut self, _: Option<&Id>, bounds: Rectangle, value: &str) {
            assert_ne!(value, self.description, "optional copy is hidden while idle");
            if value == self.title && bounds.height > 24.0 {
                self.bounds = Some(bounds);
            }
        }
    }

    let mut app = fixture();
    app.system_settings_tab = SystemTab::Agents;
    app.agents_settings.active_config_tab = crate::app::state::AgentConfigTab::QuickMessages;
    app.quick_messages.clear();
    for (locale, theme) in [
        (DesktopLocale::SimplifiedChinese, Theme::Dark),
        (DesktopLocale::TraditionalChinese, Theme::Light),
        (DesktopLocale::English, Theme::Dark),
    ] {
        app.desktop_locale = locale;
        app.app_theme = theme.clone();
        let i18n = crate::app::i18n::I18n::new(locale);
        let mut scene = Scene::new(&app, 560, 480).await;
        let mut heading = Heading {
            title: i18n.__("快捷消息"),
            description: i18n.__("配置聊天输入框上方的常用消息、摘要接力与 Git 动作。"),
            bounds: None,
        };
        scene.element.as_widget_mut().operate(
            &mut scene.tree,
            Layout::new(&scene.node),
            &scene.renderer,
            &mut heading,
        );
        let bounds = heading.bounds.expect("quick messages page heading");
        let trigger = Point::new(bounds.x + bounds.width + 15.0, bounds.center_y());
        let idle = scene.snapshot(
            &theme,
            mouse::Cursor::Unavailable,
            &format!("quick-messages-help-{locale:?}-idle"),
        );
        assert!(scene.click(trigger).is_empty());
        assert!(scene.overlay_bounds().is_some(), "click pins the help card");
        let pinned = scene.snapshot(
            &theme,
            mouse::Cursor::Unavailable,
            &format!("quick-messages-help-{locale:?}-pinned"),
        );
        assert!(pinned != idle);
        assert!(scene.key(Key::Named(Named::Escape), None).is_empty());
        assert!(scene.overlay_bounds().is_none());
    }
}

#[derive(Default)]
struct FieldContents {
    target: Option<Id>,
    bounds: Option<Rectangle>,
    scroll_translation: Vector,
    text: Vec<String>,
}

impl Operation for FieldContents {
    fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) {
        children(self);
    }

    fn container(&mut self, id: Option<&Id>, bounds: Rectangle) {
        if self.target.as_ref() == id && id.is_some() {
            self.bounds = Some(bounds);
        }
    }

    fn scrollable(
        &mut self,
        id: Option<&Id>,
        _: Rectangle,
        _: Rectangle,
        translation: Vector,
        _: &mut dyn iced::advanced::widget::operation::Scrollable,
    ) {
        if id == Some(&Id::from(SETTINGS_CONTENT_SCROLL)) {
            self.scroll_translation = translation;
        }
    }

    fn text(&mut self, _: Option<&Id>, _: Rectangle, value: &str) {
        self.text.push(value.to_owned());
    }
}

fn layout_at(layout: Layout<'_>, bounds: Rectangle) -> Option<Layout<'_>> {
    if layout.bounds() == bounds {
        Some(layout)
    } else {
        layout.children().find_map(|child| layout_at(child, bounds))
    }
}

impl Scene<'_> {
    async fn apply_scroll_task(&mut self, task: iced::Task<Message>) {
        use iced::advanced::widget::operation::Outcome;
        use iced::futures::StreamExt;

        let mut actions = iced_runtime::task::into_stream(task).expect("scroll operation");
        let Some(iced_runtime::Action::Widget(mut operation)) = actions.next().await else {
            panic!("expected a widget operation");
        };
        loop {
            self.element.as_widget_mut().operate(
                &mut self.tree,
                Layout::new(&self.node),
                &self.renderer,
                operation.as_mut(),
            );
            let Outcome::Chain(next) = operation.finish() else { break };
            operation = next;
        }
    }

    fn field_contents(&mut self, field: general_fields::GeneralSettingField) -> FieldContents {
        let mut contents = FieldContents { target: Some(Id::from(field.id)), ..Default::default() };
        self.element.as_widget_mut().operate(
            &mut self.tree,
            Layout::new(&self.node),
            &self.renderer,
            &mut contents,
        );
        assert!(contents.bounds.is_some(), "missing search field {}", field.id);
        contents
    }

    fn field_row(&mut self, field: general_fields::GeneralSettingField) -> Layout<'_> {
        let bounds = self.field_contents(field).bounds.unwrap();
        layout_at(Layout::new(&self.node), bounds).unwrap().children().next().unwrap()
    }

    fn help_trigger(&mut self, field: general_fields::GeneralSettingField) -> Point {
        let translation = self.field_contents(field).scroll_translation;
        let label = self.field_row(field).children().next().unwrap();
        let icon = label.children().nth(1).expect("field question mark").bounds();
        assert_eq!(icon.size(), Size::new(22.0, 22.0));
        let point = icon.center() - translation;
        assert!(
            self.bounds(PANEL, &[0]).contains(point),
            "question mark for {} lies outside the scroll viewport: {point:?}",
            field.id
        );
        point
    }

    fn control(&mut self, field: general_fields::GeneralSettingField) -> Layout<'_> {
        self.field_row(field)
            .children()
            .nth(1)
            .unwrap()
            .children()
            .next()
            .unwrap()
            .children()
            .next()
            .unwrap()
    }

    fn help_content(&mut self, theme: &Theme, point: Point, expected: &str) {
        let cursor = mouse::Cursor::Available(point);
        let idle = self.snapshot(theme, cursor, "");
        assert!(
            self.dispatch(Event::Mouse(mouse::Event::CursorMoved { position: point }), cursor)
                .is_empty()
        );
        let card = self.overlay_bounds().expect("hover opens the help card");
        assert!(card.x >= 0.0 && card.x + card.width <= self.viewport.width);
        assert!(card.y >= 0.0 && card.y + card.height <= self.viewport.height);
        assert!(self.snapshot(theme, cursor, "") != idle, "the card is drawn");
        assert!(
            self.dispatch(Event::Mouse(mouse::Event::CursorLeft), mouse::Cursor::Unavailable)
                .is_empty()
        );
        assert!(self.overlay_bounds().is_none());

        assert!(self.click(point).is_empty());
        assert!(
            self.dispatch(Event::Mouse(mouse::Event::CursorLeft), mouse::Cursor::Unavailable)
                .is_empty()
        );
        assert!(self.overlay_bounds().is_some(), "click pins the card");

        let start = Point::new(card.x + 12.5, card.y + 15.0);
        let end = Point::new(card.x + card.width - 1.0, card.y + card.height - 1.0);
        for (position, event) in [
            (start, mouse::Event::ButtonPressed(mouse::Button::Left)),
            (end, mouse::Event::CursorMoved { position: end }),
            (end, mouse::Event::ButtonReleased(mouse::Button::Left)),
        ] {
            assert!(
                self.dispatch(Event::Mouse(event), mouse::Cursor::Available(position)).is_empty()
            );
        }
        let copy = Key::Character("c".into());
        assert!(
            matches!(
                self.dispatch(
                    Event::Keyboard(keyboard::Event::KeyPressed {
                        key: copy.clone(),
                        modified_key: copy,
                        physical_key: keyboard::key::Physical::Code(keyboard::key::Code::KeyC),
                        location: keyboard::Location::Standard,
                        modifiers: if cfg!(target_os = "macos") {
                            keyboard::Modifiers::LOGO
                        } else {
                            keyboard::Modifiers::CTRL
                        },
                        text: None,
                        repeat: false,
                    }),
                    mouse::Cursor::Unavailable,
                ).as_slice(),
                [Message::CopyCode(value)] if value == expected
            ),
            "the card contains selectable localized help"
        );
        assert!(self.key(Key::Named(Named::Escape), None).is_empty());
        assert!(self.overlay_bounds().is_none());
    }
}

#[tokio::test]
async fn settings_help_gap_01_terminal_theme_moves_into_help() {
    use crate::app::TerminalTheme;
    use crate::app::i18n::I18n;
    use crate::app::message::TerminalMessage;

    let mut app = fixture();
    app.general_settings_tab = GeneralSettingsTab::Terminal;
    app.terminal.font_size = 14.0;
    for locale in [
        DesktopLocale::SimplifiedChinese,
        DesktopLocale::TraditionalChinese,
        DesktopLocale::English,
    ] {
        for (theme, width) in [(Theme::Light, 950), (Theme::Dark, 420)] {
            app.desktop_locale = locale;
            app.app_theme = theme.clone();
            let i18n = I18n::new(locale);
            let mut scene = Scene::new(&app, width, 760).await;
            let field = general_fields::TERMINAL_THEME;
            let explanation = i18n.__("当前仅跟随系统主题。");
            let body = scene.field_contents(field).text;
            assert!(
                !body.iter().any(|text| text == explanation),
                "terminal explanation stays in the body"
            );
            assert!(body.iter().any(|text| text == "14 pt"));

            for (field, source) in [
                (general_fields::TERMINAL_THEME, "当前仅跟随系统主题。"),
                (general_fields::TERMINAL_SHELL, "新终端默认使用的 Shell。"),
                (general_fields::TERMINAL_FONT, "选择终端字体家族。"),
                (general_fields::TERMINAL_FONT_SIZE, "在 10pt 到 18pt 之间调整终端字号。"),
            ] {
                let point = scene.help_trigger(field);
                scene.help_content(
                    &theme,
                    point,
                    &format!("{}\n{}", i18n.__(field.label), i18n.__(source)),
                );
            }

            let trigger = scene.control(field).bounds().center();
            assert!(scene.click(trigger).is_empty());
            assert!(scene.overlay_bounds().is_some(), "system theme remains selectable");
            assert!(matches!(
                scene.key(Key::Named(Named::Enter), None).as_slice(),
                [Message::Terminal(TerminalMessage::ThemeSelected(TerminalTheme::System))]
            ));

            let shell = scene.control(general_fields::TERMINAL_SHELL).bounds().center();
            assert!(scene.click(shell).is_empty());
            assert!(scene.key(Key::Named(Named::End), None).is_empty());
            assert!(matches!(
                scene.key(Key::Named(Named::Enter), None).as_slice(),
                [Message::Terminal(TerminalMessage::ShellSelected(crate::app::Shell::Zsh))]
            ));

            let font = scene.control(general_fields::TERMINAL_FONT).bounds().center();
            assert!(scene.click(font).is_empty());
            assert!(scene.key(Key::Named(Named::Home), None).is_empty());
            assert!(matches!(
                scene.key(Key::Named(Named::Enter), None).as_slice(),
                [Message::Terminal(TerminalMessage::FontSelected(family))] if family == "JetBrains Mono"
            ));

            let slider = scene
                .control(general_fields::TERMINAL_FONT_SIZE)
                .children()
                .next()
                .unwrap()
                .bounds();
            for (x, size) in [(slider.x + 0.1, 10.0), (slider.x + slider.width - 0.1, 18.0)] {
                let point = Point::new(x, slider.center_y());
                assert!(matches!(
                    scene.click(point).as_slice(),
                    [Message::Terminal(TerminalMessage::FontSizePreview(value)),
                     Message::Terminal(TerminalMessage::FontSizePreviewCommit)] if *value == size
                ));
            }
        }
    }
}

#[tokio::test]
async fn settings_help_gap_01_editor_theme_and_lsp_keep_their_controls() {
    use crate::app::i18n::I18n;
    use crate::app::message::{EditorMessage, PreviewMessage};

    let mut app = fixture();
    app.general_settings_tab = GeneralSettingsTab::Editor;
    app.editor_theme = Theme::Nord;
    app.lsp_disabled = true;

    for locale in [
        DesktopLocale::SimplifiedChinese,
        DesktopLocale::TraditionalChinese,
        DesktopLocale::English,
    ] {
        for (tab, theme, width) in
            [(SystemTab::General, Theme::Dark, 420), (SystemTab::Editor, Theme::Light, 950)]
        {
            app.desktop_locale = locale;
            app.app_theme = theme.clone();
            app.system_settings_tab = tab;
            app.editor_follow_system_theme = true;
            let i18n = I18n::new(locale);
            let field = general_fields::EDITOR_THEME;
            let explanation = i18n.__("关闭跟随系统主题时使用的编辑器配色。");
            let mut scene = Scene::new(&app, width, 760).await;
            let body = scene.field_contents(field).text;
            assert!(
                !body.iter().any(|text| text == explanation),
                "editor theme explanation stays in the body"
            );
            let lsp = general_fields::EDITOR_LSP;
            assert!(
                !body
                    .iter()
                    .any(|text| text.contains(i18n.__("启用代码补全、悬停提示与跳转定义。")))
            );
            assert!(
                body.iter().any(|text| text == i18n.__("需要对应语言服务器可用。")),
                "the LSP prerequisite stays visible"
            );

            let picker = scene.control(field).bounds().center();
            assert!(scene.click(picker).is_empty());
            assert!(scene.overlay_bounds().is_none(), "following disables the theme picker");
            let follow = scene.control(general_fields::EDITOR_FOLLOW_THEME).bounds().center();
            let messages = scene.click(follow);
            assert!(matches!(
                messages.as_slice(),
                [Message::Editor(EditorMessage::ToggleFollowSystemTheme(false))]
            ));
            drop(scene);
            let Message::Editor(toggle) = messages.into_iter().next().unwrap() else {
                unreachable!()
            };
            let _ = message::editor::update(&mut app, toggle);
            assert!(!app.editor_follow_system_theme);

            let mut scene = Scene::new(&app, width, 760).await;
            let picker = scene.control(field).bounds().center();
            let help = scene.help_trigger(field);
            scene.help_content(&theme, help, &format!("{}\n{explanation}", i18n.__(field.label)));

            assert!(scene.click(help).is_empty());
            assert!(scene.overlay_bounds().is_some());
            assert!(scene.click(picker).is_empty());
            assert!(scene.overlay_bounds().is_some(), "the picker opens and dismisses help");
            assert!(matches!(
                scene.key(Key::Named(Named::Enter), None).as_slice(),
                [Message::Editor(EditorMessage::ThemeChanged(value))] if value == &app.editor_theme
            ));
            assert!(scene.overlay_bounds().is_none());

            assert!(scene.click(picker).is_empty());
            assert!(scene.overlay_bounds().is_some());
            assert!(scene.click(help).is_empty());
            assert!(scene.overlay_bounds().is_none(), "outside click dismisses the menu");
            assert!(scene.click(help).is_empty());
            assert!(scene.overlay_bounds().is_some(), "help can open after the menu closes");
            assert!(scene.key(Key::Named(Named::Escape), None).is_empty());
            assert!(scene.overlay_bounds().is_none());

            scene.apply_scroll_task(reveal_field(lsp.id)).await;
            let point = scene.help_trigger(lsp);
            scene.help_content(
                &theme,
                point,
                &format!(
                    "{}\n{}",
                    i18n.__(lsp.label),
                    i18n.__("启用代码补全、悬停提示与跳转定义。")
                ),
            );
            let translation = scene.field_contents(lsp).scroll_translation;
            let checkbox = scene
                .control(lsp)
                .children()
                .next()
                .unwrap()
                .children()
                .next()
                .unwrap()
                .bounds()
                .center()
                - translation;
            assert!(matches!(
                scene.click(checkbox).as_slice(),
                [Message::Preview(PreviewMessage::LspToggleDisabled(false))]
            ));
        }
    }
}

#[tokio::test]
async fn settings_help_gap_01_search_reveals_both_theme_rows() {
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::SimplifiedChinese;
    app.editor_follow_system_theme = false;

    for field in [general_fields::TERMINAL_THEME, general_fields::EDITOR_THEME] {
        app.general_settings_tab = GeneralSettingsTab::Interface;
        app.system_settings_query = field.label.into();
        let mut scene = Scene::new(&app, 420, 420).await;
        let result = scene.bounds(SIDEBAR, &[0, 2, 0, 0, 0]);
        let messages = scene.click(result.center());
        assert!(matches!(
            messages.as_slice(),
            [Message::Settings(SettingsMessage::SystemSearchTargetSelected(
                SystemSettingsSearchTarget::GeneralField(selected)
            ))] if **selected == field
        ));
        drop(scene);
        let Message::Settings(selected) = messages.into_iter().next().unwrap() else {
            unreachable!()
        };
        let task = message::settings::update(&mut app, selected);
        assert_eq!(app.general_settings_tab, field.tab);

        let mut scene = Scene::new(&app, 420, 225).await;
        let trigger = scene.control(field).bounds().center();
        assert!(trigger.y > scene.viewport.height);
        scene.apply_scroll_task(task).await;
        let translation = scene.field_contents(field).scroll_translation;
        assert!(translation.y > 0.0);
        let visible_trigger = trigger - translation;
        scene.snapshot(&Theme::Light, mouse::Cursor::Unavailable, "");
        assert!(scene.click(visible_trigger).is_empty());
        assert!(scene.overlay_bounds().is_some(), "the revealed theme picker opens");
    }
}
