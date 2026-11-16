// 宿主保留依据：局部路径、附件、DTO、快照及搜索断言迁入 desktop_split_26_tests；此处保留 App 投影、UI 分块任务与完成消息契约。
// 验证附件路径约束、图像快照及文件名清理辅助逻辑。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::{
    append_local_attachments, prepare_session_ui_chunks_task, prepare_session_ui_task,
    prepare_stream_message_ui_task, refresh_file_index, run_find_task,
};
use crate::app::App;
use std::fs;
use tempfile::tempdir;

async fn collect_task_messages(task: iced::Task<crate::app::Message>) -> Vec<crate::app::Message> {
    use iced_runtime::futures::futures::StreamExt;

    let Some(mut stream) = iced_runtime::task::into_stream(task.collect()) else {
        return Vec::new();
    };
    while let Some(action) = stream.next().await {
        if let iced_runtime::Action::Output(messages) = action {
            return messages;
        }
    }
    Vec::new()
}

// 行为契约：使用代表性场景固定该分支的可观察结果。
// 断言聚焦输入、状态与返回任务，不复述内部控制流程。

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。

#[test]
fn append_local_attachments_adds_only_new_valid_files() {
    let (mut app, _task) = App::new();
    let dir = tempdir().expect("tempdir");
    let existing = dir.path().join("existing.txt");
    let added = dir.path().join("added.md");
    fs::write(&existing, "existing").expect("write existing file");
    fs::write(&added, "added").expect("write added file");
    let existing = fs::canonicalize(existing).expect("canonical existing");
    let added = fs::canonicalize(added).expect("canonical added");
    app.session_state.files = vec![existing.to_string_lossy().to_string()];

    append_local_attachments(
        &mut app,
        vec![existing.to_string_lossy().to_string(), added.to_string_lossy().to_string()],
    );

    assert_eq!(
        app.session_state.files,
        vec![existing.to_string_lossy().to_string(), added.to_string_lossy().to_string()]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn refresh_file_index_without_project_path_produces_no_messages() {
    let (app, _task) = App::new();

    let messages = collect_task_messages(refresh_file_index(&app)).await;

    assert!(messages.is_empty());
}

#[tokio::test]
async fn prepare_session_ui_tasks_return_none_for_empty_work_units() {
    let empty_chat: crate::app::session::SharedChatMessages = std::sync::Arc::from([]);

    let empty_chunk_messages = collect_task_messages(prepare_session_ui_task(
        "session-1".to_string(),
        empty_chat.clone(),
        0,
        true,
        false,
    ))
    .await;
    assert!(matches!(empty_chunk_messages.as_slice(), [crate::app::Message::None]));

    let empty_batch_messages = collect_task_messages(prepare_session_ui_chunks_task(
        "session-1".to_string(),
        empty_chat,
        Vec::new(),
        None,
        false,
    ))
    .await;
    assert!(empty_batch_messages.is_empty());
}

// 任务与结果：覆盖任务构造、调度或结果回写对公开状态的影响。
// 完成、失败与过期响应均按当前场景的既定契约处理。
#[tokio::test]
async fn prepare_stream_message_ui_task_prepares_only_target() {
    let message = crate::app::models::ChatMessage {
        role: crate::app::models::ChatRole::Assistant,
        content: "stream tail".to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    };

    let messages = collect_task_messages(prepare_stream_message_ui_task(
        "session-1".to_string(),
        message,
        47,
        9,
        false,
    ))
    .await;

    let [
        crate::app::Message::Project(crate::app::message::ProjectMessage::SessionUiPrepared {
            session_id,
            stream_generation,
            phase: crate::app::session::PreparedChatUiPhase::Message(prepared),
        }),
    ] = messages.as_slice()
    else {
        panic!("single-message task should return one prepared message")
    };
    assert_eq!(session_id, "session-1");
    assert_eq!(*stream_generation, Some(9));
    assert_eq!(prepared.message_idx, 47);
    assert_eq!(prepared.render_cache.display_text, "stream tail");
}

#[tokio::test(flavor = "multi_thread")]
async fn file_tree_empty_directory_async_refresh_retains_originating_project() {
    let directory = tempdir().unwrap();
    fs::create_dir_all(directory.path().join("empty/nested")).unwrap();
    fs::write(directory.path().join("readme.txt"), "content").unwrap();
    let root = directory.path().to_string_lossy().into_owned();
    let mut app = App::new().0;
    app.navigation_tabs.project_path = Some(root.clone());
    let task = refresh_file_index(&app);
    app.navigation_tabs.project_path = Some("/different-project".into());
    let messages = collect_task_messages(task).await;
    assert_eq!(messages.len(), 1);
    let crate::app::Message::Project(super::ProjectMessage::FileTreeSnapshotReady {
        project_path,
        snapshot,
    }) = &messages[0]
    else {
        panic!("expected scoped snapshot")
    };
    assert_eq!(project_path, &root);
    assert!(
        snapshot
            .files
            .contains(&directory.path().join("readme.txt").to_string_lossy().into_owned())
    );
    assert!(snapshot.files.iter().all(|path| std::path::Path::new(path).is_file()));
    assert_eq!(fs::read_to_string(directory.path().join("readme.txt")).unwrap(), "content");
    assert!(directory.path().join("empty/nested").is_dir());
}

// 宿主契约：后台搜索结果及本地化反馈必须携带原请求参数回到消息循环。
#[tokio::test(flavor = "multi_thread")]
async fn run_find_task_maps_request_and_localized_error_to_completion() {
    let messages = collect_task_messages(run_find_task(
        "tab".into(),
        "Search".into(),
        "/unused".into(),
        "(".into(),
        "replacement".into(),
        true,
        false,
        true,
        Vec::new(),
    ))
    .await;
    let [
        crate::app::Message::Project(
            crate::app::message::project::ProjectMessage::FileTreeFindCompleted {
                tab_id,
                title,
                scope_path,
                query,
                replace_text,
                case_sensitive,
                whole_word,
                use_regex,
                matches,
                error,
                localized_error,
                limit_reached,
            },
        ),
    ] = messages.as_slice()
    else {
        panic!("expected one find completion");
    };
    assert_eq!(tab_id, "tab");
    assert_eq!(title, "Search");
    assert_eq!(scope_path, "/unused");
    assert_eq!(query, "(");
    assert_eq!(replace_text, "replacement");
    assert!(*case_sensitive);
    assert!(!whole_word);
    assert!(*use_regex);
    assert!(matches.is_empty());
    assert!(!limit_reached);
    assert_eq!(
        error.as_deref(),
        localized_error.as_ref().map(|feedback| feedback.fallback.as_str())
    );
    assert!(localized_error.is_some());
}
