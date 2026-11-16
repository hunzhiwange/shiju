//! Frozen file-operation tasks. The host supplies only its authorized gateway executor.
use crate::file_tree::outcome;
use crate::interaction::{FileTreeClipboard, FileTreeClipboardMode};
use crate::operations::{self, Operation};
use iced::Task;
use std::future::Future;

#[derive(Debug)]
pub enum FileEvent {
    FileTreeDuplicateCompleted {
        project_path: String,
        result: Result<(), String>,
    },
    FileTreePasteCompleted {
        project_path: String,
        clipboard: FileTreeClipboard,
        completed_sources: Vec<String>,
        result: Result<(), String>,
    },
    FileTreeDeleteCompleted {
        project_path: String,
        deleted_paths: Vec<String>,
        result: Result<(), String>,
    },
    FileTreeMoveCompleted {
        project_path: String,
        source_path: String,
        target_path: String,
        result: Result<(), String>,
    },
    FileTreeBatchMoveCompleted {
        project_path: String,
        moved_paths: Vec<(String, String)>,
        errors: Vec<String>,
    },
}
pub fn duplicate<E, F>(root: String, paths: Vec<String>, execute: E) -> Task<FileEvent>
where
    E: Fn(Operation) -> F + Send + 'static,
    F: Future<Output = Result<(), String>> + Send + 'static,
{
    iced::Task::perform(
        async move {
            let effects = paths
                .into_iter()
                .map(|path| Operation::Duplicate { root: root.clone(), path })
                .collect();
            let errors = operations::run(effects, execute).await.errors;
            FileEvent::FileTreeDuplicateCompleted { project_path: root, result: outcome(errors) }
        },
        std::convert::identity,
    )
}

pub fn paste<E, F>(
    root: String,
    clipboard: FileTreeClipboard,
    targets: Vec<(String, String)>,
    execute: E,
) -> Task<FileEvent>
where
    E: Fn(Operation) -> F + Send + 'static,
    F: Future<Output = Result<(), String>> + Send + 'static,
{
    iced::Task::perform(
        async move {
            let effects = targets
                .into_iter()
                .map(|(source, target)| match clipboard.mode {
                    FileTreeClipboardMode::Copy => {
                        Operation::Copy { root: root.clone(), source, target }
                    }
                    FileTreeClipboardMode::Cut => {
                        Operation::Move { root: root.clone(), source, target }
                    }
                })
                .collect();
            let result = operations::run(effects, execute).await;
            let completed_sources = result.completed;
            let errors = result.errors;
            FileEvent::FileTreePasteCompleted {
                project_path: root,
                clipboard,
                completed_sources,
                result: outcome(errors),
            }
        },
        std::convert::identity,
    )
}

pub fn delete<E, F>(root: String, paths: Vec<(String, bool)>, execute: E) -> Task<FileEvent>
where
    E: Fn(Operation) -> F + Send + 'static,
    F: Future<Output = Result<(), String>> + Send + 'static,
{
    iced::Task::perform(
        async move {
            let effects = paths
                .into_iter()
                .map(|(path, recursive)| Operation::Delete { root: root.clone(), path, recursive })
                .collect();
            let result = operations::run(effects, execute).await;
            let deleted_paths = result.completed;
            let errors = result.errors;
            FileEvent::FileTreeDeleteCompleted {
                project_path: root,
                deleted_paths,
                result: outcome(errors),
            }
        },
        std::convert::identity,
    )
}

pub fn move_paths<E, F>(root: String, moves: Vec<(String, String)>, execute: E) -> Task<FileEvent>
where
    E: Fn(Operation) -> F + Send + 'static,
    F: Future<Output = Result<(), String>> + Send + 'static,
{
    Task::perform(
        async move {
            if moves.len() == 1 {
                let (source, target) = moves.into_iter().next().unwrap();
                let result = execute(Operation::Move {
                    root: root.clone(),
                    source: source.clone(),
                    target: target.clone(),
                })
                .await;
                return FileEvent::FileTreeMoveCompleted {
                    project_path: root,
                    source_path: source,
                    target_path: target,
                    result,
                };
            }
            let effects = moves
                .iter()
                .map(|(source, target)| crate::operations::Operation::Move {
                    root: root.clone(),
                    source: source.clone(),
                    target: target.clone(),
                })
                .collect();
            let result = crate::operations::run(effects, execute).await;
            let moved_paths =
                moves.into_iter().filter(|(source, _)| result.completed.contains(source)).collect();
            let errors = result.errors;
            FileEvent::FileTreeBatchMoveCompleted { project_path: root, moved_paths, errors }
        },
        std::convert::identity,
    )
}

pub struct ImportCompleted {
    pub project_path: String,
    pub source_path: String,
    pub result: Result<vw_api_file::file::LocalImportResponse, String>,
}
pub fn import<E, F>(
    root: String,
    source: String,
    directory: String,
    execute: E,
) -> Task<ImportCompleted>
where
    E: FnOnce(String, String, Vec<String>) -> F + Send + 'static,
    F: Future<Output = Result<vw_api_file::file::LocalImportResponse, String>> + Send + 'static,
{
    Task::perform(
        async move {
            let result = execute(root.clone(), directory, vec![source.clone()]).await;
            ImportCompleted { project_path: root, source_path: source, result }
        },
        std::convert::identity,
    )
}

impl FileEvent {
    pub fn into_message<P, H, U>(self) -> crate::project_message::message::ProjectMessage<P, H, U> {
        use crate::project_message::message::ProjectMessage;
        use FileEvent as E;
        match self {
            E::FileTreeDuplicateCompleted { project_path, result } => {
                ProjectMessage::FileTreeDuplicateCompleted { project_path, result }
            }
            E::FileTreePasteCompleted { project_path, clipboard, completed_sources, result } => {
                ProjectMessage::FileTreePasteCompleted {
                    project_path,
                    clipboard,
                    completed_sources,
                    result,
                }
            }
            E::FileTreeDeleteCompleted { project_path, deleted_paths, result } => {
                ProjectMessage::FileTreeDeleteCompleted { project_path, deleted_paths, result }
            }
            E::FileTreeMoveCompleted { project_path, source_path, target_path, result } => {
                ProjectMessage::FileTreeMoveCompleted {
                    project_path,
                    source_path,
                    target_path,
                    result,
                }
            }
            E::FileTreeBatchMoveCompleted { project_path, moved_paths, errors } => {
                ProjectMessage::FileTreeBatchMoveCompleted { project_path, moved_paths, errors }
            }
        }
    }
}
