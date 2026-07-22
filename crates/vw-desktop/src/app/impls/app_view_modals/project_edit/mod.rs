//! 渲染应用视图中的模态窗口。
//! 本模块只描述模态内容和交互消息，不直接承担持久化策略。

mod common;
mod general_tab;
mod launch_tab;
mod refresh_tab;
mod scheduling_tab;

use crate::app::components::color_picker::{parse_color, render_color_picker};
use crate::app::components::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style, settings_muted_text_style,
    settings_panel_style,
};
use crate::app::state::ProjectEditTab;
use iced::widget::{
    Space, button, column, container, mouse_area, opaque, row, scrollable, stack, text,
};
use iced::{Background, Color, Element, Length, Theme};

use super::{App, Message};
use common::{format_hex_color, parse_hex_color, tab_button};
use general_tab::general_tab;
use launch_tab::launch_tab;
use refresh_tab::refresh_tab;
use scheduling_tab::scheduling_tab;

/// 模块内可见函数，执行 with_project_edit 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(crate) fn with_project_edit<'a>(
    app: &'a App,
    mut root_content: Element<'a, Message>,
) -> Element<'a, Message> {
    if app.project_edit_path.is_some() {
        let cancel = button(text("取消").size(13))
            .on_press(Message::Project(
                crate::app::message::project::ProjectMessage::ProjectEditCanceled,
            ))
            .padding([6, 12])
            .style(rounded_action_btn_style);
        let save = button(text("保存").size(13))
            .on_press(Message::Project(
                crate::app::message::project::ProjectMessage::ProjectEditSaved,
            ))
            .padding([6, 12])
            .style(primary_action_btn_style);

        let active_tab_content: Element<'_, Message> = match app.project_edit_tab {
            ProjectEditTab::General => general_tab(app),
            ProjectEditTab::Launch => launch_tab(app),
            ProjectEditTab::Refresh => refresh_tab(app),
            ProjectEditTab::Scheduling => scheduling_tab(app),
        };

        let active_tab_hint = match app.project_edit_tab {
            ProjectEditTab::General => "项目名称、图标与展示信息",
            ProjectEditTab::Launch => "工作区启动与 worktree 行为",
            ProjectEditTab::Refresh => "自动刷新、代码审查与任务池联动",
            ProjectEditTab::Scheduling => "调度频率、并发和超时保护",
        };

        let tabs = row![
            tab_button(
                "基础信息",
                ProjectEditTab::General,
                app.project_edit_tab == ProjectEditTab::General,
            ),
            tab_button(
                "启动与工作区",
                ProjectEditTab::Launch,
                app.project_edit_tab == ProjectEditTab::Launch,
            ),
            tab_button(
                "刷新策略",
                ProjectEditTab::Refresh,
                app.project_edit_tab == ProjectEditTab::Refresh,
            ),
            tab_button(
                "调度与保护",
                ProjectEditTab::Scheduling,
                app.project_edit_tab == ProjectEditTab::Scheduling,
            ),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center);

        let tab_panel = scrollable(active_tab_content).height(Length::Fixed(420.0));

        let card = container(
            column![
                tabs,
                text(active_tab_hint).size(11).style(settings_muted_text_style),
                tab_panel,
                row![Space::new().width(Length::Fill), cancel, save].spacing(8)
            ]
            .spacing(14),
        )
        .width(Length::Fixed(760.0))
        .padding([22, 24])
        .style(|theme: &Theme| {
            let mut style = settings_panel_style(theme);
            style.border.radius = 24.0.into();
            style
        });

        let overlay = opaque(
            mouse_area(container(Space::new().width(Length::Fill).height(Length::Fill)).style(
                |_| iced::widget::container::Style {
                    background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.45))),
                    ..Default::default()
                },
            ))
            .on_press(Message::Project(
                crate::app::message::project::ProjectMessage::ProjectEditCanceled,
            )),
        );

        let modal_layer: Element<'_, Message> = container(mouse_area(card).on_press(Message::None))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into();

        root_content = stack![root_content, stack![overlay, modal_layer]].into();

        if app.project_edit_icon_color_picker_open {
            let current_color = parse_color(&app.project_edit_icon_color)
                .or_else(|| parse_hex_color(&app.project_edit_icon_color))
                .unwrap_or(Color::from_rgb8(96, 165, 250));
            let color_picker_card = container(
                column![
                    text("选择图标颜色").size(14),
                    Space::new().height(8.0),
                    render_color_picker(
                        current_color,
                        app.project_edit_icon_color_format,
                        false,
                        {
                            move |color| {
                                Message::Project(
                                    crate::app::message::project::ProjectMessage::ProjectEditIconColorChanged(
                                        format_hex_color(color),
                                    ),
                                )
                            }
                        },
                        |format| {
                            Message::Project(
                                crate::app::message::project::ProjectMessage::ProjectEditIconColorFormatChanged(
                                    format,
                                ),
                            )
                        },
                        || Message::None,
                    ),
                    Space::new().height(8.0),
                    button(text("确定").size(13))
                        .on_press(Message::Project(
                            crate::app::message::project::ProjectMessage::ProjectEditIconColorPickerClosed,
                        ))
                        .padding([6, 16])
                        .style(primary_action_btn_style),
                ]
                .spacing(4),
            )
            .width(Length::Fixed(280.0))
            .padding(16)
            .style(|theme: &Theme| {
                let mut style = settings_panel_style(theme);
                style.border.radius = 20.0.into();
                style
            });

            let color_overlay = opaque(
                mouse_area(container(Space::new().width(Length::Fill).height(Length::Fill)).style(
                    |_| iced::widget::container::Style {
                        background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.3))),
                        ..Default::default()
                    },
                ))
                .on_press(Message::Project(
                    crate::app::message::project::ProjectMessage::ProjectEditIconColorPickerClosed,
                )),
            );

            let color_modal_layer: Element<'_, Message> =
                container(mouse_area(color_picker_card).on_press(Message::None))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
                    .into();

            root_content = stack![root_content, stack![color_overlay, color_modal_layer]].into();
        }
    }

    root_content
}
#[cfg(test)]
mod tests;
