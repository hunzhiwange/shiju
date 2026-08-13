//! # 自定义小组件模块
//!
//! 本模块提供扩展的自定义小组件，用于增强 Iced UI 框架的功能。
//!
//! ## 主要组件
//!
//! - [`RightClickArea`] - 右键点击区域组件，为任意元素添加右键点击支持

use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, renderer, widget};
use iced::{Element, Event, Length, Point, Rectangle, Size, Theme};

/// 右键点击区域组件
///
/// 该组件包装任意 Iced 元素，为其添加右键点击事件处理能力。
/// 当用户在组件区域内点击鼠标右键时，会触发指定的回调函数并传递点击位置。
///
/// # 类型参数
///
/// - `'a` - 元素的生命周期
/// - `Message` - 应用程序消息类型
/// - `ThemeT` - 主题类型，默认为 [`Theme`]
/// - `RendererT` - 渲染器类型，默认为 [`iced::Renderer`]
///
/// # 示例
///
/// ```ignore
/// use iced::widget::text;
/// use crate::app::components::widgets::RightClickArea;
///
/// let content = text("右键点击我");
/// let right_click_area = RightClickArea::new(
///     content.into(),
///     Box::new(|point| Message::ShowContextMenu(point)),
/// );
/// ```
pub struct RightClickArea<'a, Message, ThemeT = Theme, RendererT = iced::Renderer> {
    /// 被包装的内容元素
    content: Element<'a, Message, ThemeT, RendererT>,
    /// 右键点击回调函数，接收相对于组件边界的本地坐标
    on_right_click: Box<dyn Fn(Point) -> Message + 'a>,
    /// 是否在右键点击时拦截子组件事件，以保留内部状态（如文本选区）
    consume_right_click: bool,
    /// 是否将右键位置作为窗口坐标传给回调，而不是组件内部坐标。
    use_global_position: bool,
}

#[derive(Default)]
struct RightClickAreaState {
    /// `Scrollable` 会把传给子控件的 cursor 转换为内容坐标；事件中的位置仍是窗口坐标。
    last_window_position: Option<Point>,
}

impl<'a, Message, ThemeT, RendererT> RightClickArea<'a, Message, ThemeT, RendererT> {
    /// 创建新的右键点击区域
    ///
    /// # 参数
    ///
    /// - `content` - 要包装的内容元素
    /// - `on_right_click` - 右键点击时的回调函数，参数为相对于组件左上角的本地坐标点
    ///
    /// # 返回值
    ///
    /// 返回新创建的 [`RightClickArea`] 实例
    pub fn new(
        content: Element<'a, Message, ThemeT, RendererT>,
        on_right_click: Box<dyn Fn(Point) -> Message + 'a>,
    ) -> Self {
        Self {
            content,
            on_right_click,
            consume_right_click: false,
            use_global_position: false,
        }
    }

    pub fn preserve_on_right_click(mut self) -> Self {
        self.consume_right_click = true;
        self
    }

    /// 让回调接收相对于窗口的点击位置。
    ///
    /// 默认回调参数是相对于当前组件左上角的本地坐标，适合将菜单锚定到
    /// 当前组件。跨越滚动容器或由外层统一渲染的菜单则应使用窗口坐标。
    pub fn use_global_position(mut self) -> Self {
        self.use_global_position = true;
        self
    }
}

impl<'a, Message, ThemeT, RendererT> Widget<Message, ThemeT, RendererT>
    for RightClickArea<'a, Message, ThemeT, RendererT>
where
    RendererT: iced::advanced::Renderer,
{
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<RightClickAreaState>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(RightClickAreaState::default())
    }

    /// 获取子组件树
    ///
    /// 返回包含内容元素的组件树向量，用于 Iced 的组件树管理
    fn children(&self) -> Vec<widget::Tree> {
        vec![widget::Tree::new(&self.content)]
    }

    /// 差异比较与更新
    ///
    /// 用于高效更新组件树，比较新旧内容元素的差异
    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(&[&self.content]);
    }

    /// 获取组件的尺寸需求
    ///
    /// 委托给内部内容元素的尺寸方法
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    /// 执行布局计算
    ///
    /// 根据给定的限制条件计算内容元素的布局节点
    ///
    /// # 参数
    ///
    /// - `tree` - 组件树，用于存储布局状态
    /// - `renderer` - 渲染器引用
    /// - `limits` - 布局限制条件
    ///
    /// # 返回值
    ///
    /// 返回计算后的布局节点
    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &RendererT,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut widget::Tree,
        layout: Layout<'_>,
        renderer: &RendererT,
        operation: &mut dyn widget::Operation,
    ) {
        self.content.as_widget_mut().operate(
            &mut tree.children[0],
            layout,
            renderer,
            operation,
        );
    }

    /// 绘制组件
    ///
    /// 将绘制操作委托给内部内容元素
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

    /// 获取尺寸提示
    ///
    /// 委托给内部内容元素的尺寸提示方法
    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    /// 处理事件并更新状态
    ///
    /// 这是核心方法，负责检测右键点击事件并触发回调。
    /// 同时将事件传递给内部内容元素进行处理。
    ///
    /// # 右键点击检测逻辑
    ///
    /// 1. 检查事件是否为鼠标右键按下
    /// 2. 获取鼠标光标位置
    /// 3. 验证光标是否在组件边界内
    /// 4. 计算相对于组件左上角的本地坐标
    /// 5. 发布右键点击消息
    ///
    /// # 参数
    ///
    /// - `tree` - 可变组件树
    /// - `event` - 待处理的事件
    /// - `layout` - 布局信息
    /// - `cursor` - 鼠标光标状态
    /// - `renderer` - 渲染器引用
    /// - `clipboard` - 剪贴板引用
    /// - `shell` - 消息壳，用于发布新消息
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
        let state = tree.state.downcast_mut::<RightClickAreaState>();
        if let Event::Mouse(mouse::Event::CursorMoved { position }) = event {
            state.last_window_position = Some(*position);
        }

        // 检测右键点击事件：
        // 1. 事件类型为鼠标按钮按下
        // 2. 按下的是右键
        // 3. 鼠标光标存在位置信息
        // 4. 光标位置在组件边界内
        let is_right_click_inside = if let Event::Mouse(mouse::Event::ButtonPressed(button)) = event
        {
            matches!(button, mouse::Button::Right)
                && cursor.position().is_some_and(|pos| layout.bounds().contains(pos))
        } else {
            false
        };

        // 将事件继续传递给内部内容元素处理
        if !(self.consume_right_click && is_right_click_inside) {
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

        if is_right_click_inside && let Some(pos) = cursor.position() {
            let bounds = layout.bounds();
            let local = Point::new(pos.x - bounds.x, pos.y - bounds.y);
            let click_position = if self.use_global_position {
                state.last_window_position.unwrap_or(pos)
            } else {
                local
            };
            shell.publish((self.on_right_click)(click_position));
        }
    }

    /// 获取鼠标交互状态
    ///
    /// 委托给内部内容元素的鼠标交互方法
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
    /// 返回鼠标交互类型（如指针、手型等）
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

    /// 获取覆盖层元素
    ///
    /// 委托给内部内容元素的覆盖层方法，用于处理弹出菜单、下拉框等覆盖层 UI
    ///
    /// # 参数
    ///
    /// - `tree` - 可变组件树
    /// - `layout` - 布局信息
    /// - `renderer` - 渲染器引用
    /// - `viewport` - 视口矩形
    /// - `translation` - 平移向量
    ///
    /// # 返回值
    ///
    /// 返回可选的覆盖层元素
    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut widget::Tree,
        layout: Layout<'b>,
        renderer: &RendererT,
        viewport: &Rectangle,
        translation: iced::Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Message, ThemeT, RendererT>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

/// 将 [`RightClickArea`] 转换为 [`Element`]
///
/// 该实现允许将 `RightClickArea` 直接用作 Iced 元素，无需手动包装
impl<'a, Message, ThemeT, RendererT> From<RightClickArea<'a, Message, ThemeT, RendererT>>
    for Element<'a, Message, ThemeT, RendererT>
where
    Message: 'a,
    ThemeT: 'a,
    RendererT: 'a + iced::advanced::Renderer,
{
    /// 执行转换
    ///
    /// # 参数
    ///
    /// - `area` - 要转换的右键点击区域
    ///
    /// # 返回值
    ///
    /// 返回包装后的 Iced 元素
    fn from(area: RightClickArea<'a, Message, ThemeT, RendererT>) -> Self {
        Element::new(area)
    }
}
