//! Native drops retain the event's source and the receiving directory snapshot.
use crate::app::message::ProjectMessage;
use crate::app::{App, Message};
use iced::Task;
use std::path::{Component, Path};
use vw_gateway_client::vw_api_types::file::LocalImportResponse;

pub(super) fn submit(
    app: &mut App,
    root: String,
    source: String,
    directory: String,
) -> Task<Message> {
    super::move_drop::clear_drag(app);
    if app.project_path.as_ref() != Some(&root)
        || !Path::new(&source).is_absolute()
        || !Path::new(&directory).starts_with(&root)
        || Path::new(&directory).components().any(|part| part == Component::ParentDir)
        || app.files.contains(&directory)
    {
        return Task::none();
    }
    // Each native callback owns its context and task; a pending move cannot discard it.
    Task::perform(
        async move {
            let result = super::super::helpers::gateway_import_local_paths(
                &root,
                &directory,
                vec![source.clone()],
            )
            .await;
            ProjectMessage::FileTreeImportCompleted {
                project_path: root,
                source_path: source,
                result,
            }
        },
        Message::Project,
    )
}

pub(super) fn complete(
    app: &mut App,
    root: String,
    source: String,
    result: Result<LocalImportResponse, String>,
) -> Task<Message> {
    let name = super::path_display_name(&source);
    let (kind, detail) = match result {
        Ok(response) if response.items.len() == 1 && response.items[0].source_path == source => {
            let item = &response.items[0];
            if item.ok {
                (crate::app::state::ToastKind::Success, format!("已导入: {name}"))
            } else {
                (
                    crate::app::state::ToastKind::Error,
                    format!("导入失败: {name}: {}", item.error.as_deref().unwrap_or("未知错误")),
                )
            }
        }
        // Transport failure can occur after publication. Never retry automatically.
        Err(error) => (
            crate::app::state::ToastKind::Error,
            format!("导入失败或结果未知，请刷新确认: {name}: {error}"),
        ),
        Ok(_) => (crate::app::state::ToastKind::Error, format!("导入结果未知，请刷新确认: {name}")),
    };
    let toast = app.show_localized_toast(kind, "%s", vec![detail.into()]);
    let current = app.project_path.as_ref() == Some(&root);
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
