//! 拖放区域组件模块
//!
//! 本模块提供了 `DropArea` 组件，用于在 Iced 应用程序中实现拖放交互功能。
//! 该组件可以包装任意现有的 Iced 元素，为其添加拖放事件处理能力。
//!
//! # 主要功能
//!
//! - **拖放检测**：当用户在区域内释放鼠标左键时，触发 `on_drop` 消息
//! - **悬停追踪**：可选地追踪鼠标悬停状态，在悬停状态变化时发送相应消息
//! - **内容包装**：完全代理内部内容元素的布局、绘制和交互行为
//!
//! # 使用示例
//!
//! ```ignore
//! use crate::app::components::input_panel::drop_area::DropArea;
//!
//! // 创建一个带有拖放功能的区域
//! let content = text("将文件拖到这里").into();
//! let drop_area = DropArea::new(
//!     content,
//!     Message::FileDropped,
//!     Some((Message::DragEntered, Message::DragLeft)),
//!     true, // 启用悬停追踪
//! );
//! ```

use crate::app::Message;
use iced::advanced::{Clipboard, Layout, Shell, Widget, mouse, widget};
use iced::{Event, Rectangle};

/// 拖放区域组件
///
/// `DropArea` 是一个包装器组件，为其内部内容添加拖放交互能力。
/// 当用户在区域内释放鼠标按钮时，会发送指定的消息。
///
/// # 类型参数
///
/// - `'a`：元素的生命周期
/// - `MessageT`：组件发送的消息类型，必须实现 `Clone`
/// - `ThemeT`：主题类型，默认为 `iced::Theme`
/// - `RendererT`：渲染器类型，默认为 `iced::Renderer`
///
/// # 字段说明
///
/// - `content`：被包装的内部内容元素
/// - `on_drop`：当拖放操作完成（鼠标释放）时发送的消息
/// - `on_hover_changed`：可选的悬停状态变化消息元组 `(悬停时消息, 离开时消息)`
/// - `track_hover`：是否启用悬停状态追踪
pub struct DropArea<'a, MessageT, ThemeT = iced::Theme, RendererT = iced::Renderer> {
    /// 被包装的内部内容元素
    content: iced::Element<'a, MessageT, ThemeT, RendererT>,
    /// 拖放完成时发送的消息
    on_drop: MessageT,
    /// 悬停状态变化时的消息对 (悬停消息, 离开消息)
    on_hover_changed: Option<(MessageT, MessageT)>,
    /// 是否追踪悬停状态
    track_hover: bool,
    on_native_drop: Option<Box<dyn Fn(std::path::PathBuf) -> MessageT + 'a>>,
}

impl<'a, MessageT, ThemeT, RendererT> DropArea<'a, MessageT, ThemeT, RendererT> {
    /// 创建新的拖放区域
    ///
    /// # 参数
    ///
    /// - `content`：要包装的内容元素
    /// - `on_drop`：拖放完成时发送的消息
    /// - `on_hover_changed`：可选的悬停状态消息对，格式为 `(进入悬停消息, 离开悬停消息)`
    /// - `track_hover`：是否启用悬停状态追踪
    ///
    /// # 返回值
    ///
    /// 返回新创建的 `DropArea` 实例
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let content = text("拖放区域").into();
    /// let drop_area = DropArea::new(
    ///     content,
    ///     Message::Dropped,
    ///     Some((Message::Hovered, Message::NotHovered)),
    ///     true,
    /// );
    /// ```
    pub fn new(
        content: iced::Element<'a, MessageT, ThemeT, RendererT>,
        on_drop: MessageT,
        on_hover_changed: Option<(MessageT, MessageT)>,
        track_hover: bool,
    ) -> Self {
        Self { content, on_drop, on_hover_changed, track_hover, on_native_drop: None }
    }
}

impl<'a, MessageT, ThemeT, RendererT> DropArea<'a, MessageT, ThemeT, RendererT> {
    /// Preserve the exact native payload independently of hover callbacks.
    pub fn on_native_drop(
        mut self,
        callback: impl Fn(std::path::PathBuf) -> MessageT + 'a,
    ) -> Self {
        self.on_native_drop = Some(Box::new(callback));
        self
    }
}

impl<'a, MessageT, ThemeT, RendererT> Widget<MessageT, ThemeT, RendererT>
    for DropArea<'a, MessageT, ThemeT, RendererT>
where
    RendererT: iced::advanced::Renderer,
    MessageT: Clone,
{
    /// 获取子组件树
    ///
    /// 返回包含内部内容元素的组件树向量，用于 Iced 的组件树管理。
    fn children(&self) -> Vec<widget::Tree> {
        vec![widget::Tree::new(&self.content)]
    }

    /// 差异比较更新
    ///
    /// 当组件状态发生变化时，更新内部内容元素的组件树。
    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(&[&self.content]);
    }

    /// 获取组件尺寸
    ///
    /// 返回内部内容元素的尺寸规格。
    fn size(&self) -> iced::Size<iced::Length> {
        self.content.as_widget().size()
    }

    /// 执行布局计算
    ///
    /// 根据给定的限制条件，计算内部内容元素的布局节点。
    ///
    /// # 参数
    ///
    /// - `tree`：组件树，用于存储布局状态
    /// - `renderer`：渲染器引用
    /// - `limits`：布局限制条件
    ///
    /// # 返回值
    ///
    /// 返回计算后的布局节点
    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &RendererT,
        limits: &iced::advanced::layout::Limits,
    ) -> iced::advanced::layout::Node {
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }

    /// 绘制组件
    ///
    /// 将内部内容元素绘制到渲染器上。
    ///
    /// # 参数
    ///
    /// - `tree`：组件树
    /// - `renderer`：可变渲染器引用
    /// - `theme`：当前主题
    /// - `style`：渲染样式
    /// - `layout`：布局信息
    /// - `cursor`：鼠标光标状态
    /// - `viewport`：视口矩形区域
    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut RendererT,
        theme: &ThemeT,
        style: &iced::advanced::renderer::Style,
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

    /// 处理事件更新
    ///
    /// 处理鼠标事件，检测拖放操作和悬停状态变化。
    ///
    /// # 事件处理逻辑
    ///
    /// 1. **悬停追踪**：如果启用了悬停追踪且配置了悬停消息，当鼠标移动或按钮按下/释放时，
    ///    检测光标是否在组件边界内，并发布相应的悬停状态消息。
    ///
    /// 2. **拖放检测**：当鼠标左键释放且光标位于组件边界内时，发布 `on_drop` 消息。
    ///
    /// 3. **事件传递**：将事件传递给内部内容元素进行处理。
    ///
    /// # 参数
    ///
    /// - `tree`：可变组件树
    /// - `event`：待处理的事件
    /// - `layout`：布局信息
    /// - `cursor`：鼠标光标状态
    /// - `renderer`：渲染器引用
    /// - `clipboard`：剪贴板接口
    /// - `shell`：消息外壳，用于发布消息
    /// - `viewport`：视口矩形区域
    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &RendererT,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, MessageT>,
        viewport: &Rectangle,
    ) {
        // 悬停状态追踪逻辑
        // 仅在启用追踪且有悬停消息配置时执行
        if (self.track_hover || (self.on_native_drop.is_some() && matches!(event, Event::Window(iced::window::Event::FileDropped(_)))))
            && let Some((on_hovered, on_not_hovered)) = &self.on_hover_changed
            // 仅响应鼠标移动和按钮按下/释放事件
            && matches!(
                event,
                Event::Mouse(mouse::Event::CursorMoved { .. })
                    | Event::Mouse(mouse::Event::ButtonPressed(_))
                    | Event::Mouse(mouse::Event::ButtonReleased(_))
                    | Event::Window(iced::window::Event::FileHovered(_))
                    | Event::Window(iced::window::Event::FilesHoveredLeft)
                    | Event::Window(iced::window::Event::FileDropped(_))
            )
        {
            // 检测光标是否在组件边界内
            let hovered = cursor.position().is_some_and(|pos| layout.bounds().contains(pos));
            // 根据悬停状态发布对应的消息
            shell.publish(if hovered { on_hovered.clone() } else { on_not_hovered.clone() });
        }

        // 拖放完成检测逻辑
        // 检测鼠标左键释放事件
        if matches!(
            event,
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                | Event::Window(iced::window::Event::FileDropped(_))
        )
            // 获取光标位置
            && let Some(pos) = cursor.position()
            // 检测光标是否在组件边界内
            && layout.bounds().contains(pos)
        {
            // 发布拖放完成消息
            if let Event::Window(iced::window::Event::FileDropped(path)) = event
                && let Some(callback) = &self.on_native_drop
            {
                shell.publish(callback(path.clone()));
                shell.capture_event();
            } else {
                shell.publish(self.on_drop.clone());
            }
        }

        // 将事件传递给内部内容元素处理
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
    /// 返回内部内容元素的鼠标交互状态（如光标样式）。
    ///
    /// # 参数
    ///
    /// - `tree`：组件树
    /// - `layout`：布局信息
    /// - `cursor`：鼠标光标状态
    /// - `viewport`：视口矩形区域
    /// - `renderer`：渲染器引用
    ///
    /// # 返回值
    ///
    /// 返回鼠标交互类型（如指针、文本选择等）
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
    /// 返回内部内容元素的覆盖层（如弹出菜单、下拉框等）。
    ///
    /// # 参数
    ///
    /// - `tree`：可变组件树
    /// - `layout`：布局信息
    /// - `renderer`：渲染器引用
    /// - `viewport`：视口矩形区域
    /// - `translation`：平移向量
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
    ) -> Option<iced::advanced::overlay::Element<'b, MessageT, ThemeT, RendererT>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

/// 拖放区域元素类型别名
///
/// 使用应用程序的 `Message` 类型作为消息类型的 `DropArea` 便捷别名。
/// 这是本模块中最常用的类型形式。
#[allow(dead_code)]
pub type DropAreaElement<'a> = DropArea<'a, Message>;
