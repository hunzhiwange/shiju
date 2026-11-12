//! 右键操作使用有序选择快照；批量文件写入仍通过网关逐项执行。
use super::*;
use std::path::Path;

pub(super) fn menu_paths(app: &App) -> Vec<String> {
    let Some(path) = &app.file_tree_menu_path else { return vec![] };
    if matches!(app.file_tree_menu_source.as_deref(), Some("tree-file" | "tree-dir"))
        && let Some(root) = &app.project_path
        && app.file_tree_selection.contains(root, path)
    {
        let paths = crate::app::components::file_tree::selection::visible_paths(app, root)
            .into_iter()
            .filter(|path| app.file_tree_selection.contains(root, path))
            .collect::<Vec<_>>();
        if !paths.is_empty() {
            return paths;
        }
    }
    vec![path.clone()]
}

/// 父目录已包含子项；不能重复删除、复制或移动同一棵子树。
pub(super) fn top_level_paths(paths: Vec<String>) -> Vec<String> {
    paths
        .iter()
        .enumerate()
        .filter(|(index, path)| {
            !paths.iter().enumerate().any(|(other_index, parent)| {
                other_index != *index
                    && Path::new(path).starts_with(parent)
                    && (parent != *path || other_index < *index)
            })
        })
        .map(|(_, path)| path.clone())
        .collect()
}

fn outcome(errors: Vec<String>) -> Result<(), String> {
    if errors.is_empty() { Ok(()) } else { Err(errors.join("\n")) }
}

pub(super) fn duplicate(app: &mut App, root: String, paths: Vec<String>) -> iced::Task<Message> {
    app.file_tree_operation_in_progress = true;
    iced::Task::perform(
        async move {
            let mut errors = Vec::new();
            for path in paths {
                if let Err(error) = gateway_duplicate_path(&root, &path).await {
                    errors.push(format!("{path}: {error}"));
                }
            }
            ProjectMessage::FileTreeDuplicateCompleted {
                project_path: root,
                result: outcome(errors),
            }
        },
        Message::Project,
    )
}

/// 预留本批目标名，避免不同目录中的同名文件相互覆盖。
fn paste_targets(app: &App, destination: &str, sources: &[String]) -> Vec<(String, String)> {
    let directory = if is_directory_path(destination, &app.files) {
        Path::new(destination)
    } else {
        Path::new(destination).parent().unwrap_or(Path::new(destination))
    };
    let mut existing = app.files.iter().cloned().collect::<HashSet<_>>();
    let mut targets = Vec::new();
    for source in sources {
        let name = Path::new(source).file_name().and_then(|name| name.to_str()).unwrap_or("");
        let target = unique_name_for_target(directory, name, &existing);
        existing.insert(target.clone());
        targets.push((source.clone(), target));
    }
    targets
}

pub(super) fn paste(app: &mut App, destination: &str) -> iced::Task<Message> {
    let Some(clipboard) = app.file_tree_clipboard.clone() else { return iced::Task::none() };
    let Some(root) = app.project_path.clone() else { return iced::Task::none() };
    let targets = paste_targets(app, destination, &clipboard.src_paths);
    if targets.is_empty() {
        return iced::Task::none();
    }
    app.file_tree_operation_in_progress = true;
    iced::Task::perform(
        async move {
            let mut errors = Vec::new();
            let mut completed_sources = Vec::new();
            for (source, target) in targets {
                let result = match clipboard.mode {
                    FileTreeClipboardMode::Copy => gateway_copy_path(&root, &source, &target).await,
                    FileTreeClipboardMode::Cut => gateway_move_path(&root, &source, &target).await,
                };
                match result {
                    Ok(()) => completed_sources.push(source),
                    Err(error) => errors.push(format!("{source}: {error}")),
                }
            }
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
    let Some((root, paths)) = app.file_tree_pending_delete.take() else {
        return iced::Task::none();
    };
    if app.project_path.as_ref() != Some(&root) || app.file_tree_operation_in_progress {
        return iced::Task::none();
    }
    let paths = top_level_paths(paths)
        .into_iter()
        .map(|path| {
            let recursive = is_directory_path(&path, &app.files);
            (path, recursive)
        })
        .collect::<Vec<_>>();
    if paths.is_empty() {
        return iced::Task::none();
    }
    for (path, recursive) in &paths {
        remove_local_file_tree_path(&mut app.files, path, *recursive);
    }
    app.file_tree_operation_in_progress = true;
    iced::Task::perform(
        async move {
            let mut errors = Vec::new();
            let mut deleted_paths = Vec::new();
            for (path, recursive) in paths {
                match gateway_delete_path(&root, &path, recursive).await {
                    Ok(()) => deleted_paths.push(path),
                    Err(error) => errors.push(format!("{path}: {error}")),
                }
            }
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
