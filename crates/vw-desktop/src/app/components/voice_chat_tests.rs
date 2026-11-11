use super::*;
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
#[tokio::test]
async fn voice_task_11_chat_renders_chinese_in_both_themes_and_routes_controls() {
    use iced::advanced::{clipboard, renderer::Headless};
    use iced::{Event as InputEvent, Font, Size, Theme, mouse};
    use iced_runtime::user_interface::{Cache, UserInterface};
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    let mut renderer =
        iced::Renderer::new(Font::DEFAULT, 14.0.into(), Some("tiny-skia")).await.unwrap();
    for phase in [Phase::Speaking, Phase::Muted, Phase::Error] {
        let state = Snapshot {
            phase,
            microphone: phase != Phase::Error,
            sending: false,
            playing: phase != Phase::Error,
            error: Some("设备状态说明：请检查麦克风。"),
            ..Default::default()
        };
        let mut ui = UserInterface::build(
            overlay(
                container(text("底层聊天")).width(Length::Fill).height(Length::Fill).into(),
                state,
            ),
            Size::new(700.0, 640.0),
            Cache::new(),
            &mut renderer,
        );
        let mut labels = Labels::default();
        ui.operate(&renderer, &mut labels);
        assert!(labels.0.iter().any(|(s, _)| s == phase_label(phase)));
        for theme in [Theme::Light, Theme::Dark] {
            ui.draw(&mut renderer, &theme, &Default::default(), mouse::Cursor::Unavailable);
            let pixels = renderer.screenshot(Size::new(700, 640), 1.0, iced::Color::TRANSPARENT);
            assert!(pixels.chunks_exact(4).any(|p| p[3] != 0));
        }
        for (label, action) in [
            (if phase == Phase::Muted { "取消静音" } else { "静音" }, 0),
            ("打断回答", 1),
            ("结束通话", 2),
        ] {
            let bounds = labels.0.iter().find(|(s, _)| s == label).unwrap().1;
            assert!(
                bounds.x >= 0.0
                    && bounds.y >= 0.0
                    && bounds.x + bounds.width <= 700.0
                    && bounds.y + bounds.height <= 640.0,
                "button outside viewport: {bounds:?}"
            );
            let mut messages = vec![];
            ui.update(
                &[
                    InputEvent::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                    InputEvent::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                ],
                mouse::Cursor::Available(bounds.center()),
                &mut renderer,
                &mut clipboard::Null,
                &mut messages,
            );
            if phase == Phase::Error && action != 2 {
                assert!(messages.is_empty());
                continue;
            }
            assert_eq!(messages.len(), 1, "phase={phase:?} action={action} bounds={bounds:?}");
            assert!(matches!(
                (&messages[0], action),
                (Message::Voice(Event::ChatMute), 0)
                    | (Message::Voice(Event::ChatInterrupt), 1)
                    | (Message::Voice(Event::ChatEnd), 2)
            ));
        }
    }
}
