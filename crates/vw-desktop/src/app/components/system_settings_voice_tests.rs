use super::*;
use crate::app::voice::preferences::{Edit, Preferences};
use iced::advanced::{
    clipboard,
    renderer::{Headless, Style},
};
use iced::{Font, Rectangle, Renderer, Size, Theme, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
use vw_config_types::{ui::DesktopLocale, voice::VoiceConfig};

#[tokio::test]
async fn voice_task_13_modal_renders_chinese_with_both_themes() {
    let mut renderer = Renderer::new(Font::DEFAULT, 15.0.into(), Some("tiny-skia")).await.unwrap();
    let mut state = State::default();
    state.edit = Some(Edit::new(
        VoiceConfig { dictionary: vec!["识矩语音".into(); 30], ..Default::default() },
        Preferences::default(),
    ));
    for theme in [Theme::Dark, Theme::Light] {
        let element =
            modal(&state, I18n::new(DesktopLocale::SimplifiedChinese), text("语音设置").into());
        let size = Size::new(780.0, 700.0);
        let mut ui = UserInterface::build(element, size, Cache::new(), &mut renderer);
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
        let pixels = renderer.screenshot(Size::new(780, 700), 1.0, theme.palette().background);
        assert!(!pixels.is_empty());
        let _ = Rectangle::with_size(size);
    }
}
