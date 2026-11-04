//! Exercise actual glyph shaping, layout and draw for the Chinese template modal.
use super::*;
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
use iced::{Color, Font, Pixels, Rectangle, Size};

#[tokio::test]
async fn video_edit_ui_chinese_modal_renders_light_and_dark() {
    let mut state = WorkflowState::default();
    state.video_edit.open = true;
    state.video_edit.path = "/视频/中文 采访.mp4".into();
    state.video_edit.restore_confirm = true;
    state.video_edit.editor = Some(text_editor::Content::with_text(
        "app:\n  name: 通用云端视频剪辑\nworkflow:\n  video_edit: {max_parallel: 5}",
    ));
    for (name, theme) in [("light", Theme::Light), ("dark", Theme::Dark)] {
        let mut element = video_edit::modal(&state);
        let mut renderer =
            <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
                .await
                .expect("headless renderer");
        let mut tree = Tree::new(element.as_widget());
        let viewport = Rectangle::with_size(Size::new(1100.0, 820.0));
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, viewport.size()),
        );
        element.as_widget().draw(
            &tree,
            &mut renderer,
            &theme,
            &renderer::Style::default(),
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &viewport,
        );
        let pixels = renderer.screenshot(Size::new(1100, 820), 1.0, Color::TRANSPARENT);
        assert!(pixels.chunks_exact(4).any(|p| p[3] > 0));
        ::image::save_buffer(
            format!("/tmp/video12-modal-{name}.png"),
            &pixels,
            1100,
            820,
            ::image::ColorType::Rgba8,
        )
        .expect("save rendering evidence");
    }
}
