use super::voice_controls::action;
use crate::app::message::{voice::Event, voice_draft::Phase};

// 宿主保留：语音草稿状态机与普通输入区宿主装配契约。
#[tokio::test]
async fn voice_ui_primary_dictation_remains_independent() {
    let app = crate::app::App::new().0;
    let messages =
        super::send_controls_tests::click_control(super::voice_controls::view(&app)).await;
    assert!(matches!(messages.as_slice(), [crate::app::Message::Voice(Event::Start)]));
}

// 宿主保留：语音草稿状态机与普通输入区宿主装配契约。
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

// 宿主保留：语音草稿状态机与普通输入区宿主装配契约。
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

// 宿主保留：语音草稿状态机与普通输入区宿主装配契约。
#[test]
fn voice_moon_ui_normal_composer_has_no_voice_panel() {
    let (app, _) = crate::app::App::new();
    assert!(crate::app::components::voice_chat::local_inline(&app).is_none());
}
