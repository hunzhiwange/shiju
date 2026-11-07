use super::{MODEL_TABS, view, view_overlays};
use crate::app::message::settings::{self, ModelConfigTabMessage, SettingsMessage};
use crate::app::state::{ModelConfigTab, ModelSummary, ProviderModelsSummary};
use crate::app::{App, Message};
use iced::advanced::renderer::{Headless, Style};
use iced::advanced::widget::operation::{Operation, Scrollable};
use iced::advanced::{Layout, clipboard, layout, widget::Tree};
use iced::keyboard::{self, Key, key::Named};
use iced::widget::button;
use iced::{Element, Event, Font, Length, Point, Rectangle, Renderer, Size, Theme, Vector, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
use serde_json::json;
use vw_config_types::ui::DesktopLocale;

const PROVIDER: &str = "fixture-provider";
const MODEL: &str = "fixture-model-with-an-unusually-long-stable-identifier-for-narrow-layouts";

fn fixture() -> App {
    let mut app = None;
    crate::app::config::with_editor_settings_gateway(|fixture, _, _| app = Some(fixture));
    let mut app = app.expect("isolated settings fixture");
    app.desktop_locale = DesktopLocale::English;
    app.model_settings.providers = vec![ProviderModelsSummary {
        id: PROVIDER.into(),
        name: "Fixture 模型供应商 測試供應商".into(),
        models: vec![
            ModelSummary {
                id: MODEL.into(),
                name: "Long model display name · 虚构模型名称 · 虛構模型名稱".into(),
                enabled: false,
                toolcall: true,
                attachment: true,
                context_limit: 128_000,
                detail: json!({
                    "id": MODEL,
                    "api": {"url": format!("https://fixture.invalid/{}/v1", "long-segment/".repeat(12))},
                }),
            },
            ModelSummary {
                id: "fixture-active".into(),
                name: "Active model".into(),
                enabled: true,
                toolcall: true,
                attachment: false,
                context_limit: 32_000,
                detail: json!({"id": "fixture-active"}),
            },
        ],
    }];
    app.model_settings.expanded_provider_ids.insert(PROVIDER.into());
    app
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
        .expect("tiny-skia renderer")
}

struct Scene<'a> {
    ui: UserInterface<'a, Message, Theme, Renderer>,
    layout: layout::Node,
    size: Size<u32>,
}

impl<'a> Scene<'a> {
    fn new(mut element: Element<'a, Message>, renderer: &mut Renderer, size: Size<u32>) -> Self {
        let viewport = Size::new(size.width as f32, size.height as f32);
        let mut tree = Tree::new(element.as_widget());
        let layout = element.as_widget_mut().layout(
            &mut tree,
            renderer,
            &layout::Limits::new(Size::ZERO, viewport),
        );
        let ui = UserInterface::build(element, viewport, Cache::new(), renderer);
        Self { ui, layout, size }
    }

    fn bounds(&self, path: &[usize]) -> Rectangle {
        path.iter()
            .fold(Layout::new(&self.layout), |layout, &index| {
                layout
                    .children()
                    .nth(index)
                    .unwrap_or_else(|| panic!("missing layout path {path:?}"))
            })
            .bounds()
    }

    fn send(
        &mut self,
        renderer: &mut Renderer,
        events: &[Event],
        cursor: mouse::Cursor,
    ) -> Vec<Message> {
        let mut messages = Vec::new();
        self.ui.update(events, cursor, renderer, &mut clipboard::Null, &mut messages);
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

    fn capture(
        &mut self,
        renderer: &mut Renderer,
        theme: &Theme,
        cursor: mouse::Cursor,
        name: &str,
    ) {
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
        if let Some(directory) = std::env::var_os("SHIJU_SETTINGS_PREVIEW_DIR") {
            image::save_buffer(
                std::path::PathBuf::from(directory).join(format!("{name}.png")),
                &pixels,
                self.size.width,
                self.size.height,
                image::ColorType::Rgba8,
            )
            .expect("save screenshot");
        }
    }
}

const SEARCH: &[usize] = &[1, 2, 0, 0, 0];
const REFRESH: &[usize] = &[1, 0, 1];
const ENABLE_ALL: &[usize] = &[1, 3, 1, 0];
const DISABLE_ALL: &[usize] = &[1, 3, 1, 1];
const PROVIDER_HEADER: &[usize] = &[1, 4, 0, 0, 0, 0];
const MODEL_ROW: &[usize] = &[1, 4, 0, 0, 2, 0];
const MODAL_CARD: &[usize] = &[1, 1, 0, 0];
const MODAL_CONTENT: &[usize] = &[1, 1, 0, 0, 0];

fn child(parent: &[usize], path: &[usize]) -> Vec<usize> {
    [parent, path].concat()
}

fn apply(app: &mut App, messages: Vec<Message>) {
    let [Message::Settings(message)]: [Message; 1] = messages.try_into().expect("one message")
    else {
        panic!("expected a settings message");
    };
    // Apply synchronous UI changes only. The returned refresh tasks are never run.
    let _ = settings::update(app, message);
}

fn cursor(point: Point) -> mouse::Cursor {
    mouse::Cursor::Available(point)
}

fn key(name: Named) -> Event {
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

fn assert_centered(left: Rectangle, right: Rectangle) {
    assert!((left.center().y - right.center().y).abs() < 0.5, "{left:?} vs {right:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn ui_refresh_10_layout_and_interaction() {
    let mut renderer = renderer().await;
    let mut app = fixture();
    for (locale, theme, width, name) in [
        (DesktopLocale::English, Theme::Light, 640, "list-en-light-640"),
        (DesktopLocale::SimplifiedChinese, Theme::Dark, 960, "list-zh-dark-960"),
        (DesktopLocale::TraditionalChinese, Theme::Light, 640, "list-zh-hant-light-640"),
        (DesktopLocale::English, Theme::Dark, 960, "list-en-dark-960"),
    ] {
        app.desktop_locale = locale;
        let mut scene = Scene::new(view(&app), &mut renderer, Size::new(width, 760));
        let search = scene.bounds(SEARCH);
        let counts = scene.bounds(&[1, 2, 0, 0, 1]);
        let detail = scene.bounds(&child(MODEL_ROW, &[1]));
        let icon = scene.bounds(&child(MODEL_ROW, &[1, 0]));
        let toggle = scene.bounds(&child(MODEL_ROW, &[2]));
        let provider_logo = scene.bounds(&child(PROVIDER_HEADER, &[0, 0, 0]));
        let provider_name = scene.bounds(&child(PROVIDER_HEADER, &[0, 1]));
        assert_centered(search, counts);
        assert_centered(detail, icon);
        assert_centered(detail, toggle);
        assert_centered(provider_logo, provider_name);
        assert!(detail.x + detail.width < toggle.x);
        assert!(toggle.x + toggle.width < width as f32);
        scene.capture(&mut renderer, &theme, mouse::Cursor::Unavailable, name);
    }

    // At 420 px the English segments need a second line. All original routes remain clickable.
    app.desktop_locale = DesktopLocale::English;
    for (index, descriptor) in MODEL_TABS.iter().enumerate() {
        let mut scene = Scene::new(view(&app), &mut renderer, Size::new(420, 760));
        let tab = scene.bounds(&[0, index]);
        assert!(tab.x + tab.width <= 420.0);
        if index == MODEL_TABS.len() - 1 {
            assert!(tab.y > scene.bounds(&[0, 0]).y);
        }
        let messages = scene.click(&mut renderer, tab.center());
        assert!(matches!(
            messages.as_slice(),
            [Message::Settings(SettingsMessage::ModelConfigTab(ModelConfigTabMessage::Selected(tab)))]
                if *tab == descriptor.tab
        ));
        drop(scene);
        apply(&mut app, messages);
        assert_eq!(app.model_settings.active_tab, descriptor.tab);
        Scene::new(view(&app), &mut renderer, Size::new(640, 760)).capture(
            &mut renderer,
            &Theme::Light,
            mouse::Cursor::Unavailable,
            &format!("tab-{index}"),
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn ui_refresh_10_preserves_values_and_disabled_states() {
    let mut renderer = renderer().await;
    let mut app = fixture();
    let size = Size::new(640, 760);
    let mut scene = Scene::new(view(&app), &mut renderer, size);
    let search = scene.bounds(SEARCH).center();
    assert!(scene.click(&mut renderer, search).is_empty());
    let messages = scene.send(
        &mut renderer,
        &[Event::InputMethod(iced::advanced::input_method::Event::Commit(MODEL.into()))],
        cursor(search),
    );
    assert!(matches!(
        messages.as_slice(),
        [Message::Settings(SettingsMessage::ModelQueryChanged(query))] if query == MODEL
    ));
    assert!(
        scene
            .send(&mut renderer, &[key(Named::Enter), key(Named::Escape)], cursor(search))
            .is_empty()
    );
    scene.capture(&mut renderer, &Theme::Light, cursor(search), "search-focused");
    assert!(matches!(
        scene.click(&mut renderer, scene.bounds(REFRESH).center()).as_slice(),
        [Message::Settings(SettingsMessage::ModelsRefresh)]
    ));
    drop(scene);
    apply(&mut app, messages);
    assert_eq!(app.model_settings.query, MODEL);
    let mut scene = Scene::new(view(&app), &mut renderer, size);
    let detail = scene.bounds(&child(MODEL_ROW, &[1])).center();
    let toggle = scene.bounds(&child(MODEL_ROW, &[2])).center();
    for (point, name) in [
        (detail, "list-detail-hover"),
        (toggle, "list-toggle-hover"),
        (Point::new(1.0, 1.0), "list-pointer-out"),
    ] {
        let movement = Event::Mouse(mouse::Event::CursorMoved { position: point });
        assert!(scene.send(&mut renderer, &[movement], cursor(point)).is_empty());
        scene.capture(&mut renderer, &Theme::Light, cursor(point), name);
    }
    assert!(matches!(
        scene.click(&mut renderer, toggle).as_slice(),
        [Message::Settings(SettingsMessage::ModelToggle(provider, model, true))]
            if provider == PROVIDER && model == MODEL
    ));
    let details = scene.click(&mut renderer, detail);
    assert!(matches!(
        details.as_slice(),
        [Message::Settings(SettingsMessage::ModelDetailOpen(provider, model))]
            if provider == PROVIDER && model == MODEL
    ));
    assert!(matches!(
        scene.click(&mut renderer, scene.bounds(ENABLE_ALL).center()).as_slice(),
        [Message::Settings(SettingsMessage::ModelsSetAll(true))]
    ));
    assert!(matches!(
        scene.click(&mut renderer, scene.bounds(DISABLE_ALL).center()).as_slice(),
        [Message::Settings(SettingsMessage::ModelsSetAll(false))]
    ));
    for (index, enabled) in [(0, true), (1, false)] {
        let action = scene.bounds(&child(PROVIDER_HEADER, &[1, index])).center();
        assert!(matches!(
            scene.click(&mut renderer, action).as_slice(),
            [Message::Settings(SettingsMessage::ProviderModelsSetAll(provider, value))]
                if provider == PROVIDER && *value == enabled
        ));
    }
    let reset = scene.bounds(&child(PROVIDER_HEADER, &[1, 2])).center();
    assert!(matches!(
        scene.click(&mut renderer, reset).as_slice(),
        [Message::Settings(SettingsMessage::ProviderModelsReset(provider))] if provider == PROVIDER
    ));
    let expand = scene.bounds(&child(PROVIDER_HEADER, &[0, 3])).center();
    let expansion = scene.click(&mut renderer, expand);
    assert!(matches!(
        expansion.as_slice(),
        [Message::Settings(SettingsMessage::ProviderModelsExpansionToggled(provider))]
            if provider == PROVIDER
    ));
    drop(scene);
    apply(&mut app, expansion);
    assert!(!app.model_settings.expanded_provider_ids.contains(PROVIDER));
    apply(&mut app, details);
    assert_eq!(app.model_settings.detail_modal.as_ref().unwrap().model_id, MODEL);
    assert_eq!(
        app.model_settings.detail_modal.as_ref().unwrap().model_name,
        app.model_settings.providers[0].models[0].name
    );
    app.model_settings.expanded_provider_ids.insert(PROVIDER.into());

    for (loading, saving, name) in [(true, false, "list-loading"), (false, true, "list-saving")] {
        app.model_settings.loading = loading;
        app.model_settings.saving = saving;
        let mut scene = Scene::new(view(&app), &mut renderer, size);
        let mut blocked = vec![
            REFRESH.to_vec(),
            ENABLE_ALL.to_vec(),
            DISABLE_ALL.to_vec(),
            child(MODEL_ROW, &[2]),
        ];
        blocked.extend((0..3).map(|index| child(PROVIDER_HEADER, &[1, index])));
        for path in blocked {
            assert!(
                scene.click(&mut renderer, scene.bounds(&path).center()).is_empty(),
                "{path:?}"
            );
        }
        assert!(matches!(
            scene.click(&mut renderer, scene.bounds(&child(MODEL_ROW, &[1])).center()).as_slice(),
            [Message::Settings(SettingsMessage::ModelDetailOpen(_, _))]
        ));
        scene.capture(&mut renderer, &Theme::Dark, mouse::Cursor::Unavailable, name);
    }
    assert!(!app.model_settings.providers[0].models[0].enabled);
    assert!(app.model_settings.providers[0].models[1].enabled);
    app.model_settings.active_tab = ModelConfigTab::Models;
    app.model_settings.loading = false;
    app.model_settings.saving = false;
    app.model_settings.query = "unmatched fixture".into();
    Scene::new(view(&app), &mut renderer, size).capture(
        &mut renderer,
        &Theme::Light,
        mouse::Cursor::Unavailable,
        "list-no-match",
    );
    app.model_settings.providers.clear();
    app.model_settings.save_error = Some("Fixture refresh failed. Please retry.".to_owned().into());
    Scene::new(view(&app), &mut renderer, size).capture(
        &mut renderer,
        &Theme::Dark,
        mouse::Cursor::Unavailable,
        "list-empty-error",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn ui_refresh_10_detail_scrolls_and_closes_without_saving() {
    let mut renderer = renderer().await;
    let mut app = fixture();
    let _ =
        settings::update(&mut app, SettingsMessage::ModelDetailOpen(PROVIDER.into(), MODEL.into()));
    let size = Size::new(420, 620);
    for (locale, theme, name) in [
        (DesktopLocale::English, Theme::Dark, "detail-en-dark-420"),
        (DesktopLocale::TraditionalChinese, Theme::Light, "detail-zh-hant-light-420"),
    ] {
        app.desktop_locale = locale;
        let mut scene = Scene::new(view_overlays(&app, view(&app)), &mut renderer, size);
        let modal = scene.bounds(MODAL_CARD);
        let header = scene.bounds(&child(MODAL_CONTENT, &[0]));
        let scroll = scene.bounds(&child(MODAL_CONTENT, &[1]));
        let content = scene.bounds(&child(MODAL_CONTENT, &[1, 0, 0]));
        let raw = scene.bounds(&child(MODAL_CONTENT, &[0, 1, 0]));
        let close = scene.bounds(&child(MODAL_CONTENT, &[0, 1, 1]));
        assert!(modal.x >= 12.0 && modal.x + modal.width <= 408.0);
        assert!(header.y + header.height < scroll.y);
        assert!(content.x >= scroll.x + 7.0);
        assert!(content.x + content.width <= scroll.x + scroll.width - 7.0);
        assert_centered(raw, close);
        scene.capture(&mut renderer, &theme, mouse::Cursor::Unavailable, name);
        let wheel = Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -140.0 },
        });
        assert!(scene.send(&mut renderer, &[wheel], cursor(scroll.center())).is_empty());
        let mut offset = ScrollOffset::default();
        scene.ui.operate(&renderer, &mut offset);
        assert!(offset.0 > 0.0);
        scene.capture(&mut renderer, &theme, cursor(scroll.center()), &format!("{name}-scrolled"));
        let messages = scene.click(&mut renderer, raw.center());
        assert!(matches!(
            messages.as_slice(),
            [Message::Settings(SettingsMessage::ModelDetailToggleRaw)]
        ));
        drop(scene);
        apply(&mut app, messages);
        assert!(app.model_settings.detail_modal.as_ref().unwrap().show_raw);
        let mut scene = Scene::new(view_overlays(&app, view(&app)), &mut renderer, size);
        scene.capture(&mut renderer, &theme, mouse::Cursor::Unavailable, &format!("{name}-raw"));
        let messages = scene.click(&mut renderer, raw.center());
        assert!(matches!(
            messages.as_slice(),
            [Message::Settings(SettingsMessage::ModelDetailToggleRaw)]
        ));
        drop(scene);
        apply(&mut app, messages);
        assert!(!app.model_settings.detail_modal.as_ref().unwrap().show_raw);
    }

    // The full-screen background button would publish a refresh if a modal click leaked through.
    let background = button("Fixture background")
        .width(Length::Fill)
        .height(Length::Fill)
        .on_press(Message::Settings(SettingsMessage::ModelsRefresh));
    let mut scene = Scene::new(view_overlays(&app, background.into()), &mut renderer, size);
    let title = scene.bounds(&child(MODAL_CONTENT, &[0, 0])).center();
    assert!(scene.click(&mut renderer, title).is_empty());
    assert!(scene.send(&mut renderer, &[key(Named::Escape)], cursor(title)).is_empty());
    let close = scene.bounds(&child(MODAL_CONTENT, &[0, 1, 1])).center();
    let messages = scene.click(&mut renderer, close);
    assert!(matches!(messages.as_slice(), [Message::Settings(SettingsMessage::ModelDetailClose)]));
    assert!(scene.click(&mut renderer, Point::new(3.0, 3.0)).is_empty());
    drop(scene);
    apply(&mut app, messages);
    assert!(app.model_settings.detail_modal.is_none());
    assert!(!app.model_settings.saving);
}

#[derive(Default)]
struct ScrollOffset(f32);

impl Operation for ScrollOffset {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        _id: Option<&iced::advanced::widget::Id>,
        _bounds: Rectangle,
        _content: Rectangle,
        translation: Vector,
        _state: &mut dyn Scrollable,
    ) {
        self.0 = translation.y;
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn settings_tabs_core_appearance_models() {
    use super::super::system_settings_common::tab_appearance_tests;
    use crate::app::i18n::settings_models::text;

    tab_appearance_tests::run(
        concat!(module_path!(), "::settings_tabs_core_appearance_models"),
        fixture,
        |scene, app| {
            app.model_settings.active_tab = ModelConfigTab::Models;
            let locale = app.desktop_locale;
            let labels: Vec<_> = MODEL_TABS.iter().map(|tab| text(locale, tab.label)).collect();
            let content = text(locale, "路由规则");
            assert!(!scene.labels(view(app)).contains(content));
            let message = scene.check(app, view(app), &labels, 1);
            assert!(matches!(
                message,
                SettingsMessage::ModelConfigTab(ModelConfigTabMessage::Selected(
                    ModelConfigTab::ModelRoutes
                ))
            ));
            apply(app, vec![Message::Settings(message)]);
            assert_eq!(app.model_settings.active_tab, ModelConfigTab::ModelRoutes);
            assert!(scene.labels(view(app)).contains(content));
        },
    )
    .await;
}
