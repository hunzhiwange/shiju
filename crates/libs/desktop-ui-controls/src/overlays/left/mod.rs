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

#[path = "widget.rs"]
mod widget_impl;
mod point_widget;
mod overlay_element;
use overlay_element::{LeftOverlayElement, PointLeftOverlayElement};

#[cfg(test)]
mod tests;
