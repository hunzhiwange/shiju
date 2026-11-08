use super::embedded_view;
use crate::app::components::{
    system_settings_model_routes, system_settings_multimodal, system_settings_query_classification,
};
use crate::app::i18n::I18n;
use crate::app::message::ViewMessage;
use crate::app::message::settings::{
    EmbeddingRoutesMessage, ModelRoutesMessage, MultimodalMessage, QueryClassificationMessage,
    SettingsMessage,
};
use crate::app::state::{
    EmbeddingRouteDraft, ModelConfigTab, ModelRoute, ModelSummary, ProviderModelsSummary,
    QueryClassificationRuleInput,
};
use crate::app::{App, Message};
use iced::advanced::renderer::{Headless, Style};
use iced::advanced::widget::operation::{Operation, Scrollable};
use iced::advanced::{Layout, clipboard, layout, widget::Tree};
use iced::keyboard::{self, Key, key::Named};
use iced::{Element, Event, Font, Point, Rectangle, Renderer, Size, Theme, Vector, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};

use serde_json::json;
use vw_config_types::ui::DesktopLocale;

const OCR_PICKER: &[usize] = &[4, 0, 4, 0, 1, 0, 0];
const CATEGORY_ROW: &[usize] = &[4, 0, 1, 0, 1, 0];
const EMBEDDING_DIMENSIONS: &[usize] = &[0, 3, 0, 1, 0, 3, 0];
const MODEL_PRIORITY: &[usize] = &[3, 0, 1, 0, 3, 0];
const RULE_PRIORITY: &[usize] = &[4, 0, 1, 0, 2, 0];

const FIRST_MODEL: &str = "fixture-a/vision";
const SECOND_MODEL: &str = "fixture-b/vision";

fn fixture() -> App {
    let mut app = None;
    crate::app::config::with_editor_settings_gateway(|fixture, _, _| app = Some(fixture));
    let mut app = app.expect("isolated settings fixture");
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
    app.embedding_routes_settings.routes = vec![EmbeddingRouteDraft {
        pattern: "记忆／記憶".into(),
        provider: format!("custom:https://fixture.invalid/{}", "long-path/".repeat(15)),
        model: "嵌入模型／嵌入模型".into(),
        dimensions: "1024".into(),
        api_key_input: "synthetic-credential".into(),
    }];
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
            ("embedding", embedded_view(&app), EMBEDDING_DIMENSIONS),
            ("routing", system_settings_model_routes::view(&app), MODEL_PRIORITY),
            ("classification", system_settings_query_classification::view(&app), CATEGORY_ROW),
            ("multimodal", system_settings_multimodal::view(&app), &[2, 0, 0, 0][..]),
        ] {
            let mut scene = Scene::new(
                view,
                &mut renderer,
                Size::new(width, if width == 420 { 1500 } else { 1300 }),
            );
            assert_row(
                &scene,
                field,
                width,
                if page == "multimodal" { &[1, 0] } else { &[1, 0, 0] },
            );
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
            if page == "embedding" && width == 420 {
                let third_template = scene.bounds(&[0, 1, 0, 1, 2]);
                assert!(third_template.width > 90.0);
                assert!(third_template.x + third_template.width < width as f32);
                assert!(matches!(
                    scene.click(&mut renderer, third_template.center()).as_slice(),
                    [Message::Settings(SettingsMessage::EmbeddingRoutes(
                        EmbeddingRoutesMessage::InsertShijuTemplate
                    ))]
                ));
            }
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
    app.embedding_routes_settings.routes[0].dimensions.clear();
    app.embedding_routes_settings.routes[0].api_key_input.clear();
    app.model_routes_settings.routes[1].priority_input.clear();
    app.query_classification_settings.rules[0].priority_input.clear();
    let mut renderer = renderer().await;
    let size = Size::new(640, 1100);

    let mut embedding = Scene::new(embedded_view(&app), &mut renderer, size);
    assert!(matches!(
        embedding.edit(&mut renderer, &child(EMBEDDING_DIMENSIONS, &[1, 0, 0]), "1536").as_slice(),
        [Message::Settings(SettingsMessage::EmbeddingRoutes(
            EmbeddingRoutesMessage::DimensionsChanged(0, value)
        ))] if value == "1536"
    ));
    assert!(matches!(
        embedding.edit(&mut renderer, &[0, 3, 0, 1, 0, 4, 0, 1, 0, 0], "synthetic-edit").as_slice(),
        [Message::Settings(SettingsMessage::EmbeddingRoutes(
            EmbeddingRoutesMessage::ApiKeyChanged(0, value)
        ))] if value == "synthetic-edit"
    ));
    let save = embedding.bounds(&[0, 4, 1]).center();
    assert!(matches!(
        embedding.click(&mut renderer, save).as_slice(),
        [Message::Settings(SettingsMessage::EmbeddingRoutes(EmbeddingRoutesMessage::Save))]
    ));

    let mut routing = Scene::new(system_settings_model_routes::view(&app), &mut renderer, size);
    assert!(matches!(
        routing.edit(&mut renderer, &child(MODEL_PRIORITY, &[1, 0, 0]), "-12").as_slice(),
        [Message::Settings(SettingsMessage::ModelRoutes(
            ModelRoutesMessage::PriorityChanged(1, value)
        ))] if value == "-12"
    ));

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
    let remote = multimodal.bounds(&[2, 0, 4, 0, 1, 0]).center();
    let current = app.multimodal_settings.allow_remote_fetch;
    assert!(matches!(
        multimodal.click(&mut renderer, remote).as_slice(),
        [Message::Settings(SettingsMessage::Multimodal(MultimodalMessage::AllowRemoteFetchToggled(value)))]
            if *value != current
    ));
    assert_eq!(app.multimodal_settings.ocr_model, FIRST_MODEL);
}

struct InlineExplanation<'a> {
    description: &'a str,
    visible: bool,
}

impl Operation for InlineExplanation<'_> {
    fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) {
        children(self);
    }

    fn text(&mut self, _: Option<&iced::advanced::widget::Id>, _: Rectangle, value: &str) {
        self.visible |= value == self.description;
    }
}

fn assert_api_key_help_actions(scene: &mut Scene<'_>, renderer: &mut Renderer, icon: Rectangle) {
    // The API key label sits on the left, with space for the help card and menu on its right.
    let card_text = Point::new(icon.x + icon.width + 22.0, icon.y + 16.0);
    let row_height =
        f32::from(iced::advanced::text::LineHeight::default().to_absolute(12.0.into())) + 10.0;
    let description = "可选覆盖凭据；阿里填 DashScope API Key。";
    let url = format!("https://www.google.com/search?q={}", urlencoding::encode(description));
    for (row, expected) in [
        (0.0, Message::CopyCode(description.into())),
        (3.0, Message::View(ViewMessage::OpenUrlExternal(url))),
    ] {
        assert!(
            scene
                .send(
                    renderer,
                    &[Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right))],
                    mouse::Cursor::Available(card_text),
                )
                .is_empty(),
            "opening help's text menu must not trigger an external action"
        );
        let item = Point::new(card_text.x + 24.0, card_text.y + 3.0 + row_height * (row + 0.5));
        let messages = scene.click(renderer, item);
        assert!(
            match (messages.as_slice(), expected) {
                ([Message::CopyCode(text)], Message::CopyCode(expected)) => text == &expected,
                (
                    [Message::View(ViewMessage::OpenUrlExternal(url))],
                    Message::View(ViewMessage::OpenUrlExternal(expected)),
                ) => url == &expected,
                _ => false,
            },
            "{messages:?}"
        );
    }
}

async fn assert_contextual_help(tab: Option<ModelConfigTab>, count: usize, description: &str) {
    fn collect_icons(layout: Layout<'_>, icons: &mut Vec<Rectangle>) {
        let bounds = layout.bounds();
        if bounds.size() == Size::new(22.0, 22.0) && !icons.contains(&bounds) {
            icons.push(bounds);
        }
        for child in layout.children() {
            collect_icons(child, icons);
        }
    }

    let mut renderer = renderer().await;
    let mut app = fixture();
    app.model_routes_settings.routes.truncate(1);
    for (locale, theme, width) in [
        (DesktopLocale::English, Theme::Light, 420),
        (DesktopLocale::SimplifiedChinese, Theme::Dark, 640),
        (DesktopLocale::TraditionalChinese, Theme::Light, 520),
    ] {
        app.desktop_locale = locale;
        app.app_theme = theme.clone();
        let element = if let Some(tab) = tab {
            app.model_settings.active_tab = tab;
            crate::app::components::system_settings_models::view(&app)
        } else {
            embedded_view(&app)
        };
        let name = format!("help-07-{tab:?}-{locale:?}");
        let size = Size::new(width, 940);
        let mut scene = Scene::new(element, &mut renderer, size);
        let mut explanation =
            InlineExplanation { description: I18n::new(locale).__(description), visible: false };
        scene.ui.operate(&renderer, &mut explanation);
        assert!(!explanation.visible, "{name}: explanation must be hidden while idle");
        let mut icons = Vec::new();
        collect_icons(Layout::new(&scene.layout), &mut icons);
        assert_eq!(icons.len(), count, "{name}: each explanation has a question mark");
        let frame = |scene: &mut Scene<'_>, renderer: &mut Renderer, suffix: &str| {
            scene.capture(renderer, &theme, mouse::Cursor::Unavailable, &format!("{name}-{suffix}"))
        };
        let idle = frame(&mut scene, &mut renderer, "idle");
        for (index, icon) in icons.into_iter().enumerate() {
            let point = icon.center();
            assert!(Rectangle::with_size(Size::new(width as f32, 940.0)).contains(point));
            assert!(scene.click(&mut renderer, point).is_empty());
            assert!(
                scene
                    .send(
                        &mut renderer,
                        &[Event::Mouse(mouse::Event::CursorLeft)],
                        mouse::Cursor::Unavailable,
                    )
                    .is_empty()
            );
            assert!(idle != frame(&mut scene, &mut renderer, &format!("open-{index}")));
            if tab.is_none() && locale == DesktopLocale::SimplifiedChinese && index + 1 == count {
                assert_api_key_help_actions(&mut scene, &mut renderer, icon);
            }
            assert!(
                scene
                    .send(&mut renderer, &[key(Named::Escape)], mouse::Cursor::Unavailable)
                    .is_empty()
            );
            assert!(idle == frame(&mut scene, &mut renderer, "dismissed"));
        }
    }
}

#[tokio::test]
async fn help_rollout_07_renders_contextual_help() {
    assert_contextual_help(
        Some(ModelConfigTab::Models),
        3,
        "浏览已连接 provider 的模型目录，搜索、筛选并切换模型可用状态。",
    )
    .await;
}

#[tokio::test]
async fn help_rollout_07_model_route_renders_contextual_help() {
    assert_contextual_help(
        Some(ModelConfigTab::ModelRoutes),
        6,
        "将匹配模式映射到指定 provider/model，并与查询分类优先级保持一致。",
    )
    .await;
}

#[tokio::test]
async fn help_rollout_07_query_classification_renders_contextual_help() {
    assert_contextual_help(
        Some(ModelConfigTab::QueryClassification),
        7,
        "将请求 pattern 映射到已配置的模型路由。",
    )
    .await;
}

#[tokio::test]
async fn help_rollout_07_embedding_route_renders_contextual_help() {
    assert_contextual_help(
        None,
        9,
        "按场景配置嵌入提供商、模型、维度和 API Key；在“嵌入与检索”中用 hint:模式名 引用。",
    )
    .await;
}
