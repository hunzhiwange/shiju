use super::utils::{render_line_content, Lang};
use crate::app::DiffTheme;
use iced::Color;

#[tokio::test]
async fn render_line_content_draws_go_tabs_as_blank_indentation() {
    use iced::advanced::{Layout, layout, renderer, renderer::Headless, widget::Tree};
    use iced::{Element, Font, Pixels, Rectangle, Size, Theme, mouse};

    iced::advanced::graphics::text::font_system()
        .write()
        .unwrap()
        .load_font(crate::fonts::NOTO_SANS_CJK_SC_REGULAR.into());
    let mut renderer = <iced::Renderer as Headless>::new(
        Font::with_name("Noto Sans CJK SC"),
        Pixels(16.0),
        Some("tiny-skia"),
    )
    .await
    .expect("tiny-skia headless renderer");
    let size = Size::new(240, 32);
    let viewport = Rectangle::with_size(Size::new(240.0, 32.0));
    let mut element: Element<'_, crate::app::Message> = render_line_content(
        "\t\t",
        Lang::Go,
        DiffTheme::GitHub,
        true,
        &[],
        Color::TRANSPARENT,
        Color::TRANSPARENT,
    )
    .into();
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, viewport.size()),
    );
    assert!(node.bounds().width > 0.0, "tabs must reserve indentation");
    element.as_widget().draw(
        &tree,
        &mut renderer,
        &Theme::Light,
        &renderer::Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &viewport,
    );
    let pixels = renderer.screenshot(size, 1.0, Color::TRANSPARENT);
    assert_eq!(pixels.len(), (size.width * size.height * 4) as usize);
    assert!(pixels.chunks_exact(4).all(|pixel| pixel[3] == 0));
}
