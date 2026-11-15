//! 项目工作区布局模块，负责侧栏、主区域、右侧面板和拖拽提示的组合。
//!
//! 本模块只负责视图组合与样式适配，不持有业务状态，也不扩大外部能力边界。

use iced::widget::container;
use iced::{Element, Length, Theme};

use super::styles::session_panel_style;

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
/// # 返回
/// 返回可直接嵌入父级视图的 Iced 控件或样式闭包。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub fn project_sessions_panel_container<'a, Message: 'a>(
    content: Element<'a, Message>,
    settings_panel_width: f32,
    left_rail_width: f32,
    session_panel_width_scale: f32,
    corner_radius: f32,
) -> Element<'a, Message> {
    let panel_width = project_sessions_panel_width(
        settings_panel_width,
        left_rail_width,
        session_panel_width_scale,
    );
    container(content)
        .width(Length::Fixed(panel_width))
        .height(Length::Fill)
        .clip(true)
        .style(move |theme: &Theme| project_sessions_panel_container_style(theme, corner_radius))
        .into()
}

#[cfg(test)]
#[path="sessions_panel_tests.rs"] mod sessions_panel_tests;
