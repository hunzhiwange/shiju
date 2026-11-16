//! Native drops retain the event's source and the receiving directory snapshot.
use crate::app::message::ProjectMessage;
use crate::app::{App, Message};
use iced::Task;
use vw_api_file::file::LocalImportResponse;

pub(super) fn submit(
    app: &mut App,
    root: String,
    source: String,
    directory: String,
) -> Task<Message> {
    if !app.file_tree_interaction.prepare_import(
        app.navigation_tabs.project_path.as_deref(),
        &root,
        &source,
        &directory,
        &app.files,
    ) {
        return Task::none();
    }
    // Each native callback owns its context and task; a pending move cannot discard it.
    desktop_workspace::operations::tasks::import(
        root,
        source,
        directory,
        |root, directory, paths| async move {
            super::super::helpers::gateway_import_local_paths(&root, &directory, paths).await
        },
    )
    .map(|event| {
        Message::Project(ProjectMessage::FileTreeImportCompleted {
            project_path: event.project_path,
            source_path: event.source_path,
            result: event.result,
        })
    })
}

pub(super) fn complete(
    app: &mut App,
    root: String,
    source: String,
    result: Result<LocalImportResponse, String>,
) -> Task<Message> {
    let (kind, detail) = desktop_workspace::file_tree::import_feedback(&source, result);
    let kind = match kind {
        desktop_workspace::file_tree::FeedbackKind::Success => {
            crate::app::state::ToastKind::Success
        }
        desktop_workspace::file_tree::FeedbackKind::Error => crate::app::state::ToastKind::Error,
    };
    let toast = app.show_localized_toast(kind, "%s", vec![detail.into()]);
    let current = app.navigation_tabs.project_path.as_ref() == Some(&root);
    let refresh = Task::perform(
        async move {
            let snapshot = super::super::helpers::refresh_file_index_with_fallback(&root).await;
            ProjectMessage::FileTreeSnapshotReady { project_path: root, snapshot }
        },
        Message::Project,
    );
    if current {
        use crate::app::message::GitMessage as G;
        Task::batch([
            toast,
            refresh,
            crate::app::message::git::update(app, G::RefreshChangedFiles),
            crate::app::message::git::update(app, G::RefreshDiffFileMetas),
        ])
    } else {
        Task::batch([toast, refresh])
    }
}
