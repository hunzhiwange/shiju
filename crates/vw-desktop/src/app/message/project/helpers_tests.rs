use super::{
    append_local_attachments, attachment_name, build_image_attachment_snapshot_path,
    collect_gateway_file_paths, collect_local_attachments, copy_recursively,
    find_session_project_path, is_supported_image_attachment, is_word_byte, map_gateway_project,
    map_loaded_project_info, map_timestamp, normalize_path, path_within_dir,
    prepare_session_ui_chunks_task, prepare_session_ui_task, refresh_file_index,
    relative_to_project, resolve_absolute_gateway_path, run_find_task, sanitize_attachment_stem,
    stabilize_image_attachment, unique_name_in_dir,
};
#[cfg(not(target_arch = "wasm32"))]
use super::{build_clipboard_image_attachment_path, persist_clipboard_image_attachment};
use crate::app::App;
use std::fs;
use tempfile::tempdir;
use vw_gateway_client::vw_api_types::common::TimestampMs;
use vw_gateway_client::vw_api_types::file::{FileNodeDto, FileNodeKind};
use vw_gateway_client::vw_api_types::id::ProjectId;
use vw_gateway_client::vw_api_types::project::{ProjectDto, ProjectGitStateDto, ProjectStatus};

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

#[test]
fn supported_image_attachment_detection_is_case_insensitive() {
    for path in ["a.png", "b.JPG", "c.jpeg", "d.WEBP", "e.gif", "f.BmP"] {
        assert!(is_supported_image_attachment(std::path::Path::new(path)), "{path}");
    }

    assert!(!is_supported_image_attachment(std::path::Path::new("notes.txt")));
    assert!(!is_supported_image_attachment(std::path::Path::new("no-extension")));
}

#[test]
fn sanitize_attachment_stem_replaces_unsafe_characters_and_falls_back() {
    assert_eq!(sanitize_attachment_stem(std::path::Path::new("screen shot!.png")), "screen-shot");
    assert_eq!(sanitize_attachment_stem(std::path::Path::new("你好.png")), "image");
    assert_eq!(sanitize_attachment_stem(std::path::Path::new("!!!.png")), "image");
}

#[test]
fn attachment_name_uses_file_name_or_display_path() {
    assert_eq!(attachment_name(std::path::Path::new("/tmp/capture.png")), "capture.png");
    assert_eq!(attachment_name(std::path::Path::new("/")), "/");
}

#[test]
fn path_within_dir_accepts_descendants_and_rejects_siblings_or_missing_paths() {
    let parent = tempdir().expect("parent tempdir");
    let sibling = tempdir().expect("sibling tempdir");
    let nested = parent.path().join("nested/file.txt");
    fs::create_dir_all(nested.parent().expect("nested parent")).expect("create nested parent");
    fs::write(&nested, "body").expect("write nested file");

    assert!(path_within_dir(&nested, parent.path()));
    assert!(!path_within_dir(&nested, sibling.path()));
    assert!(!path_within_dir(&parent.path().join("missing.txt"), parent.path()));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn clipboard_snapshot_path_is_content_addressed_png() {
    let snapshot_dir = tempdir().expect("snapshot tempdir");

    let first = build_clipboard_image_attachment_path(1, 1, &[1, 2, 3, 4], snapshot_dir.path());
    let repeated = build_clipboard_image_attachment_path(1, 1, &[1, 2, 3, 4], snapshot_dir.path());
    let changed = build_clipboard_image_attachment_path(1, 1, &[4, 3, 2, 1], snapshot_dir.path());

    assert_eq!(first, repeated);
    assert_ne!(first, changed);
    assert_eq!(first.extension().and_then(|value| value.to_str()), Some("png"));
    assert!(first.file_name().unwrap().to_string_lossy().starts_with("clipboard-"));
}

#[test]
fn external_image_is_copied_into_snapshot_dir() {
    let workspace_dir = tempdir().expect("workspace tempdir");
    let external_dir = tempdir().expect("external tempdir");
    let snapshot_dir = tempdir().expect("snapshot tempdir");
    let source = external_dir.path().join("screen shot.png");
    fs::write(&source, b"png-bytes").expect("write source image");
    let metadata = fs::metadata(&source).expect("metadata");

    let copied = stabilize_image_attachment(
        &source,
        &metadata,
        Some(workspace_dir.path()),
        snapshot_dir.path(),
    )
    .expect("stabilize image");

    assert_ne!(copied, source);
    assert!(copied.starts_with(snapshot_dir.path()));
    assert_eq!(fs::read(&copied).expect("read copied"), b"png-bytes");
}

#[test]
fn workspace_image_keeps_original_path() {
    let workspace_dir = tempdir().expect("workspace tempdir");
    let snapshot_dir = tempdir().expect("snapshot tempdir");
    let source = workspace_dir.path().join("diagram.png");
    fs::write(&source, b"workspace-image").expect("write workspace image");
    let metadata = fs::metadata(&source).expect("metadata");

    let kept = stabilize_image_attachment(
        &source,
        &metadata,
        Some(workspace_dir.path()),
        snapshot_dir.path(),
    )
    .expect("stabilize image");

    assert_eq!(kept, source);
}

#[test]
fn snapshot_path_is_stable_for_same_source_version() {
    let snapshot_dir = tempdir().expect("snapshot tempdir");
    let external_dir = tempdir().expect("external tempdir");
    let source = external_dir.path().join("capture.png");
    fs::write(&source, b"stable-bytes").expect("write source image");
    let metadata = fs::metadata(&source).expect("metadata");

    let first = build_image_attachment_snapshot_path(&source, &metadata, snapshot_dir.path());
    let second = build_image_attachment_snapshot_path(&source, &metadata, snapshot_dir.path());

    assert_eq!(first, second);
}

#[test]
fn snapshot_path_uses_img_extension_when_source_has_no_extension() {
    let snapshot_dir = tempdir().expect("snapshot tempdir");
    let external_dir = tempdir().expect("external tempdir");
    let source = external_dir.path().join("clipboard");
    fs::write(&source, b"raw-bytes").expect("write source image");
    let metadata = fs::metadata(&source).expect("metadata");

    let path = build_image_attachment_snapshot_path(&source, &metadata, snapshot_dir.path());

    assert_eq!(path.extension().and_then(|value| value.to_str()), Some("img"));
    assert!(path.file_name().unwrap().to_string_lossy().starts_with("clipboard-"));
}

#[test]
fn image_already_inside_snapshot_dir_keeps_original_path() {
    let snapshot_dir = tempdir().expect("snapshot tempdir");
    let source = snapshot_dir.path().join("existing.png");
    fs::write(&source, b"snapshot-image").expect("write snapshot image");
    let metadata = fs::metadata(&source).expect("metadata");

    let kept =
        stabilize_image_attachment(&source, &metadata, None, snapshot_dir.path()).expect("kept");

    assert_eq!(kept, source);
}

#[test]
fn stabilize_image_attachment_reports_snapshot_directory_creation_failure() {
    let external_dir = tempdir().expect("external tempdir");
    let source = external_dir.path().join("capture.png");
    let invalid_snapshot_dir = external_dir.path().join("not-a-directory");
    fs::write(&source, b"png-bytes").expect("write source image");
    fs::write(&invalid_snapshot_dir, b"occupied").expect("write blocking file");
    let metadata = fs::metadata(&source).expect("metadata");

    let error = stabilize_image_attachment(&source, &metadata, None, &invalid_snapshot_dir)
        .expect_err("file cannot be used as snapshot directory");

    assert!(error.contains("创建图片附件目录失败"));
    assert!(error.contains(&invalid_snapshot_dir.display().to_string()));
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
    assert!(errors.iter().any(|error| error.contains("不支持文件夹")));
    assert!(errors.iter().any(|error| error.contains("读取失败")));
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
    assert!(errors[0].contains("已达图片上限"));
}

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
    assert!(errors[0].contains("图片过大"));
    assert!(errors[0].contains("large.png"));
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

    assert_eq!(error, "剪贴板图片数据无效");
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

#[test]
fn word_byte_accepts_ascii_identifier_bytes_only() {
    for byte in [b'a', b'Z', b'0', b'_'] {
        assert!(is_word_byte(byte));
    }
    for byte in [b'-', b' ', b'.', 0xFF] {
        assert!(!is_word_byte(byte));
    }
}

#[test]
fn resolve_absolute_gateway_path_handles_empty_dot_and_relative_paths() {
    assert_eq!(resolve_absolute_gateway_path("/project", ""), "/project");
    assert_eq!(resolve_absolute_gateway_path("/project", "."), "/project");
    assert_eq!(resolve_absolute_gateway_path("/project", "src/lib.rs"), "/project/src/lib.rs");
}

#[test]
fn normalize_path_rewrites_separators_and_trims_trailing_slashes() {
    assert_eq!(normalize_path(r"C:\work\project\"), "C:/work/project");
    assert_eq!(normalize_path("/tmp/project///"), "/tmp/project");
}

#[test]
fn normalize_path_handles_empty_and_root_only_inputs() {
    assert_eq!(normalize_path(""), "");
    assert_eq!(normalize_path("///"), "");
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
fn collect_gateway_file_paths_recurses_directories_and_sorts_by_caller() {
    let tree = FileNodeDto {
        path: ".".to_string(),
        name: ".".to_string(),
        kind: FileNodeKind::Directory,
        size_bytes: None,
        children: Some(vec![
            FileNodeDto {
                path: "src".to_string(),
                name: "src".to_string(),
                kind: FileNodeKind::Directory,
                size_bytes: None,
                children: Some(vec![FileNodeDto {
                    path: "src/lib.rs".to_string(),
                    name: "lib.rs".to_string(),
                    kind: FileNodeKind::File,
                    size_bytes: Some(3),
                    children: None,
                }]),
            },
            FileNodeDto {
                path: "README.md".to_string(),
                name: "README.md".to_string(),
                kind: FileNodeKind::File,
                size_bytes: Some(4),
                children: None,
            },
        ]),
    };
    let mut paths = Vec::new();

    collect_gateway_file_paths("/project", &tree, &mut paths);
    paths.sort();

    assert_eq!(paths, vec!["/project/README.md".to_string(), "/project/src/lib.rs".to_string()]);
}

#[test]
fn collect_gateway_file_paths_ignores_directories_without_loaded_children() {
    let tree = FileNodeDto {
        path: "src".to_string(),
        name: "src".to_string(),
        kind: FileNodeKind::Directory,
        size_bytes: None,
        children: None,
    };
    let mut paths = vec!["already-present".to_string()];

    collect_gateway_file_paths("/project", &tree, &mut paths);

    assert_eq!(paths, vec!["already-present".to_string()]);
}

#[test]
fn map_timestamp_clamps_negative_values_to_zero() {
    assert_eq!(map_timestamp(-1), 0);
    assert_eq!(map_timestamp(42), 42);
}

#[test]
fn map_gateway_project_converts_git_and_time_fields() {
    let info = map_gateway_project(project_dto(Some("main")));

    assert_eq!(info.id, "project-id");
    assert_eq!(info.worktree, "/project");
    assert_eq!(info.name.as_deref(), Some("Gateway Project"));
    assert!(matches!(info.vcs, Some(vw_shared::project::Vcs::Git)));
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
