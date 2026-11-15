use super::*;
use iced::Color;
use iced::advanced::Renderer as _;
use iced::advanced::renderer::Headless;

#[tokio::test]
async fn sidebar_label_renders_multiline_titles_on_one_line_and_ellipsizes_to_fit() {
    let mut renderer =
        <Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia")).await.unwrap();
    let mut label = SingleLineLabel::new("中文任务\r\nMixed title\u{2028}下一步".to_owned(), 13.0)
        .font(Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT });
    let mut tree = widget::Tree::new(&label as &dyn Widget<(), Theme, Renderer>);

    for (width, theme) in [(240.0, Theme::Light), (72.0, Theme::Dark)] {
        let viewport = Rectangle::with_size(Size::new(width, 32.0));
        renderer.reset(viewport);
        let limits = layout::Limits::new(Size::ZERO, viewport.size());
        let node = <SingleLineLabel as Widget<(), Theme, Renderer>>::layout(
            &mut label, &mut tree, &renderer, &limits,
        );
        let paragraph = &tree.state.downcast_ref::<State>().paragraph;
        let lines = &paragraph.buffer().lines;
        assert_eq!(lines.len(), 1);
        assert!(paragraph.min_width() <= width);
        if theme == Theme::Light {
            assert_eq!(lines[0].text(), "中文任务 Mixed title 下一步");
        } else {
            assert!(lines[0].text().ends_with('…'));
        }
        assert_eq!(
            node.size().height,
            f32::from(text::LineHeight::default().to_absolute(label.size))
        );

        <SingleLineLabel as Widget<(), Theme, Renderer>>::draw(
            &label,
            &tree,
            &mut renderer,
            &theme,
            &renderer::Style { text_color: theme.palette().text },
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &viewport,
        );
        let background = if theme == Theme::Light { Color::WHITE } else { Color::BLACK };
        let pixels = renderer.screenshot(Size::new(width as u32, 32), 1.0, background);
        assert!(pixels.chunks_exact(4).any(|pixel| pixel != &pixels[..4]));
    }
}
