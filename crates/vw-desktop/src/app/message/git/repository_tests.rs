use super::{GitMessage, repository};
use crate::app::components::git_panel::repository_ops::{HistoryDetail, RepositorySnapshot};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn history_preview_uses_committed_snapshot_including_deleted_files() {
    use vw_api_git::{GitFileStatus, GitHistoryFileDto};
    let mut app = crate::app::App::new().0;
    app.git.git_repository.detail = Some(HistoryDetail {
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
    app.git.git_repository.file_menu = Some(0);
    use iced_runtime::futures::futures::StreamExt;
    app.git.git_repository.selected = Some("a".repeat(40));
    let task = repository::update(&mut app, GitMessage::PreviewHistoryFile(0));
    let mut stream = iced_runtime::task::into_stream(task).unwrap();
    while let Some(action) = stream.next().await {
        if let iced_runtime::Action::Output(crate::app::Message::Preview(message)) = action {
            let _ = crate::app::message::preview::update(&mut app, message);
        }
    }
    assert!(!app.git.show_git_copy_modal);
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
    assert_eq!(app.git.git_repository.file_menu, None);
    app.git.show_git_copy_modal = false;
    let _ = repository::update(&mut app, GitMessage::PreviewHistoryFile(99));
    assert!(!app.git.show_git_copy_modal);
}

#[cfg(not(target_arch = "wasm32"))]
// 宿主契约：核对 App 字段投影及 update 的消息/任务接线。
#[test]
fn non_git_project_skips_repository_requests_and_clears_history_error() {
    use crate::app::components::git_panel::repository_ops::RemoteAction;
    let project = tempfile::tempdir().expect("temporary project");
    let mut app = crate::app::App::new().0;
    app.project_path = Some(project.path().to_string_lossy().into_owned());
    app.git.git_repository.repo_path = app.project_path.clone();
    app.git.git_repository.history = true;
    app.git.git_repository.loading = true;
    app.git.git_repository.error = Some("not a repository".into());
    app.file_manager.file_manager_changes_refreshing = true;
    let task = super::update(
        &mut app,
        GitMessage::OpenChatTextDiff {
            title: "hello.md +1-0".into(),
            file: "hello.md".into(),
            before: String::new(),
            after: "555\n".into(),
        },
    );
    assert!(iced_runtime::task::into_stream(task).is_none());
    assert!(app.git.show_git_custom_diff_modal);
    assert_eq!(app.git.git_custom_diff_after_editor.text(), "555\n");
    for message in [
        GitMessage::RefreshRepository,
        GitMessage::RunRemote(RemoteAction::Fetch),
        GitMessage::RunRemote(RemoteAction::Pull),
        GitMessage::RunRemote(RemoteAction::Push),
        GitMessage::SelectHistoryCommit("a".repeat(40)),
        GitMessage::RefreshChangedFiles,
        GitMessage::RefreshDiffFileMetas,
    ] {
        let task = super::update(&mut app, message);
        assert!(iced_runtime::task::into_stream(task).is_none());
    }
    assert!(app.git.git_repository.repo_path.is_none());
    assert!(!app.git.git_repository.history);
    assert!(!app.git.git_repository.loading);
    assert!(!app.git.git_repository.remote_busy);
    assert!(app.git.git_repository.error.is_none());
    assert!(!app.git.git_changed_files_loading);
    assert!(!app.file_manager.file_manager_changes_refreshing);
    assert!(!app.git.git_diff_file_metas_loading);
    assert!(app.git.show_git_custom_diff_modal);
    assert_eq!(app.git.git_custom_diff_after_editor.text(), "555\n");
}
