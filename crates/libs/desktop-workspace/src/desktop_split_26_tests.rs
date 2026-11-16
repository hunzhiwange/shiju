// 从宿主 project/helpers_tests.rs 迁入；保留函数名、断言与平台 cfg。
#[cfg(not(target_arch = "wasm32"))]
use crate::project_message::attachments::persist_clipboard_image_attachment;
use crate::project_message::{
    attachments::{collect_local_attachments, is_supported_image_attachment},
    data::{FileTreeIndexSnapshot, map_gateway_project},
    paths::{copy_recursively, relative_to_project, unique_name_in_dir},
};
use std::fs;
use tempfile::tempdir;
use vw_api_file::file::{FileNodeDto, FileNodeKind};
use vw_api_project::{ProjectDto, ProjectGitStateDto, ProjectStatus};
use vw_api_transport::{common::TimestampMs, id::ProjectId};

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
    let files;
    let project_path: Option<String> = None;
    let max_images = 16;
    let max_image_size_mb = 20;
    let dir = tempdir().expect("tempdir");
    let file = dir.path().join("notes.txt");
    fs::write(&file, "hello").expect("write file");
    let existing = fs::canonicalize(&file).expect("canonical file").to_string_lossy().to_string();
    files = vec![existing.clone()];

    let (accepted, errors) = collect_local_attachments(
        &files,
        project_path.as_deref(),
        max_images,
        max_image_size_mb,
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
    let files = Vec::<String>::new();
    let project_path: Option<String> = None;
    let max_images = 16;
    let max_image_size_mb = 20;
    let dir = tempdir().expect("tempdir");
    let first = dir.path().join("a.txt");
    let second = dir.path().join("b.md");
    fs::write(&first, "a").expect("write first");
    fs::write(&second, "b").expect("write second");

    let (accepted, errors) = collect_local_attachments(
        &files,
        project_path.as_deref(),
        max_images,
        max_image_size_mb,
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
    let files;
    let project_path: Option<String> = None;
    let max_image_size_mb = 20;
    let max_images = 1;
    files = vec!["already.png".to_string()];
    let dir = tempdir().expect("tempdir");
    let image = dir.path().join("new.png");
    fs::write(&image, b"png").expect("write image");

    let (accepted, errors) = collect_local_attachments(
        &files,
        project_path.as_deref(),
        max_images,
        max_image_size_mb,
        vec![image.to_string_lossy().to_string()],
    );

    assert!(accepted.is_empty());
    assert_eq!(errors.len(), 1);
    assert!(errors[0].fallback.contains("已达图片上限"));
}

#[test]
fn collect_local_attachments_rejects_oversized_images() {
    let files = Vec::<String>::new();
    let project_path: Option<String> = None;
    let max_images = 16;
    let max_image_size_mb = 1;
    let dir = tempdir().expect("tempdir");
    let image = dir.path().join("large.png");
    fs::write(&image, vec![0_u8; 1024 * 1024 + 1]).expect("write oversized image");

    let (accepted, errors) = collect_local_attachments(
        &files,
        project_path.as_deref(),
        max_images,
        max_image_size_mb,
        vec![image.to_string_lossy().to_string()],
    );

    assert!(accepted.is_empty());
    assert_eq!(errors.len(), 1);
    assert!(errors[0].fallback.contains("图片过大"));
    assert!(errors[0].fallback.contains("large.png"));
}

#[test]
fn collect_local_attachments_keeps_project_image_path_without_snapshot_copy() {
    let files = Vec::<String>::new();
    let project = tempdir().expect("project tempdir");
    let image = project.path().join("diagram.png");
    fs::write(&image, b"png").expect("write image");
    let project_path = Some(project.path().to_string_lossy().to_string());
    let max_images = 16;
    let max_image_size_mb = 1;

    let (accepted, errors) = collect_local_attachments(
        &files,
        project_path.as_deref(),
        max_images,
        max_image_size_mb,
        vec![image.to_string_lossy().to_string()],
    );

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
    let snapshot = FileTreeIndexSnapshot::from_dto("/project", &dto);
    assert_eq!(snapshot.files, ["/project/readme.txt"]);
    assert!(snapshot.tree.children["empty"].children.contains_key("nested"));
    assert_eq!(snapshot.tree.files, ["readme.txt"]);
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
fn run_find_task_plain_search_filters_scope_and_reports_match_positions() {
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

    let (matches, error, limit_reached) = crate::project_message::search::search(
        scoped.to_string_lossy().to_string(),
        "alpha".to_string(),
        false,
        false,
        false,
        vec![included_path.clone(), excluded.to_string_lossy().to_string()],
    );

    assert!(error.is_none());
    assert!(!limit_reached);
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].line, 1);
    assert_eq!(matches[0].column, 1);
    assert_eq!(matches[1].column, 7);
    assert!(matches.iter().all(|item| item.path == included_path));
}

#[test]
fn run_find_task_whole_word_skips_identifier_substrings() {
    let dir = tempdir().expect("tempdir");
    let file = dir.path().join("words.txt");
    fs::write(&file, "cat scatter cat_ cat-cat").expect("write words");

    let (matches, error, _limit_reached) = crate::project_message::search::search(
        dir.path().to_string_lossy().to_string(),
        "cat".to_string(),
        true,
        true,
        false,
        vec![file.to_string_lossy().to_string()],
    );

    assert!(error.is_none());
    assert_eq!(matches.iter().map(|item| item.column).collect::<Vec<_>>(), [1, 18, 22]);
}

#[test]
fn run_find_task_invalid_regex_returns_error_without_matches() {
    let (matches, error, limit_reached) = crate::project_message::search::search(
        "/unused".to_string(),
        "(".to_string(),
        true,
        false,
        true,
        Vec::new(),
    );

    assert!(matches.is_empty());
    assert!(
        error
            .as_ref()
            .map(|feedback| feedback.fallback.as_str())
            .is_some_and(|value| value.starts_with("正则表达式错误:"))
    );
    assert!(!limit_reached);
}

#[test]
fn run_find_task_regex_whole_word_reports_limit_reached() {
    let dir = tempdir().expect("tempdir");
    let file = dir.path().join("many.txt");
    let content = (0..3005).map(|_| "cat scatter cat_").collect::<Vec<_>>().join("\n");
    fs::write(&file, content).expect("write many matches");

    let (matches, error, limit_reached) = crate::project_message::search::search(
        dir.path().to_string_lossy().to_string(),
        "cat".to_string(),
        false,
        true,
        true,
        vec![file.to_string_lossy().to_string()],
    );

    assert!(error.is_none());
    assert_eq!(matches.len(), 3000);
    assert!(matches.iter().all(|item| item.match_len == 3));
    assert!(limit_reached);
}
