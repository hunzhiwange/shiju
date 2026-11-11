#[test]
fn mini_companion_size_reset_restores_default() {
    use crate::app::{
        App,
        message::view::{ViewMessage, update},
    };
    let mut app = App::new().0;
    app.task_pet_scale = 1.5;
    let _ = update(&mut app, ViewMessage::TaskPetSizeOpen);
    let _ = update(&mut app, ViewMessage::TaskPetSizeReset);
    assert_eq!(app.task_pet_preview_scale(), 1.0);
    assert_eq!(app.task_pet_scale, 1.5);
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

#[tokio::test]
async fn mini_companion_size_modal_renders_themes_and_routes_cancel_reset() {
    use crate::app::{
        App, Message,
        message::{ViewMessage, view},
    };
    use iced::advanced::{clipboard, renderer::Headless};
    use iced::{Event, Font, Size, Theme, mouse};
    use iced_runtime::user_interface::{Cache, UserInterface};
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    let mut renderer =
        iced::Renderer::new(Font::DEFAULT, 14.0.into(), Some("tiny-skia")).await.unwrap();
    let mut app = App::new().0;
    app.desktop_locale = vw_config_types::ui::DesktopLocale::SimplifiedChinese;
    let _ = view::update(&mut app, ViewMessage::TaskPetSizeOpen);
    for scale in [0.5, 1.0, 2.0] {
        let _ = view::update(&mut app, ViewMessage::TaskPetSizePreview(scale));
        let base = iced::widget::container(
            iced::widget::button("底层按钮")
                .on_press(Message::View(ViewMessage::TaskPetAvatarCycle)),
        )
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .into();
        let mut ui = UserInterface::build(
            super::with_size_modal(&app, base),
            Size::new(640.0, 520.0),
            Cache::new(),
            &mut renderer,
        );
        let mut labels = Labels::default();
        ui.operate(&renderer, &mut labels);
        for theme in [Theme::Light, Theme::Dark] {
            ui.draw(&mut renderer, &theme, &Default::default(), mouse::Cursor::Unavailable);
            let pixels = renderer.screenshot(Size::new(640, 520), 1.0, iced::Color::TRANSPARENT);
            assert!(pixels.chunks_exact(4).any(|p| p[3] != 0));
        }
        for label in ["取消", "重置"] {
            let bounds = labels.0.iter().find(|(s, _)| s == label).unwrap().1;
            let mut messages = Vec::new();
            ui.update(
                &[
                    Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                    Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                ],
                mouse::Cursor::Available(bounds.center()),
                &mut renderer,
                &mut clipboard::Null,
                &mut messages,
            );
            if label == "取消" {
                assert!(
                    matches!(messages.as_slice(), [Message::View(ViewMessage::TaskPetSizeCancel)]),
                    "cancel bounds={bounds:?}, messages={messages:?}"
                );
            } else if scale == 1.0 {
                assert!(messages.is_empty());
            } else {
                assert!(matches!(
                    messages.as_slice(),
                    [Message::View(ViewMessage::TaskPetSizeReset)]
                ));
            }
        }
        let mut messages = Vec::new();
        ui.update(
            &[
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            ],
            mouse::Cursor::Available(iced::Point::new(10.0, 10.0)),
            &mut renderer,
            &mut clipboard::Null,
            &mut messages,
        );
        assert!(messages.is_empty(), "modal must isolate the underlying button");
    }
}
