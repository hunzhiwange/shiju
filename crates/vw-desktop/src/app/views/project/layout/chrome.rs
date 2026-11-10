//! 项目工作区布局模块，负责侧栏、主区域、右侧面板和拖拽提示的组合。
//!
//! 本模块只负责视图组合与样式适配，不持有业务状态，也不扩大外部能力边界。

use iced::widget::container;
use iced::{Element, Length, Theme};

use crate::app::Message;
use crate::app::state::WorkspaceMode;

use super::super::handles::{HResizeHandle, TopBorderCover};
use super::super::styles::{right_column_inner_style, work_surface_style};

/// 执行本模块的界面辅助逻辑。
///
/// # 参数
/// - `right_column`: 当前视图构建所需的状态、配置或消息。
/// - `corner_radius`: 当前视图构建所需的状态、配置或消息。
/// - `mode`: 当前工作区模式，用于选择背景样式。
///
/// # 返回
/// 返回可直接嵌入父级视图的 Iced 控件或样式闭包。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub fn right_column_chrome(
    right_column: Element<'_, Message>,
    corner_radius: f32,
    mode: WorkspaceMode,
) -> Element<'_, Message> {
    // 描边由各面板绘制，右栏只提供底色和裁切。
    container(right_column)
        .width(Length::Fill)
        .height(Length::Fill)
        .clip(true)
        .style(move |theme: &Theme| {
            let mut style = right_column_inner_style(theme, corner_radius);
            if mode == WorkspaceMode::Work {
                style.background = work_surface_style(theme).background;
            }
            style
        })
        .into()
}

/// 计算遮罩分割线的宽度。
///
/// 分割线宽度由交互区域宽度减去间距计算，确保最小宽度为 1px。
fn overlay_divider_width(spacing: f32) -> f32 {
    (HResizeHandle::HIT_WIDTH - spacing).max(1.0)
}

/// 执行本模块的界面辅助逻辑。
///
/// # 参数
/// - `spacing`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回可直接嵌入父级视图的 Iced 控件或样式闭包。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub fn overlay_divider(spacing: f32) -> Element<'static, Message> {
    container(TopBorderCover)
        .width(Length::Fixed(overlay_divider_width(spacing)))
        .height(Length::Fill)
        .into()
}
#[cfg(test)]
#[path = "chrome_tests.rs"]
mod chrome_tests;
