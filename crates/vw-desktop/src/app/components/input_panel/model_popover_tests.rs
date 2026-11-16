//! 验证模型选择弹窗的分组、搜索、能力标识与选择消息。
//! 同时约束限定模型名、裸模型名、Unicode 截断及刷新时保留旧目录的回退。

use super::model_popover::model_popover_content;
use crate::app::state::{ModelSummary, ProviderModelsSummary};

// 模型 fixture 默认开启工具能力；需要验证能力徽标时再显式覆盖各能力位。
fn model(id: &str, name: &str, enabled: bool) -> ModelSummary {
    ModelSummary {
        id: id.to_string(),
        name: name.to_string(),
        enabled,
        toolcall: true,
        attachment: false,
        context_limit: 128_000,
        detail: serde_json::json!({}),
    }
}

fn provider(id: &str, name: &str, models: Vec<ModelSummary>) -> ProviderModelsSummary {
    ProviderModelsSummary { id: id.to_string(), name: name.to_string(), models }
}









#[derive(Default)]
struct ForkModelLabels(Vec<(String, iced::Rectangle)>);

impl iced::advanced::widget::Operation for ForkModelLabels {
    fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn iced::advanced::widget::Operation)) {
        children(self);
    }
    fn text(
        &mut self,
        _: Option<&iced::advanced::widget::Id>,
        bounds: iced::Rectangle,
        value: &str,
    ) {
        self.0.push((value.into(), bounds));
    }
}

// 宿主保留：ACP 会话执行状态与来源控件的宿主交互契约。
#[tokio::test(flavor = "multi_thread")]
async fn fork_selection_popover_click_and_chinese_render_follow_execution_state() {
    use crate::app::{Message, message::ChatMessage};
    use iced::advanced::{
        clipboard,
        renderer::{Headless, Style},
    };
    use iced::{Color, Event, Font, Pixels, Renderer, Size, Theme, mouse};
    use iced_runtime::user_interface::{Cache, UserInterface};
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    let mut renderer =
        <Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia")).await.unwrap();
    for theme in [Theme::Light, Theme::Dark] {
        for state in [Some(false), Some(true), None] {
            let (mut app, _) = crate::app::App::new();
            app.active_session_id = Some("fork".into());
            app.current_session_runtime_mut().fork_execution_started = state;
            app.model_settings.providers =
                vec![provider("custom", "供应商", vec![model("new", "中文模型", true)])];
            if state != Some(false) {
                let _ = crate::app::message::chat::update(
                    &mut app,
                    ChatMessage::SessionAgentSelected(None),
                );
            }
            let view: iced::Element<'_, Message> = iced::widget::column![
                model_popover_content(&app, false, "custom/old"),
                crate::app::components::toast::view(&app)
            ]
            .spacing(12)
            .into();
            let mut ui =
                UserInterface::build(view, Size::new(640.0, 780.0), Cache::new(), &mut renderer);
            let mut labels = ForkModelLabels::default();
            ui.operate(&renderer, &mut labels);
            let header =
                labels.0.iter().find(|(label, _)| label == "供应商").expect("visible provider").1;
            let point = iced::Point::new(header.x + 20.0, header.y + header.height + 30.0);
            let mut messages = Vec::new();
            ui.update(
                &[
                    Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                    Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                ],
                mouse::Cursor::Available(point),
                &mut renderer,
                &mut clipboard::Null,
                &mut messages,
            );
            assert_eq!(messages.iter().any(|message| matches!(message, Message::Chat(ChatMessage::ModelSelected(model)) if model == "custom/new")), state == Some(false));
            ui.draw(
                &mut renderer,
                &theme,
                &Style { text_color: theme.palette().text },
                mouse::Cursor::Unavailable,
            );
            let pixels = renderer.screenshot(Size::new(640, 780), 1.0, Color::TRANSPARENT);
            assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
            let directory = std::env::temp_dir().join("shiju-fork-selection-render");
            std::fs::create_dir_all(&directory).unwrap();
            ::image::RgbaImage::from_raw(640, 780, pixels)
                .unwrap()
                .save(directory.join(format!("{theme:?}-{state:?}.png")))
                .unwrap();
        }
    }
}
