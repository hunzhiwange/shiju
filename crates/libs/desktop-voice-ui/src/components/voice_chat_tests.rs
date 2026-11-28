use super::*;
use iced::{
    Length,
    widget::{container, text},
};
type Message = Event;
fn overlay<'a>(base: Element<'a, Message>, state: Snapshot) -> Element<'a, Message> {
    super::overlay_screen(
        desktop_i18n::I18n::new(config_ui::DesktopLocale::default()),
        base,
        std::convert::identity,
        state,
        None,
    )
}

#[derive(Default)]
struct Labels(Vec<(String, iced::Rectangle)>, Option<iced::Rectangle>);
impl iced::advanced::widget::Operation for Labels {
    fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn iced::advanced::widget::Operation)) {
        children(self);
    }
    fn container(&mut self, id: Option<&iced::widget::Id>, bounds: iced::Rectangle) {
        if id == Some(&iced::widget::Id::new("voice-chat-settings")) {
            self.1 = Some(bounds);
        }
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
        for font in desktop_assets::fonts::load_all() {
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
                (Event::ChatMute, 0) | (Event::ChatInterrupt, 1) | (Event::ChatEnd, 2)
            ));
        }
    }
}

#[tokio::test]
async fn voice_screen_settings_reveal_permission_on_hover_without_requesting_it() {
    use iced::advanced::{clipboard, renderer::Headless};
    use iced::{Event as InputEvent, Point, Size, mouse};
    use iced_runtime::user_interface::{Cache, UserInterface};
    let screen = desktop_voice::screen_context::State::default();
    let mut renderer =
        iced::Renderer::new(iced::Font::DEFAULT, 14.0.into(), Some("tiny-skia")).await.unwrap();
    let gear = screen_ui::settings_control(
        I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
        std::convert::identity::<Event>,
        &screen,
        true,
    );
    let mut ui = UserInterface::build(
        container(gear).center(Length::Fill).into(),
        Size::new(700.0, 640.0),
        Cache::new(),
        &mut renderer,
    );

    let mut labels = Labels::default();
    ui.operate(&renderer, &mut labels);
    assert!(!labels.0.iter().any(|(label, _)| label == "屏幕录制授权"));
    let gear_center = labels.1.unwrap().center();
    let mut messages = vec![];
    ui.update(
        &[InputEvent::Mouse(mouse::Event::CursorMoved { position: gear_center })],
        mouse::Cursor::Available(gear_center),
        &mut renderer,
        &mut clipboard::Null,
        &mut messages,
    );
    assert!(messages.is_empty());

    let mut labels = Labels::default();
    ui.operate(&renderer, &mut labels);
    assert!(!labels.0.iter().any(|(label, _)| label == "确认采集并分享这一帧"));
    let permission = labels.0.iter().find(|(label, _)| label == "屏幕录制授权").unwrap().1;
    let across_gap = Point::new(gear_center.x + 18.0, gear_center.y);
    let target = permission.center();
    for position in [across_gap, target] {
        ui.update(
            &[InputEvent::Mouse(mouse::Event::CursorMoved { position })],
            mouse::Cursor::Available(position),
            &mut renderer,
            &mut clipboard::Null,
            &mut messages,
        );
    }
    assert!(messages.is_empty());
    ui.update(
        &[
            InputEvent::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            InputEvent::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        ],
        mouse::Cursor::Available(target),
        &mut renderer,
        &mut clipboard::Null,
        &mut messages,
    );
    assert!(matches!(&messages[..], [Event::Screen(ScreenEvent::Permission)]));
}
