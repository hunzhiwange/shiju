use crate::input_panel::reasoning_selector::{
    ReasoningInput, reasoning_effort_options, reasoning_effort_popover,
};

#[tokio::test(flavor = "multi_thread")]
async fn reasoning_slider_renders_non_ascii_labels_and_all_positions_in_both_themes() {
    use iced::advanced::renderer::Headless;
    use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
    use iced::{Color, Font, Pixels, Rectangle, Size, Theme};

    let options = reasoning_effort_options("codex");
    for theme in [Theme::Dark, Theme::Light] {
        let mut renderer =
            <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
                .await
                .expect("headless renderer");
        for current in options {
            let mut element = reasoning_effort_popover(
                ReasoningInput {
                    providers: &[],
                    model: "",
                    expanded: false,
                    i18n: desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
                    session_id: None,
                },
                "provider/中文模型 GPT-6",
                options,
                *current,
            );
            let bounds = Size::new(320.0, 180.0);
            let mut tree = Tree::new(element.as_widget());
            let node = element.as_widget_mut().layout(
                &mut tree,
                &renderer,
                &layout::Limits::new(Size::ZERO, bounds),
            );
            assert_eq!(node.bounds().width, 260.0);
            assert!(node.bounds().height <= 110.0);
            element.as_widget().draw(
                &tree,
                &mut renderer,
                &theme,
                &renderer::Style { text_color: theme.palette().text },
                Layout::new(&node),
                mouse::Cursor::Unavailable,
                &Rectangle::with_size(bounds),
            );
            let pixels = renderer.screenshot(Size::new(320, 180), 1.0, Color::TRANSPARENT);
            assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
        }
    }
}
