//! 侧边覆盖层组件模块
//!
//! 本模块提供了一个可定制的侧边覆盖层（overlay）组件，用于在主内容区域的侧面显示额外的UI元素。
//! 该组件支持灵活的定位、对齐和交互行为控制。
//!
//! # 主要功能
//!
//! - **侧边显示**：覆盖层在目标元素的右侧显示，支持自定义间距
//! - **智能定位**：自动计算位置，支持在视口内吸附以避免超出屏幕
//! - **点击外部关闭**：支持点击覆盖层外部区域时触发关闭消息
//! - **灵活对齐**：支持顶部对齐或跟随目标元素位置
//!
//! # 使用示例
//!
//! ```ignore
//! use crate::app::components::overlays::side::SideOverlay;
//!
//! let content = text("主内容");
//! let overlay = container(text("侧边面板")).padding(10);
//!
//! let side_overlay = SideOverlay::new(content, overlay)
//!     .show(true)
//!     .gap(10.0)
//!     .min_x(20.0)
//!     .min_y(20.0)
//!     .on_close(CloseMessage)
//!     .snap_within_viewport(true);
//! ```

use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer, widget};
use iced::{Element, Length};
use iced::{Event, Point, Rectangle, Size, Theme, Vector};

pub(super) fn side_overlay_max_width(
    bounds: Size,
    position: Point,
    target_bounds: Rectangle,
    gap: f32,
    snap_within_viewport: bool,
) -> f32 {
    let space_right = bounds.width - (position.x + target_bounds.width + gap);
    let space_left = position.x - gap;

    if snap_within_viewport { space_right.max(space_left).max(0.0) } else { bounds.width }
}

pub(super) fn compute_side_overlay_position(
    position: Point,
    target_bounds: Rectangle,
    viewport: Rectangle,
    overlay_size: Size,
    gap: f32,
    min_x: f32,
    min_y: f32,
    snap_within_viewport: bool,
    align_y_start: bool,
) -> Point {
    let space_right = viewport.width - (position.x + target_bounds.width + gap);
    let space_left = position.x - gap;
    let right_x = position.x + target_bounds.width + gap;
    let left_x = position.x - gap - overlay_size.width;
    let prefer_right = space_right >= overlay_size.width || space_right >= space_left;

    let mut x = if prefer_right { right_x } else { left_x };
    let mut y = if align_y_start { min_y } else { position.y.max(min_y) };

    if snap_within_viewport {
        let max_x = (viewport.width - overlay_size.width).max(min_x);
        x = x.clamp(min_x, max_x);

        let max_y = (viewport.height - overlay_size.height).max(min_y);
        y = y.clamp(min_y, max_y);
    }

    Point::new(x, y)
}

/// 侧边覆盖层组件
///
/// 一个组合式组件，包含主内容和侧边覆盖层两部分。覆盖层可以在主内容的右侧显示，
/// 适用于实现侧边菜单、详情面板、工具提示等UI模式。
///
/// # 类型参数
///
/// - `Message` - 组件产生的消息类型
/// - `ThemeT` - 主题类型，默认为 `iced::Theme`
/// - `RendererT` - 渲染器类型，默认为 `iced::Renderer`
///
/// # 字段说明
///
/// - `content` - 主内容元素
/// - `overlay` - 侧边覆盖层元素
/// - `show` - 是否显示覆盖层
/// - `gap` - 覆盖层与目标元素之间的间距（像素）
/// - `min_x` - 覆盖层最小X坐标限制
/// - `min_y` - 覆盖层最小Y坐标限制
/// - `snap_within_viewport` - 是否将覆盖层吸附在视口内
/// - `align_y_start` - 是否将覆盖层对齐到顶部（而非跟随目标元素）
/// - `on_close` - 点击外部时触发的关闭消息
pub struct SideOverlay<'a, Message, ThemeT = Theme, RendererT = iced::Renderer> {
    content: Element<'a, Message, ThemeT, RendererT>,
    overlay: Element<'a, Message, ThemeT, RendererT>,
    show: bool,
    gap: f32,
    min_x: f32,
    min_y: f32,
    snap_within_viewport: bool,
    align_y_start: bool,
    on_close: Option<Message>,
}

impl<'a, Message, ThemeT, RendererT> SideOverlay<'a, Message, ThemeT, RendererT> {
    /// 创建新的侧边覆盖层组件
    ///
    /// # 参数
    ///
    /// - `content` - 主内容元素，将被转换为 `Element`
    /// - `overlay` - 侧边覆盖层元素，将被转换为 `Element`
    ///
    /// # 返回值
    ///
    /// 返回一个新创建的 `SideOverlay` 实例，默认配置为：
    /// - 覆盖层隐藏
    /// - 间距为 0
    /// - 最小坐标为 0
    /// - 启用视口内吸附
    /// - 禁用顶部对齐
    /// - 无关闭消息
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let overlay = SideOverlay::new(
    ///     text("主内容"),
    ///     container("侧边面板")
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
            min_x: 0.0,
            min_y: 0.0,
            snap_within_viewport: true,
            align_y_start: false,
            on_close: None,
        }
    }

    /// 设置是否显示覆盖层
    ///
    /// # 参数
    ///
    /// - `show` - `true` 表示显示覆盖层，`false` 表示隐藏
    ///
    /// # 返回值
    ///
    /// 返回修改后的 `SideOverlay` 实例（builder 模式）
    pub fn show(mut self, show: bool) -> Self {
        self.show = show;
        self
    }

    /// 设置覆盖层与目标元素之间的间距
    ///
    /// # 参数
    ///
    /// - `gap` - 间距值（像素）
    ///
    /// # 返回值
    ///
    /// 返回修改后的 `SideOverlay` 实例（builder 模式）
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// 设置覆盖层的最小Y坐标
    ///
    /// 用于防止覆盖层显示在屏幕顶部之外或与顶部工具栏重叠。
    ///
    /// # 参数
    ///
    /// - `min_y` - 最小Y坐标值（像素）
    ///
    /// # 返回值
    ///
    /// 返回修改后的 `SideOverlay` 实例（builder 模式）
    pub fn min_y(mut self, min_y: f32) -> Self {
        self.min_y = min_y;
        self
    }

    /// 设置覆盖层的最小X坐标
    ///
    /// 用于防止覆盖层显示在屏幕左侧之外或与左侧边栏重叠。
    ///
    /// # 参数
    ///
    /// - `min_x` - 最小X坐标值（像素）
    ///
    /// # 返回值
    ///
    /// 返回修改后的 `SideOverlay` 实例（builder 模式）
    pub fn min_x(mut self, min_x: f32) -> Self {
        self.min_x = min_x;
        self
    }

    /// 设置是否将覆盖层对齐到顶部
    ///
    /// 启用时，覆盖层将固定在顶部（min_y位置）；禁用时，覆盖层将跟随目标元素的Y坐标。
    ///
    /// # 参数
    ///
    /// - `align` - `true` 表示顶部对齐，`false` 表示跟随目标元素
    ///
    /// # 返回值
    ///
    /// 返回修改后的 `SideOverlay` 实例（builder 模式）
    pub fn align_y_start(mut self, align: bool) -> Self {
        self.align_y_start = align;
        self
    }

    /// 设置点击外部时的关闭消息
    ///
    /// 当用户点击覆盖层外部且不在目标元素内时，将发送此消息。
    ///
    /// # 参数
    ///
    /// - `msg` - 关闭时触发的消息
    ///
    /// # 返回值
    ///
    /// 返回修改后的 `SideOverlay` 实例（builder 模式）
    pub fn on_close(mut self, msg: Message) -> Self {
        self.on_close = Some(msg);
        self
    }

    /// 设置是否将覆盖层吸附在视口内
    ///
    /// 启用时，覆盖层位置将被限制在视口范围内，避免超出屏幕边界。
    /// 禁用时，覆盖层可能显示在视口外部。
    ///
    /// # 参数
    ///
    /// - `snap` - `true` 表示启用吸附，`false` 表示允许超出视口
    ///
    /// # 返回值
    ///
    /// 返回修改后的 `SideOverlay` 实例（builder 模式）
    pub fn snap_within_viewport(mut self, snap: bool) -> Self {
        self.snap_within_viewport = snap;
        self
    }
}

/// 为 `SideOverlay` 实现 `Widget` trait
///
/// 该实现处理主内容的布局、绘制和事件处理，
/// 并在需要时创建侧边覆盖层的 overlay 元素。
impl<Message, ThemeT, RendererT> Widget<Message, ThemeT, RendererT>
    for SideOverlay<'_, Message, ThemeT, RendererT>
where
    RendererT: iced::advanced::Renderer,
    Message: Clone,
{
    /// 获取组件的子元素树
    ///
    /// 返回包含主内容和覆盖层两个子元素的树结构。
    fn children(&self) -> Vec<widget::Tree> {
        vec![widget::Tree::new(&self.content), widget::Tree::new(&self.overlay)]
    }

    /// 比较并更新组件树的差异
    ///
    /// 用于优化更新性能，仅更新发生变化的部分。
    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(&[self.content.as_widget(), self.overlay.as_widget()]);
    }

    /// 获取组件的尺寸
    ///
    /// 返回主内容元素的尺寸。
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    /// 获取组件的尺寸提示
    ///
    /// 返回主内容元素的尺寸提示。
    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    /// 执行组件的布局计算
    ///
    /// # 参数
    ///
    /// - `tree` - 组件树，用于存储布局状态
    /// - `renderer` - 渲染器引用
    /// - `limits` - 布局限制条件
    ///
    /// # 返回值
    ///
    /// 返回主内容的布局节点
    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &RendererT,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }

    /// 绘制组件
    ///
    /// # 参数
    ///
    /// - `tree` - 组件树
    /// - `renderer` - 可变渲染器引用
    /// - `theme` - 主题引用
    /// - `style` - 渲染器样式
    /// - `layout` - 布局信息
    /// - `cursor` - 鼠标光标状态
    /// - `viewport` - 视口矩形
    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut RendererT,
        theme: &ThemeT,
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

    /// 处理组件更新
    ///
    /// # 参数
    ///
    /// - `tree` - 可变组件树
    /// - `event` - 事件引用
    /// - `layout` - 布局信息
    /// - `cursor` - 鼠标光标状态
    /// - `renderer` - 渲染器引用
    /// - `clipboard` - 剪贴板引用
    /// - `shell` - Shell 用于发布消息
    /// - `viewport` - 视口矩形
    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &RendererT,
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

    /// 获取鼠标交互状态
    ///
    /// # 参数
    ///
    /// - `tree` - 组件树
    /// - `layout` - 布局信息
    /// - `cursor` - 鼠标光标状态
    /// - `viewport` - 视口矩形
    /// - `renderer` - 渲染器引用
    ///
    /// # 返回值
    ///
    /// 返回主内容的鼠标交互状态
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

    /// 创建覆盖层元素
    ///
    /// 这是实现侧边覆盖层的核心方法。当 `show` 为 true 时，
    /// 会创建一个 `SideOverlayElement` 作为 overlay 元素。
    ///
    /// # 参数
    ///
    /// - `tree` - 可变组件树
    /// - `layout` - 主内容的布局信息
    /// - `renderer` - 渲染器引用
    /// - `viewport` - 视口矩形
    /// - `translation` - 位置偏移向量
    ///
    /// # 返回值
    ///
    /// 如果有覆盖层需要显示，返回 `Some(overlay::Element)`；否则返回 `None`
    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut widget::Tree,
        layout: Layout<'b>,
        renderer: &RendererT,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, ThemeT, RendererT>> {
        // 获取子元素树的迭代器
        let mut children = tree.children.iter_mut();

        // 获取主内容的 overlay（如果有的话）
        let content = self.content.as_widget_mut().overlay(
            children.next().unwrap(),
            layout,
            renderer,
            viewport,
            translation,
        );

        // 如果启用了显示，创建侧边覆盖层元素
        let side = if self.show {
            Some(overlay::Element::new(Box::new(SideOverlayElement {
                position: layout.position() + translation,
                target_bounds: layout.bounds(),
                viewport: *viewport,
                gap: self.gap,
                min_x: self.min_x,
                min_y: self.min_y,
                snap_within_viewport: self.snap_within_viewport,
                align_y_start: self.align_y_start,
                overlay: &mut self.overlay,
                tree: children.next().unwrap(),
                on_close: self.on_close.clone(),
            })))
        } else {
            None
        };

        // 如果有任何 overlay 需要显示，将它们组合成一个 overlay 组
        if content.is_some() || side.is_some() {
            Some(overlay::Group::with_children(content.into_iter().chain(side).collect()).overlay())
        } else {
            None
        }
    }
}

/// 为 `SideOverlay` 实现 `From` trait，允许直接转换为 `Element`
///
/// 这使得 `SideOverlay` 可以直接用作 `Element` 类型，
/// 方便在需要 `Element` 的地方使用。
impl<'a, Message, ThemeT, RendererT> From<SideOverlay<'a, Message, ThemeT, RendererT>>
    for Element<'a, Message, ThemeT, RendererT>
where
    Message: 'a + Clone,
    RendererT: iced::advanced::Renderer + 'a,
    ThemeT: 'a,
{
    /// 将 `SideOverlay` 转换为 `Element`
    ///
    /// # 参数
    ///
    /// - `widget` - 要转换的 `SideOverlay` 实例
    ///
    /// # 返回值
    ///
    /// 返回包含该组件的 `Element`
    fn from(widget: SideOverlay<'a, Message, ThemeT, RendererT>) -> Self {
        Element::new(widget)
    }
}

/// 侧边覆盖层元素
///
/// 这是实际渲染在侧边的覆盖层元素，实现了 `overlay::Overlay` trait。
/// 负责计算位置、处理事件和绘制覆盖层内容。
///
/// # 生命周期
///
/// - `'a` - 覆盖层内容的生命周期
/// - `'b` - 对外部数据（如组件树）的引用的生命周期
struct SideOverlayElement<'a, 'b, Message, ThemeT, RendererT> {
    /// 覆盖层的基准位置（目标元素位置 + 偏移）
    position: Point,
    /// 目标元素的边界矩形
    target_bounds: Rectangle,
    /// 视口矩形
    viewport: Rectangle,
    /// 覆盖层与目标元素之间的间距
    gap: f32,
    /// 覆盖层最小X坐标限制
    min_x: f32,
    /// 覆盖层最小Y坐标限制
    min_y: f32,
    /// 是否将覆盖层吸附在视口内
    snap_within_viewport: bool,
    /// 是否将覆盖层对齐到顶部
    align_y_start: bool,
    /// 覆盖层内容的可变引用
    overlay: &'b mut Element<'a, Message, ThemeT, RendererT>,
    /// 覆盖层的组件树
    tree: &'b mut widget::Tree,
    /// 点击外部时的关闭消息
    on_close: Option<Message>,
}

/// 为 `SideOverlayElement` 实现 `overlay::Overlay` trait
///
/// 该实现负责：
/// - 计算覆盖层的布局位置（考虑边界、吸附和对齐）
/// - 处理鼠标事件（包括点击外部关闭）
/// - 绘制覆盖层内容
impl<Message, ThemeT, RendererT> overlay::Overlay<Message, ThemeT, RendererT>
    for SideOverlayElement<'_, '_, Message, ThemeT, RendererT>
where
    RendererT: iced::advanced::Renderer,
    Message: Clone,
{
    /// 执行覆盖层的布局计算
    ///
    /// 计算覆盖层的最佳位置和尺寸。位置计算考虑以下因素：
    /// 1. 覆盖层在目标元素右侧，中间有 gap 间距
    /// 2. 根据 `align_y_start` 决定是顶部对齐还是跟随目标元素
    /// 3. 如果 `snap_within_viewport` 为 true，将位置限制在视口内
    ///
    /// # 参数
    ///
    /// - `renderer` - 渲染器引用
    /// - `bounds` - 可用的边界尺寸
    ///
    /// # 返回值
    ///
    /// 返回布局节点，包含计算后的位置和尺寸
    fn layout(&mut self, renderer: &RendererT, bounds: Size) -> layout::Node {
        let viewport = Rectangle::with_size(bounds);

        // 如果启用视口内吸附，布局时使用两侧更大的可用宽度；否则使用整个视口宽度。
        let max_w = side_overlay_max_width(
            bounds,
            self.position,
            self.target_bounds,
            self.gap,
            self.snap_within_viewport,
        );

        // 使用计算的最大宽度进行布局
        let node = self.overlay.as_widget_mut().layout(
            self.tree,
            renderer,
            &layout::Limits::new(Size::ZERO, Size::new(max_w, viewport.height)),
        );

        let size = node.size();

        let position = compute_side_overlay_position(
            self.position,
            self.target_bounds,
            self.viewport,
            size,
            self.gap,
            self.min_x,
            self.min_y,
            self.snap_within_viewport,
            self.align_y_start,
        );

        // 将布局节点移动到计算的位置
        node.move_to(position)
    }

    /// 处理覆盖层的事件更新
    ///
    /// 主要处理：
    /// 1. 点击覆盖层外部时的关闭逻辑
    /// 2. 将事件转发给覆盖层内容
    /// 3. 捕获覆盖层内的鼠标事件，防止穿透到下层
    ///
    /// # 参数
    ///
    /// - `event` - 事件引用
    /// - `layout` - 覆盖层的布局信息
    /// - `cursor` - 鼠标光标状态
    /// - `renderer` - 渲染器引用
    /// - `clipboard` - 剪贴板引用
    /// - `shell` - Shell 用于发布消息
    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &RendererT,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        let bounds = layout.bounds();

        // 处理点击外部关闭的逻辑
        // 条件：鼠标左键按下 && 有关闭消息 && 光标在目标元素外 && 光标在覆盖层外
        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event
            && let Some(on_close) = &self.on_close
            && let Some(cursor_position) = cursor.position()
            && !self.target_bounds.contains(cursor_position)
        {
            // 如果点击不在覆盖层内，触发关闭消息
            if !bounds.contains(cursor_position) {
                shell.publish(on_close.clone());
                shell.capture_event();
            }
        }

        // 将事件转发给覆盖层内容处理
        self.overlay
            .as_widget_mut()
            .update(self.tree, event, layout, cursor, renderer, clipboard, shell, &bounds);

        // 如果是鼠标事件且光标在覆盖层内，捕获事件防止穿透
        if matches!(event, Event::Mouse(_))
            && let Some(cursor_position) = cursor.position()
            && bounds.contains(cursor_position)
        {
            shell.capture_event();
        }
    }

    /// 获取鼠标交互状态
    ///
    /// # 参数
    ///
    /// - `layout` - 覆盖层的布局信息
    /// - `cursor` - 鼠标光标状态
    /// - `renderer` - 渲染器引用
    ///
    /// # 返回值
    ///
    /// 返回覆盖层内容的鼠标交互状态
    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &RendererT,
    ) -> mouse::Interaction {
        let bounds = layout.bounds();
        self.overlay.as_widget().mouse_interaction(self.tree, layout, cursor, &bounds, renderer)
    }

    /// 绘制覆盖层
    ///
    /// # 参数
    ///
    /// - `renderer` - 可变渲染器引用
    /// - `theme` - 主题引用
    /// - `defaults` - 渲染器默认样式
    /// - `layout` - 覆盖层的布局信息
    /// - `cursor` - 鼠标光标状态
    fn draw(
        &self,
        renderer: &mut RendererT,
        theme: &ThemeT,
        defaults: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        let bounds = layout.bounds();

        // 绘制覆盖层内容
        self.overlay
            .as_widget()
            .draw(self.tree, renderer, theme, defaults, layout, cursor, &bounds);
    }
}

#[cfg(test)]
#[path = "side_tests.rs"]
mod tests;
