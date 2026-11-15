//! 用量视图的共享组件，封装统计卡片、列表和辅助展示元素。

use iced::advanced::{Clipboard, Layout, Shell, Widget, widget};
use iced::{Element, Event, Point, Rectangle, Theme, mouse};

/// RightClickArea 数据结构，承载当前模块对外传递的显式状态。
pub struct RightClickArea<'a, MessageT, ThemeT = Theme, RendererT = iced::Renderer> {
    content: Element<'a, MessageT, ThemeT, RendererT>,
    on_right_click: Box<dyn Fn(Point) -> MessageT + 'a>,
}

impl<'a, MessageT, ThemeT, RendererT> RightClickArea<'a, MessageT, ThemeT, RendererT> {
    /// 构建或更新 new 相关行为。
    ///
    /// 参数由调用方提供，返回值直接交给上层视图或状态处理；本函数不执行额外错误恢复。
    pub fn new(
        content: Element<'a, MessageT, ThemeT, RendererT>,
        on_right_click: Box<dyn Fn(Point) -> MessageT + 'a>,
    ) -> Self {
        Self { content, on_right_click }
    }
}

impl<'a, MessageT, ThemeT, RendererT> Widget<MessageT, ThemeT, RendererT>
    for RightClickArea<'a, MessageT, ThemeT, RendererT>
where
    RendererT: iced::advanced::Renderer,
{
    fn children(&self) -> Vec<widget::Tree> {
        vec![widget::Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(&[&self.content]);
    }

    fn size(&self) -> iced::Size<iced::Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &RendererT,
        limits: &iced::advanced::layout::Limits,
    ) -> iced::advanced::layout::Node {
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut RendererT,
        theme: &ThemeT,
        style: &iced::advanced::renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &RendererT,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, MessageT>,
        viewport: &Rectangle,
    ) {
        if let Event::Mouse(mouse::Event::ButtonPressed(button)) = event
            && matches!(button, mouse::Button::Right)
            && let Some(pos) = cursor.position()
            && layout.bounds().contains(pos)
        {
            let bounds = layout.bounds();
            let local = Point::new(pos.x - bounds.x, pos.y - bounds.y);
            shell.publish((self.on_right_click)(local));
        }

        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &RendererT,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }
}

#[cfg(test)]
#[path = "components_tests.rs"]
mod components_tests;
