use super::voice_controls::action;
use crate::app::message::{voice::Event, voice_draft::Phase};
#[test]
fn voice_task_07_controls_keep_finish_cancel_and_apply_distinct() {
    assert!(matches!(action(Phase::Idle, false).0, Event::Start));
    assert!(matches!(action(Phase::Recording, false).0, Event::Finish));
    assert!(matches!(action(Phase::Transcribing, false).0, Event::Cancel));
    assert!(matches!(action(Phase::Idle, true).0, Event::Apply));
    assert!(matches!(action(Phase::Error, false).0, Event::Start));
}
#[test]
fn voice_task_07_toolbar_style_has_contrast_in_both_themes() {
    for theme in [iced::Theme::Dark, iced::Theme::Light] {
        let style = super::styles::square_icon_button_style(
            &theme,
            iced::widget::button::Status::Active,
            true,
        );
        assert!(style.text_color.a > 0.0);
        let text = theme.palette().text;
        let background = theme.palette().background;
        assert!(
            (text.r - background.r).abs()
                + (text.g - background.g).abs()
                + (text.b - background.b).abs()
                > 0.5
        );
    }
}

#[tokio::test]
async fn voice_ui_primary_dictation_remains_independent() {
    let app = crate::app::App::new().0;
    let messages =
        super::send_controls_tests::click_control(super::voice_controls::view(&app)).await;
    assert!(matches!(messages.as_slice(), [crate::app::Message::Voice(Event::Start)]));
}

#[test]
fn voice_ui_primary_macos_dictation_hint_is_platform_scoped() {
    let hint = super::voice_controls::dictation_tooltip(action(Phase::Idle, false).1);
    #[cfg(target_os = "macos")]
    {
        assert!(hint.starts_with("听写\n"));
        assert!(hint.contains("连按两次 Fn/地球仪键"));
        assert!(hint.contains("可编辑输入框"));
    }
    #[cfg(not(target_os = "macos"))]
    assert_eq!(hint, "听写");
    assert_eq!(super::voice_controls::dictation_tooltip("应用听写"), "应用听写");
}

#[test]
fn voice_ui_dictation_stop_tooltip_is_explicit() {
    assert_eq!(action(Phase::Recording, false).1, "停止听写");
}

#[tokio::test]
async fn voice_ui_dictation_hover_does_not_finish() {
    use iced::advanced::{Layout, Shell, layout, renderer::Headless, widget::Tree};
    let mut app = crate::app::App::new().0;
    app.voice.draft.phase = Phase::Recording;
    let mut element = super::voice_controls::recording_bar(&app);
    let renderer = <iced::Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(16.0),
        Some("tiny-skia"),
    )
    .await
    .expect("headless renderer");
    let mut tree = Tree::new(element.as_widget());
    let size = iced::Size::new(240.0, 64.0);
    let node = element.as_widget_mut().layout(
        &mut tree,
        &renderer,
        &layout::Limits::new(iced::Size::ZERO, size),
    );
    let mut messages = Vec::new();
    element.as_widget_mut().update(
        &mut tree,
        &iced::Event::Mouse(iced::mouse::Event::CursorMoved {
            position: iced::Point::new(16.0, 16.0),
        }),
        Layout::new(&node),
        iced::mouse::Cursor::Available(iced::Point::new(16.0, 16.0)),
        &renderer,
        &mut iced::advanced::clipboard::Null,
        &mut Shell::new(&mut messages),
        &iced::Rectangle::with_size(size),
    );
    assert!(messages.is_empty());
    drop(element);
    let messages =
        super::send_controls_tests::click_control(super::voice_controls::recording_bar(&app)).await;
    assert!(matches!(messages.as_slice(), [crate::app::Message::Voice(Event::Cancel)]));
}

#[tokio::test]
async fn voice_ui_dictation_shapes_chinese_in_both_themes() {
    use iced::advanced::renderer::{Headless, Style};
    use iced_runtime::user_interface::{Cache, UserInterface};
    for font in crate::fonts::load_all() {
        iced::advanced::graphics::text::font_system().write().unwrap().load_font(font);
    }
    let mut renderer = iced::Renderer::new(iced::Font::DEFAULT, 14.0.into(), Some("tiny-skia"))
        .await
        .expect("tiny-skia");
    let mut app = crate::app::App::new().0;
    for theme in [iced::Theme::Dark, iced::Theme::Light] {
        for width in [240u32, 640] {
            for phase in [Phase::Recording, Phase::Transcribing] {
                app.voice.draft.phase = phase;
                let size = iced::Size::new(width as f32, 64.0);
                let mut ui = UserInterface::build(
                    super::voice_controls::recording_bar(&app),
                    size,
                    Cache::default(),
                    &mut renderer,
                );
                ui.draw(
                    &mut renderer,
                    &theme,
                    &Style { text_color: theme.palette().text },
                    iced::mouse::Cursor::Unavailable,
                );
            }
        }
    }
}
