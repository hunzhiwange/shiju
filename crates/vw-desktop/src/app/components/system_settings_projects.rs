//! 系统设置中 projects 配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::components::system_settings_common::{
    SETTINGS_LABEL_WIDTH, danger_action_btn_style, primary_action_btn_style,
    rounded_action_btn_style, settings_checkbox_style, settings_muted_text_style,
    settings_page_intro, settings_panel, settings_panel_style, settings_section_card,
    settings_text_input_style,
};
use crate::app::{App, Message, message};
use iced::widget::{
    button, checkbox, column, container, row, text, text_input,
    tooltip::{Position as TooltipPosition, Tooltip},
};
use iced::{Alignment, Element, Length};

fn field_row<'a>(
    label: &'static str,
    description: &'static str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    container(
        row![
            column![
                text(label).size(13),
                text(description).size(11).style(settings_muted_text_style),
            ]
            .spacing(4)
            .width(Length::Fixed(SETTINGS_LABEL_WIDTH)),
            container(control.into()).width(Length::Fill),
        ]
        .spacing(22)
        .align_y(Alignment::Center),
    )
    .padding([14, 0])
    .width(Length::Fill)
    .into()
}

/// 构建或处理 `view` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回可交给 Iced 渲染树使用的 `Element`，其中已绑定必要的消息回调。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn view(app: &App) -> Element<'_, Message> {
    let open_folder_btn = button(
        container(text("打开文件夹"))
            .width(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center),
    )
    .on_press(Message::Project(message::ProjectMessage::OpenFolderPressed))
    .width(Length::Fill)
    .padding([10, 12])
    .style(primary_action_btn_style);

    let current_project = app.project_path.clone();
    let current_project_worktree_enabled = current_project
        .as_ref()
        .and_then(|p| app.project_worktree_enabled.get(p))
        .copied()
        .unwrap_or(false);
    let worktree_toggle: Element<'_, Message> = if let Some(project_path) = current_project {
        field_row(
            "启用 Worktree",
            "当前项目新建会话时允许选择/创建工作区。",
            checkbox(current_project_worktree_enabled)
                .label("当前项目新建会话时允许选择/创建工作区")
                .on_toggle(move |v| {
                    Message::Settings(message::SettingsMessage::ProjectEnableWorktreeToggled(
                        project_path.clone(),
                        v,
                    ))
                })
                .style(settings_checkbox_style),
        )
    } else {
        field_row(
            "启用 Worktree",
            "仅在打开一个项目后才能配置该开关。",
            text("请先打开一个项目，再配置该项目的 Worktree 开关")
                .size(12)
                .style(settings_muted_text_style),
        )
    };

    let mut col = column![
        settings_page_intro(
            "项目配置",
            "配置项目打开入口、当前项目 Worktree 行为，以及历史项目管理。"
        ),
        settings_section_card("当前项目", "打开项目并控制当前项目的新会话 Worktree 策略。"),
        settings_panel(column![open_folder_btn, worktree_toggle].spacing(12)),
        settings_section_card("历史项目", "管理最近打开过的项目名称与快捷入口。"),
    ]
    .spacing(16);

    for (i, p) in app.recent_projects_edits.iter().enumerate() {
        let path = app.recent_projects.get(i).cloned().unwrap_or_default();
        let open_path = path.clone();
        let is_confirming_delete = app.recent_project_delete_confirm_idx == Some(i);

        let mut content = column![
            row![
                container(text("名字")).width(Length::Fixed(52.0)).style(|t: &iced::Theme| {
                    iced::widget::container::Style {
                        text_color: Some(t.palette().text.scale_alpha(0.7)),
                        ..Default::default()
                    }
                }),
                text_input("项目名称", p)
                    .on_input(move |v| {
                        Message::Settings(message::SettingsMessage::RecentProjectRenameChanged(
                            i, v,
                        ))
                    })
                    .padding([10, 12])
                    .size(13)
                    .style(settings_text_input_style)
                    .width(Length::Fill),
                button(text("保存"))
                    .on_press(Message::Settings(message::SettingsMessage::RecentProjectRenameSave(
                        i
                    ),))
                    .padding([6, 10])
                    .style(rounded_action_btn_style),
                button(text("删除"))
                    .on_press(Message::Settings(
                        message::SettingsMessage::RecentProjectDeleteRequested(i),
                    ))
                    .padding([6, 10])
                    .style(rounded_action_btn_style),
            ]
            .spacing(10)
            .align_y(Alignment::Center)
            .width(Length::Fill),
        ]
        .spacing(8)
        .width(Length::Fill);

        if is_confirming_delete {
            content = content.push(
                row![
                    container(text("确认删除该项目？")).width(Length::Fill),
                    button(text("取消"))
                        .on_press(Message::Settings(
                            message::SettingsMessage::RecentProjectDeleteCanceled,
                        ))
                        .padding([6, 10])
                        .style(rounded_action_btn_style),
                    button(text("确认删除"))
                        .on_press(Message::Settings(
                            message::SettingsMessage::RecentProjectDeleteConfirmed(i),
                        ))
                        .padding([6, 10])
                        .style(danger_action_btn_style),
                ]
                .spacing(10)
                .align_y(Alignment::Center)
                .width(Length::Fill),
            );
        }

        let tip_content =
            container(text(path.clone())).padding([6, 10]).style(settings_panel_style);

        let open_btn = button(
            container(text("打开项目"))
                .width(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center),
        )
        .on_press(Message::Project(message::ProjectMessage::OpenRecentPressed(open_path)))
        .width(Length::Fill)
        .padding([10, 12])
        .style(primary_action_btn_style);

        content = content.push(Tooltip::new(open_btn, tip_content, TooltipPosition::Top).gap(10));

        let card = container(content).padding(12).width(Length::Fill).style(settings_panel_style);

        col = col.push(card);
    }

    col.into()
}
#[cfg(test)]
#[path = "system_settings_projects_tests.rs"]
mod system_settings_projects_tests;
