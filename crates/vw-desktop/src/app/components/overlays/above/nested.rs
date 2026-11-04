//! 将嵌套浮层作为整体命中；提示内容未命中时继续检查父弹层。
//! Iced 0.14 的 Nested 只返回最深层的交互结果，非交互式 tooltip 会丢失父层命中。
//! 保留 Iced 的布局与事件流程，仅在这一整体浮层内部补齐命中回退。
use iced::advanced::{Clipboard, Layout, Shell, layout, mouse, overlay, renderer, widget};
use iced::{Event, Size};
use std::cell::RefCell;

pub(super) struct NestedOverlay<'a, Message, Theme, Renderer> {
    root: RefCell<overlay::Element<'a, Message, Theme, Renderer>>,
}

impl<'a, Message: 'a, Theme: 'a, Renderer: iced::advanced::Renderer + 'a>
    NestedOverlay<'a, Message, Theme, Renderer>
{
    pub(super) fn wrap(
        root: overlay::Element<'a, Message, Theme, Renderer>,
    ) -> overlay::Element<'a, Message, Theme, Renderer> {
        overlay::Element::new(Box::new(Self { root: RefCell::new(root) }))
    }
}

impl<Message, Theme, Renderer: iced::advanced::Renderer> overlay::Overlay<Message, Theme, Renderer>
    for NestedOverlay<'_, Message, Theme, Renderer>
{
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let mut root = self.root.borrow_mut();
        nested(root.as_overlay_mut()).layout(renderer, bounds)
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        let mut root = self.root.borrow_mut();
        nested(root.as_overlay_mut()).draw(renderer, theme, style, layout, cursor);
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        let mut root = self.root.borrow_mut();
        nested(root.as_overlay_mut()).update(event, layout, cursor, renderer, clipboard, shell);
    }

    fn operate(
        &mut self,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        let mut root = self.root.borrow_mut();
        nested(root.as_overlay_mut()).operate(layout, renderer, operation);
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let mut root = self.root.borrow_mut();
        mouse_interaction(root.as_overlay_mut(), layout, cursor, renderer)
    }
}

fn mouse_interaction<Message, Theme, Renderer: iced::advanced::Renderer>(
    root: &mut dyn overlay::Overlay<Message, Theme, Renderer>,
    layout: Layout<'_>,
    cursor: mouse::Cursor,
    renderer: &Renderer,
) -> mouse::Interaction {
    let mut children = layout.children();
    let Some(parent_layout) = children.next() else {
        return mouse::Interaction::None;
    };
    if let Some(nested_layout) = children.next()
        && let Some(mut child) = root.overlay(parent_layout, renderer)
    {
        let interaction =
            mouse_interaction(child.as_overlay_mut(), nested_layout, cursor, renderer);
        if interaction != mouse::Interaction::None {
            return interaction;
        }
    }
    root.mouse_interaction(parent_layout, cursor, renderer)
}

fn nested<Message, Theme, Renderer: iced::advanced::Renderer>(
    root: &mut dyn overlay::Overlay<Message, Theme, Renderer>,
) -> overlay::Nested<'_, Message, Theme, Renderer> {
    overlay::Nested::new(overlay::Element::new(Box::new(BorrowedOverlay(root))))
}

/// 借用同一棵浮层树，让 Iced 继续负责布局、绘制、事件和控件操作。
struct BorrowedOverlay<'a, Message, Theme, Renderer>(
    &'a mut dyn overlay::Overlay<Message, Theme, Renderer>,
);

impl<Message, Theme, Renderer: iced::advanced::Renderer> overlay::Overlay<Message, Theme, Renderer>
    for BorrowedOverlay<'_, Message, Theme, Renderer>
{
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        self.0.layout(renderer, bounds)
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        self.0.draw(renderer, theme, style, layout, cursor);
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        self.0.update(event, layout, cursor, renderer, clipboard, shell);
    }

    fn operate(
        &mut self,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        self.0.operate(layout, renderer, operation);
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.0.mouse_interaction(layout, cursor, renderer)
    }

    fn overlay<'a>(
        &'a mut self,
        layout: Layout<'a>,
        renderer: &Renderer,
    ) -> Option<overlay::Element<'a, Message, Theme, Renderer>> {
        self.0.overlay(layout, renderer)
    }
}
