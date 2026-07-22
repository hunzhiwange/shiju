//! 渲染应用视图中的模态窗口。
//! 本模块只描述模态内容和交互消息，不直接承担持久化策略。

use crate::app::components::system_settings_common::{
    settings_muted_text_style, settings_panel, settings_section_card, settings_text_editor_style,
};
use iced::widget::{column, row, text, text_editor, toggler};
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
    let start_script_input = column![
        text("工作区启动脚本").size(13),
        text("在创建工作区后执行，可填写多行命令。").size(11).style(settings_muted_text_style),
        text_editor(&app.project_edit_start_script_editor)
            .placeholder("例如：\nbun install\nbun dev")
            .on_action(start_script_editor_action)
            .height(Length::Fixed(96.0))
            .padding([8, 10])
            .size(13)
            .style(settings_text_editor_style)
    ]
    .spacing(6);

    let worktree_toggle = row![
        text("启用 Worktree").size(13).width(Length::Fill),
        toggler(app.project_edit_worktree_enabled).on_toggle(worktree_toggled)
    ]
    .align_y(iced::Alignment::Center);

    column![
        settings_section_card(
            "启动与工作区",
            "设置 worktree 启用策略，以及创建工作区后的启动命令。"
        ),
        settings_panel(column![start_script_input, worktree_toggle].spacing(14))
    ]
    .spacing(12)
    .into()
}
#[cfg(test)]
#[path = "launch_tab_tests.rs"]
mod launch_tab_tests;
