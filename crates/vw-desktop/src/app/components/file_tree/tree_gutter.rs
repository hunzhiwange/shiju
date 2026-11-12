//! 逐行绘制祖先引导线；不接收输入，不改变行的布局与命中范围。
use iced::advanced::{Layout, Widget, layout, renderer, widget::Tree};
use iced::widget::stack;
use iced::{Element, Length, Rectangle, Size, Theme, mouse};

use super::icons::ICON_SIZE;
use super::row_layout::{INDENT, ROW_PADDING};
use crate::app::Message;

pub(super) fn view(depth: usize, content: Element<'_, Message>) -> Element<'_, Message> {
    if depth == 0 {
        return content;
    }
    stack![content, Element::new(TreeGutter { depth })].width(Length::Fill).into()
}

struct TreeGutter {
    depth: usize,
}

impl Widget<Message, Theme, iced::Renderer> for TreeGutter {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(
        &mut self,
        _: &mut Tree,
        _: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(limits.resolve(Length::Fill, Length::Fill, Size::ZERO))
    }

    fn draw(
        &self,
        _: &Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        _: &renderer::Style,
        layout: Layout<'_>,
        _: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        use iced::advanced::Renderer as _;
        let bounds = layout.bounds();
        let color = theme.extended_palette().background.strong.color;
        for ancestor in 0..self.depth {
            let line = Rectangle {
                x: bounds.x + ROW_PADDING[1] as f32 + ICON_SIZE / 2.0 + ancestor as f32 * INDENT
                    - 0.5,
                y: bounds.y,
                width: 1.0,
                height: bounds.height,
            };
            if let Some(visible) = line.intersection(viewport) {
                renderer.fill_quad(renderer::Quad { bounds: visible, ..Default::default() }, color);
            }
        }
    }
}
