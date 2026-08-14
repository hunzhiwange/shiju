//! 预览面板视图组件。
//!
//! 本模块负责预览内容、菜单、面包屑、LSP 标识或浮层宿主的局部构建。

use crate::app::Message;
/// 重新导出 use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, renderer, widget}，让上层模块通过稳定路径访问。
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, renderer, widget};
/// 重新导出 use iced::{Element, Event, Length, Point, Rectangle, Size, Theme}，让上层模块通过稳定路径访问。
use iced::{Element, Event, Length, Point, Rectangle, Size, Theme};
/// 重新导出 use iced::{Element as IcedElement, Renderer as IcedRenderer, Theme as IcedTheme}，让上层模块通过稳定路径访问。
use iced::{Element as IcedElement, Renderer as IcedRenderer, Theme as IcedTheme};

/// PreviewOverlayHost 保存 widgets 模块需要跨函数传递的状态。
///
/// 字段保持贴近调用方的真实数据，避免在 UI 边界处隐藏额外转换。
pub struct PreviewOverlayHost<'a, Message, ThemeT = IcedTheme, RendererT = IcedRenderer> {
    // content 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    content: IcedElement<'a, Message, ThemeT, RendererT>,
    // overlay 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    overlay: IcedElement<'a, Message, ThemeT, RendererT>,
    // show 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    show: bool,
    // pos 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    pos: Option<(f32, f32)>,
    // on_close 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    on_close: Option<crate::app::Message>,
}

/// DraggableAreaState 保存 widgets 模块需要跨函数传递的状态。
///
/// 字段保持贴近调用方的真实数据，避免在 UI 边界处隐藏额外转换。
#[derive(Default)]
pub(super) struct DraggableAreaState {
    // left_down_inside 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    left_down_inside: bool,
    // press_pos 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    press_pos: Option<Point>,
    // dragging_started 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    dragging_started: bool,
}

/// DraggableArea 保存 widgets 模块需要跨函数传递的状态。
///
/// 字段保持贴近调用方的真实数据，避免在 UI 边界处隐藏额外转换。
pub(super) struct DraggableArea<'a, MessageT, ThemeT = Theme, RendererT = iced::Renderer> {
    // content 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    content: Element<'a, MessageT, ThemeT, RendererT>,
    // on_drag_start 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    on_drag_start: MessageT,
    // on_drag_end 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    on_drag_end: MessageT,
}

impl<'a, MessageT, ThemeT, RendererT> DraggableArea<'a, MessageT, ThemeT, RendererT> {
    /// 处理 new 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 无返回值时，函数通过发布消息或更新局部状态完成交互。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    pub(super) fn new(
        content: Element<'a, MessageT, ThemeT, RendererT>,
        // on_drag_start 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        on_drag_start: MessageT,
        // on_drag_end 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        on_drag_end: MessageT,
    ) -> Self {
        Self { content, on_drag_start, on_drag_end }
    }
}

impl<'a, MessageT, ThemeT, RendererT> Widget<MessageT, ThemeT, RendererT>
    for DraggableArea<'a, MessageT, ThemeT, RendererT>
where
    RendererT: iced::advanced::Renderer,
    // MessageT 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    MessageT: Clone,
{
    /// 处理 tag 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 返回值保持当前模块的领域语义，供相邻视图或测试继续使用。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn tag(&self) -> widget::tree::Tag {
        // widget 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        widget::tree::Tag::of::<DraggableAreaState>()
    }

    /// 处理 state 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 返回值保持当前模块的领域语义，供相邻视图或测试继续使用。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn state(&self) -> widget::tree::State {
        // widget 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        widget::tree::State::new(DraggableAreaState::default())
    }

    /// 处理 children 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 返回集合保持输入顺序或界面展示顺序，空集合表示没有可展示项。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn children(&self) -> Vec<widget::Tree> {
        vec![widget::Tree::new(&self.content)]
    }

    /// 处理 diff 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 无返回值时，函数通过发布消息或更新局部状态完成交互。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(&[&self.content]);
    }

    /// 处理 size 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 返回值保持当前模块的领域语义，供相邻视图或测试继续使用。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    /// 处理 layout 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 无返回值时，函数通过发布消息或更新局部状态完成交互。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn layout(
        &mut self,
        // tree 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        tree: &mut widget::Tree,
        // renderer 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        renderer: &RendererT,
        // limits 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }

    /// 处理 draw 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 无返回值时，函数通过发布消息或更新局部状态完成交互。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn draw(
        &self,
        // tree 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        tree: &widget::Tree,
        // renderer 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        renderer: &mut RendererT,
        // theme 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        theme: &ThemeT,
        // style 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        style: &renderer::Style,
        // layout 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        layout: Layout<'_>,
        // cursor 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        cursor: mouse::Cursor,
        // viewport 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
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

    /// 处理 update 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 无返回值时，函数通过发布消息或更新局部状态完成交互。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn update(
        &mut self,
        // tree 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        tree: &mut widget::Tree,
        // event 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        event: &Event,
        // layout 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        layout: Layout<'_>,
        // cursor 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        cursor: mouse::Cursor,
        // renderer 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        renderer: &RendererT,
        // clipboard 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        clipboard: &mut dyn Clipboard,
        // shell 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        shell: &mut Shell<'_, MessageT>,
        // viewport 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<DraggableAreaState>();

        if let Event::Mouse(mouse::Event::ButtonPressed(button)) = event
            && matches!(button, mouse::Button::Left)
            && let Some(pos) = cursor.position()
            && layout.bounds().contains(pos)
        {
            state.left_down_inside = true;
            state.press_pos = Some(pos);
            state.dragging_started = false;
        }

        if let Event::Mouse(mouse::Event::CursorMoved { position }) = event
            && state.left_down_inside
            && !state.dragging_started
            && let Some(anchor) = state.press_pos
        {
            let dx = position.x - anchor.x;
            let dy = position.y - anchor.y;
            let moved_enough = (dx * dx + dy * dy) >= 9.0;
            if moved_enough {
                state.dragging_started = true;
                shell.publish(self.on_drag_start.clone());
            }
        }

        if let Event::Mouse(mouse::Event::ButtonReleased(button)) = event
            && matches!(button, mouse::Button::Left)
        {
            if state.dragging_started {
                shell.publish(self.on_drag_end.clone());
            }
            state.left_down_inside = false;
            state.press_pos = None;
            state.dragging_started = false;
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

    /// 处理 mouse interaction 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 无返回值时，函数通过发布消息或更新局部状态完成交互。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn mouse_interaction(
        &self,
        // tree 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        tree: &widget::Tree,
        // layout 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        layout: Layout<'_>,
        // cursor 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        cursor: mouse::Cursor,
        // viewport 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        viewport: &Rectangle,
        // renderer 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
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
impl<'a, Message, ThemeT, RendererT> PreviewOverlayHost<'a, Message, ThemeT, RendererT> {
    /// 处理 new 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 无返回值时，函数通过发布消息或更新局部状态完成交互。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    pub(super) fn new(
        content: impl Into<IcedElement<'a, Message, ThemeT, RendererT>>,
        // overlay 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        overlay: impl Into<IcedElement<'a, Message, ThemeT, RendererT>>,
    ) -> Self {
        Self {
            // content 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            content: content.into(),
            // overlay 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            overlay: overlay.into(),
            // show 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            show: false,
            // pos 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            pos: None,
            // on_close 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            on_close: None,
        }
    }
    /// 处理 show 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 返回值保持当前模块的领域语义，供相邻视图或测试继续使用。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    pub(super) fn show(mut self, show: bool) -> Self {
        self.show = show;
        self
    }
    /// 处理 pos 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 返回值保持当前模块的领域语义，供相邻视图或测试继续使用。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    pub(super) fn pos(mut self, pos: Option<(f32, f32)>) -> Self {
        self.pos = pos;
        self
    }
    /// 处理 on close 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 返回值保持当前模块的领域语义，供相邻视图或测试继续使用。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    pub(super) fn on_close(mut self, on_close: crate::app::Message) -> Self {
        self.on_close = Some(on_close);
        self
    }
}
impl<ThemeT, RendererT> Widget<crate::app::Message, ThemeT, RendererT>
    for PreviewOverlayHost<'_, crate::app::Message, ThemeT, RendererT>
where
    RendererT: iced::advanced::Renderer,
{
    /// 处理 children 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 返回集合保持输入顺序或界面展示顺序，空集合表示没有可展示项。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn children(&self) -> Vec<widget::Tree> {
        vec![widget::Tree::new(&self.content), widget::Tree::new(&self.overlay)]
    }
    /// 处理 diff 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 无返回值时，函数通过发布消息或更新局部状态完成交互。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(&[self.content.as_widget(), self.overlay.as_widget()]);
    }
    /// 处理 size 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 返回值保持当前模块的领域语义，供相邻视图或测试继续使用。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }
    /// 处理 size hint 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 返回值保持当前模块的领域语义，供相邻视图或测试继续使用。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }
    /// 处理 layout 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 无返回值时，函数通过发布消息或更新局部状态完成交互。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn layout(
        &mut self,
        // tree 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        tree: &mut widget::Tree,
        // renderer 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        renderer: &RendererT,
        // limits 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }
    /// 将 Iced widget operation 转发给内容树。
    ///
    /// 编辑器的 `scroll_to`、`snap_to` 和聚焦命令依赖 operation 遍历；
    /// 自定义宿主如果不转发，命令会在这一层被截断。
    fn operate(
        &mut self,
        tree: &mut widget::Tree,
        layout: Layout<'_>,
        renderer: &RendererT,
        operation: &mut dyn widget::Operation,
    ) {
        self.content.as_widget_mut().operate(&mut tree.children[0], layout, renderer, operation);
    }
    /// 处理 draw 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 无返回值时，函数通过发布消息或更新局部状态完成交互。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn draw(
        &self,
        // tree 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        tree: &widget::Tree,
        // renderer 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        renderer: &mut RendererT,
        // theme 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        theme: &ThemeT,
        // style 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        style: &renderer::Style,
        // layout 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        layout: Layout<'_>,
        // cursor 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        cursor: mouse::Cursor,
        // viewport 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
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
    /// 处理 update 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 无返回值时，函数通过发布消息或更新局部状态完成交互。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn update(
        &mut self,
        // tree 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        tree: &mut widget::Tree,
        // event 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        event: &Event,
        // layout 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        layout: Layout<'_>,
        // cursor 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        cursor: mouse::Cursor,
        // renderer 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        renderer: &RendererT,
        // clipboard 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        clipboard: &mut dyn Clipboard,
        // shell 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        shell: &mut Shell<'_, crate::app::Message>,
        // viewport 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
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
    /// 处理 mouse interaction 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 无返回值时，函数通过发布消息或更新局部状态完成交互。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn mouse_interaction(
        &self,
        // tree 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        tree: &widget::Tree,
        // layout 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        layout: Layout<'_>,
        // cursor 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        cursor: mouse::Cursor,
        // viewport 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        viewport: &Rectangle,
        // renderer 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
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
    /// 构建或定位 overlay，用于把浮层稳定附着到目标控件。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 无返回值时，函数通过发布消息或更新局部状态完成交互。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn overlay<'b>(
        &'b mut self,
        // tree 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        tree: &'b mut widget::Tree,
        // layout 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        layout: Layout<'b>,
        // renderer 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        renderer: &RendererT,
        // viewport 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        viewport: &Rectangle,
        // translation 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        translation: iced::Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, crate::app::Message, ThemeT, RendererT>> {
        let (content_tree, overlay_tree) = tree.children.split_at_mut(1);
        let content = self.content.as_widget_mut().overlay(
            &mut content_tree[0],
            layout,
            renderer,
            viewport,
            translation,
        );

        if !self.show {
            return content;
        }

        let preview_overlay =
            iced::advanced::overlay::Element::new(Box::new(PreviewOverlayElement {
                // overlay 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                overlay: &mut self.overlay,
                // tree 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                tree: &mut overlay_tree[0],
                // pos 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                pos: self.pos,
                // on_close 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                on_close: self.on_close.clone(),
            }));

        Some(
            iced::advanced::overlay::Group::with_children(
                content.into_iter().chain(std::iter::once(preview_overlay)).collect(),
            )
            .overlay(),
        )
    }
}
impl<'a, ThemeT, RendererT> From<PreviewOverlayHost<'a, crate::app::Message, ThemeT, RendererT>>
    for IcedElement<'a, crate::app::Message, ThemeT, RendererT>
where
    RendererT: iced::advanced::Renderer + 'a,
    // ThemeT 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    ThemeT: 'a,
{
    /// 处理 from 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 返回值保持当前模块的领域语义，供相邻视图或测试继续使用。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn from(widget: PreviewOverlayHost<'a, crate::app::Message, ThemeT, RendererT>) -> Self {
        // IcedElement 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        IcedElement::new(widget)
    }
}
/// PreviewOverlayElement 保存 widgets 模块需要跨函数传递的状态。
///
/// 字段保持贴近调用方的真实数据，避免在 UI 边界处隐藏额外转换。
struct PreviewOverlayElement<'a, 'b, Message, ThemeT, RendererT> {
    // overlay 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    overlay: &'b mut IcedElement<'a, Message, ThemeT, RendererT>,
    // tree 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    tree: &'b mut widget::Tree,
    // pos 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    pos: Option<(f32, f32)>,
    // on_close 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    on_close: Option<crate::app::Message>,
}
impl<ThemeT, RendererT> iced::advanced::overlay::Overlay<crate::app::Message, ThemeT, RendererT>
    for PreviewOverlayElement<'_, '_, crate::app::Message, ThemeT, RendererT>
where
    RendererT: iced::advanced::Renderer,
{
    /// 处理 layout 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 返回值保持当前模块的领域语义，供相邻视图或测试继续使用。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn layout(&mut self, renderer: &RendererT, bounds: Size) -> layout::Node {
        let limits = layout::Limits::new(Size::ZERO, bounds);
        let node0 = self.overlay.as_widget_mut().layout(self.tree, renderer, &limits);
        let size = node0.size();
        let (mut x, mut y) = if let Some((mx, my)) = self.pos { (mx, my) } else { (0.0, 0.0) };
        x = x.clamp(0.0, (bounds.width - size.width).max(0.0));
        y = y.clamp(0.0, (bounds.height - size.height).max(0.0));

        node0.move_to(Point::new(x, y))
    }
    /// 处理 update 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 无返回值时，函数通过发布消息或更新局部状态完成交互。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn update(
        &mut self,
        // event 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        event: &Event,
        // layout 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        layout: Layout<'_>,
        // cursor 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        cursor: mouse::Cursor,
        // renderer 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        renderer: &RendererT,
        // clipboard 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        clipboard: &mut dyn Clipboard,
        // shell 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        shell: &mut Shell<'_, Message>,
    ) {
        if let Event::Mouse(mouse::Event::ButtonPressed(_)) = event
            && let Some(pos) = cursor.position()
        {
            let bounds = layout.bounds();
            if !bounds.contains(pos)
                && let Some(on_close) = &self.on_close
            {
                shell.publish(on_close.clone());
            }
        }
        self.overlay.as_widget_mut().update(
            self.tree,
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            &layout.bounds(),
        );
    }
    /// 处理 mouse interaction 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 无返回值时，函数通过发布消息或更新局部状态完成交互。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn mouse_interaction(
        &self,
        // layout 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        layout: Layout<'_>,
        // cursor 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        cursor: mouse::Cursor,
        // renderer 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        renderer: &RendererT,
    ) -> mouse::Interaction {
        self.overlay.as_widget().mouse_interaction(
            self.tree,
            layout,
            cursor,
            &layout.bounds(),
            renderer,
        )
    }
    /// 处理 draw 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 无返回值时，函数通过发布消息或更新局部状态完成交互。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn draw(
        &self,
        // renderer 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        renderer: &mut RendererT,
        // theme 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        theme: &ThemeT,
        // defaults 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        defaults: &renderer::Style,
        // layout 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        layout: Layout<'_>,
        // cursor 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        cursor: mouse::Cursor,
    ) {
        self.overlay.as_widget().draw(
            self.tree,
            renderer,
            theme,
            defaults,
            layout,
            cursor,
            &layout.bounds(),
        );
    }
}
