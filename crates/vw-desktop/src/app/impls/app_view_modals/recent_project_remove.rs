//! 渲染从最近项目列表移除项目的确认对话框。

use super::{App, Message, message};
use crate::app::components::toast::confirm_dialog;
use iced::Element;

pub(crate) fn with_recent_project_remove_confirm<'a>(
    app: &App,
    root_content: Element<'a, Message>,
) -> Element<'a, Message> {
    let Some(path) = app.recent_project_remove_confirm_path.as_ref() else {
        return root_content;
    };

    let display_name = app
        .recent_projects
        .iter()
        .position(|p| p == path)
        .and_then(|idx| app.recent_projects_edits.get(idx))
        .map(|name| name.trim())
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .or_else(|| {
            app.recent_projects_meta
                .iter()
                .find(|meta| &meta.path == path)
                .map(|meta| meta.name.trim().to_string())
                .filter(|name| !name.is_empty())
        })
        .unwrap_or_else(|| {
            std::path::Path::new(path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(path)
                .to_string()
        });

    let confirm = Message::Project(message::ProjectMessage::RecentRemoveConfirmed(path.clone()));
    let cancel = Message::Project(message::ProjectMessage::RecentRemoveCanceled);
    let dialog = confirm_dialog(
        "确认移除项目",
        format!("确定从列表中移除「{display_name}」吗？这不会删除磁盘上的文件。"),
        "确认移除",
        "取消",
        confirm,
        cancel,
    );

    iced::widget::stack![root_content, dialog].into()
}

#[cfg(test)]
#[path = "recent_project_remove_tests.rs"]
mod recent_project_remove_tests;
