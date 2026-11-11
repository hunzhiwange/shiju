use super::*;
use iced::advanced::{
    clipboard,
    renderer::{Headless, Style},
};
use iced::{Font, Renderer, Size, Theme, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
#[tokio::test]
async fn voice_task_14_recordings_render_chinese_in_both_themes() {
    let mut renderer = Renderer::new(Font::DEFAULT, 15.0.into(), Some("tiny-skia")).await.unwrap();
    let mut state = State::default();
    state.entries.push(crate::app::voice::recordings::Entry {
        id: "11111111111111111111111111111111.wav".into(),
        bytes: 96044,
        modified: std::time::SystemTime::now(),
    });
    state.result = Some("重新识别的中文结果，等待用户选择回填。".into());
    for theme in [Theme::Dark, Theme::Light] {
        let mut ui = UserInterface::build(
            content(&state),
            Size::new(900.0, 600.0),
            Cache::new(),
            &mut renderer,
        );
        ui.update(
            &[],
            mouse::Cursor::Unavailable,
            &mut renderer,
            &mut clipboard::Null,
            &mut Vec::new(),
        );
        ui.draw(
            &mut renderer,
            &theme,
            &Style { text_color: theme.palette().text },
            mouse::Cursor::Unavailable,
        );
        assert!(
            !renderer.screenshot(Size::new(900, 600), 1.0, theme.palette().background).is_empty()
        );
    }
}
