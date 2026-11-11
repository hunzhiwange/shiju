//! 候选就绪前不创建选择控件，并验证加载、失败和完成状态的实际绘制路径。
use super::execution::selectors;
use crate::app::App;
use crate::app::state::ChannelExecutionChoices;
use iced::advanced::renderer::Headless;
use iced::advanced::widget::operation::focusable::Focusable;
use iced::advanced::{
    Layout, layout, mouse, renderer,
    widget::{Id, Operation, Tree},
};
use iced::{Color, Font, Pixels, Rectangle, Size, Theme};

#[derive(Default)]
struct Controls {
    focusable: usize,
    labels: Vec<String>,
}
impl Operation for Controls {
    fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) {
        children(self);
    }
    fn focusable(&mut self, _: Option<&Id>, _: Rectangle, _: &mut dyn Focusable) {
        self.focusable += 1;
    }
    fn text(&mut self, _: Option<&Id>, _: Rectangle, value: &str) {
        self.labels.push(value.into());
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn channel_execution_loading_shows_unified_agent_and_provider_models_when_ready() {
    let mut app = App::new().0;
    app.desktop_locale = vw_config_types::ui::DesktopLocale::SimplifiedChinese;
    let key = "wechat:default";
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("headless renderer");
    for stage in ["initial", "loading", "ready", "failed", "retry"] {
        match stage {
            "loading" | "retry" => {
                app.channels_settings.begin_execution_choices("tenant", key);
            }
            "ready" => {
                let generation = app.channels_settings.execution_request[key].1;
                app.channels_settings.finish_execution_choices(
                    "tenant",
                    key,
                    generation,
                    Ok(ChannelExecutionChoices {
                        agents: vec![("acp-research".into(), "研究助手".into())],
                        models: vec![vw_config_types::agent::AgentModelReference {
                            provider: "provider".into(),
                            model: "model".into(),
                        }],
                    }),
                );
            }
            "failed" => {
                let generation = app.channels_settings.begin_execution_choices("tenant", key);
                app.channels_settings.finish_execution_choices(
                    "tenant",
                    key,
                    generation,
                    Err("网络错误".into()),
                );
            }
            _ => {}
        }
        if stage == "ready" {
            app.channels_settings.execution_provider.insert(key.into(), "provider".into());
        }
        for (theme, width) in
            [(Theme::Dark, 960), (Theme::Light, 960), (Theme::Dark, 420), (Theme::Light, 420)]
        {
            let mut element = selectors(&app, key.into());
            let bounds = Size::new(width as f32, 500.0);
            let mut tree = Tree::new(element.as_widget());
            let node = element.as_widget_mut().layout(
                &mut tree,
                &renderer,
                &layout::Limits::new(Size::ZERO, bounds),
            );
            let mut controls = Controls::default();
            element.as_widget_mut().operate(
                &mut tree,
                Layout::new(&node),
                &renderer,
                &mut controls,
            );
            if stage == "ready" {
                assert_eq!(controls.focusable, 3);
                assert!(controls.labels.iter().any(|label| label == "智能体"));
                assert!(controls.labels.iter().any(|label| label == "模型提供商"));
                assert!(controls.labels.iter().any(|label| label == "模型"));
                assert!(!controls.labels.iter().any(|label| label.contains("ACP")));
                assert!(!controls.labels.iter().any(|label| label == "加载中…"));
            } else if stage == "failed" {
                assert!(controls.focusable <= 1, "only retry may receive focus");
                assert_eq!(controls.labels.iter().filter(|label| *label == "加载失败").count(), 3);
                assert!(controls.labels.iter().any(|label| label == "重试"));
            } else {
                assert_eq!(controls.focusable, 0, "unloaded controls cannot receive focus");
                assert_eq!(controls.labels.iter().filter(|label| *label == "加载中…").count(), 3);
            }
            element.as_widget().draw(
                &tree,
                &mut renderer,
                &theme,
                &renderer::Style::default(),
                Layout::new(&node),
                mouse::Cursor::Unavailable,
                &Rectangle::with_size(bounds),
            );
            let pixels = renderer.screenshot(Size::new(width, 500), 1.0, Color::TRANSPARENT);
            assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
        }
    }
}
