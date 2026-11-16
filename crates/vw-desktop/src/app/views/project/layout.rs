//! 项目工作区布局模块，负责侧栏、主区域、右侧面板和拖拽提示的组合。
//!
//! 本模块只负责视图组合与样式适配，不持有业务状态，也不扩大外部能力边界。

use iced::widget::Space;
use iced::{Element, Length};

use crate::app::components::chat_panel;
use crate::app::{App, Message};

mod bottom;
mod chat;
mod chrome;
mod drag;
mod main;
mod sessions_panel;
mod sidebar;
mod work;
mod work_resources;
#[cfg(test)]
#[path = "layout/work_resources_tests.rs"]
mod work_resources_tests;
mod work_sidebar;
pub(crate) use work_sidebar::WorkSidebarModel;
#[cfg(test)]
#[path = "layout/work_sidebar_tests.rs"]
mod work_sidebar_tests;
#[cfg(test)]
#[path = "layout/work_tests.rs"]
mod work_tests;

use bottom::bottom_panel;
use main::main_area;
use sidebar::sidebar_group;
pub use work::build_work_layout;

#[cfg(test)]
const WORKSPACE_BOTTOM_GAP: f32 = desktop_workspace::views::project::layout::WORKSPACE_BOTTOM_GAP;

/// 构建对应界面片段。
///
/// # 参数
/// - `app`: 当前视图构建所需的状态、配置或消息。
/// - `settings_panel_width`: 当前视图构建所需的状态、配置或消息。
/// - `left_rail_width`: 当前视图构建所需的状态、配置或消息。
/// - `session_panel_width_scale`: 当前视图构建所需的状态、配置或消息。
/// - `corner_radius`: 当前视图构建所需的状态、配置或消息。
/// - `spacing`: 当前视图构建所需的状态、配置或消息。
/// - `content_pad`: 当前视图构建所需的状态、配置或消息。
/// - `chat_content_pad`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回可直接嵌入父级视图的 Iced 控件或样式闭包。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub fn build_base_layout(
    app: &App,
    settings_panel_width: f32,
    left_rail_width: f32,
    session_panel_width_scale: f32,
    corner_radius: f32,
    spacing: f32,
    content_pad: f32,
    chat_content_pad: f32,
) -> Element<'_, Message> {
    if app.terminal.is_fullscreen {
        return bottom::fullscreen_terminal(app);
    }

    let (left_sidebar, left_resize_handle) = sidebar_group(
        app,
        settings_panel_width,
        left_rail_width,
        session_panel_width_scale,
        corner_radius,
    );

    let hover_panel = if left_resize_handle.is_none() {
        app.hovered_recent_project.clone().map(|path| {
            sessions_panel::project_sessions_panel_container(
                app,
                settings_panel_width,
                left_rail_width,
                session_panel_width_scale,
                corner_radius,
                Some(path),
            )
        })
    } else {
        None
    };
    desktop_workspace::views::project::layout::code_workspace(
        left_sidebar,
        left_resize_handle,
        main_area(app, spacing, content_pad, chat_content_pad, corner_radius),
        chat_panel::tool_dialog_overlay(app)
            .unwrap_or_else(|| Space::new().width(Length::Fill).height(Length::Fill).into()),
        hover_panel,
        Message::Project(crate::app::message::ProjectMessage::RecentOverlayClosed),
        if app.chat_panel_fullscreen || app.git.git_diff_fullscreen || !app.terminal.is_visible {
            None
        } else {
            Some(bottom_panel(app))
        },
        settings_panel_width,
        left_rail_width,
        session_panel_width_scale,
        corner_radius,
        spacing,
    )
}

/// 构建拖拽交互界面。
///
/// # 参数
/// - `app`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回可直接嵌入父级视图的 Iced 控件或样式闭包。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub fn drag_badge_layer(app: &App) -> Element<'_, Message> {
    drag::drag_badge_layer(app)
}
#[cfg(test)]
#[path = "layout_tests.rs"]
mod layout_tests;
