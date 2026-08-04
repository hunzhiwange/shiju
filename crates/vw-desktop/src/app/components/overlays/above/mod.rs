//! 上方覆盖层组件模块
//!
//! 本模块提供了在目标元素上方显示覆盖层的组件实现。覆盖层可以用于显示下拉菜单、
//! 工具提示、弹出框等临时性 UI 元素。
//!
//! # 主要组件
//!
//! - [`AboveOverlay`]：基于目标元素位置的上方覆盖层
//! - [`PointAboveOverlay`]：基于指定锚点的上方覆盖层
//!
//! # 特性
//!
//! - 支持在视口内自动调整位置（snap_within_viewport）
//! - 支持自定义覆盖层与目标之间的间距（gap）
//! - 支持点击外部区域关闭覆盖层（on_close 回调）
//! - 完全集成到 Iced 的 Widget 和 Overlay 体系中

mod overlay_element;
mod widget;

#[cfg(test)]
#[path = "overlay_element_tests.rs"]
mod overlay_element_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
#[path = "widget_tests.rs"]
mod widget_tests;

use self::overlay_element::{AboveOverlayElement, PointAboveOverlayElement};
use iced::{Element, Point, Theme};

/// 上方覆盖层组件
///
/// 该组件在目标内容元素的上方显示一个覆盖层。覆盖层会自动定位在目标元素的上方，
/// 并支持在视口边界内自动调整位置。
///
/// # 类型参数
///
/// - `Message`：组件产生的消息类型
/// - `ThemeT`：主题类型，默认为 `Theme`
/// - `RendererT`：渲染器类型，默认为 `iced::Renderer`
///
/// # 示例
///
/// ```rust,ignore
/// let overlay = AboveOverlay::new(
///     button("点击我"),
///     column![text("选项 1"), text("选项 2")]
/// )
/// .show(is_open)
/// .gap(4.0)
/// .on_close(Message::CloseOverlay);
/// ```
pub struct AboveOverlay<'a, Message, ThemeT = Theme, RendererT = iced::Renderer> {
    /// 底层内容元素，覆盖层将显示在此元素上方
    content: Element<'a, Message, ThemeT, RendererT>,
    /// 覆盖层元素
    overlay: Element<'a, Message, ThemeT, RendererT>,
    /// 是否显示覆盖层
    show: bool,
    /// 覆盖层与目标元素之间的间距（像素）
    gap: f32,
    /// 是否将覆盖层限制在视口范围内
    snap_within_viewport: bool,
    /// 关闭覆盖层时触发的消息
    on_close: Option<Message>,
    /// 是否拦截覆盖层显示期间的所有用户输入事件
    block_background_events: bool,
}

/// 基于锚点的上方覆盖层组件
///
/// 该组件在指定的锚点位置上方显示覆盖层。与 [`AboveOverlay`] 不同，
/// 该组件允许指定一个相对锚点，覆盖层将基于该锚点定位。
///
/// # 类型参数
///
/// - `Message`：组件产生的消息类型
/// - `ThemeT`：主题类型，默认为 `Theme`
/// - `RendererT`：渲染器类型，默认为 `iced::Renderer`
///
/// # 示例
///
/// ```rust,ignore
/// let overlay = PointAboveOverlay::new(
///     container("主内容"),
///     text("工具提示")
/// )
/// .show(true)
/// .anchor(Point::new(50.0, 20.0))
/// .gap(8.0);
/// ```
pub struct PointAboveOverlay<'a, Message, ThemeT = Theme, RendererT = iced::Renderer> {
    /// 底层内容元素
    content: Element<'a, Message, ThemeT, RendererT>,
    /// 覆盖层元素
    overlay: Element<'a, Message, ThemeT, RendererT>,
    /// 是否显示覆盖层
    show: bool,
    /// 锚点位置，相对于内容元素的左上角
    anchor: Point,
    /// 覆盖层与锚点之间的间距（像素）
    gap: f32,
    /// 是否将覆盖层限制在视口范围内
    snap_within_viewport: bool,
    /// 关闭覆盖层时触发的消息
    on_close: Option<Message>,
}

impl<'a, Message, ThemeT, RendererT> AboveOverlay<'a, Message, ThemeT, RendererT> {
    /// 创建新的上方覆盖层
    ///
    /// # 参数
    ///
    /// - `content`：底层内容元素
    /// - `overlay`：覆盖层元素
    ///
    /// # 返回值
    ///
    /// 返回一个新创建的 `AboveOverlay` 实例，默认状态为：
    /// - `show`：`false`
    /// - `gap`：`0.0`
    /// - `snap_within_viewport`：`true`
    /// - `on_close`：`None`
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let overlay = AboveOverlay::new(
    ///     button("菜单"),
    ///     column![text("选项 A"), text("选项 B")]
    /// );
    /// ```
    pub fn new(
        content: impl Into<Element<'a, Message, ThemeT, RendererT>>,
        overlay: impl Into<Element<'a, Message, ThemeT, RendererT>>,
    ) -> Self {
        Self {
            content: content.into(),
            overlay: overlay.into(),
            show: false,
            gap: 0.0,
            snap_within_viewport: true,
            on_close: None,
            block_background_events: false,
        }
    }

    /// 设置是否显示覆盖层
    ///
    /// # 参数
    ///
    /// - `show`：是否显示覆盖层的布尔值
    ///
    /// # 返回值
    ///
    /// 返回修改后的 `AboveOverlay` 实例
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let overlay = AboveOverlay::new(content, popup).show(true);
    /// ```
    pub fn show(mut self, show: bool) -> Self {
        self.show = show;
        self
    }

    /// 设置覆盖层与目标元素之间的间距
    ///
    /// # 参数
    ///
    /// - `gap`：间距大小（像素）
    ///
    /// # 返回值
    ///
    /// 返回修改后的 `AboveOverlay` 实例
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let overlay = AboveOverlay::new(content, popup).gap(8.0);
    /// ```
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// 设置关闭覆盖层时触发的消息
    ///
    /// 当用户点击覆盖层外部区域时，将触发此消息。
    ///
    /// # 参数
    ///
    /// - `msg`：关闭时触发的消息
    ///
    /// # 返回值
    ///
    /// 返回修改后的 `AboveOverlay` 实例
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let overlay = AboveOverlay::new(content, popup)
    ///     .on_close(Message::CloseMenu);
    /// ```
    pub fn on_close(mut self, msg: Message) -> Self {
        self.on_close = Some(msg);
        self
    }

    /// 设置覆盖层显示期间是否阻止事件传递到底层界面。
    pub fn block_background_events(mut self, block: bool) -> Self {
        self.block_background_events = block;
        self
    }

    /// 设置是否将覆盖层限制在视口范围内
    ///
    /// 当设置为 `true` 时，覆盖层会自动调整位置以确保完全可见。
    /// 当设置为 `false` 时，覆盖层可能会超出视口边界。
    ///
    /// # 参数
    ///
    /// - `snap`：是否限制在视口内
    ///
    /// # 返回值
    ///
    /// 返回修改后的 `AboveOverlay` 实例
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let overlay = AboveOverlay::new(content, popup)
    ///     .snap_within_viewport(false);
    /// ```
    pub fn snap_within_viewport(mut self, snap: bool) -> Self {
        self.snap_within_viewport = snap;
        self
    }
}

impl<'a, Message, ThemeT, RendererT> PointAboveOverlay<'a, Message, ThemeT, RendererT> {
    /// 创建新的基于锚点的上方覆盖层
    ///
    /// # 参数
    ///
    /// - `content`：底层内容元素
    /// - `overlay`：覆盖层元素
    ///
    /// # 返回值
    ///
    /// 返回一个新创建的 `PointAboveOverlay` 实例，默认状态为：
    /// - `show`：`false`
    /// - `anchor`：`Point::ORIGIN`
    /// - `gap`：`0.0`
    /// - `snap_within_viewport`：`true`
    /// - `on_close`：`None`
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let overlay = PointAboveOverlay::new(
    ///     container("内容"),
    ///     text("提示文本")
    /// );
    /// ```
    pub fn new(
        content: impl Into<Element<'a, Message, ThemeT, RendererT>>,
        overlay: impl Into<Element<'a, Message, ThemeT, RendererT>>,
    ) -> Self {
        Self {
            content: content.into(),
            overlay: overlay.into(),
            show: false,
            anchor: Point::ORIGIN,
            gap: 0.0,
            snap_within_viewport: true,
            on_close: None,
        }
    }

    /// 设置是否显示覆盖层
    ///
    /// # 参数
    ///
    /// - `show`：是否显示覆盖层的布尔值
    ///
    /// # 返回值
    ///
    /// 返回修改后的 `PointAboveOverlay` 实例
    pub fn show(mut self, show: bool) -> Self {
        self.show = show;
        self
    }

    /// 设置覆盖层的锚点位置
    ///
    /// 锚点是相对于内容元素左上角的偏移量，覆盖层将显示在锚点上方。
    ///
    /// # 参数
    ///
    /// - `anchor`：锚点位置坐标
    ///
    /// # 返回值
    ///
    /// 返回修改后的 `PointAboveOverlay` 实例
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let overlay = PointAboveOverlay::new(content, tooltip)
    ///     .anchor(Point::new(100.0, 50.0));
    /// ```
    pub fn anchor(mut self, anchor: Point) -> Self {
        self.anchor = anchor;
        self
    }

    /// 设置覆盖层与锚点之间的间距
    ///
    /// # 参数
    ///
    /// - `gap`：间距大小（像素）
    ///
    /// # 返回值
    ///
    /// 返回修改后的 `PointAboveOverlay` 实例
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// 设置关闭覆盖层时触发的消息
    ///
    /// 当用户点击覆盖层外部区域时，将触发此消息。
    ///
    /// # 参数
    ///
    /// - `msg`：关闭时触发的消息
    ///
    /// # 返回值
    ///
    /// 返回修改后的 `PointAboveOverlay` 实例
    pub fn on_close(mut self, msg: Message) -> Self {
        self.on_close = Some(msg);
        self
    }

    /// 设置是否将覆盖层限制在视口范围内
    ///
    /// # 参数
    ///
    /// - `snap`：是否限制在视口内
    ///
    /// # 返回值
    ///
    /// 返回修改后的 `PointAboveOverlay` 实例
    pub fn snap_within_viewport(mut self, snap: bool) -> Self {
        self.snap_within_viewport = snap;
        self
    }
}
