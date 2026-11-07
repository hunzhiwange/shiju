//! 系统设置中 projects 配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, danger_action_btn_style,
    primary_action_btn_style, rounded_action_btn_style, settings_checkbox_style,
    settings_field_label, settings_group_card, settings_group_heading, settings_muted_text_style,
    settings_page_heading, settings_panel_style, settings_row_with_flexible_control,
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
    settings_row_with_flexible_control(label, description, 310.0, control)
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
    let i18n = crate::app::i18n::I18n::new(app.desktop_locale);
    let open_folder_btn = button(
        container(text(i18n.__("打开文件夹")))
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
            i18n.__("启用 Worktree"),
            i18n.__("当前项目新建会话时允许选择/创建工作区。"),
            checkbox(current_project_worktree_enabled)
                .label(i18n.__("当前项目新建会话时允许选择/创建工作区"))
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
            i18n.__("启用 Worktree"),
            i18n.__("仅在打开一个项目后才能配置该开关。"),
            text(i18n.__("请先打开一个项目，再配置该项目的 Worktree 开关"))
                .size(12)
                .style(settings_muted_text_style),
        )
    };

    let mut recent_heading = column![settings_group_heading(
        i18n.__("历史项目"),
        i18n.__("管理最近打开过的项目名称与快捷入口。"),
    )]
    .spacing(8);
    if app.recent_projects_edits.is_empty() {
        recent_heading = recent_heading
            .push(text(i18n.__("暂无最近项目")).size(13).style(settings_muted_text_style));
    }

    let mut col = column![
        settings_page_heading(
            i18n.__("项目配置"),
            i18n.__("配置项目打开入口、当前项目 Worktree 行为，以及历史项目管理。")
        ),
        settings_group_heading(
            i18n.__("当前项目"),
            i18n.__("打开项目并控制当前项目的新会话 Worktree 策略。")
        ),
        settings_group_card(column![
            worktree_toggle,
            container(open_folder_btn).padding(iced::padding::bottom(16))
        ]),
        recent_heading,
    ]
    .spacing(16);

    for (i, p) in app.recent_projects_edits.iter().enumerate() {
        let path = app.recent_projects.get(i).cloned().unwrap_or_default();
        let open_path = path.clone();
        let is_confirming_delete = app.recent_project_delete_confirm_idx == Some(i);

        let mut content = column![
            settings_field_label(i18n.__("项目名称")),
            row![
                text_input(i18n.__("项目名称"), p)
                    .on_input(move |v| {
                        Message::Settings(message::SettingsMessage::RecentProjectRenameChanged(
                            i, v,
                        ))
                    })
                    .padding(SETTINGS_CONTROL_PADDING)
                    .size(SETTINGS_CONTROL_TEXT_SIZE)
                    .style(settings_text_input_style)
                    .width(Length::Fill),
                button(text(i18n.__("保存")).size(13))
                    .on_press(Message::Settings(message::SettingsMessage::RecentProjectRenameSave(
                        i
                    ),))
                    .padding([9, 12])
                    .style(primary_action_btn_style),
                button(text(i18n.__("删除")).size(13))
                    .on_press(Message::Settings(
                        message::SettingsMessage::RecentProjectDeleteRequested(i),
                    ))
                    .padding([9, 12])
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
                    container(text(i18n.__("确认删除该项目？"))).width(Length::Fill),
                    button(text(i18n.__("取消")))
                        .on_press(Message::Settings(
                            message::SettingsMessage::RecentProjectDeleteCanceled,
                        ))
                        .padding([9, 12])
                        .style(rounded_action_btn_style),
                    button(text(i18n.__("确认删除")))
                        .on_press(Message::Settings(
                            message::SettingsMessage::RecentProjectDeleteConfirmed(i),
                        ))
                        .padding([9, 12])
                        .style(danger_action_btn_style),
                ]
                .spacing(10)
                .align_y(Alignment::Center)
                .width(Length::Fill),
            );
        }

        let tip_content =
            container(text(path).size(13).wrapping(iced::widget::text::Wrapping::WordOrGlyph))
                .max_width(360)
                .padding([9, 12])
                .style(settings_panel_style);

        let open_btn = button(
            container(text(i18n.__("打开项目")))
                .width(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center),
        )
        .on_press(Message::Project(message::ProjectMessage::OpenRecentPressed(open_path)))
        .width(Length::Fill)
        .padding([10, 12])
        .style(primary_action_btn_style);

        content = content.push(Tooltip::new(open_btn, tip_content, TooltipPosition::Top).gap(10));

        let card = settings_group_card(container(content).padding([16, 0]));

        col = col.push(card);
    }

    col.into()
}
#[cfg(test)]
#[path = "system_settings_projects_tests.rs"]
mod system_settings_projects_tests;

#[cfg(test)]
#[path = "system_settings_projects_ui_refresh_30_tests.rs"]
mod system_settings_projects_ui_refresh_30_tests;
