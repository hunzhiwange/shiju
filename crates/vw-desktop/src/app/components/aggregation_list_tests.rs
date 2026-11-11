use super::*;
use crate::app::{
    message::settings::aggregation_list::{Selection, Snapshot},
    state::{AgentAppsContext, app_providers::list::Scope},
};
use iced::{
    Font, Renderer, Size, Theme,
    advanced::renderer::{Headless, Style},
    mouse,
};
use iced_runtime::user_interface::{Cache, UserInterface};
use vw_gateway_client::vw_api_types::{
    agent_apps::{AppId, aggregation::*},
    id::TenantScope,
};

#[tokio::test]
async fn aggregation_t56_chinese_directory_shapes_and_draws_in_both_themes() {
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    let mut renderer =
        Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
            .await
            .expect("native text renderer");
    for theme in [Theme::Light, Theme::Dark] {
        for case in 0..5 {
            let selection = Selection {
                scope: Scope {
                    context: AgentAppsContext {
                        gateway: "http://fixture".into(),
                        tenant: TenantScope::default(),
                        project: None,
                    },
                    host: "主机".into(),
                    app: AppId::ClaudeDesktop,
                },
                project: "project".into(),
                catalog_generation: 1,
            };
            let mut form = Form::new(selection, 1);
            form.loading = case == 0;
            if case == 1 {
                form.error = Some("无权读取聚合目录，请检查授权");
            }
            if case == 2 {
                form.error = Some("此应用或执行目标不支持聚合目录");
            }
            if case >= 3 {
                form.snapshot = Some(Snapshot {
                    state: AggregationState {
                        revision: "r".into(),
                        mode: AggregationMode::Aggregation,
                        attached: true,
                        default_provider_id: (case == 4).then(|| "default".into()),
                        direct_provider_id: None,
                        members: if case == 4 {
                            (0..30)
                                .map(|i| AggregationMember {
                                    provider_id: if i == 0 {
                                        "default".into()
                                    } else {
                                        i.to_string()
                                    },
                                    name: format!("动态中文供应商 {i} 😀"),
                                    models: vec![AggregationModel {
                                        id: "ascii-model".into(),
                                        label: "中文模型".into(),
                                        supports_1m: false,
                                    }],
                                })
                                .collect()
                        } else {
                            Vec::new()
                        },
                        notices: vec![AggregationNotice::RestartDesktop],
                        recovery: RecoveryState::None,
                    },
                    candidates: if case == 4 {
                        vec![vw_gateway_client::vw_api_types::app_providers::AppProviderSummary {
                            id: "candidate".into(),
                            app_id: AppId::ClaudeDesktop,
                            name: "动态中文候选供应商".into(),
                            configured: true,
                        }]
                    } else {
                        Vec::new()
                    },
                });
            }
            let base =
                container(text("底层页面")).width(iced::Length::Fill).height(iced::Length::Fill);
            let element = overlay(&form, base.into());
            let mut ui =
                UserInterface::build(element, Size::new(800.0, 700.0), Cache::new(), &mut renderer);
            let mut geometry = Geometry::default();
            ui.operate(&mut renderer, &mut geometry);
            assert!(geometry.labels.iter().any(|(label, _)| label == "聚合目录"));
            if case == 4 {
                assert!(geometry.labels.iter().any(|(label, _)| label == "动态中文供应商 0 😀"));
                assert!(
                    geometry.labels.iter().any(|(label, _)| label == "默认 · 已添加 · 1 个模型")
                );
                let (viewport, content) = geometry
                    .scrolls
                    .iter()
                    .find(|(viewport, content)| content.height > viewport.height)
                    .expect("long directory must scroll");
                assert!(content.height > viewport.height);
                let bounds =
                    |name: &str| geometry.labels.iter().find(|(label, _)| label == name).unwrap().1;
                assert!(bounds("聚合目录").y < viewport.y);
                assert!(bounds("关闭").y >= viewport.y + viewport.height);
                assert!(
                    bounds("动态中文供应商 0 😀").x >= viewport.x + 7.0,
                    "label={:?}, viewport={viewport:?}",
                    bounds("动态中文供应商 0 😀")
                );
            }
            ui.draw(
                &mut renderer,
                &theme,
                &Style { text_color: theme.palette().text },
                mouse::Cursor::Unavailable,
            );
            let pixels = renderer.screenshot(Size::new(800, 700), 1.0, theme.palette().background);
            assert_eq!(pixels.len(), 800 * 700 * 4);
            assert!(pixels.chunks_exact(4).any(|pixel| pixel != &pixels[0..4]));
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
