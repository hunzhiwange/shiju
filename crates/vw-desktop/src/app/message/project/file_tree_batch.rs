//! 右键操作使用有序选择快照；批量文件写入仍通过网关逐项执行。
use super::*;
pub(super) use desktop_workspace::file_tree::top_level_paths;
use desktop_workspace::file_tree::outcome;
use desktop_workspace::operations::{self, Operation};

pub(super) async fn execute(operation: Operation) -> Result<(), String> {
    match operation {
        Operation::Duplicate { root, path } => gateway_duplicate_path(&root, &path).await,
        Operation::Copy { root, source, target } => gateway_copy_path(&root, &source, &target).await,
        Operation::Move { root, source, target } => gateway_move_path(&root, &source, &target).await,
        Operation::Delete { root, path, recursive } => gateway_delete_path(&root, &path, recursive).await,
    }
}

pub(super) fn menu_paths(app: &App) -> Vec<String> {
    let visible = app.navigation_tabs.project_path.as_deref().map(|root| {
        crate::app::components::file_tree::selection::visible_paths(app, root)
    }).unwrap_or_default();
    app.file_tree_interaction.menu_paths(app.navigation_tabs.project_path.as_deref(), &visible)
}

pub(super) fn duplicate(app: &mut App, root: String, paths: Vec<String>) -> iced::Task<Message> {
    app.file_tree_interaction.file_tree_operation_in_progress = true;
    iced::Task::perform(
        async move {
            let effects = paths.into_iter().map(|path| Operation::Duplicate { root: root.clone(), path }).collect();
            let errors = operations::run(effects, execute).await.errors;
            ProjectMessage::FileTreeDuplicateCompleted {
                project_path: root,
                result: outcome(errors),
            }
        },
        Message::Project,
    )
}

pub(super) fn paste(app: &mut App, destination: &str) -> iced::Task<Message> {
    let Some((root, clipboard, targets)) = app.file_tree_interaction.prepare_paste(app.navigation_tabs.project_path.as_deref(), &app.files, destination) else {
        return iced::Task::none();
    };
    iced::Task::perform(
        async move {
            let effects = targets.into_iter().map(|(source, target)| match clipboard.mode {
                FileTreeClipboardMode::Copy => Operation::Copy { root: root.clone(), source, target },
                FileTreeClipboardMode::Cut => Operation::Move { root: root.clone(), source, target },
            }).collect();
            let result = operations::run(effects, execute).await;
            let completed_sources = result.completed;
            let errors = result.errors;
            ProjectMessage::FileTreePasteCompleted {
                project_path: root,
                clipboard,
                completed_sources,
                result: outcome(errors),
            }
        },
        Message::Project,
    )
}

pub(super) fn delete(app: &mut App) -> iced::Task<Message> {
    let Some((root, paths)) = app.file_tree_interaction.prepare_delete(app.navigation_tabs.project_path.as_deref(), &mut app.files) else {
        return iced::Task::none();
    };
    iced::Task::perform(
        async move {
            let effects = paths.into_iter().map(|(path, recursive)| Operation::Delete { root: root.clone(), path, recursive }).collect();
            let result = operations::run(effects, execute).await;
            let deleted_paths = result.completed;
            let errors = result.errors;
            ProjectMessage::FileTreeDeleteCompleted {
                project_path: root,
                deleted_paths,
                result: outcome(errors),
            }
        },
        Message::Project,
    )
}

#[cfg(test)]
#[path = "file_tree_batch_tests.rs"]
mod tests;
