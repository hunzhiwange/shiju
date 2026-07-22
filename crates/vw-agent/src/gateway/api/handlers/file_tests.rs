use super::*;
use crate::app::agent::project;

#[test]
fn router_builds_with_unit_state() {
    let _ = router::<()>();
}

#[test]
fn file_write_body_defaults_create_if_missing_to_false() {
    let body: FileWriteBody = serde_json::from_value(serde_json::json!({
        "path": "notes/todo.md",
        "content": "hello"
    }))
    .expect("valid body");

    assert_eq!(body.path, "notes/todo.md");
    assert_eq!(body.content, "hello");
    assert!(!body.create_if_missing);
}

#[test]
fn response_types_serialize_stable_json_fields() {
    let read = serde_json::to_value(FileReadBodyResponse {
        root_directory: "/workspace".to_string(),
        path: "notes/todo.md".to_string(),
        content: "hello".to_string(),
    })
    .expect("serialize read response");
    assert_eq!(
        read,
        serde_json::json!({
            "root_directory": "/workspace",
            "path": "notes/todo.md",
            "content": "hello"
        })
    );

    let write = serde_json::to_value(FileWriteBodyResponse {
        ok: true,
        root_directory: "/workspace".to_string(),
        path: "notes/todo.md".to_string(),
        bytes_written: 5,
    })
    .expect("serialize write response");
    assert_eq!(write["ok"], true);
    assert_eq!(write["bytes_written"], 5);

    let ripgrep_match = serde_json::to_value(RipgrepMatch {
        path: "src/main.rs".to_string(),
        line_number: 7,
        line: "needle".to_string(),
        start: 1,
        end: 4,
    })
    .expect("serialize ripgrep match");
    assert_eq!(ripgrep_match["lineNumber"], 7);
    assert!(ripgrep_match.get("line_number").is_none());
}

#[test]
fn resolve_workspace_path_rejects_absolute_path_outside_root() {
    let root = std::path::PathBuf::from("/tmp/vibewindow-root");
    let result = resolve_workspace_path(&root, "/etc/passwd");

    assert!(result.is_err());
}

#[test]
fn find_file_query_defaults_are_optional() {
    let query: FindFileQuery = serde_json::from_value(serde_json::json!({
        "query": "main"
    }))
    .expect("valid query");

    assert_eq!(query.query, "main");
    assert!(query.directory.is_none());
    assert!(query.dirs.is_none());
    assert!(query.r#type.is_none());
    assert!(query.limit.is_none());
}

#[test]
fn contains_path_handles_existing_and_future_children() {
    let temp = tempfile::tempdir().expect("temp dir");
    let child = temp.path().join("child.txt");
    std::fs::write(&child, "child").expect("child");

    assert!(contains_path(temp.path(), &child));
    assert!(contains_path(temp.path(), &temp.path().join("future.txt")));
    assert!(!contains_path(temp.path(), &temp.path().join("../outside.txt")));
}

#[test]
fn contains_path_accepts_future_child_below_missing_parent() {
    let temp = tempfile::tempdir().expect("temp dir");
    let future_child = temp.path().join("missing/nested.txt");

    assert!(!future_child.parent().expect("future parent").exists());
    assert!(contains_path(temp.path(), &future_child));
}

#[test]
fn resolve_agent_workspace_root_uses_env_and_agent_suffix() {
    let temp = tempfile::tempdir().expect("temp dir");
    let previous = std::env::var_os("VIBEWINDOW_WORKSPACE");
    unsafe {
        std::env::set_var("VIBEWINDOW_WORKSPACE", temp.path());
    }

    assert_eq!(
        resolve_agent_workspace_root(Some("main")),
        Some(temp.path().to_string_lossy().to_string())
    );
    assert_eq!(
        resolve_agent_workspace_root(Some("worker")),
        Some(format!("{}-worker", temp.path().to_string_lossy()))
    );

    unsafe {
        match previous {
            Some(value) => std::env::set_var("VIBEWINDOW_WORKSPACE", value),
            None => std::env::remove_var("VIBEWINDOW_WORKSPACE"),
        }
    }
}

#[test]
fn decode_worktree_directory_decodes_url_safe_base64() {
    let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode("/tmp/worktree");

    assert_eq!(decode_worktree_directory(&encoded).expect("decoded"), "/tmp/worktree");
    assert!(decode_worktree_directory("%%%").is_err());
}

#[test]
fn decode_worktree_directory_rejects_non_utf8_payload() {
    let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([0xff, 0xfe]);
    let error = decode_worktree_directory(&encoded).expect_err("non-UTF-8 path should fail");

    assert_eq!(error.status, axum::http::StatusCode::NOT_FOUND);
    assert_eq!(error.message, "worktree not found");
}

#[test]
fn resolve_workspace_path_accepts_normal_child() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = temp.path().canonicalize().expect("canonical temp dir");

    assert_eq!(
        resolve_workspace_path(&root, "nested/file.txt").expect("workspace child"),
        root.join("nested/file.txt")
    );
}

#[cfg(unix)]
#[test]
fn resolve_workspace_path_rejects_existing_and_future_symlink_escapes() {
    let root_dir = tempfile::tempdir().expect("root temp dir");
    let outside_dir = tempfile::tempdir().expect("outside temp dir");
    let root = root_dir.path().canonicalize().expect("canonical root");
    let outside = outside_dir.path().canonicalize().expect("canonical outside");
    std::fs::write(outside.join("secret.txt"), "secret").expect("outside file");
    std::os::unix::fs::symlink(&outside, root.join("escape")).expect("directory symlink");

    assert!(resolve_workspace_path(&root, "escape/secret.txt").is_err());
    assert!(resolve_workspace_path(&root, "escape/future.txt").is_err());
}

#[test]
fn remove_existing_path_requires_recursive_for_directories() {
    let temp = tempfile::tempdir().expect("temp dir");
    let file = temp.path().join("file.txt");
    let dir = temp.path().join("dir");
    std::fs::write(&file, "file").expect("file");
    std::fs::create_dir(&dir).expect("dir");

    assert!(remove_existing_path(&temp.path().join("missing.txt"), false).is_err());
    assert!(remove_existing_path(&dir, false).is_err());
    remove_existing_path(&file, false).expect("remove file");
    remove_existing_path(&dir, true).expect("remove dir");
    assert!(!file.exists());
    assert!(!dir.exists());
}

#[cfg(unix)]
#[test]
fn remove_existing_path_removes_symlink_without_deleting_target() {
    let temp = tempfile::tempdir().expect("temp dir");
    let target = temp.path().join("target.txt");
    let link = temp.path().join("link.txt");
    std::fs::write(&target, "target").expect("target file");
    std::os::unix::fs::symlink(&target, &link).expect("file symlink");

    remove_existing_path(&link, false).expect("remove symlink");

    assert!(!link.exists());
    assert_eq!(std::fs::read_to_string(target).expect("target remains"), "target");
}

#[test]
fn copy_path_recursive_copies_files_and_directories() {
    let temp = tempfile::tempdir().expect("temp dir");
    let source = temp.path().join("source");
    let dest = temp.path().join("dest");
    let source_file = temp.path().join("source.txt");
    let dest_file = temp.path().join("file-dest/nested/source.txt");
    let blocked_parent = temp.path().join("blocked-parent");
    std::fs::create_dir_all(source.join("nested")).expect("nested");
    std::fs::write(source.join("nested/file.txt"), "hello").expect("source file");
    std::fs::write(&source_file, "single").expect("single source file");
    std::fs::write(&blocked_parent, "blocked").expect("blocked parent");

    copy_path_recursive(&source, &dest).expect("copy dir");
    copy_path_recursive(&source_file, &dest_file).expect("copy file");
    assert!(copy_path_recursive(&source_file, &blocked_parent.join("child.txt")).is_err());

    assert_eq!(std::fs::read_to_string(dest.join("nested/file.txt")).expect("dest file"), "hello");
    assert_eq!(std::fs::read_to_string(dest_file).expect("dest file"), "single");
}

#[test]
fn copy_path_recursive_reports_missing_source() {
    let temp = tempfile::tempdir().expect("temp dir");
    let missing = temp.path().join("missing.txt");
    let destination = temp.path().join("destination.txt");

    let error =
        copy_path_recursive(&missing, &destination).expect_err("missing source should fail");

    assert_eq!(error.status, axum::http::StatusCode::BAD_REQUEST);
    assert!(!destination.exists());
}

#[tokio::test]
async fn file_read_and_write_v1_round_trip_with_directory_body() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = temp.path().canonicalize().expect("canonical temp dir");
    let headers = axum::http::HeaderMap::new();

    let missing = file_read_v1(
        headers.clone(),
        Json(FileReadBody {
            directory: Some(root.to_string_lossy().to_string()),
            agent_key: None,
            path: "notes/todo.md".to_string(),
        }),
    )
    .await
    .expect("missing file reads empty")
    .0;
    assert_eq!(missing.content, "");

    let written = file_write_v1(
        headers.clone(),
        Json(FileWriteBody {
            directory: Some(root.to_string_lossy().to_string()),
            agent_key: None,
            path: "notes/todo.md".to_string(),
            content: "hello".to_string(),
            create_if_missing: true,
        }),
    )
    .await
    .expect("write file")
    .0;
    assert!(written.ok);
    assert_eq!(written.bytes_written, 5);

    let read = file_read_v1(
        headers,
        Json(FileReadBody {
            directory: Some(root.to_string_lossy().to_string()),
            agent_key: None,
            path: "notes/todo.md".to_string(),
        }),
    )
    .await
    .expect("read file")
    .0;
    assert_eq!(read.content, "hello");
}

#[tokio::test]
async fn file_read_and_write_v1_reject_escape_and_missing_write_target() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = temp.path().canonicalize().expect("canonical temp dir");
    std::fs::create_dir(root.join("dir")).expect("dir");
    let headers = axum::http::HeaderMap::new();

    let escape = file_read_v1(
        headers.clone(),
        Json(FileReadBody {
            directory: Some(root.to_string_lossy().to_string()),
            agent_key: None,
            path: "../outside.txt".to_string(),
        }),
    )
    .await
    .expect_err("escape path should fail");
    assert_eq!(escape.status, axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(escape.message, "path escapes workspace root");

    let dir_read = file_read_v1(
        headers.clone(),
        Json(FileReadBody {
            directory: Some(root.to_string_lossy().to_string()),
            agent_key: None,
            path: "dir".to_string(),
        }),
    )
    .await
    .expect_err("directory read should fail");
    assert_eq!(dir_read.status, axum::http::StatusCode::BAD_REQUEST);

    let write_escape = file_write_v1(
        headers.clone(),
        Json(FileWriteBody {
            directory: Some(root.to_string_lossy().to_string()),
            agent_key: None,
            path: "../outside.txt".to_string(),
            content: "content".to_string(),
            create_if_missing: true,
        }),
    )
    .await
    .expect_err("escape write should fail");
    assert_eq!(write_escape.message, "path escapes workspace root");

    let missing = file_write_v1(
        headers,
        Json(FileWriteBody {
            directory: Some(root.to_string_lossy().to_string()),
            agent_key: None,
            path: "missing.txt".to_string(),
            content: "content".to_string(),
            create_if_missing: false,
        }),
    )
    .await
    .expect_err("missing write target should fail");
    assert_eq!(missing.status, axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(missing.message, "file not found: missing.txt");
}

#[tokio::test]
async fn file_write_v1_overwrites_existing_file_and_counts_utf8_bytes() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = temp.path().canonicalize().expect("canonical temp dir");
    std::fs::write(root.join("notes.txt"), "old").expect("existing file");

    let response = file_write_v1(
        axum::http::HeaderMap::new(),
        Json(FileWriteBody {
            directory: Some(root.to_string_lossy().to_string()),
            agent_key: None,
            path: "notes.txt".to_string(),
            content: "你好".to_string(),
            create_if_missing: false,
        }),
    )
    .await
    .expect("overwrite existing file")
    .0;

    assert!(response.ok);
    assert_eq!(response.root_directory, root.to_string_lossy().to_string());
    assert_eq!(response.path, "notes.txt");
    assert_eq!(response.bytes_written, 6);
    assert_eq!(std::fs::read_to_string(root.join("notes.txt")).expect("written file"), "你好");
}

#[tokio::test]
async fn file_write_v1_reports_parent_directory_creation_error() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = temp.path().canonicalize().expect("canonical temp dir");
    std::fs::write(root.join("blocked"), "blocked").expect("blocked file");

    let err = file_write_v1(
        axum::http::HeaderMap::new(),
        Json(FileWriteBody {
            directory: Some(root.to_string_lossy().to_string()),
            agent_key: None,
            path: "blocked/child.txt".to_string(),
            content: "content".to_string(),
            create_if_missing: true,
        }),
    )
    .await
    .expect_err("blocked parent should fail");

    assert_eq!(err.status, axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn file_find_text_lists_reads_and_statuses_directory() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = temp.path().canonicalize().expect("canonical temp dir");
    std::fs::create_dir(root.join("src")).expect("src dir");
    std::fs::write(root.join("src/main.rs"), "fn main() {\n    println!(\"needle\");\n}\n")
        .expect("source file");
    std::fs::write(root.join("README.md"), "needle readme\n").expect("readme");
    std::fs::create_dir(root.join(".git")).expect("git dir");
    std::fs::write(root.join(".DS_Store"), "ignored").expect("ds store");

    let headers = axum::http::HeaderMap::new();
    let Json(matches) = file_find_text(
        Query(FindTextQuery {
            directory: Some(root.to_string_lossy().to_string()),
            pattern: "needle".to_string(),
        }),
        headers.clone(),
    )
    .await
    .expect("find text");
    assert_eq!(matches.len(), 2);
    assert!(matches.iter().any(|item| item.path == "README.md"));
    assert!(matches.iter().any(|item| item.path == "src/main.rs"));

    let err = file_find_text(
        Query(FindTextQuery {
            directory: Some(root.to_string_lossy().to_string()),
            pattern: "(".to_string(),
        }),
        headers.clone(),
    )
    .await
    .expect_err("invalid regex should fail");
    assert_eq!(err.status, axum::http::StatusCode::BAD_REQUEST);

    let Json(nodes) = file_list(
        Query(FileListQuery {
            directory: Some(root.to_string_lossy().to_string()),
            path: ".".to_string(),
        }),
        headers.clone(),
    )
    .await
    .expect("list root");
    assert_eq!(
        nodes.iter().map(|node| node.path.as_str()).collect::<Vec<_>>(),
        ["src", "README.md"]
    );

    let Json(content) = file_read(
        Query(FileReadQuery {
            directory: Some(root.to_string_lossy().to_string()),
            path: "README.md".to_string(),
        }),
        headers.clone(),
    )
    .await
    .expect("read content");
    assert_eq!(content.content, "needle readme");

    let Json(status) = file_status(
        Query(InstanceQuery { directory: Some(root.to_string_lossy().to_string()) }),
        headers,
    )
    .await
    .expect("status");
    assert!(status.is_empty());
}

#[tokio::test]
async fn file_read_rejects_absolute_path_outside_workspace() {
    let root_dir = tempfile::tempdir().expect("root temp dir");
    let outside_dir = tempfile::tempdir().expect("outside temp dir");
    let root = root_dir.path().canonicalize().expect("canonical root");
    let outside_file = outside_dir.path().join("outside.txt");
    std::fs::write(&outside_file, "outside").expect("outside file");

    let error = file_read(
        Query(FileReadQuery {
            directory: Some(root.to_string_lossy().to_string()),
            path: outside_file.to_string_lossy().to_string(),
        }),
        axum::http::HeaderMap::new(),
    )
    .await
    .expect_err("outside read should fail");

    assert_eq!(error.status, axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn file_find_file_filters_directories_files_and_limit() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = temp.path().canonicalize().expect("canonical temp dir");
    std::fs::create_dir(root.join("alpha-dir")).expect("alpha dir");
    std::fs::write(root.join("alpha-dir/nested.txt"), "nested").expect("nested");
    std::fs::write(root.join("Alpha.txt"), "alpha").expect("alpha");
    std::fs::write(root.join("beta.txt"), "beta").expect("beta");
    let headers = axum::http::HeaderMap::new();

    let Json(files) = file_find_file(
        Query(FindFileQuery {
            directory: Some(root.to_string_lossy().to_string()),
            query: "ALPHA".to_string(),
            dirs: Some("false".to_string()),
            r#type: Some("file".to_string()),
            limit: Some(0),
        }),
        headers.clone(),
    )
    .await
    .expect("find files");
    assert_eq!(files, ["Alpha.txt"]);

    let Json(dirs) = file_find_file(
        Query(FindFileQuery {
            directory: Some(root.to_string_lossy().to_string()),
            query: "alpha".to_string(),
            dirs: Some("true".to_string()),
            r#type: Some("directory".to_string()),
            limit: Some(2000),
        }),
        headers,
    )
    .await
    .expect("find directories");
    assert_eq!(dirs, ["alpha-dir"]);
}

#[tokio::test]
async fn file_find_file_skips_ignored_dirs_and_stops_at_limit() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = temp.path().canonicalize().expect("canonical temp dir");
    std::fs::create_dir(root.join("alpha-one")).expect("alpha one");
    std::fs::create_dir(root.join("alpha-two")).expect("alpha two");
    std::fs::create_dir_all(root.join("target/alpha-ignored")).expect("ignored dir");
    let headers = axum::http::HeaderMap::new();

    let Json(dirs) = file_find_file(
        Query(FindFileQuery {
            directory: Some(root.to_string_lossy().to_string()),
            query: "alpha-".to_string(),
            dirs: Some("true".to_string()),
            r#type: Some("directory".to_string()),
            limit: Some(1),
        }),
        headers,
    )
    .await
    .expect("find limited directories");

    assert_eq!(dirs.len(), 1);
    assert!(dirs[0].starts_with("alpha-"));

    let Json(ignored) = file_find_file(
        Query(FindFileQuery {
            directory: Some(root.to_string_lossy().to_string()),
            query: "ignored".to_string(),
            dirs: Some("true".to_string()),
            r#type: Some("other".to_string()),
            limit: Some(10),
        }),
        axum::http::HeaderMap::new(),
    )
    .await
    .expect("ignored dirs");
    assert!(ignored.is_empty());
}

#[tokio::test]
async fn file_move_copy_and_delete_v1_use_worktree_id_root() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = temp.path().canonicalize().expect("canonical temp dir");
    std::fs::write(root.join("source.txt"), "source").expect("source");
    let encoded =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(root.to_string_lossy().as_ref());
    let project_id = vw_api_types::id::ProjectId("project".to_string());
    let worktree_id = Some(vw_api_types::id::WorktreeId(encoded));

    let _ = file_copy_v1(Json(CopyFileRequest {
        project_id: project_id.clone(),
        worktree_id: worktree_id.clone(),
        from_path: "source.txt".to_string(),
        to_path: "copies/source.txt".to_string(),
        overwrite: false,
    }))
    .await
    .expect("copy file");
    assert_eq!(std::fs::read_to_string(root.join("copies/source.txt")).expect("copy"), "source");

    let _ = file_move_v1(Json(MoveFileRequest {
        project_id: project_id.clone(),
        worktree_id: worktree_id.clone(),
        from_path: "copies/source.txt".to_string(),
        to_path: "moved.txt".to_string(),
        overwrite: false,
    }))
    .await
    .expect("move file");
    assert!(root.join("moved.txt").is_file());
    assert!(!root.join("copies/source.txt").exists());

    let _ = file_delete_v1(Json(DeleteFileRequest {
        project_id,
        worktree_id,
        path: "moved.txt".to_string(),
        recursive: false,
    }))
    .await
    .expect("delete file");
    assert!(!root.join("moved.txt").exists());
}

#[tokio::test]
async fn file_move_copy_and_delete_v1_cover_overwrite_and_error_paths() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = temp.path().canonicalize().expect("canonical temp dir");
    std::fs::write(root.join("source.txt"), "source").expect("source");
    std::fs::write(root.join("existing.txt"), "existing").expect("existing");
    std::fs::create_dir(root.join("dir")).expect("dir");
    let encoded =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(root.to_string_lossy().as_ref());
    let project_id = vw_api_types::id::ProjectId("project".to_string());
    let worktree_id = Some(vw_api_types::id::WorktreeId(encoded));

    let missing = file_copy_v1(Json(CopyFileRequest {
        project_id: project_id.clone(),
        worktree_id: worktree_id.clone(),
        from_path: "missing.txt".to_string(),
        to_path: "copy.txt".to_string(),
        overwrite: false,
    }))
    .await
    .expect_err("missing source should fail");
    assert_eq!(missing.status, axum::http::StatusCode::BAD_REQUEST);

    let missing_move = file_move_v1(Json(MoveFileRequest {
        project_id: project_id.clone(),
        worktree_id: worktree_id.clone(),
        from_path: "missing.txt".to_string(),
        to_path: "moved-missing.txt".to_string(),
        overwrite: false,
    }))
    .await
    .expect_err("missing move source should fail");
    assert_eq!(missing_move.message, "file not found: missing.txt");

    let conflict = file_copy_v1(Json(CopyFileRequest {
        project_id: project_id.clone(),
        worktree_id: worktree_id.clone(),
        from_path: "source.txt".to_string(),
        to_path: "existing.txt".to_string(),
        overwrite: false,
    }))
    .await
    .expect_err("copy conflict should fail");
    assert_eq!(conflict.message, "path already exists: existing.txt");

    let _ = file_copy_v1(Json(CopyFileRequest {
        project_id: project_id.clone(),
        worktree_id: worktree_id.clone(),
        from_path: "source.txt".to_string(),
        to_path: "existing.txt".to_string(),
        overwrite: true,
    }))
    .await
    .expect("copy overwrite");
    assert_eq!(std::fs::read_to_string(root.join("existing.txt")).expect("existing"), "source");

    std::fs::write(root.join("other.txt"), "other").expect("other");
    let move_conflict = file_move_v1(Json(MoveFileRequest {
        project_id: project_id.clone(),
        worktree_id: worktree_id.clone(),
        from_path: "source.txt".to_string(),
        to_path: "other.txt".to_string(),
        overwrite: false,
    }))
    .await
    .expect_err("move conflict should fail");
    assert_eq!(move_conflict.message, "path already exists: other.txt");

    std::fs::write(root.join("blocked-parent"), "blocked").expect("blocked parent");
    let parent_error = file_move_v1(Json(MoveFileRequest {
        project_id: project_id.clone(),
        worktree_id: worktree_id.clone(),
        from_path: "source.txt".to_string(),
        to_path: "blocked-parent/child.txt".to_string(),
        overwrite: false,
    }))
    .await
    .expect_err("blocked parent should fail");
    assert_eq!(parent_error.status, axum::http::StatusCode::BAD_REQUEST);

    let _ = file_move_v1(Json(MoveFileRequest {
        project_id: project_id.clone(),
        worktree_id: worktree_id.clone(),
        from_path: "source.txt".to_string(),
        to_path: "other.txt".to_string(),
        overwrite: true,
    }))
    .await
    .expect("move overwrite");
    assert_eq!(std::fs::read_to_string(root.join("other.txt")).expect("other"), "source");

    let dir_delete = file_delete_v1(Json(DeleteFileRequest {
        project_id,
        worktree_id,
        path: "dir".to_string(),
        recursive: false,
    }))
    .await
    .expect_err("directory delete should require recursive");
    assert!(dir_delete.message.contains("directory requires recursive delete"));
}

#[tokio::test]
async fn file_copy_and_delete_v1_handle_directories_recursively() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = temp.path().canonicalize().expect("canonical temp dir");
    std::fs::create_dir_all(root.join("source/nested")).expect("source tree");
    std::fs::write(root.join("source/nested/file.txt"), "content").expect("source file");
    let encoded =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(root.to_string_lossy().as_ref());
    let project_id = vw_api_types::id::ProjectId("project".to_string());
    let worktree_id = Some(vw_api_types::id::WorktreeId(encoded));

    let _ = file_copy_v1(Json(CopyFileRequest {
        project_id: project_id.clone(),
        worktree_id: worktree_id.clone(),
        from_path: "source".to_string(),
        to_path: "copy".to_string(),
        overwrite: false,
    }))
    .await
    .expect("copy directory");
    assert_eq!(
        std::fs::read_to_string(root.join("copy/nested/file.txt")).expect("copied file"),
        "content"
    );

    let _ = file_delete_v1(Json(DeleteFileRequest {
        project_id,
        worktree_id,
        path: "copy".to_string(),
        recursive: true,
    }))
    .await
    .expect("delete directory recursively");
    assert!(!root.join("copy").exists());
}

#[tokio::test]
async fn file_move_copy_and_delete_v1_reject_workspace_escapes() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = temp.path().canonicalize().expect("canonical temp dir");
    std::fs::write(root.join("source.txt"), "source").expect("source file");
    let encoded =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(root.to_string_lossy().as_ref());
    let project_id = vw_api_types::id::ProjectId("project".to_string());
    let worktree_id = Some(vw_api_types::id::WorktreeId(encoded));

    let copy_error = file_copy_v1(Json(CopyFileRequest {
        project_id: project_id.clone(),
        worktree_id: worktree_id.clone(),
        from_path: "source.txt".to_string(),
        to_path: "../copy.txt".to_string(),
        overwrite: false,
    }))
    .await
    .expect_err("copy escape should fail");
    assert_eq!(copy_error.message, "path escapes workspace root");

    let move_error = file_move_v1(Json(MoveFileRequest {
        project_id: project_id.clone(),
        worktree_id: worktree_id.clone(),
        from_path: "source.txt".to_string(),
        to_path: "../moved.txt".to_string(),
        overwrite: false,
    }))
    .await
    .expect_err("move escape should fail");
    assert_eq!(move_error.message, "path escapes workspace root");

    let delete_error = file_delete_v1(Json(DeleteFileRequest {
        project_id,
        worktree_id,
        path: "../source.txt".to_string(),
        recursive: false,
    }))
    .await
    .expect_err("delete escape should fail");
    assert_eq!(delete_error.message, "path escapes workspace root");
    assert!(root.join("source.txt").is_file());
}

#[tokio::test]
async fn resolve_file_root_uses_project_worktree_and_reports_missing_project() {
    let temp = tempfile::tempdir().expect("temp dir");
    let worktree = temp.path().join("worktree");
    let sandbox = temp.path().join("sandbox");
    std::fs::create_dir(&worktree).expect("worktree");
    std::fs::create_dir(&sandbox).expect("sandbox");

    let worktree_id = format!("file-root-worktree-{}", uuid::Uuid::new_v4());
    store_file_test_project(&project_info_for_file_tests(
        &worktree_id,
        worktree.to_string_lossy().to_string(),
        vec![],
    ));
    assert_eq!(
        resolve_file_root(&worktree_id, None).await.expect("worktree root"),
        worktree.to_string_lossy()
    );

    let missing =
        resolve_file_root("missing-project", None).await.expect_err("missing project should fail");
    assert_eq!(missing.status, axum::http::StatusCode::NOT_FOUND);

    remove_file_test_project(&worktree_id);
}

#[test]
fn resolve_directory_or_agent_root_prefers_directory_and_home_fallback() {
    let temp = tempfile::tempdir().expect("temp dir");
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        "x-vibewindow-directory",
        axum::http::HeaderValue::from_str(temp.path().to_str().expect("utf8 path"))
            .expect("header"),
    );

    assert_eq!(
        resolve_directory_or_agent_root(
            &InstanceQuery { directory: Some(temp.path().to_string_lossy().to_string()) },
            Some("worker"),
            &headers,
        ),
        temp.path().to_string_lossy()
    );

    let previous = std::env::var_os("VIBEWINDOW_WORKSPACE");
    unsafe {
        std::env::set_var("VIBEWINDOW_WORKSPACE", "   ");
    }

    let expected = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .map(|home| vw_config_types::paths::home_config_dir(home).join("workspace-worker"))
        .map(|path| path.to_string_lossy().into_owned());
    assert_eq!(resolve_agent_workspace_root(Some("worker")), expected);
    assert_eq!(
        resolve_directory_or_agent_root(
            &InstanceQuery { directory: Some("   ".to_string()) },
            Some("worker"),
            &headers
        ),
        expected.expect("home fallback")
    );

    unsafe {
        match previous {
            Some(value) => std::env::set_var("VIBEWINDOW_WORKSPACE", value),
            None => std::env::remove_var("VIBEWINDOW_WORKSPACE"),
        }
    }
}

fn project_info_for_file_tests(
    id: &str,
    worktree: String,
    sandboxes: Vec<String>,
) -> project::Info {
    project::Info {
        id: id.to_string(),
        worktree,
        vcs: None,
        name: None,
        icon: None,
        commands: None,
        time: project::TimeInfo { created: 10, updated: 20, initialized: None },
        sandboxes,
    }
}

fn store_file_test_project(info: &crate::app::agent::project::Info) {
    crate::app::agent::project::store_local_test_project(info).expect("project should write");
}

fn remove_file_test_project(project_id: &str) {
    crate::app::agent::project::remove_local_test_project(project_id)
        .expect("project should clean up");
}
