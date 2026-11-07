//! Overlay 会重新构建按钮；悬停背景直接依据绘制时的光标，避免依赖 Button 临时状态。

use super::styles;
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, renderer, widget};
use iced::widget::button;
use iced::{Color, Element, Event, Length, Rectangle, Renderer, Size, Theme};

pub(super) struct MenuOption {
    pub button: Element<'static, usize>,
    pub keyboard_highlight: Option<bool>,
}

impl Widget<usize, Theme, Renderer> for MenuOption {
    fn tag(&self) -> widget::tree::Tag {
        self.button.as_widget().tag()
    }
    fn state(&self) -> widget::tree::State {
        self.button.as_widget().state()
    }
    fn children(&self) -> Vec<widget::Tree> {
        self.button.as_widget().children()
    }
    fn diff(&self, tree: &mut widget::Tree) {
        self.button.as_widget().diff(tree);
    }
    fn size(&self) -> Size<Length> {
        self.button.as_widget().size()
    }
    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.button.as_widget_mut().layout(tree, renderer, limits)
    }
    fn operate(
        &mut self,
        tree: &mut widget::Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        self.button.as_widget_mut().operate(tree, layout, renderer, operation);
    }
    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, usize>,
        viewport: &Rectangle,
    ) {
        self.button
            .as_widget_mut()
            .update(tree, event, layout, cursor, renderer, clipboard, shell, viewport);
    }
    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        defaults: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        use iced::advanced::Renderer as _;
        let highlighted = self
            .keyboard_highlight
            .unwrap_or_else(|| cursor.is_over(layout.bounds()) && cursor.is_over(*viewport));
        let style = styles::option(theme, button::Status::Active, highlighted);
        renderer.fill_quad(
            renderer::Quad { bounds: layout.bounds(), border: style.border, ..Default::default() },
            style.background.unwrap_or(Color::TRANSPARENT.into()),
        );
        self.button.as_widget().draw(tree, renderer, theme, defaults, layout, cursor, viewport);
    }
    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.button.as_widget().mouse_interaction(tree, layout, cursor, viewport, renderer)
    }
}
