// 宿主契约：MODEL_TABS 路由、SettingsMessage 映射、目录消息更新与详情 overlay 装配；保留原绘制和状态断言。
use super::{MODEL_TABS, view};
use crate::app::message::settings::{self, ModelConfigTabMessage, SettingsMessage};
use crate::app::state::{ModelConfigTab, ModelSummary, ProviderModelsSummary};
use crate::app::{App, Message};
use iced::advanced::renderer::{Headless, Style};
use iced::advanced::{Layout, clipboard, layout, widget::Tree};
use iced::{Element, Event, Font, Point, Rectangle, Renderer, Size, Theme, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
use serde_json::json;
use config_ui::DesktopLocale;

const PROVIDER: &str = "fixture-provider";
const MODEL: &str = "fixture-model-with-an-unusually-long-stable-identifier-for-narrow-layouts";

fn fixture() -> App {
    let mut app = None;
    crate::app::config::with_editor_settings_gateway(|fixture, _, _| app = Some(fixture));
    let mut app = app.expect("isolated settings fixture");
    app.general_settings.desktop_locale = DesktopLocale::English;
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


fn apply(app: &mut App, messages: Vec<Message>) {
    let [Message::Settings(message)]: [Message; 1] = messages.try_into().expect("one message")
    else {
        panic!("expected a settings message");
    };
    // Apply synchronous UI changes only. The returned refresh tasks are never run.
    let _ = settings::update(app, message);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn ui_refresh_10_layout_and_interaction() {
    let mut renderer = renderer().await;
    let mut app = fixture();
    // At 420 px the English segments need a second line. All original routes remain clickable.
    app.general_settings.desktop_locale = DesktopLocale::English;
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
            let locale = app.general_settings.desktop_locale;
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
