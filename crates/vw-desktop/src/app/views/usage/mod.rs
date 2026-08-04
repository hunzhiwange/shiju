//! 使用统计视图模块
//!
//! 本模块负责构建和渲染应用程序的使用统计界面，展示会话使用情况、
//! 资源消耗等相关信息。
//!
//! # 模块结构
//!
//! - `components`：UI 组件定义
//! - `data`：使用数据结构与转换逻辑
//! - `layout`：视图布局构建
//! - `session_menu`：会话菜单组件
//! - `steps`：步骤展示组件
//! - `styles`：样式定义
//! - `utils`：工具函数

use iced::Element;

use crate::app::{App, Message};

mod components;
mod data;
mod layout;
mod raw_modal;
mod session_menu;
mod steps;
mod styles;
#[cfg(test)]
mod tests;
mod utils;

/// 构建使用统计视图
///
/// 根据应用程序当前状态生成使用统计界面的 Element，该视图会从
/// App 中提取使用数据并构建完整的布局。
///
/// # 参数
///
/// * `app` - 应用程序状态的不可变引用，包含会话和使用统计信息
///
/// # 返回值
///
/// 返回一个 `Element`，代表使用统计视图的完整 UI 结构
///
/// # 示例
///
/// ```ignore
/// let element = view(&app);
/// // 将 element 返回给 iced 框架进行渲染
/// ```
pub fn view(app: &App) -> Element<'_, Message> {
    // 从应用状态中提取使用数据
    let usage_data = data::UsageData::from_app(app);
    // 使用提取的数据构建完整的使用统计视图布局
    let content = layout::build_usage_view(app, &usage_data);
    if let Some(modal) = raw_modal::view(app) {
        iced::widget::stack![content, modal].into()
    } else {
        content
    }
}
