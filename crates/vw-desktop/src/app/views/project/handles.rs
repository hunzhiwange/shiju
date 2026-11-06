//! 项目视图调整手柄和边框组件
//!
//! 本模块提供了一组用于界面调整和边框装饰的自定义 Widget 组件。
//! 这些组件主要用于实现可调整大小的面板分割线和各种装饰性边框。
//!
//! # 主要组件
//!
//! - [`HResizeHandle`]: 水平方向的调整大小手柄，用于调整垂直分割线的位置
//! - [`VResizeHandle`]: 垂直方向的调整大小手柄，用于调整水平分割线的位置
//! - [`TopBorderCover`]: 顶部边框覆盖组件，用于绘制顶部分割线
//! - [`SessionPanelRightBorder`]: 会话面板右边框装饰组件
//! - [`SessionPanelLeftBorder`]: 会话面板左边框装饰组件
//!
//! # 设计特性
//!
//! - 所有组件都支持主题感知，能够根据深色/浅色主题自动调整颜色
//! - 调整手柄具有更大的交互区域（hit area），便于用户操作
//! - 边框装饰组件使用统一的分割线样式，保持界面一致性

use iced::advanced::{Layout, Widget, layout, mouse, renderer, widget::Tree};
use iced::border::{Border, Radius};
use iced::{Background, Color, Element as IcedElement, Length, Rectangle, Size, Theme};

/// 判断当前主题是否为深色主题
///
/// 通过计算主题背景色的 RGB 分量和来判断主题类型。
/// 当 RGB 分量和小于 1.5 时，认为是深色主题。
///
/// # 参数
///
/// * `theme` - iced 主题引用
///
/// # 返回值
///
/// 如果是深色主题返回 `true`，否则返回 `false`
fn is_dark_theme(theme: &Theme) -> bool {
    let palette = theme.palette();
    palette.background.r + palette.background.g + palette.background.b < 1.5
}

/// 获取分割线颜色
///
/// 根据当前主题返回对应的分割线颜色。
/// - 深色主题: 使用 RGB(60, 60, 60) 的深灰色
/// - 浅色主题: 使用 RGB(226, 226, 226) 的浅灰色
///
/// # 参数
///
/// * `theme` - iced 主题引用
///
/// # 返回值
///
/// 返回适合当前主题的分割线颜色
fn divider_line_color(theme: &Theme) -> Color {
    let is_dark = is_dark_theme(theme);
    if is_dark { Color::from_rgb8(60, 60, 60) } else { Color::from_rgba8(226, 226, 226, 1.0) }
}

/// 水平调整大小手柄
///
/// 用于创建可拖拽的水平调整手柄，允许用户通过拖拽来调整垂直分割线的位置。
/// 该组件在视觉上表现为一条垂直分割线，但具有更大的交互区域以便于操作。
///
/// # 特性
///
/// - 视觉宽度: 0.5 像素（细线）
/// - 交互宽度: 6.0 像素（便于点击和拖拽）
/// - 鼠标悬停时显示水平调整光标
/// - 自动适应主题颜色
///
/// # 示例
///
/// ```rust,ignore
/// use crate::app::views::project::handles::HResizeHandle;
///
/// let handle = HResizeHandle::new();
/// // 在 iced 布局中使用
/// row![HResizeHandle].into()
/// ```
pub(crate) struct HResizeHandle;

impl HResizeHandle {
    /// 交互区域的宽度（像素）
    ///
    /// 设置为 6.0 像素以提供更好的用户体验，
    /// 让用户更容易点击和拖拽调整手柄。
    pub(crate) const HIT_WIDTH: f32 = 6.0;
}

impl<Message, Renderer> Widget<Message, Theme, Renderer> for HResizeHandle
where
    Renderer: iced::advanced::Renderer,
{
    /// 返回组件的尺寸约束
    ///
    /// 宽度固定为交互宽度，高度填充父容器。
    fn size(&self) -> Size<Length> {
        Size { width: Length::Fixed(Self::HIT_WIDTH), height: Length::Fill }
    }

    /// 计算组件的布局
    ///
    /// 根据父容器的限制条件计算组件的实际尺寸。
    ///
    /// # 参数
    ///
    /// * `_tree` - 组件树（未使用）
    /// * `_renderer` - 渲染器（未使用）
    /// * `limits` - 布局限制条件
    ///
    /// # 返回值
    ///
    /// 返回计算后的布局节点
    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let size = limits.resolve(
            Length::Fixed(Self::HIT_WIDTH),
            Length::Fill,
            Size::new(Self::HIT_WIDTH, 0.0),
        );
        layout::Node::new(size)
    }

    /// 绘制组件
    ///
    /// 绘制调整手柄的视觉效果，包括：
    /// 1. 背景填充（使用主题背景色）
    /// 2. 右侧分割线
    /// 3. 顶部和底部的横向线条（用于视觉装饰）
    ///
    /// # 参数
    ///
    /// * `_state` - 组件状态树（未使用）
    /// * `renderer` - 渲染器，用于绘制图形
    /// * `theme` - 当前主题，用于获取颜色
    /// * `_style` - 渲染样式（未使用）
    /// * `layout` - 布局信息
    /// * `_cursor` - 鼠标光标位置（未使用）
    /// * `_viewport` - 视口区域（未使用）
    fn draw(
        &self,
        _state: &Tree,
        _renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
    }

    /// 处理鼠标交互
    ///
    /// 当鼠标位于手柄区域时，显示水平调整光标，
    /// 提示用户可以拖拽调整大小。
    ///
    /// # 参数
    ///
    /// * `_state` - 组件状态树（未使用）
    /// * `layout` - 布局信息
    /// * `cursor` - 鼠标光标位置
    /// * `_viewport` - 视口区域（未使用）
    /// * `_renderer` - 渲染器（未使用）
    ///
    /// # 返回值
    ///
    /// 返回适当的鼠标交互类型
    fn mouse_interaction(
        &self,
        _state: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if cursor.is_over(layout.bounds()) {
            mouse::Interaction::ResizingHorizontally
        } else {
            mouse::Interaction::Idle
        }
    }
}

/// 实现 From trait，允许将 HResizeHandle 转换为 Element
impl<'a, Message, Renderer> From<HResizeHandle> for IcedElement<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    fn from(widget: HResizeHandle) -> Self {
        Self::new(widget)
    }
}

/// 垂直调整大小手柄
///
/// 用于创建可拖拽的垂直调整手柄，允许用户通过拖拽来调整水平分割线的位置。
/// 该组件在视觉上表现为一条水平分割线，但具有更大的交互区域以便于操作。
///
/// # 特性
///
/// - 视觉高度: 1.0 像素（细线）
/// - 交互高度: 8.0 像素（便于点击和拖拽）
/// - 鼠标悬停时显示垂直调整光标
/// - 自动适应主题颜色
///
/// # 示例
///
/// ```rust,ignore
/// use crate::app::views::project::handles::VResizeHandle;
///
/// let handle = VResizeHandle::new();
/// // 在 iced 布局中使用
/// column![VResizeHandle].into()
/// ```
pub(crate) struct VResizeHandle;

impl VResizeHandle {
    /// 交互区域的高度（像素）
    ///
    /// 使用 8.0 像素命中高度，分隔细线居中绘制，便于从两侧抓取。
    pub(crate) const HIT_HEIGHT: f32 = 8.0;
}

impl<Message, Renderer> Widget<Message, Theme, Renderer> for VResizeHandle
where
    Renderer: iced::advanced::Renderer,
{
    /// 返回组件的尺寸约束
    ///
    /// 宽度填充父容器，高度固定为交互高度。
    fn size(&self) -> Size<Length> {
        Size { width: Length::Fill, height: Length::Fixed(Self::HIT_HEIGHT) }
    }

    /// 计算组件的布局
    ///
    /// 根据父容器的限制条件计算组件的实际尺寸。
    ///
    /// # 参数
    ///
    /// * `_tree` - 组件树（未使用）
    /// * `_renderer` - 渲染器（未使用）
    /// * `limits` - 布局限制条件
    ///
    /// # 返回值
    ///
    /// 返回计算后的布局节点
    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let size = limits.resolve(
            Length::Fill,
            Length::Fixed(Self::HIT_HEIGHT),
            Size::new(0.0, Self::HIT_HEIGHT),
        );
        layout::Node::new(size)
    }

    /// 绘制组件
    ///
    /// 绘制调整手柄的视觉效果，包括：
    /// 在命中区中央绘制 1px 分割线，悬停时使用主题强调色。
    ///
    /// # 参数
    ///
    /// * `_state` - 组件状态树（未使用）
    /// * `renderer` - 渲染器，用于绘制图形
    /// * `theme` - 当前主题，用于获取颜色
    /// * `_style` - 渲染样式（未使用）
    /// * `layout` - 布局信息
    /// * `cursor` - 鼠标光标位置
    /// * `_viewport` - 视口区域（未使用）
    fn draw(
        &self,
        _state: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let color = if cursor.is_over(bounds) {
            theme.extended_palette().primary.base.color
        } else {
            divider_line_color(theme)
        };
        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle {
                    x: bounds.x,
                    y: bounds.y + (bounds.height - 1.0).max(0.0) / 2.0,
                    width: bounds.width,
                    height: bounds.height.min(1.0),
                },
                ..Default::default()
            },
            color,
        );
    }

    /// 处理鼠标交互
    ///
    /// 当鼠标位于手柄区域时，显示垂直调整光标，
    /// 提示用户可以拖拽调整大小。
    ///
    /// # 参数
    ///
    /// * `_state` - 组件状态树（未使用）
    /// * `layout` - 布局信息
    /// * `cursor` - 鼠标光标位置
    /// * `_viewport` - 视口区域（未使用）
    /// * `_renderer` - 渲染器（未使用）
    ///
    /// # 返回值
    ///
    /// 返回适当的鼠标交互类型
    fn mouse_interaction(
        &self,
        _state: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if cursor.is_over(layout.bounds()) {
            mouse::Interaction::ResizingVertically
        } else {
            mouse::Interaction::Idle
        }
    }
}

/// 实现 From trait，允许将 VResizeHandle 转换为 Element
impl<'a, Message, Renderer> From<VResizeHandle> for IcedElement<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    fn from(widget: VResizeHandle) -> Self {
        Self::new(widget)
    }
}

/// 顶部边框覆盖组件
///
/// 用于在组件的顶部绘制一条横向分割线。
/// 这是一个装饰性组件，不提供交互功能。
///
/// # 特性
///
/// - 填充父容器的宽度和高度
/// - 在顶部绘制 1 像素高的分割线
/// - 自动适应主题颜色
///
/// # 使用场景
///
/// 主要用于在需要顶部边框装饰的位置添加分割线，
/// 例如面板之间的视觉分隔。
pub(crate) struct TopBorderCover;

impl<Message, Renderer> Widget<Message, Theme, Renderer> for TopBorderCover
where
    Renderer: iced::advanced::Renderer,
{
    /// 返回组件的尺寸约束
    ///
    /// 宽度和高度都填充父容器。
    fn size(&self) -> Size<Length> {
        Size { width: Length::Fill, height: Length::Fill }
    }

    /// 计算组件的布局
    ///
    /// 根据父容器的限制条件计算组件的实际尺寸。
    ///
    /// # 参数
    ///
    /// * `_tree` - 组件树（未使用）
    /// * `_renderer` - 渲染器（未使用）
    /// * `limits` - 布局限制条件
    ///
    /// # 返回值
    ///
    /// 返回计算后的布局节点
    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let size = limits.resolve(Length::Fill, Length::Fill, Size::new(0.0, 0.0));
        layout::Node::new(size)
    }

    /// 绘制组件
    ///
    /// 在组件的顶部绘制 1 像素高的分割线。
    ///
    /// # 参数
    ///
    /// * `_state` - 组件状态树（未使用）
    /// * `renderer` - 渲染器，用于绘制图形
    /// * `theme` - 当前主题，用于获取颜色
    /// * `_style` - 渲染样式（未使用）
    /// * `layout` - 布局信息
    /// * `_cursor` - 鼠标光标位置（未使用）
    /// * `_viewport` - 视口区域（未使用）
    fn draw(
        &self,
        _state: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();

        // 计算顶部分割线的位置和尺寸
        let top_strip = Rectangle { x: bounds.x, y: bounds.y, width: bounds.width, height: 1.0 };
        let line_color = divider_line_color(theme);

        // 渲染顶部分割线
        renderer.fill_quad(
            renderer::Quad {
                bounds: top_strip,
                border: Border { color: Color::TRANSPARENT, width: 0.0, radius: Radius::from(0.0) },
                ..Default::default()
            },
            Background::Color(line_color),
        );
    }
}

/// 实现 From trait，允许将 TopBorderCover 转换为 Element
impl<'a, Message, Renderer> From<TopBorderCover> for IcedElement<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    fn from(widget: TopBorderCover) -> Self {
        Self::new(widget)
    }
}

/// 会话面板右边框装饰组件
///
/// 用于在会话面板的右侧绘制垂直分割线。
/// 这是一个装饰性组件，不提供交互功能。
///
/// # 特性
///
/// - 填充父容器的宽度和高度
/// - 在右侧绘制 1 像素宽的分割线
/// - 自动适应主题颜色
///
/// # 使用场景
///
/// 主要用于会话面板的右侧边界装饰，
/// 提供视觉上的分隔效果。
pub(crate) struct SessionPanelRightBorder;

impl<Message, Renderer> Widget<Message, Theme, Renderer> for SessionPanelRightBorder
where
    Renderer: iced::advanced::Renderer,
{
    /// 返回组件的尺寸约束
    ///
    /// 宽度和高度都填充父容器。
    fn size(&self) -> Size<Length> {
        Size { width: Length::Fill, height: Length::Fill }
    }

    /// 计算组件的布局
    ///
    /// 根据父容器的限制条件计算组件的实际尺寸。
    ///
    /// # 参数
    ///
    /// * `_tree` - 组件树（未使用）
    /// * `_renderer` - 渲染器（未使用）
    /// * `limits` - 布局限制条件
    ///
    /// # 返回值
    ///
    /// 返回计算后的布局节点
    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let size = limits.resolve(Length::Fill, Length::Fill, Size::new(0.0, 0.0));
        layout::Node::new(size)
    }

    /// 绘制组件
    ///
    /// 在组件的右侧绘制 1 像素宽的垂直分割线。
    ///
    /// # 参数
    ///
    /// * `_state` - 组件状态树（未使用）
    /// * `renderer` - 渲染器，用于绘制图形
    /// * `theme` - 当前主题，用于获取颜色
    /// * `_style` - 渲染样式（未使用）
    /// * `layout` - 布局信息
    /// * `_cursor` - 鼠标光标位置（未使用）
    /// * `_viewport` - 视口区域（未使用）
    fn draw(
        &self,
        _state: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let line_color = divider_line_color(theme);

        // 计算右侧分割线的位置和尺寸
        let right_line = Rectangle {
            x: bounds.x + bounds.width - 1.0,
            y: bounds.y,
            width: 1.0,
            height: bounds.height,
        };

        // 渲染右侧分割线
        renderer.fill_quad(
            renderer::Quad {
                bounds: right_line,
                border: Border { color: Color::TRANSPARENT, width: 0.0, radius: Radius::from(0.0) },
                ..Default::default()
            },
            Background::Color(line_color),
        );
    }
}

/// 实现 From trait，允许将 SessionPanelRightBorder 转换为 Element
impl<'a, Message, Renderer> From<SessionPanelRightBorder>
    for IcedElement<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    fn from(widget: SessionPanelRightBorder) -> Self {
        Self::new(widget)
    }
}

/// 会话面板左边框装饰组件
///
/// 用于在会话面板的左侧绘制垂直分割线。
/// 这是一个装饰性组件，不提供交互功能。
///
/// # 特性
///
/// - 填充父容器的宽度和高度
/// - 在左侧绘制 1 像素宽的分割线
/// - 自动适应主题颜色
///
/// # 使用场景
///
/// 主要用于会话面板的左侧边界装饰，
/// 提供视觉上的分隔效果。
pub(crate) struct SessionPanelLeftBorder;

impl<Message, Renderer> Widget<Message, Theme, Renderer> for SessionPanelLeftBorder
where
    Renderer: iced::advanced::Renderer,
{
    /// 返回组件的尺寸约束
    ///
    /// 宽度和高度都填充父容器。
    fn size(&self) -> Size<Length> {
        Size { width: Length::Fill, height: Length::Fill }
    }

    /// 计算组件的布局
    ///
    /// 根据父容器的限制条件计算组件的实际尺寸。
    ///
    /// # 参数
    ///
    /// * `_tree` - 组件树（未使用）
    /// * `_renderer` - 渲染器（未使用）
    /// * `limits` - 布局限制条件
    ///
    /// # 返回值
    ///
    /// 返回计算后的布局节点
    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let size = limits.resolve(Length::Fill, Length::Fill, Size::new(0.0, 0.0));
        layout::Node::new(size)
    }

    /// 绘制组件
    ///
    /// 在组件的左侧绘制 1 像素宽的垂直分割线。
    ///
    /// # 参数
    ///
    /// * `_state` - 组件状态树（未使用）
    /// * `renderer` - 渲染器，用于绘制图形
    /// * `theme` - 当前主题，用于获取颜色
    /// * `_style` - 渲染样式（未使用）
    /// * `layout` - 布局信息
    /// * `_cursor` - 鼠标光标位置（未使用）
    /// * `_viewport` - 视口区域（未使用）
    fn draw(
        &self,
        _state: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let line_color = divider_line_color(theme);

        // 计算左侧分割线的位置和尺寸
        let left_line = Rectangle { x: bounds.x, y: bounds.y, width: 1.0, height: bounds.height };

        // 渲染左侧分割线
        renderer.fill_quad(
            renderer::Quad {
                bounds: left_line,
                border: Border { color: Color::TRANSPARENT, width: 0.0, radius: Radius::from(0.0) },
                ..Default::default()
            },
            Background::Color(line_color),
        );
    }
}

/// 实现 From trait，允许将 SessionPanelLeftBorder 转换为 Element
impl<'a, Message, Renderer> From<SessionPanelLeftBorder>
    for IcedElement<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    fn from(widget: SessionPanelLeftBorder) -> Self {
        Self::new(widget)
    }
}
#[cfg(test)]
#[path = "handles_tests.rs"]
mod handles_tests;
