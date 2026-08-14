//! Overlay 定位组件。
//!
//! 本模块封装 Iced overlay 的定位、尺寸裁剪和外部点击关闭行为。

use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer, widget};
/// 重新导出 use iced::{Element, Length}，让上层模块通过稳定路径访问。
use iced::{Element, Length};
/// 重新导出 use iced::{Event, Point, Rectangle, Size, Theme, Vector}，让上层模块通过稳定路径访问。
use iced::{Event, Point, Rectangle, Size, Theme, Vector};

fn compute_left_position(
    position: Point,
    target_bounds: Rectangle,
    viewport: Rectangle,
    overlay_size: Size,
    gap: f32,
    snap_within_viewport: bool,
) -> Point {
    let mut x = position.x - gap - overlay_size.width;
    let mut y = position.y + (target_bounds.height / 2.0) - (overlay_size.height / 2.0);

    if snap_within_viewport {
        x = x.clamp(0.0, (viewport.width - overlay_size.width).max(0.0));
        y = y.clamp(0.0, (viewport.height - overlay_size.height).max(0.0));
    }

    Point::new(x, y)
}

fn compute_point_left_position(
    anchor: Point,
    viewport: Rectangle,
    overlay_size: Size,
    gap: f32,
    snap_within_viewport: bool,
) -> Point {
    let mut x = anchor.x - gap - overlay_size.width;
    let mut y = anchor.y - (overlay_size.height / 2.0);

    if snap_within_viewport {
        x = x.clamp(0.0, (viewport.width - overlay_size.width).max(0.0));
        y = y.clamp(0.0, (viewport.height - overlay_size.height).max(0.0));
    }

    Point::new(x, y)
}

/// LeftOverlay 保存 left 模块需要跨函数传递的状态。
///
/// 字段保持贴近调用方的真实数据，避免在 UI 边界处隐藏额外转换。
pub struct LeftOverlay<'a, Message, ThemeT = Theme, RendererT = iced::Renderer> {
    // content 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    content: Element<'a, Message, ThemeT, RendererT>,
    // overlay 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    overlay: Element<'a, Message, ThemeT, RendererT>,
    // show 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    show: bool,
    // gap 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    gap: f32,
    // snap 开关让浮层在靠近窗口边缘时仍保持可见。
    // snap_within_viewport 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    snap_within_viewport: bool,
    // on_close 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    on_close: Option<Message>,
}

/// PointLeftOverlay 保存 left 模块需要跨函数传递的状态。
///
/// 字段保持贴近调用方的真实数据，避免在 UI 边界处隐藏额外转换。
pub struct PointLeftOverlay<'a, Message, ThemeT = Theme, RendererT = iced::Renderer> {
    // content 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    content: Element<'a, Message, ThemeT, RendererT>,
    // overlay 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    overlay: Element<'a, Message, ThemeT, RendererT>,
    // show 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    show: bool,
    // anchor 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    anchor: Point,
    // gap 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    gap: f32,
    // snap 开关让浮层在靠近窗口边缘时仍保持可见。
    // snap_within_viewport 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    snap_within_viewport: bool,
    // on_close 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    on_close: Option<Message>,
    // 被动提示浮层不应拦截底层控件的鼠标移动或滚轮事件。
    capture_events: bool,
}

impl<'a, Message, ThemeT, RendererT> LeftOverlay<'a, Message, ThemeT, RendererT> {
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
    pub fn new(
        content: impl Into<Element<'a, Message, ThemeT, RendererT>>,
        // overlay 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        overlay: impl Into<Element<'a, Message, ThemeT, RendererT>>,
    ) -> Self {
        Self {
            // content 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            content: content.into(),
            // overlay 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            overlay: overlay.into(),
            // show 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            show: false,
            // gap 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            gap: 0.0,
            // snap 开关让浮层在靠近窗口边缘时仍保持可见。
            // snap_within_viewport 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            snap_within_viewport: true,
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
    pub fn show(mut self, show: bool) -> Self {
        self.show = show;
        self
    }

    /// 处理 gap 对应的局部职责。
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
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
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
    pub fn on_close(mut self, msg: Message) -> Self {
        self.on_close = Some(msg);
        self
    }

    /// 处理 snap within viewport 对应的局部职责。
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
    pub fn snap_within_viewport(mut self, snap: bool) -> Self {
        self.snap_within_viewport = snap;
        self
    }
}

impl<'a, Message, ThemeT, RendererT> PointLeftOverlay<'a, Message, ThemeT, RendererT> {
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
    pub fn new(
        content: impl Into<Element<'a, Message, ThemeT, RendererT>>,
        // overlay 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        overlay: impl Into<Element<'a, Message, ThemeT, RendererT>>,
    ) -> Self {
        Self {
            // content 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            content: content.into(),
            // overlay 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            overlay: overlay.into(),
            // show 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            show: false,
            // anchor 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            anchor: Point::ORIGIN,
            // gap 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            gap: 0.0,
            // snap 开关让浮层在靠近窗口边缘时仍保持可见。
            // snap_within_viewport 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            snap_within_viewport: true,
            // on_close 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            on_close: None,
            capture_events: true,
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
    pub fn show(mut self, show: bool) -> Self {
        self.show = show;
        self
    }

    /// 处理 anchor 对应的局部职责。
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
    pub fn anchor(mut self, anchor: Point) -> Self {
        self.anchor = anchor;
        self
    }

    /// 处理 gap 对应的局部职责。
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
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
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
    pub fn on_close(mut self, msg: Message) -> Self {
        self.on_close = Some(msg);
        self
    }

    /// 处理 snap within viewport 对应的局部职责。
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
    pub fn snap_within_viewport(mut self, snap: bool) -> Self {
        self.snap_within_viewport = snap;
        self
    }

    /// 设置浮层是否拦截鼠标事件。
    pub fn capture_events(mut self, capture: bool) -> Self {
        self.capture_events = capture;
        self
    }
}

impl<Message, ThemeT, RendererT> Widget<Message, ThemeT, RendererT>
    for LeftOverlay<'_, Message, ThemeT, RendererT>
where
    RendererT: iced::advanced::Renderer,
    // Message 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    Message: Clone,
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
        shell: &mut Shell<'_, Message>,
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
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, ThemeT, RendererT>> {
        let mut children = tree.children.iter_mut();

        let content = self.content.as_widget_mut().overlay(
            children.next().unwrap(),
            layout,
            renderer,
            viewport,
            translation,
        );

        let side = if self.show {
            Some(overlay::Element::new(Box::new(LeftOverlayElement {
                // position 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                position: layout.position() + translation,
                // target_bounds 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                target_bounds: layout.bounds(),
                // viewport 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                viewport: *viewport,
                // gap 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                gap: self.gap,
                // snap 开关让浮层在靠近窗口边缘时仍保持可见。
                // snap_within_viewport 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                snap_within_viewport: self.snap_within_viewport,
                // overlay 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                overlay: &mut self.overlay,
                // tree 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                tree: children.next().unwrap(),
                // on_close 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                on_close: self.on_close.clone(),
            })))
        } else {
            None
        };

        if content.is_some() || side.is_some() {
            Some(overlay::Group::with_children(content.into_iter().chain(side).collect()).overlay())
        } else {
            None
        }
    }
}

impl<Message, ThemeT, RendererT> Widget<Message, ThemeT, RendererT>
    for PointLeftOverlay<'_, Message, ThemeT, RendererT>
where
    RendererT: iced::advanced::Renderer,
    // Message 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    Message: Clone,
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
        shell: &mut Shell<'_, Message>,
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
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, ThemeT, RendererT>> {
        let mut children = tree.children.iter_mut();

        let content = self.content.as_widget_mut().overlay(
            children.next().unwrap(),
            layout,
            renderer,
            viewport,
            translation,
        );

        let side = if self.show {
            let base = layout.position() + translation;
            let anchor = Point::new(base.x + self.anchor.x, base.y + self.anchor.y);
            Some(overlay::Element::new(Box::new(PointLeftOverlayElement {
                anchor,
                // viewport 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                viewport: *viewport,
                // gap 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                gap: self.gap,
                // snap 开关让浮层在靠近窗口边缘时仍保持可见。
                // snap_within_viewport 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                snap_within_viewport: self.snap_within_viewport,
                // overlay 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                overlay: &mut self.overlay,
                // tree 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                tree: children.next().unwrap(),
                // on_close 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                on_close: self.on_close.clone(),
                capture_events: self.capture_events,
            })))
        } else {
            None
        };

        if content.is_some() || side.is_some() {
            Some(overlay::Group::with_children(content.into_iter().chain(side).collect()).overlay())
        } else {
            None
        }
    }
}

impl<'a, Message, ThemeT, RendererT> From<LeftOverlay<'a, Message, ThemeT, RendererT>>
    for Element<'a, Message, ThemeT, RendererT>
where
    Message: 'a + Clone,
    // RendererT 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
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
    fn from(widget: LeftOverlay<'a, Message, ThemeT, RendererT>) -> Self {
        // Element 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Element::new(widget)
    }
}

impl<'a, Message, ThemeT, RendererT> From<PointLeftOverlay<'a, Message, ThemeT, RendererT>>
    for Element<'a, Message, ThemeT, RendererT>
where
    Message: 'a + Clone,
    // RendererT 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
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
    fn from(widget: PointLeftOverlay<'a, Message, ThemeT, RendererT>) -> Self {
        // Element 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Element::new(widget)
    }
}

/// LeftOverlayElement 保存 left 模块需要跨函数传递的状态。
///
/// 字段保持贴近调用方的真实数据，避免在 UI 边界处隐藏额外转换。
struct LeftOverlayElement<'a, 'b, Message, ThemeT, RendererT> {
    // position 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    position: Point,
    // target_bounds 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    target_bounds: Rectangle,
    // viewport 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    viewport: Rectangle,
    // gap 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    gap: f32,
    // snap 开关让浮层在靠近窗口边缘时仍保持可见。
    // snap_within_viewport 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    snap_within_viewport: bool,
    // overlay 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    overlay: &'b mut Element<'a, Message, ThemeT, RendererT>,
    // tree 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    tree: &'b mut widget::Tree,
    // on_close 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    on_close: Option<Message>,
}

impl<Message, ThemeT, RendererT> overlay::Overlay<Message, ThemeT, RendererT>
    for LeftOverlayElement<'_, '_, Message, ThemeT, RendererT>
where
    RendererT: iced::advanced::Renderer,
    // Message 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    Message: Clone,
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
        let viewport = Rectangle::with_size(bounds);

        // snap 开关让浮层在靠近窗口边缘时仍保持可见。
        let max_w = if self.snap_within_viewport { viewport.width } else { bounds.width };
        let node = self.overlay.as_widget_mut().layout(
            self.tree,
            renderer,
            &layout::Limits::new(Size::ZERO, Size::new(max_w, viewport.height)),
        );

        let size = node.size();

        let position = compute_left_position(
            self.position,
            self.target_bounds,
            self.viewport,
            size,
            self.gap,
            self.snap_within_viewport,
        );

        node.move_to(position)
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
        let bounds = layout.bounds();

        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event
            && let Some(on_close) = &self.on_close
            && let Some(cursor_position) = cursor.position()
            && !self.target_bounds.contains(cursor_position)
            && !bounds.contains(cursor_position)
        {
            shell.publish(on_close.clone());
            shell.capture_event();
        }

        self.overlay
            .as_widget_mut()
            .update(self.tree, event, layout, cursor, renderer, clipboard, shell, &bounds);

        if matches!(event, Event::Mouse(_)) {
            shell.capture_event();
        }
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
            &self.viewport,
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
        let bounds = layout.bounds();

        self.overlay
            .as_widget()
            .draw(self.tree, renderer, theme, defaults, layout, cursor, &bounds);
    }
}

/// PointLeftOverlayElement 保存 left 模块需要跨函数传递的状态。
///
/// 字段保持贴近调用方的真实数据，避免在 UI 边界处隐藏额外转换。
struct PointLeftOverlayElement<'a, 'b, Message, ThemeT, RendererT> {
    // anchor 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    anchor: Point,
    // viewport 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    viewport: Rectangle,
    // gap 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    gap: f32,
    // snap 开关让浮层在靠近窗口边缘时仍保持可见。
    // snap_within_viewport 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    snap_within_viewport: bool,
    // overlay 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    overlay: &'b mut Element<'a, Message, ThemeT, RendererT>,
    // tree 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    tree: &'b mut widget::Tree,
    // on_close 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    on_close: Option<Message>,
    // capture_events 控制被动浮层是否阻止事件继续传递到底层内容。
    capture_events: bool,
}

impl<Message, ThemeT, RendererT> overlay::Overlay<Message, ThemeT, RendererT>
    for PointLeftOverlayElement<'_, '_, Message, ThemeT, RendererT>
where
    RendererT: iced::advanced::Renderer,
    // Message 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    Message: Clone,
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
        let viewport = Rectangle::with_size(bounds);

        // snap 开关让浮层在靠近窗口边缘时仍保持可见。
        let max_w = if self.snap_within_viewport { viewport.width } else { bounds.width };
        let node = self.overlay.as_widget_mut().layout(
            self.tree,
            renderer,
            &layout::Limits::new(Size::ZERO, Size::new(max_w, viewport.height)),
        );

        let size = node.size();

        let position = compute_point_left_position(
            self.anchor,
            self.viewport,
            size,
            self.gap,
            self.snap_within_viewport,
        );

        node.move_to(position)
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
        let bounds = layout.bounds();
        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event
            && let Some(on_close) = &self.on_close
            && let Some(cursor_position) = cursor.position()
        {
            if !bounds.contains(cursor_position) {
                #[cfg(debug_assertions)]
                println!(
                    "PointLeftOverlay: Closing overlay (click outside). Bounds: {:?}, Cursor: {:?}",
                    bounds, cursor_position
                );
                shell.publish(on_close.clone());
                shell.capture_event();
            } else {
                #[cfg(debug_assertions)]
                println!(
                    "PointLeftOverlay: Click inside. Bounds: {:?}, Cursor: {:?}",
                    bounds, cursor_position
                );
            }
        }

        self.overlay
            .as_widget_mut()
            .update(self.tree, event, layout, cursor, renderer, clipboard, shell, &bounds);

        if self.capture_events && matches!(event, Event::Mouse(_)) {
            shell.capture_event();
        }
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
            &self.viewport,
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

#[cfg(test)]
#[path = "left_tests.rs"]
mod tests;
