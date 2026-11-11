use super::*;
use iced::advanced::{
    clipboard,
    renderer::{Headless, Style},
};
use iced::{Font, Renderer, Size, Theme, mouse, widget::container};
use iced_runtime::user_interface::{Cache, UserInterface};
#[tokio::test]
async fn voice_task_16_onboarding_draws_chinese_and_multiline_results_in_both_themes() {
    let mut renderer = Renderer::new(Font::DEFAULT, 15.0.into(), Some("tiny-skia")).await.unwrap();
    for theme in [Theme::Light, Theme::Dark] {
        for pending in [None, Some("你好，世界！\n第二行😀")] {
            let base = container(text("语音设置")).width(Length::Fill).height(Length::Fill).into();
            let element = modal(
                base,
                [PermissionStatus::Unknown, PermissionStatus::Granted, PermissionStatus::Denied],
                pending,
            );
            let size = Size::new(900.0, 760.0);
            let mut ui = UserInterface::build(element, size, Cache::new(), &mut renderer);
            let mut messages = Vec::new();
            ui.update(
                &[],
                mouse::Cursor::Unavailable,
                &mut renderer,
                &mut clipboard::Null,
                &mut messages,
            );
            assert!(messages.is_empty());
            let mut labels = Labels::default();
            ui.operate(&renderer, &mut labels);
            for (label, expected) in [
                ("刷新权限", 0),
                ("复制听写结果", 1),
                ("丢弃结果", 2),
                ("关闭，前往目标应用试用", 3),
            ] {
                let found = labels.0.iter().find(|(value, _)| value == label);
                if pending.is_none() && [1, 2].contains(&expected) {
                    assert!(found.is_none());
                    continue;
                }
                let bounds = found.unwrap().1;
                assert!(
                    bounds.x >= 0.0
                        && bounds.y >= 0.0
                        && bounds.x + bounds.width <= size.width
                        && bounds.y + bounds.height <= size.height
                );
                ui.update(
                    &[
                        iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                        iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                    ],
                    mouse::Cursor::Available(bounds.center()),
                    &mut renderer,
                    &mut clipboard::Null,
                    &mut messages,
                );
                assert_eq!(messages.len(), 1, "{label}");
                assert!(matches!(
                    (&messages[0], expected),
                    (Message::Voice(Event::Onboarding(OnboardingEvent::Refresh)), 0)
                        | (Message::Voice(Event::Onboarding(OnboardingEvent::Copy)), 1)
                        | (Message::Voice(Event::Onboarding(OnboardingEvent::Discard)), 2)
                        | (Message::Voice(Event::Onboarding(OnboardingEvent::Close)), 3)
                ));
                messages.clear();
            }

            ui.draw(
                &mut renderer,
                &theme,
                &Style { text_color: theme.palette().text },
                mouse::Cursor::Unavailable,
            );
            assert!(
                !renderer
                    .screenshot(Size::new(900, 760), 1.0, theme.palette().background)
                    .is_empty()
            );
        }
    }
}

#[derive(Default)]
struct Labels(Vec<(String, iced::Rectangle)>);
impl iced::advanced::widget::Operation for Labels {
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
