//! 项目视图模块
//!
//! 本模块负责渲染应用程序的主项目视图界面。它协调多个子模块来构建完整的 UI 布局，
//! 包括基础布局、拖拽徽章层和会话选择器层。
//!
//! # 主要组件
//!
//! - **基础布局**：左侧导航栏、会话面板和设置面板的主要排列
//! - **拖拽徽章层**：显示拖拽操作状态的浮动徽章
//! - **会话选择器层**：用于创建或选择会话的覆盖层
//!
//! # 子模块
//!
//! - `components`：可复用的 UI 组件
//! - `handles`：事件处理器
//! - `layout`：布局构建逻辑
//! - `styles`：样式定义
//! - `utils`：工具函数

use iced::Element;
use iced::Length;
use iced::widget::{container, stack};

use crate::app::state::WorkspaceMode;
use crate::app::{App, Message};

mod components;
#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) use components::project_list_action_order;
mod handles;
use components::new_session_picker_layer;
mod layout;
pub(crate) use layout::WorkSidebarModel;
mod styles;
use layout::{build_base_layout, build_work_layout, drag_badge_layer};
use styles::workspace_background_style;
pub mod utils;
pub use utils::truncate_display_width;

/// 渲染应用程序的主项目视图
///
/// 此函数构建应用程序的主要 UI 视图，通过组合多个层次来创建完整的界面：
/// 1. **基础布局层**：包含左侧导航栏、会话面板和设置面板
/// 2. **拖拽徽章层**：显示当前拖拽操作的状态
/// 3. **会话选择器层**：提供创建或选择新会话的界面
///
/// # 参数
///
/// - `app`：应用程序状态的不可变引用，包含所有渲染所需的数据
///
/// # 返回值
///
/// 返回一个 `Element<Message>`，这是 Iced 框架中可渲染的 UI 元素
///
/// # 布局尺寸
///
/// - `spacing`：元素之间的间距（0.0 像素）
/// - `content_pad`：内容区域的内边距（6.0 像素）
/// - `left_rail_width`：左侧导航栏的宽度（56.0 像素）
/// - `session_panel_width_scale`：会话面板占窗口宽度的比例（60%）
/// - `corner_radius`：圆角半径（12.0 像素）
/// - `chat_content_pad`：聊天内容的内边距（0.0 像素）
/// - `settings_panel_width`：设置面板宽度（可调整，默认 370.0 像素，范围 200-800 像素）
///
/// # 示例
///
/// ```ignore
/// let app = App::new();
/// let view_element = view(&app);
/// // 将 view_element 传递给 Iced 的运行时进行渲染
/// ```
pub fn view(app: &App) -> Element<'_, Message> {
    // 布局参数定义
    let spacing = 0.0; // 元素之间的间距
    let content_pad = 6.0; // 内容区域的内边距
    let left_rail_width = 56.0; // 左侧导航栏的固定宽度
    let session_panel_width_scale = 0.6; // 会话面板占窗口宽度的比例（60%）
    let corner_radius = 12.0; // UI 元素的圆角半径
    let chat_content_pad = 0.0; // 聊天内容区域的内边距

    // 设置面板宽度：检查是否为有限值，如果是则限制在 200-800 像素范围内
    // 如果是无限值（如初始状态），则使用默认值 370.0 像素
    let settings_panel_width = if app.settings_panel_width.is_finite() {
        app.settings_panel_width.clamp(200.0, 800.0)
    } else {
        370.0
    };

    // 构建基础布局：包含左侧导航栏、会话面板和设置面板的主布局结构
    let base_layout = if uses_work_layout(app) {
        build_work_layout(
            app,
            settings_panel_width,
            left_rail_width,
            session_panel_width_scale,
            corner_radius,
            spacing,
            chat_content_pad,
        )
    } else {
        build_base_layout(
            app,
            settings_panel_width,
            left_rail_width,
            session_panel_width_scale,
            corner_radius,
            spacing,
            content_pad,
            chat_content_pad,
        )
    };

    // 构建拖拽徽章层：显示当前拖拽操作状态的浮动层
    let drag_badge_layer = drag_badge_layer(app);

    // 构建会话选择器层：用于创建或选择新会话的覆盖层
    let session_picker_layer = new_session_picker_layer(app);

    // 使用容器和堆栈将所有层组合在一起
    // 堆栈顺序：base_layout（底层） -> drag_badge_layer -> session_picker_layer（顶层）
    container(stack![base_layout, drag_badge_layer, session_picker_layer])
        .width(Length::Fill) // 容器宽度填满可用空间
        .height(Length::Fill) // 容器高度填满可用空间
        .style(workspace_background_style)
        .into() // 将容器转换为 Element
}

fn uses_work_layout(app: &App) -> bool {
    app.workspace_mode == WorkspaceMode::Work
}
#[cfg(test)]
#[path = "project_tests.rs"]
mod project_tests;
