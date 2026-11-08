//! 渲染应用视图中的模态窗口。
//! 本模块只描述模态内容和交互消息，不直接承担持久化策略。

use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, settings_field_label,
    settings_group_heading_with_help, settings_hover_help_label, settings_panel,
    settings_text_editor_style, settings_toggle,
};
use iced::widget::{column, row, text_editor};
use iced::{Element, Length};

use super::{App, Message};

fn start_script_editor_action(action: text_editor::Action) -> Message {
    Message::Project(
        crate::app::message::project::ProjectMessage::ProjectEditStartScriptEditorAction(action),
    )
}

fn worktree_toggled(value: bool) -> Message {
    Message::Project(crate::app::message::project::ProjectMessage::ProjectEditWorktreeToggled(
        value,
    ))
}

/// 模块内可见函数，执行 launch_tab 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn launch_tab<'a>(app: &'a App) -> Element<'a, Message> {
    let i18n = crate::app::i18n::I18n::new(app.desktop_locale);
    let start_script_label = i18n.__("工作区启动脚本");
    let start_script_input = column![
        settings_hover_help_label(
            i18n,
            &app.app_theme,
            start_script_label,
            settings_field_label(start_script_label),
            i18n.__("在创建工作区后执行，可填写多行命令。"),
        )
        .wrap(),
        text_editor(&app.project_edit_start_script_editor)
            .placeholder(i18n.__("例如：\nbun install\nbun dev"))
            .on_action(start_script_editor_action)
            .height(Length::Fixed(96.0))
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_text_editor_style)
    ]
    .spacing(6);

    let worktree_toggle = row![
        settings_field_label(i18n.__("启用 Worktree")).width(Length::Fill),
        settings_toggle(app.project_edit_worktree_enabled).on_toggle(worktree_toggled)
    ]
    .align_y(iced::Alignment::Center);

    column![
        settings_group_heading_with_help(
            i18n,
            &app.app_theme,
            i18n.__("启动与工作区"),
            i18n.__("设置 worktree 启用策略，以及创建工作区后的启动命令。")
        ),
        settings_panel(column![start_script_input, worktree_toggle].spacing(14))
    ]
    .spacing(12)
    .into()
}
#[cfg(test)]
#[path = "launch_tab_tests.rs"]
mod launch_tab_tests;
