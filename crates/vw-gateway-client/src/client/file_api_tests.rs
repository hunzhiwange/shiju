//! 验证结构化文件接口与旧版目录接口的路径和载荷契约。
//!
//! 测试覆盖跨平台分隔符规整、根目录编码、可选工作树上下文及错误传播。

use serde_json::json;
use vw_api_types::file::{
    CopyFileRequest, DeleteFileRequest, FileNodeKind, ListFilesRequest, MoveFileRequest,
    ReadFileRequest, SearchFilesRequest, StatFileRequest, WriteFileRequest,
};

use crate::client::test_support;

#[test]
fn normalize_entry_path_trims_after_backslash_conversion() {
    assert_eq!(super::normalize_entry_path(r"\src\main.rs\"), "src/main.rs");
}

#[test]
fn normalize_entry_path_handles_root_whitespace_and_mixed_separators() {
    assert_eq!(super::normalize_entry_path(" / "), ".");
    assert_eq!(super::normalize_entry_path(r" /src\nested/file.rs/ "), "src/nested/file.rs");
    assert_eq!(super::normalize_entry_path("README.md"), "README.md");
}

#[test]
fn normalize_requested_path_defaults_blank_values_and_normalizes_entries() {
    assert_eq!(super::normalize_requested_path(None), ".");
    assert_eq!(super::normalize_requested_path(Some("  ")), ".");
    assert_eq!(super::normalize_requested_path(Some(r" /src\lib.rs/ ")), "src/lib.rs");
}

#[test]
fn legacy_file_list_query_encodes_root_as_empty_and_preserves_nested_path() {
    assert_eq!(
        super::legacy_file_list_query("/repo", "."),
        vec![("directory".to_string(), "/repo".to_string()), ("path".to_string(), String::new()),]
    );
    assert_eq!(
        super::legacy_file_list_query("/repo", "src/lib.rs"),
        vec![
            ("directory".to_string(), "/repo".to_string()),
            ("path".to_string(), "src/lib.rs".to_string()),
        ]
    );
}

#[test]
fn directory_node_name_uses_directory_or_relative_path_basename() {
    assert_eq!(super::directory_node_name("/workspace/repo", "."), "repo");
    assert_eq!(super::directory_node_name("/", "."), "/");
    assert_eq!(super::directory_node_name("/workspace/repo", "src/nested"), "nested");
}

#[test]
fn directory_file_requests_omit_absent_context_and_preserve_write_options() {
    let read = super::DirectoryFileReadRequest {
        directory: None,
        agent_key: None,
        path: "README.md".to_string(),
    };
    assert_eq!(serde_json::to_value(read).expect("serialize read"), json!({"path": "README.md"}));

    let write = super::DirectoryFileWriteRequest {
        directory: Some("/repo".to_string()),
        agent_key: Some("agent-1".to_string()),
        path: "README.md".to_string(),
        content: "hello".to_string(),
        create_if_missing: false,
    };
    assert_eq!(
        serde_json::to_value(write).expect("serialize write"),
        json!({
            "directory": "/repo",
            "agent_key": "agent-1",
            "path": "README.md",
            "content": "hello",
            "create_if_missing": false,
        })
    );
}

#[test]
fn directory_file_responses_deserialize_all_wire_fields() {
    let read: super::DirectoryFileReadResponse = serde_json::from_value(json!({
        "root_directory": "/repo",
        "path": "README.md",
        "content": "hello",
    }))
    .expect("deserialize read");
    assert_eq!(read.root_directory, "/repo");
    assert_eq!(read.path, "README.md");
    assert_eq!(read.content, "hello");

    let write: super::DirectoryFileWriteResponse = serde_json::from_value(json!({
        "ok": true,
        "root_directory": "/repo",
        "path": "README.md",
        "bytes_written": 5,
    }))
    .expect("deserialize write");
    assert!(write.ok);
    assert_eq!(write.root_directory, "/repo");
    assert_eq!(write.path, "README.md");
    assert_eq!(write.bytes_written, 5);
}

#[test]
fn legacy_file_node_deserializes_wire_kinds_and_rejects_unknown_kind() {
    let file: super::LegacyFileNode = serde_json::from_value(json!({
        "path": "src/lib.rs",
        "name": "lib.rs",
        "type": "file",
    }))
    .expect("deserialize file node");
    assert_eq!(file.path, "src/lib.rs");
    assert_eq!(file.name, "lib.rs");
    assert!(matches!(FileNodeKind::from(file.kind), FileNodeKind::File));

    let directory: super::LegacyFileNode = serde_json::from_value(json!({
        "path": "src",
        "name": "src",
        "type": "directory",
    }))
    .expect("deserialize directory node");
    assert!(matches!(FileNodeKind::from(directory.kind), FileNodeKind::Directory));

    let error = serde_json::from_value::<super::LegacyFileNode>(json!({
        "path": "link",
        "name": "link",
        "type": "symlink",
    }))
    .expect_err("unknown kind should be rejected");
    assert!(error.to_string().contains("unknown variant"));
}

#[tokio::test]
async fn file_api_routes_structured_and_legacy_calls() {
    let server = test_support::server(vec![
        (
            200,
            json!({
                "project": {
                    "id": "project",
                    "name": "Repo",
                    "directory": "/repo",
                    "display_path": "/repo",
                    "status": "ready",
                    "created_at_ms": 1,
                    "updated_at_ms": 2,
                    "git": {"is_repo": true, "has_uncommitted_changes": false}
                }
            }),
        ),
        (200, json!([{"path": "\\src\\", "name": "src", "type": "directory"}])),
        (200, json!([{"path": "\\src\\main.rs", "name": "main.rs", "type": "file"}])),
        (
            200,
            json!({"path": "src/main.rs", "content": "fn main() {}", "encoding": "utf-8", "offset_line": 0, "line_count": 1, "truncated": false}),
        ),
        (200, json!({"ok": true, "path": "src/main.rs", "bytes_written": 12})),
        (200, json!({"ok": true})),
        (200, json!({"ok": true})),
        (200, json!({"ok": true})),
        (
            200,
            json!({"matches": [{"path": "src/main.rs", "line": 1, "column": 4, "text": "main"}]}),
        ),
        (
            200,
            json!({"entry": {"path": "src/main.rs", "name": "main.rs", "kind": "file", "size_bytes": 12}}),
        ),
        (200, json!({"root_directory": "/repo", "path": "README.md", "content": "hi"})),
        (
            200,
            json!({"ok": true, "root_directory": "/repo", "path": "README.md", "bytes_written": 2}),
        ),
    ]);

    let root = server
        .client()
        .file_list(&ListFilesRequest {
            project_id: "project".into(),
            worktree_id: None,
            path: None,
            depth: Some(2),
        })
        .await
        .expect("list")
        .root;
    assert_eq!(root.children.unwrap()[0].path, "src");
    assert_eq!(
        server
            .client()
            .file_read(&ReadFileRequest {
                project_id: "project".into(),
                worktree_id: None,
                path: "src/main.rs".to_string(),
                offset_line: Some(0),
                limit_lines: Some(10),
            })
            .await
            .expect("read")
            .line_count,
        1
    );
    assert!(
        server
            .client()
            .file_write(&WriteFileRequest {
                project_id: "project".into(),
                worktree_id: None,
                path: "src/main.rs".to_string(),
                content: "fn main() {}".to_string(),
                create_if_missing: true,
            })
            .await
            .expect("write")
            .ok
    );
    server
        .client()
        .file_move(&MoveFileRequest {
            project_id: "project".into(),
            worktree_id: None,
            from_path: "a".to_string(),
            to_path: "b".to_string(),
            overwrite: true,
        })
        .await
        .expect("move");
    server
        .client()
        .file_copy(&CopyFileRequest {
            project_id: "project".into(),
            worktree_id: None,
            from_path: "a".to_string(),
            to_path: "b".to_string(),
            overwrite: true,
        })
        .await
        .expect("copy");
    server
        .client()
        .file_delete(&DeleteFileRequest {
            project_id: "project".into(),
            worktree_id: None,
            path: "b".to_string(),
            recursive: true,
        })
        .await
        .expect("delete");
    assert_eq!(
        server
            .client()
            .file_search(&SearchFilesRequest {
                project_id: "project".into(),
                worktree_id: None,
                pattern: "main".to_string(),
                include: None,
                regex: false,
                case_sensitive: false,
            })
            .await
            .expect("search")
            .matches[0]
            .line,
        1
    );
    assert_eq!(
        server
            .client()
            .file_stat(&StatFileRequest {
                project_id: "project".into(),
                worktree_id: None,
                path: "src/main.rs".to_string(),
            })
            .await
            .expect("stat")
            .entry
            .name,
        "main.rs"
    );
    assert_eq!(
        server
            .client()
            .file_read_in_directory(Some("/repo"), Some("agent"), "README.md")
            .await
            .expect("legacy read")
            .content,
        "hi"
    );
    assert!(
        server
            .client()
            .file_write_in_directory(Some("/repo"), None, "README.md", "hi", true)
            .await
            .expect("legacy write")
            .ok
    );

    assert_eq!(server.take_request().path, "/v1/projects/project");
    assert!(server.take_request().path.contains("/v1/file?directory=%2Frepo&path="));
    assert!(server.take_request().path.contains("/v1/file?directory=%2Frepo&path=src"));
    assert_eq!(server.take_request().path, "/v1/files/read");
    assert_eq!(server.take_request().path, "/v1/files/write");
    assert_eq!(server.take_request().path, "/v1/files/move");
    assert_eq!(server.take_request().path, "/v1/files/copy");
    assert_eq!(server.take_request().path, "/v1/files/delete");
    assert_eq!(server.take_request().path, "/v1/files/search");
    assert_eq!(server.take_request().path, "/v1/files/stat");
    assert_eq!(server.take_request().path, "/v1/file/read");
    assert_eq!(server.take_request().path, "/v1/file/write");
    server.join();
}

#[tokio::test]
async fn file_list_supports_worktree_directory_and_depth_zero() {
    let server = test_support::server(vec![
        (
            200,
            json!({
                "worktree": {
                    "id": "worktree",
                    "project_id": "project",
                    "name": "Feature",
                    "branch": "feature",
                    "directory": "/repo-feature",
                    "status": "ready",
                    "created_at_ms": 1,
                    "updated_at_ms": 2
                }
            }),
        ),
        (200, json!([{"path": "\\src\\", "name": "src", "type": "directory"}])),
    ]);

    let root = server
        .client()
        .file_list(&ListFilesRequest {
            project_id: "project".into(),
            worktree_id: Some("worktree".into()),
            path: Some("src".to_string()),
            depth: Some(0),
        })
        .await
        .expect("list")
        .root;
    assert_eq!(root.path, "src");
    assert!(root.children.is_none());

    assert_eq!(server.take_request().path, "/v1/projects/project/worktrees/worktree");
    assert!(server.take_request().path.contains("/v1/file?directory=%2Frepo-feature&path=src"));
    server.join();
}

#[tokio::test]
async fn file_list_with_unbounded_depth_keeps_empty_children_collection() {
    let server = test_support::server(vec![
        (
            200,
            json!({
                "project": {
                    "id": "project",
                    "name": "Repo",
                    "directory": "/repo",
                    "display_path": "/repo",
                    "status": "ready",
                    "created_at_ms": 1,
                    "updated_at_ms": 2,
                    "git": {"is_repo": true, "has_uncommitted_changes": false}
                }
            }),
        ),
        (200, json!([])),
    ]);

    let root = server
        .client()
        .file_list(&ListFilesRequest {
            project_id: "project".into(),
            worktree_id: None,
            path: None,
            depth: None,
        })
        .await
        .expect("unbounded file list")
        .root;

    assert_eq!(root.path, ".");
    assert_eq!(root.children, Some(Vec::new()));
    assert_eq!(server.take_request().path, "/v1/projects/project");
    assert!(server.take_request().path.contains("/v1/file?directory=%2Frepo&path="));
    server.join();
}

#[tokio::test]
async fn file_read_propagates_gateway_and_response_decode_errors() {
    let server = test_support::server(vec![
        (500, json!({"error": "read failed"})),
        (200, json!({"path": 7})),
    ]);
    let request = ReadFileRequest {
        project_id: "project".into(),
        worktree_id: None,
        path: "README.md".to_string(),
        offset_line: None,
        limit_lines: None,
    };

    let gateway_error = server.client().file_read(&request).await.expect_err("gateway error");
    assert!(gateway_error.contains("500"));
    assert!(gateway_error.contains("read failed"));

    let decode_error = server.client().file_read(&request).await.expect_err("decode error");
    assert!(decode_error.contains("decoding response body"));

    for _ in 0..2 {
        let recorded = server.take_request();
        assert_eq!(recorded.method, "POST");
        assert_eq!(recorded.path, "/v1/files/read");
        assert_eq!(recorded.body["path"], "README.md");
    }
    server.join();
}

#[tokio::test]
async fn file_list_propagates_nested_directory_errors() {
    let server = test_support::server(vec![
        (
            200,
            json!({
                "project": {
                    "id": "project",
                    "name": "Repo",
                    "directory": "/repo",
                    "display_path": "/repo",
                    "status": "ready",
                    "created_at_ms": 1,
                    "updated_at_ms": 2,
                    "git": {"is_repo": true, "has_uncommitted_changes": false}
                }
            }),
        ),
        (200, json!([{"path": "src", "name": "src", "type": "directory"}])),
        (500, json!({"error": "nested list failed"})),
    ]);

    let error = server
        .client()
        .file_list(&ListFilesRequest {
            project_id: "project".into(),
            worktree_id: None,
            path: None,
            depth: Some(1),
        })
        .await
        .expect_err("nested directory error");

    assert!(error.contains("500"));
    assert!(error.contains("nested list failed"));
    assert_eq!(server.take_request().path, "/v1/projects/project");
    assert!(server.take_request().path.contains("/v1/file?directory=%2Frepo&path="));
    assert!(server.take_request().path.contains("/v1/file?directory=%2Frepo&path=src"));
    server.join();
}
