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
