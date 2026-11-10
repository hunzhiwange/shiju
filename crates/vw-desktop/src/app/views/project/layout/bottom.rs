//! 底部面板布局模块
//!
//! 本模块负责构建项目视图中的底部面板UI组件。底部面板主要用于显示可调整大小的终端面板，
//! 并提供垂直调整手柄供用户拖动调整面板高度。
//!
//! # 主要功能
//!
//! - 根据终端可见性状态动态渲染底部面板
//! - 集成终端面板组件和垂直调整手柄
//! - 支持通过拖动手柄调整终端面板高度
//!
//! # 布局结构
//!
//! 当终端可见时：
//! ```text
//! +---------------------------+
//! |  垂直调整手柄 (可拖动)    |
//! +---------------------------+
//! |                           |
//! |     终端面板内容          |
//! |                           |
//! +---------------------------+
//! ```

use iced::widget::{Space, column, container, mouse_area};
use iced::{Element, Length};

use crate::app::components::terminal_panel;
use crate::app::{App, Message, message};

use super::super::handles::VResizeHandle;
use super::super::styles::panel_style_no_border;

/// 构建底部面板UI元素
///
/// 该函数根据应用状态创建底部面板的完整UI结构。当终端面板可见时，
/// 返回包含调整手柄和终端内容的垂直布局；当终端不可见时，返回空白区域。
///
/// # 参数
///
/// * `app` - 应用状态引用，包含终端配置和可见性信息
///
/// # 返回值
///
/// 返回一个 `Element<Message>` 类型的UI元素，表示渲染后的底部面板
///
/// # 示例
///
/// ```rust,ignore
/// let panel = bottom_panel(&app);
/// // panel 可以直接用于 iced 的视图函数中
/// ```
///
/// # 行为说明
///
/// - **终端可见**：创建包含垂直分隔器和终端面板的垂直布局
///   - 垂直分隔器高度：由 `VResizeHandle::HIT_HEIGHT` 决定
///   - 终端面板高度：由 `app.terminal.height` 决定（最小为0）
/// - **终端隐藏**：返回一个空白的 `Space` 组件，占据全部可用宽度
///
/// # 消息处理
///
/// 当用户按下垂直调整手柄时，会发送 `TerminalMessage::DragStarted` 消息，
/// 用于启动拖动调整终端高度的操作。
pub fn bottom_panel(app: &App) -> Element<'_, Message> {
    // 检查终端面板是否可见
    if app.terminal.is_visible {
        // 确保终端高度不为负数，最小为0.0
        let terminal_h = app.terminal.height.max(0.0);

        // 获取垂直调整手柄的点击区域高度
        let handle_h = VResizeHandle::HIT_HEIGHT;
        let content_h = terminal_content_height(terminal_h, handle_h);

        // 创建可拖动的垂直调整手柄
        // 当用户按下时，触发终端面板拖动开始消息
        let vdivider = mouse_area(VResizeHandle)
            .on_press(Message::Terminal(message::TerminalMessage::DragStarted));

        // 构建终端面板容器
        // 使用无边框样式，填充全部可用空间
        let terminal_content = container(terminal_panel::view(app))
            .width(Length::Fill)
            .height(Length::Fixed(content_h))
            .style(panel_style_no_border)
            .padding(0);

        // 组合垂直布局：调整手柄在上，终端内容在下
        let terminal = column![
            // 顶部：固定高度的垂直调整手柄
            container(vdivider).width(Length::Fill).height(Length::Fixed(handle_h)),
            // 底部：自适应高度的终端内容区域
            terminal_content
        ]
        .spacing(0)
        .width(Length::Fill)
        .height(Length::Fill);

        // 返回固定高度的完整终端面板
        column![terminal].width(Length::Fill).height(Length::Fixed(terminal_h)).into()
    } else {
        // 终端不可见时，返回空白区域以保持布局一致性
        container(Space::new()).width(Length::Fill).into()
    }
}

pub(super) fn fullscreen_terminal(app: &App) -> Element<'_, Message> {
    container(terminal_panel::view(app))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(panel_style_no_border)
        .into()
}

fn terminal_content_height(terminal_h: f32, handle_h: f32) -> f32 {
    (terminal_h.max(0.0) - handle_h.max(0.0)).max(0.0)
}

#[cfg(test)]
#[path = "bottom_tests.rs"]
mod bottom_tests;
