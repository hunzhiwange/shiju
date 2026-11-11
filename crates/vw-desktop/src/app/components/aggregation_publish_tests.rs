use super::*;
use iced::{
    Font, Renderer, Size, Theme,
    advanced::renderer::{Headless, Style},
    mouse,
};
use iced_runtime::user_interface::{Cache, UserInterface};
use vw_gateway_client::vw_api_types::app_providers::operations::{
    ProviderOperationPhase, ProviderOperationProgress,
};
fn preview() -> AggregationPreview {
    AggregationPreview {
        revision: "r1".into(),
        preview_token: Some("token".into()),
        default_provider_id: Some("default".into()),
        members: (0..30)
            .map(|i| AggregationMember {
                provider_id: if i == 0 { "default".into() } else { i.to_string() },
                name: format!("动态中文供应商 {i} 😀"),
                models: vec![AggregationModel {
                    id: format!("model-{i}"),
                    label: "中文模型".into(),
                    supports_1m: true,
                }],
            })
            .collect(),
        issues: vec![],
        notices: vec![AggregationNotice::RestartDesktop, AggregationNotice::ModelReselect],
    }
}
#[tokio::test]
async fn aggregation_t58_confirmation_shapes_draws_and_dispatches_in_both_themes() {
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    let mut renderer =
        Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
            .await
            .expect("native renderer");
    for theme in [Theme::Light, Theme::Dark] {
        for case in 0..6 {
            let mut session = Session { preview: Some(preview()), ..Session::default() };
            if case == 1 {
                session.preview.as_mut().unwrap().issues.push(PreviewIssue {
                    code: PreviewIssueCode::FailoverEnabled,
                    blocking: true,
                    provider_id: Some("default".into()),
                    model_id: None,
                });
            }
            if case == 2 {
                session.busy = true;
            }
            if case >= 3 {
                session.preview = None;
                session.operation = Some(ProviderOperationProgress {
                    operation_id: "operation".into(),
                    request_id: "request".into(),
                    status: if case == 5 { TaskStatus::Failed } else { TaskStatus::Running },
                    phase: if case == 5 {
                        ProviderOperationPhase::Terminal
                    } else {
                        ProviderOperationPhase::Preparing
                    },
                    completed: 1,
                    total: 3,
                    cancellation_requested: case == 4,
                    editor_outcome: None,
                    aggregation_outcome: (case == 5).then_some(AggregationOutcome::Failed {
                        status: 502,
                        code: AggregationErrorCode::RecoveryFailed,
                        recovery: RecoveryState::Pending,
                    }),
                });
            }
            let element = overlay(
                &session,
                container(text("底层页面"))
                    .width(iced::Length::Fill)
                    .height(iced::Length::Fill)
                    .into(),
            );
            let mut ui =
                UserInterface::build(element, Size::new(820.0, 720.0), Cache::new(), &mut renderer);
            let mut geometry = Geometry::default();
            ui.operate(&mut renderer, &mut geometry);
            let bounds =
                |name: &str| geometry.labels.iter().find(|(label, _)| label == name).unwrap().1;
            assert!(geometry.labels.iter().any(|(label, _)| label == "确认聚合发布"));
            if case < 3 {
                assert!(
                    geometry
                        .labels
                        .iter()
                        .any(|(label, _)| label == "默认供应商：动态中文供应商 0 😀")
                );
                assert!(
                    geometry
                        .labels
                        .iter()
                        .any(|(label, _)| label == "中文模型 · model-0 · 支持 1M")
                );
                let (viewport, content) = geometry
                    .scrolls
                    .iter()
                    .find(|(v, c)| c.height > v.height)
                    .expect("preview scroll");
                assert!(content.height > viewport.height);
                assert!(bounds("确认聚合发布").y < viewport.y);
                assert!(bounds("确认发布").y >= viewport.y + viewport.height);
                assert!(
                    bounds("默认供应商：动态中文供应商 0 😀").x >= viewport.x + 7.0,
                    "label={:?}, viewport={viewport:?}",
                    bounds("默认供应商：动态中文供应商 0 😀")
                );
                for name in ["确认发布", "取消"] {
                    let mut messages = vec![];
                    ui.update(
                        &[
                            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                        ],
                        mouse::Cursor::Available(bounds(name).center()),
                        &mut renderer,
                        &mut iced::advanced::clipboard::Null,
                        &mut messages,
                    );
                    let expected = if name == "确认发布" { case == 0 } else { case != 2 };
                    assert_eq!(!messages.is_empty(), expected, "{name} case={case}");
                    for msg in messages {
                        let s = format!("{msg:?}");
                        assert!(s.contains(if name == "取消" { "Dismiss" } else { "Confirm" }));
                    }
                }
            } else {
                let mut messages = vec![];
                ui.update(
                    &[
                        iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                        iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                    ],
                    mouse::Cursor::Available(bounds("取消发布").center()),
                    &mut renderer,
                    &mut iced::advanced::clipboard::Null,
                    &mut messages,
                );
                assert_eq!(!messages.is_empty(), case == 3);
            }
            // Match Iced's native redraw path; buttons cache their visual status here.
            let mut redraw_messages = vec![];
            ui.update(
                &[iced::Event::Window(iced::window::Event::RedrawRequested(
                    iced::time::Instant::now(),
                ))],
                mouse::Cursor::Unavailable,
                &mut renderer,
                &mut iced::advanced::clipboard::Null,
                &mut redraw_messages,
            );
            assert!(redraw_messages.is_empty());
            ui.draw(
                &mut renderer,
                &theme,
                &Style { text_color: theme.palette().text },
                mouse::Cursor::Unavailable,
            );
            let pixels = renderer.screenshot(Size::new(820, 720), 1.0, theme.palette().background);
            assert_eq!(pixels.len(), 820 * 720 * 4);
            assert!(pixels.chunks_exact(4).any(|p| p != &pixels[0..4]));
            if case == 0 {
                image::save_buffer(
                    format!(
                        "/tmp/t58-confirm-{}.png",
                        if theme == Theme::Dark { "dark" } else { "light" }
                    ),
                    &pixels,
                    820,
                    720,
                    image::ColorType::Rgba8,
                )
                .unwrap();
            }
        }
    }
}
#[derive(Default)]
struct Geometry {
    labels: Vec<(String, iced::Rectangle)>,
    scrolls: Vec<(iced::Rectangle, iced::Rectangle)>,
}
impl iced::advanced::widget::Operation for Geometry {
    fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn iced::advanced::widget::Operation)) {
        children(self);
    }
    fn text(
        &mut self,
        _: Option<&iced::advanced::widget::Id>,
        bounds: iced::Rectangle,
        value: &str,
    ) {
        self.labels.push((value.into(), bounds));
    }
    fn scrollable(
        &mut self,
        _: Option<&iced::advanced::widget::Id>,
        bounds: iced::Rectangle,
        content: iced::Rectangle,
        _: iced::Vector,
        _: &mut dyn iced::advanced::widget::operation::Scrollable,
    ) {
        self.scrolls.push((bounds, content));
    }
}
