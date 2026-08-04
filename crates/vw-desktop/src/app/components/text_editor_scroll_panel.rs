//! 文本编辑器组件的上下文菜单或滚动面板控件。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use iced::advanced::layout;
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::mouse;
use iced::widget::slider::Rail;
use iced::widget::{container, row, vertical_slider};
use iced::{Border, Color, Element, Event, Length, Rectangle, Size, Theme, Vector};

#[cfg(test)]
#[path = "text_editor_scroll_panel_tests.rs"]
mod tests;

#[derive(Debug, Clone, Copy)]
/// `TextEditorScrollPanelMetrics` 结构体，用于表达本模块对该领域对象的建模。
///
/// 该定义保持在当前模块职责内，调用方应通过显式字段、变体或别名理解其语义。
pub struct TextEditorScrollPanelMetrics {
    pub viewport_padding: f32,
    pub line_height: f32,
    pub line_count: usize,
    pub scroll_top_line: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ScrollPanelState {
    viewport_height: f32,
    max_scroll: f32,
    scroll_top_line: f32,
}

fn scroll_panel_state(size: Size, metrics: TextEditorScrollPanelMetrics) -> ScrollPanelState {
    let viewport_height = (size.height - metrics.viewport_padding).max(1.0);
    let line_height = metrics.line_height.max(1.0);
    let total_lines = metrics.line_count.max(1) as f32;
    let visible_lines = (viewport_height / line_height).floor().max(1.0);
    let max_scroll = (total_lines - visible_lines).max(0.0);
    let scroll_top_line = metrics.scroll_top_line.clamp(0.0, max_scroll);

    ScrollPanelState { viewport_height, max_scroll, scroll_top_line }
}

fn scrollbar_style(
    theme: &Theme,
    status: iced::widget::vertical_slider::Status,
) -> iced::widget::vertical_slider::Style {
    let palette = theme.extended_palette();
    let thumb = match status {
        iced::widget::vertical_slider::Status::Active => {
            palette.background.strong.color.scale_alpha(0.85)
        }
        iced::widget::vertical_slider::Status::Hovered => theme.palette().primary.scale_alpha(0.75),
        iced::widget::vertical_slider::Status::Dragged => theme.palette().primary,
    };

    iced::widget::vertical_slider::Style {
        rail: Rail {
            backgrounds: (
                palette.background.weak.color.scale_alpha(0.30).into(),
                palette.background.weak.color.scale_alpha(0.30).into(),
            ),
            width: 4.0,
            border: Border { radius: 999.0.into(), width: 0.0, color: Color::TRANSPARENT },
        },
        handle: iced::widget::vertical_slider::Handle {
            shape: iced::widget::vertical_slider::HandleShape::Rectangle {
                width: 8,
                border_radius: 999.0.into(),
            },
            background: thumb.into(),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
        },
    }
}

fn panel_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    iced::widget::container::Style {
        background: Some(palette.background.base.color.into()),
        border: Border { width: 1.0, color: palette.background.strong.color, radius: 10.0.into() },
        ..Default::default()
    }
}

/// 构建或处理 `text_editor_scroll_panel` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn text_editor_scroll_panel<'a, Message>(
    content: impl Into<Element<'a, Message>>,
    size: Size,
    metrics: TextEditorScrollPanelMetrics,
    on_wheel: impl Fn(mouse::ScrollDelta, f32) -> Message + 'a,
    on_scrollbar_changed: impl Fn(f32, f32) -> Message + 'a,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let state = scroll_panel_state(size, metrics);

    let content = wheel_interceptor(content, move |delta| on_wheel(delta, state.viewport_height));
    let mut body = row![container(content).width(Length::Fill).height(Length::Fill)];

    if state.max_scroll > 0.0 {
        let slider = vertical_slider(
            0.0..=state.max_scroll,
            state.max_scroll - state.scroll_top_line,
            move |value| on_scrollbar_changed(state.max_scroll - value, state.viewport_height),
        )
        .step(1.0)
        .width(10)
        .height(Length::Fill)
        .style(scrollbar_style);

        body = body.push(container(slider).width(Length::Fixed(10.0)).height(Length::Fill));
    }

    container(body.spacing(8).height(Length::Fill))
        .padding(12)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(panel_style)
        .into()
}

fn wheel_interceptor<'a, Message, Theme, Renderer>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
    on_scroll: impl Fn(mouse::ScrollDelta) -> Message + 'a,
) -> Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    Element::new(WheelInterceptor {
        content: content.into(),
        on_scroll: Some(Box::new(on_scroll)),
    })
}

/// 阻止嵌套控件消费鼠标滚轮或触摸板事件，让外层滚动区域继续处理。
pub(crate) fn pass_wheel_to_parent<'a, Message, Theme, Renderer>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    Element::new(WheelInterceptor { content: content.into(), on_scroll: None })
}

struct WheelInterceptor<'a, Message, Theme, Renderer> {
    content: Element<'a, Message, Theme, Renderer>,
    on_scroll: Option<Box<dyn Fn(mouse::ScrollDelta) -> Message + 'a>>,
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for WheelInterceptor<'_, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content.as_widget_mut().operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        if cursor.is_over(layout.bounds())
            && let Event::Mouse(mouse::Event::WheelScrolled { delta }) = event
        {
            if let Some(on_scroll) = &self.on_scroll {
                shell.publish(on_scroll(*delta));
                shell.capture_event();
            }
            return;
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

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
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

    fn mouse_interaction(
        &self,
        tree: &Tree,
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

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}
