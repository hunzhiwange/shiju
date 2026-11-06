use super::{GitMessage, repository};
use crate::app::components::git_panel::repository_ops::{HistoryDetail, RepositorySnapshot};

#[test]
fn history_results_ignore_old_generation_and_old_commit_selection() {
    let mut app = crate::app::App::new().0;
    let path = "/gateway-only/repository".to_owned();
    app.project_path = Some(path.clone());
    app.git_repository.repo_path = Some(path.clone());
    app.git_repository.generation = 2;
    app.git_repository.loading = true;
    let _task = repository::update(
        &mut app,
        GitMessage::RepositoryReady {
            path: path.clone(),
            generation: 1,
            result: Ok(RepositorySnapshot { branch: Some("stale".into()), ..Default::default() }),
        },
    );
    assert!(app.git_repository.loading);
    assert!(app.git_repository.snapshot.branch.is_none());
    app.git_repository.selected = Some("newer".into());
    app.git_repository.detail_loading = true;
    let _task = repository::update(
        &mut app,
        GitMessage::HistoryDetailReady {
            path,
            generation: 2,
            sha: "older".into(),
            result: Ok(HistoryDetail { message: "stale".into(), files: vec![] }),
        },
    );
    assert!(app.git_repository.detail_loading);
    assert!(app.git_repository.detail.is_none());
}

#[test]
fn history_file_selection_expands_one_file_and_closes_others() {
    let mut app = crate::app::App::new().0;
    let _ = repository::update(&mut app, GitMessage::SelectHistoryFile(0));
    assert_eq!(app.git_repository.selected_file, Some(0));
    let _ = repository::update(&mut app, GitMessage::SelectHistoryFile(1));
    assert_eq!(app.git_repository.selected_file, Some(1));
    let _ = repository::update(&mut app, GitMessage::SelectHistoryFile(1));
    assert_eq!(app.git_repository.selected_file, None);
}

#[test]
fn remote_animation_advances_only_while_syncing() {
    let mut app = crate::app::App::new().0;
    let _ = repository::update(&mut app, GitMessage::RemoteAnimationTick);
    assert_eq!(app.git_repository.remote_animation_frame, 0);
    app.git_repository.remote_busy = true;
    let _ = repository::update(&mut app, GitMessage::RemoteAnimationTick);
    assert_eq!(app.git_repository.remote_animation_frame, 1);
    app.git_repository.remote_busy = false;
    let _ = repository::update(&mut app, GitMessage::RemoteAnimationTick);
    assert_eq!(app.git_repository.remote_animation_frame, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn history_preview_uses_committed_snapshot_including_deleted_files() {
    use vw_gateway_client::vw_api_types::git::{GitFileStatus, GitHistoryFileDto};
    let mut app = crate::app::App::new().0;
    app.git_repository.detail = Some(HistoryDetail {
        message: String::new(),
        files: vec![GitHistoryFileDto {
            path: "/gateway-only/中文文件.txt".into(),
            status: GitFileStatus::Deleted,
            old_content: Some("提交中的已删除内容".into()),
            new_content: Some(String::new()),
            notice: None,
            insertions: 0,
            deletions: 1,
        }],
    });
    app.git_repository.file_menu = Some(0);
    use iced_runtime::futures::futures::StreamExt;
    app.git_repository.selected = Some("a".repeat(40));
    let task = repository::update(&mut app, GitMessage::PreviewHistoryFile(0));
    let mut stream = iced_runtime::task::into_stream(task).unwrap();
    while let Some(action) = stream.next().await {
        if let iced_runtime::Action::Output(crate::app::Message::Preview(message)) = action {
            let _ = crate::app::message::preview::update(&mut app, message);
        }
    }
    assert!(!app.show_git_copy_modal);
    let tab = &app.preview_tabs[0];
    assert_eq!(tab.content, "提交中的已删除内容");
    assert!(tab.read_only);
    assert!(tab.title.contains("aaaaaaaa"));
    assert_eq!(app.active_preview_path.as_deref(), Some(tab.path.as_str()));
    let key = tab.path.clone();
    let _ = crate::app::message::preview::update(
        &mut app,
        crate::app::message::PreviewMessage::EditorEvent(iced_code_editor::Message::Paste(
            "changed".into(),
        )),
    );
    assert_eq!(app.preview_tabs[0].editor.content(), "提交中的已删除内容");
    let _ = crate::app::message::editor::update(
        &mut app,
        crate::app::message::editor::EditorMessage::ClipboardContentReceived(Some(
            "changed".into(),
        )),
    );
    assert_eq!(app.preview_tabs[0].editor.content(), "提交中的已删除内容");
    app.preview_tabs[0].is_dirty = true;
    let task = crate::app::message::preview::update(
        &mut app,
        crate::app::message::PreviewMessage::SaveFilePath { path: key, notify: false },
    );
    assert!(iced_runtime::task::into_stream(task).is_none());
    assert_eq!(app.git_repository.file_menu, None);
    app.show_git_copy_modal = false;
    let _ = repository::update(&mut app, GitMessage::PreviewHistoryFile(99));
    assert!(!app.show_git_copy_modal);
}
