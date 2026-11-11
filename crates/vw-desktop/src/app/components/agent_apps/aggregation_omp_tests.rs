use super::overlay;
use crate::app::{Message, message::settings::aggregation_omp::Form};
use iced::advanced::renderer::{Headless, Style};
use iced::{
    Font, Renderer, Size, Theme, mouse,
    widget::{Space, text_editor},
};
use iced_runtime::user_interface::{Cache, UserInterface};
use vw_gateway_client::vw_api_types::agent_apps::aggregation::OmpDirectoryState;

#[derive(Default)]
struct Geometry {
    header: Option<iced::Rectangle>,
    footer: Option<iced::Rectangle>,
    scroll: Option<(iced::Rectangle, iced::Rectangle, iced::Vector)>,
}
impl iced::advanced::widget::operation::Operation for Geometry {
    fn traverse(
        &mut self,
        operate: &mut dyn FnMut(&mut dyn iced::advanced::widget::operation::Operation),
    ) {
        operate(self);
    }
    fn text(
        &mut self,
        _: Option<&iced::advanced::widget::Id>,
        bounds: iced::Rectangle,
        value: &str,
    ) {
        match value {
            "OMP 独立模型条目" => self.header = Some(bounds),
            "保存条目" => self.footer = Some(bounds),
            _ => {}
        }
    }
    fn scrollable(
        &mut self,
        _: Option<&iced::advanced::widget::Id>,
        bounds: iced::Rectangle,
        content: iced::Rectangle,
        translation: iced::Vector,
        _: &mut dyn iced::advanced::widget::operation::Scrollable,
    ) {
        self.scroll.get_or_insert((bounds, content, translation));
    }
}
#[tokio::test]
async fn aggregation_t66_iced_draws_dynamic_chinese_in_both_themes() {
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    let mut renderer =
        Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
            .await
            .expect("software renderer");
    let size = Size::new(900, 820);
    for (theme_name, theme) in [("light", Theme::Light), ("dark", Theme::Dark)] {
        for failed in [false, true] {
            let form = Form {
                state: Some(OmpDirectoryState {
                    revision: "r1".into(),
                    profile: Some("中文工作档案／动态配置".into()),
                    members: vec!["one".into(), "two".into()],
                    default_provider_id: Some("内置供应商".into()),
                    default_model_id: Some("中文模型／model-a".into()),
                }),
                name: "动态中文模型名称".into(),
                id: "one".into(),
                declaration: text_editor::Content::with_text(
                    "baseUrl: https://example.invalid/v1\napi: openai-completions\nmodels:\n  - id: model-a\n    name: 动态中文模型",
                ),
                error: failed.then(|| "OMP YAML 无法安全编辑，草稿已保留，请修正后重试".into()),
                ..Form::default()
            };
            let element = overlay(
                &form,
                Space::new().width(iced::Length::Fill).height(iced::Length::Fill).into(),
            );
            let mut ui: UserInterface<'_, Message, Theme, Renderer> = UserInterface::build(
                element,
                Size::new(size.width as f32, size.height as f32),
                Cache::new(),
                &mut renderer,
            );
            let mut messages = Vec::new();
            ui.update(
                &[iced::Event::Window(iced::window::Event::RedrawRequested(
                    iced::time::Instant::now(),
                ))],
                mouse::Cursor::Unavailable,
                &mut renderer,
                &mut iced::advanced::clipboard::Null,
                &mut messages,
            );
            ui.draw(
                &mut renderer,
                &theme,
                &Style { text_color: theme.palette().text },
                mouse::Cursor::Unavailable,
            );
            let pixels = renderer.screenshot(size, 1.0, theme.palette().background);
            assert_eq!(pixels.len(), size.width as usize * size.height as usize * 4);
            assert!(pixels.chunks_exact(4).any(|p| p != &pixels[..4]), "modal must paint content");
            if let Some(root) = std::env::var_os("SHIJU_T66_RENDER_DIR") {
                std::fs::create_dir_all(&root).unwrap();
                image::save_buffer(
                    std::path::PathBuf::from(root).join(format!(
                        "{theme_name}-{}.png",
                        if failed { "error" } else { "ready" }
                    )),
                    &pixels,
                    size.width,
                    size.height,
                    image::ColorType::Rgba8,
                )
                .unwrap();
            }
            let mut before = Geometry::default();
            ui.operate(&renderer, &mut before);
            messages.clear();
            ui.update(
                &[
                    iced::Event::Mouse(mouse::Event::WheelScrolled {
                        delta: mouse::ScrollDelta::Lines { x: 0.0, y: -80.0 },
                    }),
                    iced::Event::Window(iced::window::Event::RedrawRequested(
                        iced::time::Instant::now(),
                    )),
                ],
                mouse::Cursor::Available(iced::Point::new(450.0, 400.0)),
                &mut renderer,
                &mut iced::advanced::clipboard::Null,
                &mut messages,
            );
            ui.draw(
                &mut renderer,
                &theme,
                &Style { text_color: theme.palette().text },
                mouse::Cursor::Unavailable,
            );
            let bottom = renderer.screenshot(size, 1.0, theme.palette().background);
            assert!(pixels != bottom, "body must scroll while drawing the editor");
            if let Some(root) = std::env::var_os("SHIJU_T66_RENDER_DIR") {
                image::save_buffer(
                    std::path::PathBuf::from(root).join(format!(
                        "{theme_name}-{}-editor.png",
                        if failed { "error" } else { "ready" }
                    )),
                    &bottom,
                    size.width,
                    size.height,
                    image::ColorType::Rgba8,
                )
                .unwrap();
            }
            let mut after = Geometry::default();
            ui.operate(&renderer, &mut after);
            assert!(before.header.is_some() && before.footer.is_some());
            assert_eq!(before.header, after.header, "title geometry must stay fixed");
            assert_eq!(before.footer, after.footer, "footer geometry must stay fixed");
            let (viewport, content, offset) = after.scroll.expect("body scroll viewport");
            assert!(content.height > viewport.height);
            assert!(offset.y > before.scroll.unwrap().2.y, "body must scroll to reveal the editor");
            let _ = ui.into_cache();
        }
    }
}
