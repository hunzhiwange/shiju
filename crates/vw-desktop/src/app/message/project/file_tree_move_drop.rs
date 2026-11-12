//! Single-consumer tree gestures move a frozen selection through the gateway.
use super::{gateway_move_path, replace_path_prefix};
use crate::app::message::ProjectMessage;
use crate::app::{App, Message, PreviewTab};
use iced::Task;
use std::path::{Component, Path};

pub(super) fn clear_drag(app: &mut App) {
    app.file_tree_drag_source = None;
    app.file_tree_drop_target = None;
    app.dragging_file_paths.clear();
    app.dragging_file_position = None;
    app.pending_drop_file_paths.clear();
    app.pending_drop_file_position = None;
    app.input_drop_hovered = false;
}

pub(super) fn release(app: &mut App) -> Option<Task<Message>> {
    let (root, source) = app.file_tree_drag_source.clone()?;
    let target = app.file_tree_drop_target.clone()?;
    Some(submit(app, root, source, target))
}

pub(super) fn submit(
    app: &mut App,
    root: String,
    source: String,
    directory: String,
) -> Task<Message> {
    if app.file_tree_operation_in_progress
        || app.file_tree_drag_source.as_ref() != Some(&(root.clone(), source.clone()))
    {
        return Task::none();
    }
    let sources = super::batch::top_level_paths(app.dragging_file_paths.clone());
    // Consume even rejected tree targets, so the release cannot also reach chat.
    clear_drag(app);
    if app.project_path.as_ref() != Some(&root) {
        return Task::none();
    }
    let directory_path = Path::new(&directory);
    let root_path = Path::new(&root);
    if !directory_path.starts_with(root_path)
        || directory_path.components().any(|part| part == Component::ParentDir)
        || app.files.contains(&directory)
        || sources.iter().any(|source| {
            let path = Path::new(source);
            !path.starts_with(root_path)
                || path == root_path
                || directory_path.starts_with(path)
                || path.components().any(|part| part == Component::ParentDir)
        })
    {
        return Task::none();
    }
    let moves = sources
        .into_iter()
        .filter_map(|source| {
            let path = Path::new(&source);
            if path.parent() == Some(directory_path) {
                return None;
            }
            let target = directory_path.join(path.file_name()?).to_string_lossy().into_owned();
            Some((source, target))
        })
        .collect::<Vec<_>>();
    if moves.is_empty() {
        return Task::none();
    }
    app.file_tree_operation_in_progress = true;
    Task::perform(
        async move {
            if moves.len() == 1 {
                let (source, target) = moves.into_iter().next().unwrap();
                let result = gateway_move_path(&root, &source, &target).await;
                return ProjectMessage::FileTreeMoveCompleted {
                    project_path: root,
                    source_path: source,
                    target_path: target,
                    result,
                };
            }
            let mut moved_paths = Vec::new();
            let mut errors = Vec::new();
            for (source, target) in moves {
                match gateway_move_path(&root, &source, &target).await {
                    Ok(()) => moved_paths.push((source, target)),
                    Err(error) => errors.push(format!("{source}: {error}")),
                }
            }
            ProjectMessage::FileTreeBatchMoveCompleted { project_path: root, moved_paths, errors }
        },
        Message::Project,
    )
}

fn update_tabs(tabs: &mut [PreviewTab], source: &str, target: &str) {
    for tab in tabs {
        if let Some(path) = replace_path_prefix(&tab.path, source, target) {
            #[cfg(not(target_arch = "wasm32"))]
            crate::app::message::preview::lsp::detach_lsp_for_tab(tab);
            tab.path = path;
            tab.title = super::path_display_name(&tab.path);
        }
    }
}

pub(super) fn complete(
    app: &mut App,
    root: String,
    source: String,
    target: String,
    result: Result<(), String>,
) -> Task<Message> {
    match result {
        Ok(()) => complete_batch(app, root, vec![(source, target)], vec![]),
        Err(error) => complete_batch(app, root, vec![], vec![error]),
    }
}

pub(super) fn complete_batch(
    app: &mut App,
    root: String,
    moved_paths: Vec<(String, String)>,
    errors: Vec<String>,
) -> Task<Message> {
    app.file_tree_operation_in_progress = false;
    let feedback = if errors.is_empty() {
        Task::none()
    } else {
        app.show_localized_toast(
            crate::app::state::ToastKind::Error,
            "移动失败: %s",
            vec![errors.join("\n").into()],
        )
    };
    if moved_paths.is_empty() {
        return feedback;
    }
    let current = app.project_path.as_ref() == Some(&root);
    for (source, target) in moved_paths {
        // Stored tabs belong to the originating project even after a project switch.
        if let Some(tabs) = app.project_preview_tabs.get_mut(&root) {
            update_tabs(tabs, &source, &target);
        }
        if let Some(Some(path)) = app.project_preview_active_path.get_mut(&root)
            && let Some(updated) = replace_path_prefix(path, &source, &target)
        {
            *path = updated;
        }
        if current {
            update_tabs(&mut app.preview_tabs, &source, &target);
            if let Some(path) = &mut app.active_preview_path
                && let Some(updated) = replace_path_prefix(path, &source, &target)
            {
                *path = updated;
            }
            app.file_tree_selection.relocate(&root, &source, &target);
            if let (Ok(old), Ok(new)) =
                (Path::new(&source).strip_prefix(&root), Path::new(&target).strip_prefix(&root))
            {
                let old = old.to_string_lossy();
                let new = new.to_string_lossy();
                for key in &mut app.file_tree_expanded {
                    if let Some(updated) = replace_path_prefix(key, &old, &new) {
                        *key = updated;
                    }
                }
            }
        }
    }
    if current {
        app.file_tree_expanded_set = app.file_tree_expanded.iter().cloned().collect();
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = app.active_preview_path.clone() {
            crate::app::message::preview::lsp::sync_lsp_for_path(app, &path);
        }
    }
    let refresh = Task::perform(
        async move {
            let snapshot =
                crate::app::message::project::helpers::refresh_file_index_with_fallback(&root)
                    .await;
            Message::Project(ProjectMessage::FileTreeSnapshotReady { project_path: root, snapshot })
        },
        std::convert::identity,
    );
    if current {
        use crate::app::message::GitMessage as G;
        Task::batch([
            feedback,
            refresh,
            crate::app::message::git::update(app, G::RefreshWorktreeOptions),
            crate::app::message::git::update(app, G::RefreshChangedFiles),
            crate::app::message::git::update(app, G::RefreshDiffFileMetas),
        ])
    } else {
        Task::batch([refresh, feedback])
    }
}

#[cfg(test)]
#[path = "file_tree_move_drop_tests.rs"]
mod tests;
