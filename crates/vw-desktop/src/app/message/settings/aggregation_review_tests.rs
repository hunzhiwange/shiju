use super::*;
#[tokio::test]
async fn aggregation_t61_aggregation_review_preserves_contract() {
    use iced::{Size, advanced::widget::operation::Operation};
    use iced_runtime::user_interface::{Cache, UserInterface};
    #[derive(Default)]
    struct Probe(Vec<String>);
    impl Operation for Probe {
        fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) {
            children(self);
        }
        fn text(
            &mut self,
            _: Option<&iced::advanced::widget::Id>,
            _: iced::Rectangle,
            value: &str,
        ) {
            self.0.push(value.into());
        }
    }
    let state = CodexAggregationSettings {
        revision: "r61".into(),
        requested: Default::default(),
        effective: Default::default(),
        choices: vec![],
        notices: vec![
            AggregationNotice::MultiAgentV2Forced,
            AggregationNotice::EncryptedTaskCrossProvider,
        ],
    };
    let form = Form { draft: state.requested.clone(), snapshot: Some(state), ..Default::default() };
    use iced::advanced::renderer::Headless;
    let mut renderer =
        iced::Renderer::new(iced::Font::DEFAULT, 14.0.into(), Some("tiny-skia")).await.unwrap();
    let element = crate::app::components::aggregation_subagent::overlay(
        &form,
        iced::widget::Space::new().into(),
    );
    let mut ui =
        UserInterface::build(element, Size::new(960.0, 800.0), Cache::new(), &mut renderer);
    let mut probe = Probe::default();
    ui.operate(&renderer, &mut probe);
    assert!(probe.0.iter().any(|v| v == "自动审批模型"), "missing real review control");
    assert!(probe.0.iter().any(|v| v.contains("经典工具（默认开启）")));
    assert!(probe.0.iter().any(|v| v.contains("账号额度")));
}

fn loaded() -> Form {
    let mut state = CodexAggregationSettings {
        revision: "r61".into(),
        requested: Default::default(),
        effective: Default::default(),
        choices: vec![CodexSettingsChoice {
            model_id: "published".into(),
            label: "审批／日本語".into(),
            efforts: vec![],
        }],
        notices: vec![
            AggregationNotice::MultiAgentV2Forced,
            AggregationNotice::EncryptedTaskCrossProvider,
        ],
    };
    state.effective.classic_tools = false;
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
async fn aggregation_t61_actual_text_rendering_in_both_themes() {
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
        for font in crate::fonts::load_all() {
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
                form.draft.subagent_model_id = Some("published".into());
                let s = form.snapshot.as_mut().unwrap();
                s.choices.clear();
                s.requested.auto_review_model_id = Some("removed-reviewer".into());
                form.draft.auto_review_model_id = Some("removed-reviewer".into());
                s.effective.subagent_model_id = None;
                s.effective.subagent_effort = None;
                s.notices.push(AggregationNotice::MultiAgentV2Forced);
                s.notices.push(AggregationNotice::EncryptedTaskCrossProvider);
            }
            let element = crate::app::components::aggregation_subagent::overlay(
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
            for label in ["自动审批模型", "经典工具（默认开启）"] {
                assert!(probe.0.iter().any(|(t, _)| t == label), "{label}");
            }
            assert!(probe.0.iter().any(|(t, _)| t.contains("账号额度")));
            assert!(probe.0.iter().any(|(t, _)| t.contains("加密任务")));
            if removed {
                assert!(probe.0.iter().any(|(t, _)| t.contains("审批模型已不在发布目录")));
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
            messages.retain(|message| !matches!(message, crate::app::Message::None));
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
            let classic =
                probe.0.iter().find(|(t, _)| t == "经典工具（默认开启）").unwrap().1.center();
            let mut toggled = Vec::new();
            ui.update(
                &[
                    iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                    iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                ],
                mouse::Cursor::Available(classic),
                &mut renderer,
                &mut iced::advanced::clipboard::Null,
                &mut toggled,
            );
            assert!(toggled.iter().any(|m| matches!(
                m,
                crate::app::Message::Settings(crate::app::message::SettingsMessage::AppProviders(
                    crate::app::message::app_providers::list::ListMessage::CodexSubagent(
                        Event::Classic(false)
                    )
                ))
            )));
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
            assert!(clicked.iter().any(|m| matches!(
                m,
                crate::app::Message::Settings(crate::app::message::SettingsMessage::AppProviders(
                    crate::app::message::app_providers::list::ListMessage::CodexSubagent(
                        Event::Save
                    )
                ))
            )));
            if let Some(directory) = std::env::var_os("SHIJU_T61_PREVIEW_DIR") {
                image::save_buffer(
                    std::path::PathBuf::from(directory)
                        .join(format!("t61-{theme_name}-removed-{removed}.png")),
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
