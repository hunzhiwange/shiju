//! Domain layout, drawing, and interaction contracts migrated from task 17.
use crate::app_init::state::EmbeddingRouteDraft;
use crate::embedding_view_message::EmbeddingRoutesMessage;
use crate::state::ModelRoute;
use crate::view_message::{Message, ModelRoutesMessage, SettingsMessage};
use iced::advanced::renderer::{Headless, Style};
use iced::advanced::{Layout, clipboard, layout, widget::Tree};
use iced::{Element, Event, Font, Point, Rectangle, Renderer, Size, Theme, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};

use config_ui::DesktopLocale;

const EMBEDDING_DIMENSIONS: &[usize] = &[0, 3, 0, 1, 0, 3, 0];
const MODEL_PRIORITY: &[usize] = &[3, 0, 1, 0, 3, 0];

struct Fixture {
    embedding_routes_settings: crate::app_init::state::EmbeddingRoutesSettingsState,
    model_routes_settings: crate::state::ModelRoutesSettingsState,
    desktop_locale: DesktopLocale,
}
fn fixture() -> Fixture {
    let mut app = Fixture {
        embedding_routes_settings: Default::default(),
        model_routes_settings: Default::default(),
        desktop_locale: DesktopLocale::English,
    };
    app.embedding_routes_settings.routes = vec![EmbeddingRouteDraft {
        pattern: "记忆／記憶".into(),
        provider: format!("custom:https://fixture.invalid/{}", "long-path/".repeat(15)),
        model: "嵌入模型／嵌入模型".into(),
        dimensions: "1024".into(),
        api_key_input: "synthetic-credential".into(),
    }];
    app.model_routes_settings.routes = ["写代码／寫程式", "reasoning"]
        .into_iter()
        .map(|pattern| ModelRoute {
            pattern: pattern.into(),
            provider: "fixture-a".into(),
            model: "vision".into(),
            priority_input: "-10".into(),
        })
        .collect();
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
        ] {
            let mut scene = Scene::new(
                view,
                &mut renderer,
                Size::new(width, if width == 420 { 1500 } else { 1300 }),
            );
            assert_row(&scene, field, width, &[1, 0, 0]);
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

fn style(app: &Fixture) -> crate::view_style::ViewStyle<'_> {
    static THEME: Theme = Theme::Dark;
    crate::view_style::ViewStyle {
        locale: app.desktop_locale,
        theme: &THEME,
        help_text: |_, body, _, _| iced::widget::text(body).into(),
    }
}
fn embedded_view(app: &Fixture) -> Element<'_, Message> {
    crate::embedding_routes_view::embedded_view(&app.embedding_routes_settings, style(app))
}
mod system_settings_model_routes {
    use super::*;
    pub fn view(app: &Fixture) -> Element<'_, Message> {
        crate::model_routes_view::view(&app.model_routes_settings.routes, style(app))
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn ui_refresh_11_draft_messages_keep_route_indices_and_values() {
    let mut app = fixture();
    app.embedding_routes_settings.routes[0].dimensions.clear();
    app.embedding_routes_settings.routes[0].api_key_input.clear();
    app.model_routes_settings.routes[1].priority_input.clear();
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
}
