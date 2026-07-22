//! 渲染应用视图中的模态窗口。
//! 本模块只描述模态内容和交互消息，不直接承担持久化策略。

use iced::Element;

use super::{App, Message};

fn file_tree_rename_old_name(path: Option<&str>) -> &str {
    path.and_then(|path| std::path::Path::new(path).file_name())
        .and_then(|segment| segment.to_str())
        .unwrap_or("")
}

fn session_rename_old_title<'a>(app: &'a App, session_id: &str) -> &'a str {
    app.sessions
        .iter()
        .find(|session| session.id.as_str() == session_id)
        .map(|session| session.title.as_str())
        .or_else(|| {
            app.project_sessions
                .values()
                .find_map(|sessions| {
                    sessions.iter().find(|session| session.id.as_str() == session_id)
                })
                .map(|session| session.title.as_str())
        })
        .unwrap_or("")
}

/// 模块内可见函数，执行 with_file_tree_rename 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(crate) fn with_file_tree_rename<'a>(
    app: &'a App,
    mut root_content: Element<'a, Message>,
) -> Element<'a, Message> {
    if app.file_tree_rename_path.is_some() {
        use crate::app::components::system_settings_common::{
            primary_action_btn_style, rounded_action_btn_style, settings_modal_card,
            settings_modal_overlay, settings_muted_text_style, settings_text_input_style,
        };
        use iced::Length;
        use iced::widget::{Space, button, column, row, text, text_input};

        let old_name = file_tree_rename_old_name(app.file_tree_rename_path.as_deref());

        let input = text_input("输入新名称", &app.file_tree_rename_value)
            .on_input(|value| {
                Message::Project(
                    crate::app::message::project::ProjectMessage::FileTreeRenameChanged(value),
                )
            })
            .on_submit(Message::Project(
                crate::app::message::project::ProjectMessage::FileTreeRenameSave,
            ))
            .padding([10, 12])
            .style(settings_text_input_style);

        let cancel = button(text("取消").size(13))
            .on_press(Message::Project(
                crate::app::message::project::ProjectMessage::FileTreeRenameCancel,
            ))
            .padding([8, 14])
            .style(rounded_action_btn_style);
        let save = button(text("确定").size(13))
            .on_press(Message::Project(
                crate::app::message::project::ProjectMessage::FileTreeRenameSave,
            ))
            .padding([8, 14])
            .style(primary_action_btn_style);

        let close_message =
            Message::Project(crate::app::message::project::ProjectMessage::FileTreeRenameCancel);

        let card = settings_modal_card(
            column![
                text("重命名").size(18),
                text(old_name).size(12).style(settings_muted_text_style),
                Space::new().height(8.0),
                input,
                Space::new().height(10.0),
                row![Space::new().width(Length::Fill), cancel, save].spacing(8)
            ]
            .spacing(8),
        )
        .width(Length::Fixed(400.0));

        root_content = settings_modal_overlay(Some(root_content), close_message, card);
    }

    root_content
}

/// 模块内可见函数，执行 with_session_rename 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(crate) fn with_session_rename<'a>(
    app: &'a App,
    mut root_content: Element<'a, Message>,
) -> Element<'a, Message> {
    if let Some(ref session_id) = app.session_rename_id {
        use crate::app::components::system_settings_common::{
            primary_action_btn_style, rounded_action_btn_style, settings_modal_card,
            settings_modal_overlay, settings_muted_text_style, settings_text_input_style,
        };
        use iced::Length;
        use iced::widget::{Space, button, column, row, text, text_input};

        let old_title = session_rename_old_title(app, session_id);

        let input = text_input("输入新名称", &app.session_rename_value)
            .on_input(|value| {
                Message::Project(
                    crate::app::message::project::ProjectMessage::SessionRenameChanged(value),
                )
            })
            .on_submit(Message::Project(
                crate::app::message::project::ProjectMessage::SessionRenameSave,
            ))
            .padding([10, 12])
            .style(settings_text_input_style);

        let cancel = button(text("取消").size(13))
            .on_press(Message::Project(
                crate::app::message::project::ProjectMessage::SessionRenameCancel,
            ))
            .padding([8, 14])
            .style(rounded_action_btn_style);
        let save = button(text("确定").size(13))
            .on_press(Message::Project(
                crate::app::message::project::ProjectMessage::SessionRenameSave,
            ))
            .padding([8, 14])
            .style(primary_action_btn_style);

        let close_message =
            Message::Project(crate::app::message::project::ProjectMessage::SessionRenameCancel);

        let card = settings_modal_card(
            column![
                text("会话重命名").size(18),
                text(old_title).size(12).style(settings_muted_text_style),
                Space::new().height(8.0),
                input,
                Space::new().height(10.0),
                row![Space::new().width(Length::Fill), cancel, save].spacing(8)
            ]
            .spacing(8),
        )
        .width(Length::Fixed(500.0));

        root_content = settings_modal_overlay(Some(root_content), close_message, card);
    }

    root_content
}
#[cfg(test)]
#[path = "rename_tests.rs"]
mod rename_tests;
