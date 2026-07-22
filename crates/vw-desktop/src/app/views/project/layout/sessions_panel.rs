//! 项目工作区布局模块，负责侧栏、主区域、右侧面板和拖拽提示的组合。
//!
//! 本模块只负责视图组合与样式适配，不持有业务状态，也不扩大外部能力边界。

use iced::widget::container;
use iced::{Element, Length, Theme};

use crate::app::{App, Message};

use super::super::components::project_sessions_panel;
use super::super::styles::session_panel_style;

fn project_sessions_panel_width(
    settings_panel_width: f32,
    left_rail_width: f32,
    session_panel_width_scale: f32,
) -> f32 {
    ((settings_panel_width - left_rail_width) * session_panel_width_scale).max(0.0)
}

fn project_sessions_panel_container_style(
    theme: &Theme,
    corner_radius: f32,
) -> iced::widget::container::Style {
    session_panel_style(theme, corner_radius)
}

/// 构建面板界面。
///
/// # 参数
/// - `app`: 当前视图构建所需的状态、配置或消息。
/// - `settings_panel_width`: 当前视图构建所需的状态、配置或消息。
/// - `left_rail_width`: 当前视图构建所需的状态、配置或消息。
/// - `session_panel_width_scale`: 当前视图构建所需的状态、配置或消息。
/// - `corner_radius`: 当前视图构建所需的状态、配置或消息。
/// - `target_path`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回可直接嵌入父级视图的 Iced 控件或样式闭包。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub fn project_sessions_panel_container(
    app: &App,
    settings_panel_width: f32,
    left_rail_width: f32,
    session_panel_width_scale: f32,
    corner_radius: f32,
    target_path: Option<String>,
) -> Element<'_, Message> {
    let panel_width = project_sessions_panel_width(
        settings_panel_width,
        left_rail_width,
        session_panel_width_scale,
    );
    container(project_sessions_panel(
        app,
        settings_panel_width,
        left_rail_width,
        session_panel_width_scale,
        target_path,
    ))
    .width(Length::Fixed(panel_width))
    .height(Length::Fill)
    .clip(true)
    .style(move |theme: &Theme| project_sessions_panel_container_style(theme, corner_radius))
    .into()
}
#[cfg(test)]
#[path = "sessions_panel_tests.rs"]
mod sessions_panel_tests;
