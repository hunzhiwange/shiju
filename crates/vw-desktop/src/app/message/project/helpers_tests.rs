// 验证附件路径约束、图像快照及文件名清理辅助逻辑。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::{
    append_local_attachments,
    collect_local_attachments, copy_recursively,
    find_session_project_path, is_supported_image_attachment, map_gateway_project,
    map_loaded_project_info,
    prepare_session_ui_chunks_task, prepare_session_ui_task, prepare_stream_message_ui_task,
    refresh_file_index, relative_to_project, run_find_task,
    unique_name_in_dir,
};
#[cfg(not(target_arch = "wasm32"))]
use super::persist_clipboard_image_attachment;
use crate::app::App;
use std::fs;
use tempfile::tempdir;
use vw_api_transport::common::TimestampMs;
use vw_api_file::file::{FileNodeDto, FileNodeKind};
use vw_api_transport::id::ProjectId;
use vw_api_project::{ProjectDto, ProjectGitStateDto, ProjectStatus};

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
#[test]
fn supported_image_attachment_detection_is_case_insensitive() {
    for path in ["a.png", "b.JPG", "c.jpeg", "d.WEBP", "e.gif", "f.BmP"] {
        assert!(is_supported_image_attachment(std::path::Path::new(path)), "{path}");
    }

    assert!(!is_supported_image_attachment(std::path::Path::new("notes.txt")));
    assert!(!is_supported_image_attachment(std::path::Path::new("no-extension")));
}

#[test]
fn collect_local_attachments_skips_empty_missing_folders_and_duplicates() {
    let (mut app, _task) = App::new();
    let dir = tempdir().expect("tempdir");
    let file = dir.path().join("notes.txt");
    fs::write(&file, "hello").expect("write file");
    let existing = fs::canonicalize(&file).expect("canonical file").to_string_lossy().to_string();
    app.files = vec![existing.clone()];

    let (accepted, errors) = collect_local_attachments(
        &app,
        vec![
            "   ".to_string(),
            file.to_string_lossy().to_string(),
            dir.path().to_string_lossy().to_string(),
            dir.path().join("missing.txt").to_string_lossy().to_string(),
        ],
    );

    assert!(accepted.is_empty());
    assert!(errors.iter().any(|error| error.fallback.contains("不支持文件夹")));
    assert!(errors.iter().any(|error| error.fallback.contains("读取失败")));
}

#[test]
fn collect_local_attachments_accepts_non_image_files_once() {
    let (app, _task) = App::new();
    let dir = tempdir().expect("tempdir");
    let first = dir.path().join("a.txt");
    let second = dir.path().join("b.md");
    fs::write(&first, "a").expect("write first");
    fs::write(&second, "b").expect("write second");

    let (accepted, errors) = collect_local_attachments(
        &app,
        vec![
            first.to_string_lossy().to_string(),
            first.to_string_lossy().to_string(),
            second.to_string_lossy().to_string(),
        ],
    );

    assert!(errors.is_empty());
    assert_eq!(accepted.len(), 2);
    assert!(accepted.iter().any(|path| path.ends_with("a.txt")));
    assert!(accepted.iter().any(|path| path.ends_with("b.md")));
}

#[test]
fn collect_local_attachments_reports_image_limit_before_copying() {
    let (mut app, _task) = App::new();
    app.multimodal_settings.max_images = 1;
    app.files = vec!["already.png".to_string()];
    let dir = tempdir().expect("tempdir");
    let image = dir.path().join("new.png");
    fs::write(&image, b"png").expect("write image");

    let (accepted, errors) =
        collect_local_attachments(&app, vec![image.to_string_lossy().to_string()]);

    assert!(accepted.is_empty());
    assert_eq!(errors.len(), 1);
    assert!(errors[0].fallback.contains("已达图片上限"));
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn collect_local_attachments_rejects_oversized_images() {
    let (mut app, _task) = App::new();
    app.multimodal_settings.max_image_size_mb = 1;
    let dir = tempdir().expect("tempdir");
    let image = dir.path().join("large.png");
    fs::write(&image, vec![0_u8; 1024 * 1024 + 1]).expect("write oversized image");

    let (accepted, errors) =
        collect_local_attachments(&app, vec![image.to_string_lossy().to_string()]);

    assert!(accepted.is_empty());
    assert_eq!(errors.len(), 1);
    assert!(errors[0].fallback.contains("图片过大"));
    assert!(errors[0].fallback.contains("large.png"));
}

#[test]
fn collect_local_attachments_keeps_project_image_path_without_snapshot_copy() {
    let (mut app, _task) = App::new();
    let project = tempdir().expect("project tempdir");
    let image = project.path().join("diagram.png");
    fs::write(&image, b"png").expect("write image");
    app.project_path = Some(project.path().to_string_lossy().to_string());
    app.multimodal_settings.max_images = 16;
    app.multimodal_settings.max_image_size_mb = 1;

    let (accepted, errors) =
        collect_local_attachments(&app, vec![image.to_string_lossy().to_string()]);

    assert!(errors.is_empty());
    assert_eq!(accepted, vec![fs::canonicalize(image).unwrap().to_string_lossy().to_string()]);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn persist_clipboard_image_attachment_rejects_invalid_rgba_buffer() {
    let error = persist_clipboard_image_attachment(1, 1, &[255, 0, 0])
        .expect_err("invalid rgba buffer should fail");

    assert_eq!(error.fallback, "剪贴板图片数据无效");
}

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
    app.files = vec![existing.to_string_lossy().to_string()];

    append_local_attachments(
        &mut app,
        vec![existing.to_string_lossy().to_string(), added.to_string_lossy().to_string()],
    );

    assert_eq!(
        app.files,
        vec![existing.to_string_lossy().to_string(), added.to_string_lossy().to_string()]
    );
}

#[test]
fn unique_name_in_dir_returns_original_or_copy_suffix() {
    let dir = tempdir().expect("tempdir");
    assert_eq!(unique_name_in_dir(dir.path(), "note.txt"), dir.path().join("note.txt"));

    fs::write(dir.path().join("note.txt"), "original").expect("write original");
    assert_eq!(unique_name_in_dir(dir.path(), "note.txt"), dir.path().join("note copy.txt"));

    fs::write(dir.path().join("note copy.txt"), "copy").expect("write copy");
    assert_eq!(unique_name_in_dir(dir.path(), "note.txt"), dir.path().join("note copy 2.txt"));
}

#[test]
fn copy_recursively_copies_nested_directories_and_files() {
    let src = tempdir().expect("src");
    let dst = tempdir().expect("dst");
    fs::create_dir_all(src.path().join("nested")).expect("create nested");
    fs::write(src.path().join("root.txt"), "root").expect("write root");
    fs::write(src.path().join("nested/child.txt"), "child").expect("write child");
    let target = dst.path().join("copied");

    copy_recursively(src.path(), &target).expect("copy recursively");

    assert_eq!(fs::read_to_string(target.join("root.txt")).unwrap(), "root");
    assert_eq!(fs::read_to_string(target.join("nested/child.txt")).unwrap(), "child");
}

#[test]
fn copy_recursively_creates_parent_for_single_file_target() {
    let src = tempdir().expect("src");
    let dst = tempdir().expect("dst");
    let source_file = src.path().join("source.txt");
    fs::write(&source_file, "body").expect("write source");
    let target_file = dst.path().join("missing/target.txt");

    copy_recursively(&source_file, &target_file).expect("copy file");

    assert_eq!(fs::read_to_string(target_file).unwrap(), "body");
}

#[test]
fn copy_recursively_reports_missing_source() {
    let src = tempdir().expect("src");
    let dst = tempdir().expect("dst");

    let error = copy_recursively(&src.path().join("missing.txt"), &dst.path().join("target.txt"))
        .expect_err("missing source must fail");

    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
}

#[test]
fn find_session_project_path_falls_back_to_active_project() {
    let (mut app, _task) = App::new();
    app.project_path = Some("/active/project".to_string());

    assert_eq!(
        find_session_project_path(&app, "missing-session"),
        Some("/active/project".to_string())
    );

    app.project_path = None;
    assert_eq!(find_session_project_path(&app, "missing-session"), None);
}

#[test]
fn relative_to_project_handles_exact_nested_and_outside_paths() {
    assert_eq!(relative_to_project("/tmp/project", "/tmp/project"), Some(".".to_string()));
    assert_eq!(
        relative_to_project("/tmp/project", "/tmp/project/src/main.rs"),
        Some("src/main.rs".to_string())
    );
    assert_eq!(relative_to_project("/tmp/project", "/tmp/other/main.rs"), None);
}

#[test]
fn relative_to_project_normalizes_windows_separators_without_filesystem_access() {
    assert_eq!(
        relative_to_project(r"C:\work\project\", r"C:\work\project\src\main.rs"),
        Some("src/main.rs".to_string())
    );
}

#[test]
fn relative_to_project_uses_canonical_paths_when_string_prefix_fails() {
    let dir = tempdir().expect("tempdir");
    let nested = dir.path().join("nested");
    fs::create_dir_all(&nested).expect("create nested");
    let file = nested.join("file.txt");
    fs::write(&file, "body").expect("write file");
    let root_with_dot = dir.path().join(".");

    assert_eq!(
        relative_to_project(&root_with_dot.to_string_lossy(), &file.to_string_lossy()),
        Some("nested/file.txt".to_string())
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn refresh_file_index_without_project_path_produces_no_messages() {
    let (app, _task) = App::new();

    let messages = collect_task_messages(refresh_file_index(&app)).await;

    assert!(messages.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn run_find_task_plain_search_filters_scope_and_reports_match_positions() {
    let dir = tempdir().expect("tempdir");
    let scoped = dir.path().join("scoped");
    let outside = dir.path().join("outside");
    fs::create_dir_all(&scoped).expect("create scope");
    fs::create_dir_all(&outside).expect("create outside");
    let included = scoped.join("included.txt");
    let excluded = outside.join("excluded.txt");
    let included_path = included.to_string_lossy().to_string();
    fs::write(&included, "Alpha alpha\nno match").expect("write included");
    fs::write(&excluded, "alpha").expect("write excluded");

    let messages = collect_task_messages(run_find_task(
        "tab".to_string(),
        "Search".to_string(),
        scoped.to_string_lossy().to_string(),
        "alpha".to_string(),
        "replacement".to_string(),
        false,
        false,
        false,
        vec![included_path.clone(), excluded.to_string_lossy().to_string()],
    ))
    .await;

    let crate::app::Message::Project(
        crate::app::message::project::ProjectMessage::FileTreeFindCompleted {
            matches,
            error,
            limit_reached,
            ..
        },
    ) = &messages[0]
    else {
        panic!("expected find completion message");
    };
    assert!(error.is_none());
    assert!(!limit_reached);
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].line, 1);
    assert_eq!(matches[0].column, 1);
    assert_eq!(matches[1].column, 7);
    assert!(matches.iter().all(|item| item.path == included_path));
}

#[tokio::test(flavor = "multi_thread")]
async fn run_find_task_whole_word_skips_identifier_substrings() {
    let dir = tempdir().expect("tempdir");
    let file = dir.path().join("words.txt");
    fs::write(&file, "cat scatter cat_ cat-cat").expect("write words");

    let messages = collect_task_messages(run_find_task(
        "tab".to_string(),
        "Search".to_string(),
        dir.path().to_string_lossy().to_string(),
        "cat".to_string(),
        String::new(),
        true,
        true,
        false,
        vec![file.to_string_lossy().to_string()],
    ))
    .await;

    let crate::app::Message::Project(
        crate::app::message::project::ProjectMessage::FileTreeFindCompleted {
            matches, error, ..
        },
    ) = &messages[0]
    else {
        panic!("expected find completion message");
    };
    assert!(error.is_none());
    assert_eq!(matches.iter().map(|item| item.column).collect::<Vec<_>>(), [1, 18, 22]);
}

#[tokio::test(flavor = "multi_thread")]
async fn run_find_task_invalid_regex_returns_error_without_matches() {
    let messages = collect_task_messages(run_find_task(
        "tab".to_string(),
        "Search".to_string(),
        "/unused".to_string(),
        "(".to_string(),
        String::new(),
        true,
        false,
        true,
        Vec::new(),
    ))
    .await;

    let crate::app::Message::Project(
        crate::app::message::project::ProjectMessage::FileTreeFindCompleted {
            matches,
            error,
            limit_reached,
            ..
        },
    ) = &messages[0]
    else {
        panic!("expected find completion message");
    };
    assert!(matches.is_empty());
    assert!(error.as_deref().is_some_and(|value| value.starts_with("正则表达式错误:")));
    assert!(!limit_reached);
}

#[tokio::test]
async fn run_find_task_regex_whole_word_reports_limit_reached() {
    let dir = tempdir().expect("tempdir");
    let file = dir.path().join("many.txt");
    let content = (0..3005).map(|_| "cat scatter cat_").collect::<Vec<_>>().join("\n");
    fs::write(&file, content).expect("write many matches");

    let messages = collect_task_messages(run_find_task(
        "tab".to_string(),
        "Search".to_string(),
        dir.path().to_string_lossy().to_string(),
        "cat".to_string(),
        String::new(),
        false,
        true,
        true,
        vec![file.to_string_lossy().to_string()],
    ))
    .await;

    let crate::app::Message::Project(
        crate::app::message::project::ProjectMessage::FileTreeFindCompleted {
            matches,
            error,
            limit_reached,
            ..
        },
    ) = &messages[0]
    else {
        panic!("expected find completion message");
    };
    assert!(error.is_none());
    assert_eq!(matches.len(), 3000);
    assert!(matches.iter().all(|item| item.match_len == 3));
    assert!(*limit_reached);
}

#[test]
fn map_gateway_project_converts_git_and_time_fields() {
    let info = map_gateway_project(project_dto(Some("main")));

    assert_eq!(info.id, "project-id");
    assert_eq!(info.worktree, "/project");
    assert_eq!(info.name.as_deref(), Some("Gateway Project"));
    assert!(matches!(info.vcs, Some(project_types::Vcs::Git)));
    assert_eq!(info.time.created, 0);
    assert_eq!(info.time.updated, 5000);
    assert_eq!(info.sandboxes, vec!["/project".to_string()]);
}

#[test]
fn map_gateway_project_omits_vcs_for_non_repository() {
    let mut project = project_dto(None);
    project.git.is_repo = false;

    let info = map_gateway_project(project);

    assert!(info.vcs.is_none());
}

#[test]
fn map_loaded_project_info_drops_blank_current_branch() {
    let loaded = map_loaded_project_info("/current".to_string(), project_dto(Some("  ")));

    assert_eq!(loaded.project_path, "/current");
    assert_eq!(loaded.info.id, "project-id");
    assert!(loaded.current_branch.is_none());
}

#[test]
fn map_loaded_project_info_keeps_non_blank_current_branch() {
    let loaded = map_loaded_project_info("/current".to_string(), project_dto(Some("feature")));

    assert_eq!(loaded.current_branch.as_deref(), Some("feature"));
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

fn long_chat_chunk_snapshot() -> super::ChatUiChunkSnapshot {
    let chat: crate::app::session::SharedChatMessages = std::sync::Arc::from(
        (0..10_000)
            .map(|idx| crate::app::models::ChatMessage {
                role: crate::app::models::ChatRole::Assistant,
                content: format!("message-{idx}"),
                model: None,
                think_timing: Vec::new(),
                processing_timing: None,
            })
            .collect::<Vec<_>>(),
    );

    super::chat_ui_chunk_snapshot(&chat, 9_952)
        .expect("the requested history chunk should have a bounded snapshot")
}

#[test]
fn chat_ui_chunk_snapshots_requested_32_only() {
    let snapshot = long_chat_chunk_snapshot();

    assert_eq!(snapshot.chunk_start_idx, 9_952);
    assert_eq!(snapshot.messages.len(), crate::app::session::CHAT_UI_CHUNK_SIZE);
    assert_eq!(snapshot.messages[0].content, "message-9952");
    assert_eq!(snapshot.messages[31].content, "message-9983");
}

#[test]
fn prepare_session_ui_does_not_clone_full_history() {
    let snapshot = long_chat_chunk_snapshot();

    assert_eq!(snapshot.messages.len(), 32);
    assert!(snapshot.messages.len() < 10_000);
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

fn project_dto(current_branch: Option<&str>) -> ProjectDto {
    ProjectDto {
        id: ProjectId("project-id".to_string()),
        name: "Gateway Project".to_string(),
        directory: "/project".to_string(),
        display_path: "/project".to_string(),
        status: ProjectStatus::Ready,
        created_at_ms: TimestampMs(-100),
        updated_at_ms: TimestampMs(5000),
        default_branch: Some("main".to_string()),
        current_branch: current_branch.map(ToString::to_string),
        git: ProjectGitStateDto {
            is_repo: true,
            has_uncommitted_changes: false,
            ahead: None,
            behind: None,
        },
        active_worktree_id: None,
        session_count: Some(2),
    }
}

#[test]
fn file_tree_empty_directory_snapshot_keeps_only_regular_files_in_index() {
    let dto = FileNodeDto {
        path: ".".into(),
        name: ".".into(),
        kind: FileNodeKind::Directory,
        size_bytes: None,
        children: Some(vec![
            FileNodeDto {
                path: "empty/nested".into(),
                name: "nested".into(),
                kind: FileNodeKind::Directory,
                size_bytes: None,
                children: None,
            },
            FileNodeDto {
                path: "readme.txt".into(),
                name: "readme.txt".into(),
                kind: FileNodeKind::File,
                size_bytes: Some(4),
                children: None,
            },
        ]),
    };
    let snapshot = super::FileTreeIndexSnapshot::from_dto("/project", &dto);
    assert_eq!(snapshot.files, ["/project/readme.txt"]);
    assert!(snapshot.tree.children["empty"].children.contains_key("nested"));
    assert_eq!(snapshot.tree.files, ["readme.txt"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn file_tree_empty_directory_async_refresh_retains_originating_project() {
    let directory = tempdir().unwrap();
    fs::create_dir_all(directory.path().join("empty/nested")).unwrap();
    fs::write(directory.path().join("readme.txt"), "content").unwrap();
    let root = directory.path().to_string_lossy().into_owned();
    let mut app = App::new().0;
    app.project_path = Some(root.clone());
    let task = refresh_file_index(&app);
    app.project_path = Some("/different-project".into());
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
