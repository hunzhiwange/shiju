use crate::aggregation::subagent::*;
use vw_api_app_aggregation::*;
use vw_api_chat_identity::ReasoningEffortDto as Effort;
fn snapshot() -> CodexAggregationSettings {
    let requested = CodexSettingsValues {
        subagent_model_id: Some("published-a".into()),
        subagent_effort: Some(Effort::High),
        auto_review_model_id: Some("review-keep".into()),
        classic_tools: false,
    };
    CodexAggregationSettings {
        revision: "r1".into(),
        effective: requested.clone(),
        requested,
        choices: vec![
            CodexSettingsChoice {
                model_id: "published-a".into(),
                label: "中文模型／日本語".into(),
                efforts: vec![Effort::Low, Effort::High],
            },
            CodexSettingsChoice {
                model_id: "published-b".into(),
                label: "另一个模型".into(),
                efforts: vec![Effort::Low],
            },
        ],
        notices: vec![AggregationNotice::NewSession],
    }
}
fn loaded() -> Form {
    let state = snapshot();
    Form { draft: state.requested.clone(), snapshot: Some(state), ..Default::default() }
}

#[derive(Default)]
struct TextProbe(Vec<(String, iced::Rectangle)>);
impl iced::advanced::widget::operation::Operation for TextProbe {
    fn traverse(
        &mut self,
        children: &mut dyn FnMut(&mut dyn iced::advanced::widget::operation::Operation),
    ) {
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
#[tokio::test]
async fn aggregation_t60_actual_text_rendering_in_both_themes() {
    use iced::{
        Font, Size, Theme,
        advanced::renderer::{Headless, Style},
        mouse,
        widget::Space,
    };
    use iced_runtime::user_interface::{Cache, UserInterface};
    static FONTS: std::sync::Once = std::sync::Once::new();
    FONTS.call_once(|| {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in desktop_assets::fonts::load_all() {
            fonts.load_font(font);
        }
    });
    let mut renderer =
        iced::Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
            .await
            .unwrap();
    for (theme_name, theme) in [("light", Theme::Light), ("dark", Theme::Dark)] {
        for removed in [false, true] {
            let mut form = loaded();
            if removed {
                let s = form.snapshot.as_mut().unwrap();
                s.choices.clear();
                s.effective.subagent_model_id = None;
                s.effective.subagent_effort = None;
                s.notices.push(AggregationNotice::MultiAgentV2Forced);
                s.notices.push(AggregationNotice::EncryptedTaskCrossProvider);
            }
            let element = desktop_apps_ui::aggregation::subagent_view::overlay(
                &form,
                Space::new().width(iced::Length::Fill).height(iced::Length::Fill).into(),
            );
            let size = Size::new(960, 800);
            let mut ui =
                UserInterface::build(element, Size::new(960.0, 800.0), Cache::new(), &mut renderer);
            let mut probe = TextProbe::default();
            ui.operate(&renderer, &mut probe);
            for label in ["Codex 子代理设置", "保存", "取消"] {
                let (_, bounds) = probe.0.iter().find(|(t, _)| t == label).expect(label);
                assert!(
                    bounds.x >= 0.0
                        && bounds.y >= 0.0
                        && bounds.x + bounds.width <= 960.0
                        && bounds.y + bounds.height <= 800.0
                );
            }
            assert!(probe.0.iter().any(|(t, _)| t.contains("新建 Codex 会话")));
            if removed {
                assert!(probe.0.iter().any(|(t, _)| t.contains("所选模型已不在发布目录")));
            }
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
            assert!(messages.is_empty());
            ui.draw(
                &mut renderer,
                &theme,
                &Style { text_color: theme.palette().text },
                mouse::Cursor::Unavailable,
            );
            let pixels = renderer.screenshot(size, 1.0, theme.palette().background);
            assert_eq!(pixels.len(), 960 * 800 * 4);
            assert!(pixels.chunks_exact(4).any(|p| p[0] != p[1] || p[1] != p[2]));
            let save = probe.0.iter().find(|(t, _)| t == "保存").unwrap().1.center();
            let mut clicked = Vec::new();
            ui.update(
                &[
                    iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                    iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                ],
                mouse::Cursor::Available(save),
                &mut renderer,
                &mut iced::advanced::clipboard::Null,
                &mut clicked,
            );
            assert!(clicked.iter().any(|m| matches!(m, Event::Save)));
            if let Some(directory) = std::env::var_os("SHIJU_T60_PREVIEW_DIR") {
                image::save_buffer(
                    std::path::PathBuf::from(directory)
                        .join(format!("t60-{theme_name}-removed-{removed}.png")),
                    &pixels,
                    960,
                    800,
                    image::ColorType::Rgba8,
                )
                .unwrap();
            }
        }
    }
}
