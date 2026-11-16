//! 右键操作使用有序选择快照；批量文件写入仍通过网关逐项执行。
use super::*;
pub(super) use desktop_workspace::file_tree::top_level_paths;
use desktop_workspace::operations::Operation;

pub(super) async fn execute(operation: Operation) -> Result<(), String> {
    match operation {
        Operation::Duplicate { root, path } => gateway_duplicate_path(&root, &path).await,
        Operation::Copy { root, source, target } => {
            gateway_copy_path(&root, &source, &target).await
        }
        Operation::Move { root, source, target } => {
            gateway_move_path(&root, &source, &target).await
        }
        Operation::Delete { root, path, recursive } => {
            gateway_delete_path(&root, &path, recursive).await
        }
    }
}

pub(super) fn menu_paths(app: &App) -> Vec<String> {
    let visible = app
        .navigation_tabs
        .project_path
        .as_deref()
        .map(|root| crate::app::components::file_tree::selection::visible_paths(app, root))
        .unwrap_or_default();
    app.file_tree_interaction.menu_paths(app.navigation_tabs.project_path.as_deref(), &visible)
}

pub(super) fn duplicate(app: &mut App, root: String, paths: Vec<String>) -> iced::Task<Message> {
    app.file_tree_interaction.file_tree_operation_in_progress = true;
    desktop_workspace::operations::tasks::duplicate(root, paths, execute).map(file_event)
}
pub(super) fn paste(app: &mut App, destination: &str) -> iced::Task<Message> {
    let Some((root, clipboard, targets)) = app.file_tree_interaction.prepare_paste(
        app.navigation_tabs.project_path.as_deref(),
        &app.session_state.files,
        destination,
    ) else {
        return iced::Task::none();
    };
    desktop_workspace::operations::tasks::paste(root, clipboard, targets, execute).map(file_event)
}
pub(super) fn delete(app: &mut App) -> iced::Task<Message> {
    let Some((root, paths)) = app
        .file_tree_interaction
        .prepare_delete(app.navigation_tabs.project_path.as_deref(), &mut app.session_state.files)
    else {
        return iced::Task::none();
    };
    desktop_workspace::operations::tasks::delete(root, paths, execute).map(file_event)
}
pub(super) fn file_event(event: desktop_workspace::operations::tasks::FileEvent) -> Message {
    Message::Project(event.into_message())
}
#[cfg(test)]
#[path = "file_tree_batch_tests.rs"]
mod tests;
