//! Domain layout, drawing, and interaction contracts migrated from task 17.
use crate::{
    Message,
    messages::{MultimodalMessage, QueryClassificationMessage, SettingsMessage},
    state::*,
};
use desktop_settings_providers::state::{ModelRoute, ModelSummary, ProviderModelsSummary};
use iced::advanced::renderer::{Headless, Style};
use iced::advanced::widget::operation::{Operation, Scrollable};
use iced::advanced::{Layout, clipboard, layout, widget::Tree};
use iced::keyboard::{self, Key, key::Named};
use iced::{Element, Event, Font, Point, Rectangle, Renderer, Size, Theme, Vector, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};

use config_ui::DesktopLocale;
use serde_json::json;

const OCR_PICKER: &[usize] = &[4, 0, 4, 0, 1, 0, 0, 0];
const REMOTE_FETCH: &[usize] = &[2, 0, 4, 0, 1, 0, 0];
const CATEGORY_ROW: &[usize] = &[4, 0, 1, 0, 1, 0];
const RULE_PRIORITY: &[usize] = &[4, 0, 1, 0, 2, 0];

const FIRST_MODEL: &str = "fixture-a/vision";
const SECOND_MODEL: &str = "fixture-b/vision";

fn fixture() -> Fixture {
    let mut app = Fixture::default();
    app.model_settings.providers = ["fixture-a", "fixture-b"]
        .into_iter()
        .map(|id| ProviderModelsSummary {
            id: id.into(),
            name: "测试提供商／測試供應商".into(),
            models: vec![ModelSummary {
                id: "vision".into(),
                name: "同名视觉模型／同名視覺模型".into(),
                enabled: true,
                toolcall: false,
                attachment: true,
                context_limit: 8192,
                detail: json!({"capabilities": {"input": {"image": true}}}),
            }],
        })
        .collect();
    app.multimodal_settings.ocr_model = FIRST_MODEL.into();
    app.multimodal_settings.ocr_enabled = true;
    app.model_routes_settings.routes = ["写代码／寫程式", "reasoning"]
        .into_iter()
        .map(|pattern| ModelRoute {
            pattern: pattern.into(),
            provider: "fixture-a".into(),
            model: "vision".into(),
            priority_input: "-10".into(),
        })
        .collect();
    app.query_classification_settings.enabled = true;
    app.query_classification_settings.rules = vec![QueryClassificationRuleInput {
        pattern: "fixture".into(),
        category: "写代码／寫程式".into(),
        priority_input: "-5".into(),
    }];
    app
}

async fn renderer() -> Renderer {
    static FONTS: std::sync::Once = std::sync::Once::new();
    FONTS.call_once(|| {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in desktop_assets::fonts::load_all() {
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

    fn edit(&mut self, renderer: &mut Renderer, path: &[usize], value: &str) -> Vec<Message> {
        let point = self.bounds(path).center();
        assert!(self.click(renderer, point).is_empty());
        self.send(
            renderer,
            &[Event::InputMethod(iced::advanced::input_method::Event::Commit(value.into()))],
            mouse::Cursor::Available(point),
        )
    }

    fn capture(
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
        pixels
    }
}

fn child(path: &[usize], rest: &[usize]) -> Vec<usize> {
    [path, rest].concat()
}

fn assert_centered(a: Rectangle, b: Rectangle) {
    assert!((a.center().y - b.center().y).abs() < 0.5, "{a:?} vs {b:?}");
}

fn assert_row(scene: &Scene<'_>, path: &[usize], width: u32, control_path: &[usize]) {
    let label = scene.bounds(&child(path, &[0]));
    let control = scene.bounds(&child(path, control_path));
    assert_centered(label, control);
    assert!(label.width > 0.0 && label.x + label.width < control.x);
    assert!(control.x + control.width <= width as f32);
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

#[derive(Default)]
struct Fixture {
    model_settings: desktop_settings_providers::state::ModelSettingsState,
    model_routes_settings: desktop_settings_providers::state::ModelRoutesSettingsState,
    multimodal_settings: MultimodalSettingsState,
    query_classification_settings: QueryClassificationSettingsState,
    desktop_locale: DesktopLocale,
}
mod system_settings_multimodal {
    use super::*;
    pub fn view(app: &Fixture) -> Element<'_, Message> {
        static THEME: Theme = Theme::Dark;
        crate::system_settings_multimodal::view(crate::system_settings_multimodal::View {
            multimodal_settings: &app.multimodal_settings,
            model_settings: &app.model_settings,
            desktop_locale: app.desktop_locale,
            app_theme: &THEME,
            render_help: |_, body, _, _| iced::widget::text(body).into(),
        })
    }
}
mod system_settings_query_classification {
    use super::*;
    pub fn view(app: &Fixture) -> Element<'_, Message> {
        static THEME: Theme = Theme::Dark;
        crate::system_settings_query_classification::view(
            crate::system_settings_query_classification::View {
                query_classification_settings: &app.query_classification_settings,
                model_routes: &app.model_routes_settings.routes,
                desktop_locale: app.desktop_locale,
                app_theme: &THEME,
                render_help: |_, body, _, _| iced::widget::text(body).into(),
            },
        )
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn ui_refresh_11_layout_and_interaction() {
    let mut renderer = renderer().await;
    let mut app = fixture();
    for (locale, theme, width, name) in [
        (DesktopLocale::English, Theme::Light, 640, "en-light-640"),
        (DesktopLocale::SimplifiedChinese, Theme::Dark, 960, "zh-dark-960"),
        (DesktopLocale::TraditionalChinese, Theme::Light, 420, "hant-light-420"),
        (DesktopLocale::English, Theme::Dark, 420, "en-dark-420"),
        (DesktopLocale::SimplifiedChinese, Theme::Light, 640, "zh-light-640"),
        (DesktopLocale::TraditionalChinese, Theme::Dark, 960, "hant-dark-960"),
    ] {
        app.desktop_locale = locale;
        for (page, view, field) in [
            ("classification", system_settings_query_classification::view(&app), CATEGORY_ROW),
            ("multimodal", system_settings_multimodal::view(&app), &[2, 0, 0, 0][..]),
        ] {
            let mut scene = Scene::new(
                view,
                &mut renderer,
                Size::new(width, if width == 420 { 1500 } else { 1300 }),
            );
            assert_row(&scene, field, width, &[1, 0, 0]);
            if page == "multimodal" {
                let picker = scene.bounds(OCR_PICKER);
                assert_centered(picker, scene.bounds(&child(OCR_PICKER, &[0, 0])));
                assert_centered(picker, scene.bounds(&child(OCR_PICKER, &[0, 1])));
            }
            scene.capture(
                &mut renderer,
                &theme,
                mouse::Cursor::Unavailable,
                &format!("{page}-{name}"),
            );
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn ui_refresh_11_preserves_values_and_disabled_states() {
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::English;
    let mut renderer = renderer().await;
    let mut scene =
        Scene::new(system_settings_multimodal::view(&app), &mut renderer, Size::new(640, 1000));
    let picker = scene.bounds(OCR_PICKER).center();
    assert!(scene.click(&mut renderer, picker).is_empty());
    let cursor = mouse::Cursor::Unavailable;
    assert!(scene.send(&mut renderer, &[key(Named::Escape)], cursor).is_empty());
    for event in [Named::Enter, Named::ArrowDown] {
        assert!(scene.send(&mut renderer, &[key(event)], cursor).is_empty());
    }
    let messages = scene.send(&mut renderer, &[key(Named::Enter)], cursor);
    assert!(
        matches!(
            messages.as_slice(),
            [Message::Settings(SettingsMessage::Multimodal(MultimodalMessage::OcrModelChanged(value)))]
                if value == SECOND_MODEL
        ),
        "keyboard selection must submit the second provider's original ID once"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn ui_refresh_11_draft_messages_keep_route_indices_and_values() {
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::English;
    app.model_routes_settings.routes[1].priority_input.clear();
    app.query_classification_settings.rules[0].priority_input.clear();
    let mut renderer = renderer().await;
    let size = Size::new(640, 1100);

    let mut classification =
        Scene::new(system_settings_query_classification::view(&app), &mut renderer, size);
    assert!(matches!(
        classification.edit(&mut renderer, &child(RULE_PRIORITY, &[1, 0, 0]), "-6").as_slice(),
        [Message::Settings(SettingsMessage::QueryClassification(
            QueryClassificationMessage::PriorityChanged(0, value)
        ))] if value == "-6"
    ));
    let enabled = classification.bounds(&[2, 0, 0, 1, 0, 0]).center();
    assert!(matches!(
        classification.click(&mut renderer, enabled).as_slice(),
        [Message::Settings(SettingsMessage::QueryClassification(
            QueryClassificationMessage::EnabledToggled(false)
        ))]
    ));

    let category = classification.bounds(&child(CATEGORY_ROW, &[1, 0, 0]));
    assert!(classification.click(&mut renderer, category.center()).is_empty());
    assert!(
        classification
            .send(&mut renderer, &[key(Named::End)], mouse::Cursor::Unavailable)
            .is_empty()
    );
    assert!(matches!(
        classification.send(&mut renderer, &[key(Named::Enter)], mouse::Cursor::Unavailable).as_slice(),
        [Message::Settings(SettingsMessage::QueryClassification(
            QueryClassificationMessage::CategoryChanged(0, value)
        ))] if value == "reasoning"
    ));
}

#[derive(Default)]
struct MenuScroll(Option<(Rectangle, f32)>);

impl Operation for MenuScroll {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        _id: Option<&iced::advanced::widget::Id>,
        bounds: Rectangle,
        _content: Rectangle,
        translation: Vector,
        _state: &mut dyn Scrollable,
    ) {
        self.0 = Some((bounds, translation.y));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn ui_refresh_11_long_menu_scrolls_and_cancel_does_not_click_through() {
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::English;
    app.model_routes_settings.routes.extend((0..30).map(|index| ModelRoute {
        pattern: format!("route-{index:02} / {} / 中文路由", "very-long-category-name-".repeat(4)),
        ..ModelRoute::default()
    }));

    let mut renderer = renderer().await;
    let size = Size::new(420, 940);
    let mut scene =
        Scene::new(system_settings_query_classification::view(&app), &mut renderer, size);
    let picker = scene.bounds(&child(CATEGORY_ROW, &[1, 0, 0])).center();
    assert!(scene.click(&mut renderer, picker).is_empty());

    let mut scroll = MenuScroll::default();
    scene.ui.operate(&renderer, &mut scroll);
    let (menu, offset) = scroll.0.expect("open menu scroll area");
    assert_eq!(offset, 0.0);
    assert!(menu.x >= 8.0 && menu.y >= 8.0);
    assert!(menu.x + menu.width <= 412.0 && menu.y + menu.height <= 932.0);
    for (point, name) in [
        (Point::new(menu.center().x, menu.y + 18.0), "category-hover-first"),
        (Point::new(menu.center().x, menu.y + 56.0), "category-hover-second"),
        (Point::new(2.0, 2.0), "category-pointer-out"),
    ] {
        let cursor = mouse::Cursor::Available(point);
        assert!(
            scene
                .send(
                    &mut renderer,
                    &[Event::Mouse(mouse::Event::CursorMoved { position: point })],
                    cursor
                )
                .is_empty()
        );
        // UserInterface::draw rebuilds the overlay from the widget's state after update.
        scene.capture(&mut renderer, &Theme::Dark, cursor, name);
    }
    let cursor = mouse::Cursor::Available(menu.center());
    let wheel = Event::Mouse(mouse::Event::WheelScrolled {
        delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -145.0 },
    });
    assert!(scene.send(&mut renderer, &[wheel], cursor).is_empty());
    scene.ui.operate(&renderer, &mut scroll);
    assert!(scroll.0.unwrap().1 > 0.0);
    scene.capture(&mut renderer, &Theme::Dark, cursor, "category-scrolled");
    assert!(scene.send(&mut renderer, &[key(Named::Escape)], cursor).is_empty());

    assert!(scene.click(&mut renderer, picker).is_empty());
    let add = scene.bounds(&[0, 2]).center();
    assert!(scene.click(&mut renderer, add).is_empty(), "dismiss click reached the add button");
    assert!(matches!(
        scene.click(&mut renderer, add).as_slice(),
        [Message::Settings(SettingsMessage::QueryClassification(
            QueryClassificationMessage::AddRule
        ))]
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn ui_refresh_11_empty_catalog_and_numeric_limits_keep_existing_actions() {
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::English;
    app.model_routes_settings.routes.clear();
    app.model_settings.providers.clear();
    app.model_settings.loading = true;
    app.multimodal_settings.max_images = 1;
    let mut renderer = renderer().await;
    let size = Size::new(640, 1100);
    let mut classification =
        Scene::new(system_settings_query_classification::view(&app), &mut renderer, size);
    let placeholder = classification.bounds(&child(CATEGORY_ROW, &[1, 0, 0])).center();
    assert!(classification.click(&mut renderer, placeholder).is_empty());
    assert!(
        classification
            .send(&mut renderer, &[key(Named::Enter)], mouse::Cursor::Unavailable)
            .is_empty()
    );
    let mut scroll = MenuScroll::default();
    classification.ui.operate(&renderer, &mut scroll);
    assert!(scroll.0.is_none());
    classification.capture(
        &mut renderer,
        &Theme::Light,
        mouse::Cursor::Unavailable,
        "category-no-routes",
    );

    Scene::new(system_settings_multimodal::view(&app), &mut renderer, size).capture(
        &mut renderer,
        &Theme::Light,
        mouse::Cursor::Unavailable,
        "image-model-loading",
    );
    drop(classification);
    app.model_settings.loading = false;
    app.model_settings.save_error = Some("Fixture catalog unavailable".to_owned().into());
    let mut multimodal = Scene::new(system_settings_multimodal::view(&app), &mut renderer, size);
    let picker = multimodal.bounds(OCR_PICKER).center();
    multimodal.capture(
        &mut renderer,
        &Theme::Dark,
        mouse::Cursor::Unavailable,
        "image-model-error",
    );
    assert!(multimodal.click(&mut renderer, picker).is_empty());
    assert!(
        multimodal.send(&mut renderer, &[key(Named::End)], mouse::Cursor::Unavailable).is_empty()
    );
    // Clearing remains available, but an empty or failed catalog provides no model ID.
    assert!(
        matches!(multimodal.send(&mut renderer, &[key(Named::Enter)], mouse::Cursor::Unavailable).as_slice(),
            [Message::Settings(SettingsMessage::Multimodal(MultimodalMessage::OcrModelChanged(value)))]
                if value.is_empty()
        )
    );
    let max_images = [2, 0, 0, 0, 1, 0, 0];
    assert!(matches!(
        multimodal.edit(&mut renderer, &max_images, "99").as_slice(),
        [Message::Settings(SettingsMessage::Multimodal(MultimodalMessage::MaxImagesChanged(16)))]
    ));
    let remote = multimodal.bounds(REMOTE_FETCH).center();
    let current = app.multimodal_settings.allow_remote_fetch;
    assert!(matches!(
        multimodal.click(&mut renderer, remote).as_slice(),
        [Message::Settings(SettingsMessage::Multimodal(MultimodalMessage::AllowRemoteFetchToggled(value)))]
            if *value != current
    ));
    assert_eq!(app.multimodal_settings.ocr_model, FIRST_MODEL);
}
