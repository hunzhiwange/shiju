//! 项目工作区布局模块，负责侧栏、主区域、右侧面板和拖拽提示的组合。
//!
//! 本模块只负责视图组合与样式适配，不持有业务状态，也不扩大外部能力边界。

use iced::widget::{column, container, mouse_area, row};
use iced::{Element, Length};

use crate::app::{App, Message, message};

use super::super::components::projects_list;
use super::super::handles::HResizeHandle;
use super::super::styles::left_rail_style;
use super::chrome::overlay_divider;
use super::sessions_panel::project_sessions_panel_container;

/// 构建侧栏界面。
///
/// # 参数
/// - `app`: 当前视图构建所需的状态、配置或消息。
/// - `settings_panel_width`: 当前视图构建所需的状态、配置或消息。
/// - `left_rail_width`: 当前视图构建所需的状态、配置或消息。
/// - `session_panel_width_scale`: 当前视图构建所需的状态、配置或消息。
/// - `corner_radius`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回值遵循函数签名约定，调用方据此继续组装界面或更新状态。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub fn sidebar_group(
    app: &App,
    settings_panel_width: f32,
    left_rail_width: f32,
    session_panel_width_scale: f32,
    corner_radius: f32,
) -> (Element<'_, Message>, Option<Element<'_, Message>>) {
    if app.show_settings {
        let projects_list: Element<'_, Message> = projects_list(app, false);

        let sidebar = container(
            container(
                column![projects_list].align_x(iced::alignment::Horizontal::Center).spacing(2),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .padding([10, 0])
            .style(left_rail_style),
        )
        .width(Length::Fixed(left_rail_width))
        .height(Length::Fill);

        let projects_panel: Element<'_, Message> = project_sessions_panel_container(
            app,
            settings_panel_width,
            left_rail_width,
            session_panel_width_scale,
            corner_radius,
            app.project_path.clone(),
        );

        let resize_handle = mouse_area(HResizeHandle)
            .on_press(Message::View(message::ViewMessage::SettingsDragStarted));

        (
            row![sidebar, projects_panel]
                .width(Length::Fixed(
                    left_rail_width
                        + ((settings_panel_width - left_rail_width) * session_panel_width_scale)
                            .max(0.0),
                ))
                .height(Length::Fill)
                .into(),
            Some(resize_handle.into()),
        )
    } else {
        let projects_list: Element<'_, Message> = projects_list(app, true);

        let settings_toggle = container(
            container(projects_list)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding([10, 0])
                .style(left_rail_style),
        )
        .width(Length::Fixed(left_rail_width))
        .height(Length::Fill);

        (settings_toggle.into(), None)
    }
}

/// 执行本模块的界面辅助逻辑。
///
/// # 参数
/// - `app`: 当前视图构建所需的状态、配置或消息。
/// - `left_sidebar`: 当前视图构建所需的状态、配置或消息。
/// - `right_column`: 当前视图构建所需的状态、配置或消息。
/// - `settings_panel_width`: 当前视图构建所需的状态、配置或消息。
/// - `left_rail_width`: 当前视图构建所需的状态、配置或消息。
/// - `session_panel_width_scale`: 当前视图构建所需的状态、配置或消息。
/// - `corner_radius`: 当前视图构建所需的状态、配置或消息。
/// - `spacing`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回可直接嵌入父级视图的 Iced 控件或样式闭包。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub fn hover_overlay_layout<'a>(
    app: &'a App,
    left_sidebar: Element<'a, Message>,
    right_column: Element<'a, Message>,
    settings_panel_width: f32,
    left_rail_width: f32,
    session_panel_width_scale: f32,
    corner_radius: f32,
    spacing: f32,
) -> Element<'a, Message> {
    if let Some(path) = app.hovered_recent_project.clone() {
        let panel_width =
            ((settings_panel_width - left_rail_width) * session_panel_width_scale).max(0.0);
        let hover_sessions_panel = project_sessions_panel_container(
            app,
            settings_panel_width,
            left_rail_width,
            session_panel_width_scale,
            corner_radius,
            Some(path),
        );
        let left_group = mouse_area(
            row![left_sidebar, hover_sessions_panel]
                .width(Length::Fixed(left_rail_width + panel_width))
                .height(Length::Fill),
        )
        .on_exit(Message::Project(message::ProjectMessage::RecentOverlayClosed));
        row![left_group, overlay_divider(spacing), right_column].spacing(spacing).into()
    } else {
        row![left_sidebar, overlay_divider(spacing), right_column].spacing(spacing).into()
    }
}
#[cfg(test)]
#[path = "sidebar_tests.rs"]
mod sidebar_tests;
