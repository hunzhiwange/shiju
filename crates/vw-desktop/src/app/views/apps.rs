//! 应用列表视图模块
//!
//! 本模块负责渲染应用列表主界面，提供应用磁贴网格展示和模态框管理功能。
//! 该视图是 VibeWindow 的核心界面之一，用于展示和操作已集成的应用程序。
//!
//! # 主要功能
//!
//! - 渲染应用磁贴网格布局
//! - 管理和显示模态框（如应用详情、设置等）
//! - 处理模态框打开时的界面锁定状态
//!
//! # 架构
//!
//! 模块包含三个子模块：
//! - `modals`: 模态框管理，负责创建和管理弹出窗口
//! - `tiles`: 磁贴渲染，负责生成应用磁贴网格
//! - `ui`: UI 组件，提供通用界面元素

use crate::app::components::system_settings_common::{
    settings_modal_overlay, settings_panel_style,
};
use crate::app::{App, Message};
use iced::widget::{column, container, scrollable};
use iced::{Element, Length};

mod modals;
mod tiles;
mod ui;

/// 渲染应用列表主视图
///
/// 生成完整的应用列表界面，包括头部、磁贴网格和可选的模态框层。
/// 当有模态框激活时，会在主内容上方显示半透明遮罩层。
///
/// # 参数
///
/// - `app`: 应用状态引用，包含所有需要渲染的数据
///
/// # 返回值
///
/// 返回 Iced 框架的 `Element`，代表完整的视图元素树
///
/// # 布局结构
///
/// ```text
/// ┌─────────────────────────────┐
/// │  Header (标题栏/工具栏)      │
/// ├─────────────────────────────┤
/// │                             │
/// │  Tiles Grid (应用磁贴网格)   │
/// │                             │
/// └─────────────────────────────┘
///
/// 当模态框激活时：
/// ┌─────────────────────────────┐
/// │  Base Content (半透明)      │
/// │  ┌───────────────────────┐  │
/// │  │  Modal Overlay        │  │
/// │  │  (半透明黑色遮罩)       │  │
/// │  └───────────────────────┘  │
/// │  ┌───────────────────────┐  │
/// │  │  Modal Layer          │  │
/// │  │  (居中的模态框内容)     │  │
/// │  └───────────────────────┘  │
/// └─────────────────────────────┘
/// ```
///
/// # 示例
///
/// ```rust,ignore
/// let element = view(&app);
/// // 返回可直接用于 Iced 渲染的元素
/// ```
pub fn view(app: &App) -> Element<'_, Message> {
    // 检查当前是否有活动的模态框
    // 如果有模态框显示，界面将被"锁定"（blocked=true）
    let active_modal = modals::active_modal(app);
    let blocked = active_modal.is_some();

    // 渲染头部区域和磁贴网格
    // blocked 参数用于控制这些区域在模态框激活时的视觉状态
    let header = tiles::render_header(app, blocked);
    let grid = tiles::render_tiles_grid(app, blocked);

    // 构建基础内容容器
    // 包含头部和磁贴网格的垂直布局
    let header_panel: Element<'_, Message> =
        container(header).padding([18, 20]).width(Length::Fill).style(settings_panel_style).into();

    let grid_panel: Element<'_, Message> = grid;

    let base_content: Element<'_, Message> =
        container(column![header_panel, grid_panel].spacing(14).width(Length::Fill))
            .width(Length::Fill)
            .padding([18, 24])
            .into();

    // 将基础内容包装在可滚动容器中
    // 确保内容超出视口时可以滚动查看
    let base_content: Element<'_, Message> =
        scrollable(base_content).width(Length::Fill).height(Length::Fill).into();

    if let Some((modal, close_msg)) = active_modal {
        settings_modal_overlay(Some(base_content), close_msg, modal)
    } else {
        // 没有模态框时，直接返回基础内容
        base_content
    }
}

#[cfg(test)]
#[path = "apps_tests.rs"]
mod apps_tests;
