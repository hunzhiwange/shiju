use crate::input_panel::voice_controls::{self as domain, Event};
use desktop_voice_ui::message::{voice::State, voice_draft::Phase};
fn input(state: &State) -> domain::Input {
    domain::Input {
        phase: state.draft.phase,
        pending: state.draft.pending.is_some(),
        ready: state.ready(),
    }
}
fn view(state: &State) -> iced::Element<'static, Event> {
    domain::view(input(state))
}
fn recording_bar(state: &State) -> iced::Element<'static, Event> {
    domain::recording_bar(input(state), state.waveform.view())
}
#[tokio::test]
async fn voice_ui_primary_dictation_remains_independent() {
    let app = State::default();
    let messages = super::send_controls::click_control(view(&app)).await;
    assert!(matches!(messages.as_slice(), [Event::Start]));
}

#[tokio::test]
async fn voice_ui_dictation_hover_does_not_finish() {
    use iced::advanced::{Layout, Shell, layout, renderer::Headless, widget::Tree};
    let mut app = State::default();
    app.draft.phase = Phase::Recording;
    let mut element = recording_bar(&app);
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
    let messages = super::send_controls::click_control(recording_bar(&app)).await;
    assert!(matches!(messages.as_slice(), [Event::Cancel]));
}

#[tokio::test]
async fn voice_ui_dictation_shapes_chinese_in_both_themes() {
    use iced::advanced::renderer::{Headless, Style};
    use iced_runtime::user_interface::{Cache, UserInterface};
    for font in desktop_assets::fonts::load_all() {
        iced::advanced::graphics::text::font_system().write().unwrap().load_font(font);
    }
    let mut renderer = iced::Renderer::new(iced::Font::DEFAULT, 14.0.into(), Some("tiny-skia"))
        .await
        .expect("tiny-skia");
    let mut app = State::default();
    for theme in [iced::Theme::Dark, iced::Theme::Light] {
        for width in [240u32, 640] {
            for phase in [Phase::Recording, Phase::Transcribing] {
                app.draft.phase = phase;
                let size = iced::Size::new(width as f32, 64.0);
                let mut ui = UserInterface::build(
                    recording_bar(&app),
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
