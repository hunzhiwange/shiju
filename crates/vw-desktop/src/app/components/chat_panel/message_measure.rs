//! 在 Iced 布局阶段采集聊天消息的位置与高度，供视口定位使用。

use iced::advanced::widget::Operation;
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer, widget};
use iced::{Element, Event, Length, Rectangle, Size, Vector};
use std::any::Any;

#[derive(Debug, Clone, Copy, PartialEq)]
/// 单条可见聊天消息在滚动内容坐标系中的测量结果。
pub struct ChatMessageMeasurement {
    /// 消息在当前聊天列表中的索引。
    pub message_idx: usize,
    /// 消息边界的纵向起点。
    pub y: f32,
    /// 消息布局后的实际高度。
    pub height: f32,
}

#[derive(Debug)]
struct MeasuredChatMessageState {
    message_idx: usize,
}

/// 包装聊天消息并向 Iced 操作树暴露其布局边界。
pub(crate) struct MeasuredChatMessage<'a, Message, Theme, Renderer> {
    message_idx: usize,
    content: Element<'a, Message, Theme, Renderer>,
}

impl<'a, Message, Theme, Renderer> MeasuredChatMessage<'a, Message, Theme, Renderer> {
    /// 使用列表索引标记内容，渲染行为仍完全委托给内部元素。
    pub(crate) fn new(
        message_idx: usize,
        content: impl Into<Element<'a, Message, Theme, Renderer>>,
    ) -> Self {
        Self { message_idx, content: content.into() }
    }
}

impl<'a, Message, Theme, Renderer> From<MeasuredChatMessage<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    fn from(message: MeasuredChatMessage<'a, Message, Theme, Renderer>) -> Self {
        Element::new(message)
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for MeasuredChatMessage<'_, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    fn children(&self) -> Vec<widget::Tree> {
        vec![widget::Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut widget::Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        let mut state = MeasuredChatMessageState { message_idx: self.message_idx };
        operation.custom(None, layout.bounds(), &mut state);
        operation.traverse(&mut |operation| {
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                layout,
                renderer,
                operation,
            );
        });
    }

    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
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

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        // Session switches can briefly retain an invalid scroll translation.
        // cosmic-text converts glyph coordinates to i32 subpixel bins, so very large
        // finite values are just as unsafe as NaN and infinity.
        if !rectangle_has_safe_text_coordinates(layout.bounds())
            || !rectangle_has_safe_text_coordinates(*viewport)
        {
            return;
        }

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

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut widget::Tree,
        layout: Layout<'a>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'a, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

const TEXT_COORDINATE_LIMIT: f32 = i32::MAX as f32 / 2.0;

fn text_coordinate_is_safe(value: f32) -> bool {
    value.is_finite() && value.abs() <= TEXT_COORDINATE_LIMIT
}

fn rectangle_has_safe_text_coordinates(bounds: Rectangle) -> bool {
    bounds.width >= 0.0
        && bounds.height >= 0.0
        && text_coordinate_is_safe(bounds.x)
        && text_coordinate_is_safe(bounds.y)
        && text_coordinate_is_safe(bounds.x + bounds.width)
        && text_coordinate_is_safe(bounds.y + bounds.height)
}

#[derive(Debug, Default)]
/// 收集视口内消息测量结果的 Iced 自定义操作。
pub struct CollectVisibleChatMessageMeasurements {
    measurements: Vec<ChatMessageMeasurement>,
}

impl Operation<Vec<ChatMessageMeasurement>> for CollectVisibleChatMessageMeasurements {
    fn traverse(
        &mut self,
        operate: &mut dyn FnMut(&mut dyn Operation<Vec<ChatMessageMeasurement>>),
    ) {
        operate(self);
    }

    fn custom(&mut self, _id: Option<&widget::Id>, bounds: Rectangle, state: &mut dyn Any) {
        if let Some(state) = state.downcast_ref::<MeasuredChatMessageState>() {
            self.measurements.push(ChatMessageMeasurement {
                message_idx: state.message_idx,
                y: bounds.y,
                height: bounds.height.max(0.0),
            });
        }
    }

    fn finish(&self) -> iced::advanced::widget::operation::Outcome<Vec<ChatMessageMeasurement>> {
        iced::advanced::widget::operation::Outcome::Some(self.measurements.clone())
    }
}

/// 创建一次性的可见消息测量操作。
pub fn collect_visible_chat_message_measurements() -> CollectVisibleChatMessageMeasurements {
    CollectVisibleChatMessageMeasurements::default()
}
