//! Single-consumer tree gestures move a frozen selection through the gateway.
use super::replace_path_prefix;
use crate::app::message::ProjectMessage;
use crate::app::{App, Message, PreviewTab};
use iced::Task;

pub(super) fn clear_drag(app: &mut App) {
    app.file_tree_interaction.clear_drag();
}

pub(super) fn release(app: &mut App) -> Option<Task<Message>> {
    let (root, source) = app.file_tree_interaction.file_tree_drag_source.clone()?;
    let target = app.file_tree_interaction.file_tree_drop_target.clone()?;
    Some(submit(app, root, source, target))
}

pub(super) fn submit(
    app: &mut App,
    root: String,
    source: String,
    directory: String,
) -> Task<Message> {
    let Some(moves) = app.file_tree_interaction.prepare_move(
        app.navigation_tabs.project_path.as_deref(),
        &root,
        &source,
        &directory,
        &app.session_state.files,
    ) else {
        return Task::none();
    };
    desktop_workspace::operations::tasks::move_paths(root, moves, super::batch::execute)
        .map(super::batch::file_event)
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
    app.file_tree_interaction.file_tree_operation_in_progress = false;
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
    let current = app.navigation_tabs.project_path.as_ref() == Some(&root);
    for (source, target) in moved_paths {
        // Stored tabs belong to the originating project even after a project switch.
        if let Some(tabs) = app.navigation_tabs.project_preview_tabs.get_mut(&root) {
            update_tabs(tabs, &source, &target);
        }
        if let Some(Some(path)) = app.navigation_tabs.project_preview_active_path.get_mut(&root)
            && let Some(updated) = replace_path_prefix(path, &source, &target)
        {
            *path = updated;
        }
        if current {
            update_tabs(&mut app.navigation_tabs.preview_tabs, &source, &target);
            if let Some(path) = &mut app.navigation_tabs.active_preview_path
                && let Some(updated) = replace_path_prefix(path, &source, &target)
            {
                *path = updated;
            }
            app.file_tree_interaction.relocate(
                &root,
                &source,
                &target,
                &mut app.workspace_state.file_tree_expanded,
            );
        }
    }
    if current {
        app.workspace_state.file_tree_expanded_set = app.workspace_state.file_tree_expanded.iter().cloned().collect();
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = app.navigation_tabs.active_preview_path.clone() {
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
