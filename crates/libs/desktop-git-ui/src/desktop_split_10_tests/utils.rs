use crate::DiffTheme;
use crate::git_panel::utils::{Lang, render_line_content};
use iced::advanced::{Layout, layout, renderer, renderer::Headless, widget::Tree};
use iced::{Color, Element, Font, Pixels, Rectangle, Size, Theme, mouse};

async fn diff_renderer() -> iced::Renderer {
    iced::advanced::graphics::text::font_system()
        .write()
        .unwrap()
        .load_font(desktop_assets::fonts::NOTO_SANS_CJK_SC_REGULAR.into());
    <iced::Renderer as Headless>::new(
        Font::with_name("Noto Sans CJK SC"),
        Pixels(16.0),
        Some("tiny-skia"),
    )
    .await
    .expect("tiny-skia headless renderer")
}

#[test]
fn render_line_content_draws_go_tabs_as_blank_indentation() {
    iced::futures::executor::block_on(async {
        let mut renderer = diff_renderer().await;
        let size = Size::new(240, 32);
        let viewport = Rectangle::with_size(Size::new(240.0, 32.0));
        let mut element: Element<'_, ()> = render_line_content(
            "\t\t",
            Lang::Go,
            DiffTheme::GitHub,
            true,
            &[],
            Color::TRANSPARENT,
            Color::TRANSPARENT,
        );
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
        assert!(pixels.as_chunks::<4>().0.iter().all(|pixel| pixel[3] == 0));
    });
}

#[test]
fn highlighted_cjk_diff_wraps_at_the_same_width_as_plain_text() {
    iced::futures::executor::block_on(async {
        let renderer = diff_renderer().await;
        let content = "中文差异长行应该从正文左侧续行".repeat(3);
        let limits = layout::Limits::new(Size::ZERO, Size::new(120.0, 400.0));
        let layout_size = |ranges: &[std::ops::Range<usize>]| {
            let mut element = render_line_content::<()>(
                &content,
                Lang::Other,
                DiffTheme::GitHub,
                false,
                ranges,
                Color::TRANSPARENT,
                Color::BLACK,
            );
            let mut tree = Tree::new(element.as_widget());
            element.as_widget_mut().layout(&mut tree, &renderer, &limits).size()
        };

        let plain = layout_size(&[]);
        assert!(plain.height > 32.0);
        assert!(plain.width <= 120.0);
        assert_eq!(layout_size(&[0.."中文差异长行".len()]), plain);
    });
}
