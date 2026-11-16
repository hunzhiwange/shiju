use iced::{Element, Length};
use iced::widget::{button,column,container,mouse_area,row,text,tooltip};
use super::super::handles::HResizeHandle;
use super::styles::left_rail_style;
use super::chrome::overlay_divider;
/// 在两种侧栏状态下提供同一应用工作区入口，沿用设置页的连接与加载流程。
pub fn workspace_rail<'a, Message: Clone + 'static>(projects: Element<'a, Message>, open: Message, external: Message) -> Element<'a, Message> {
    let entry = button(text("工作区").size(12))
        .padding([8, 6])
        .style(desktop_ui_controls::system_settings_common::rounded_action_btn_style)
        .on_press(open);
    column![
        projects,
        tooltip(entry, text("应用工作区"), tooltip::Position::Right),
        button(text("外部会话").size(12))
            .padding([8, 6])
            .style(desktop_ui_controls::system_settings_common::rounded_action_btn_style)
            .on_press(external),
    ]
    .align_x(iced::alignment::Horizontal::Center)
    .spacing(8)
    .height(Length::Fill)
    .into()
}

/// 构建侧栏界面。
///
/// # 返回
/// 返回值遵循函数签名约定，调用方据此继续组装界面或更新状态。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub fn sidebar_group<'a, Message: Clone + 'static>(show_settings: bool, rail: Element<'a, Message>, projects_panel: Element<'a, Message>, drag: Message, settings_panel_width: f32, left_rail_width: f32, session_panel_width_scale: f32) -> (Element<'a, Message>, Option<Element<'a, Message>>) {
    if show_settings {
        let projects_list = rail;

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

        let resize_handle = mouse_area(HResizeHandle)
            .on_press(drag);

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
        let projects_list = rail;

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
/// # 返回
/// 返回可直接嵌入父级视图的 Iced 控件或样式闭包。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub fn hover_overlay_layout<'a, Message: Clone + 'static>(left_sidebar: Element<'a, Message>, right_column: Element<'a, Message>, hover_panel: Option<Element<'a, Message>>, close: Message, settings_panel_width: f32, left_rail_width: f32, session_panel_width_scale: f32, spacing: f32) -> Element<'a, Message> {
    if let Some(hover_sessions_panel) = hover_panel {
        let panel_width = ((settings_panel_width-left_rail_width)*session_panel_width_scale).max(0.0);
        let left_group = mouse_area(
            row![left_sidebar, hover_sessions_panel]
                .width(Length::Fixed(left_rail_width + panel_width))
                .height(Length::Fill),
        )
        .on_exit(close);
        row![left_group, overlay_divider(spacing), right_column].spacing(spacing).into()
    } else {
        row![left_sidebar, overlay_divider(spacing), right_column].spacing(spacing).into()
    }
}
