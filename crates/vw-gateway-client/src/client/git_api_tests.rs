use serde_json::json;
use vw_api_types::git::{
    GitCommandRequest, GitCommitRequest, GitHunkSelectionDto, GitLineSelectionDto, GitMergeRequest,
};

use crate::client::test_support;

#[tokio::test]
async fn git_commit_posts_all_selection_fields_and_decodes_not_created_response() {
    let server = test_support::server(vec![(
        200,
        json!({
            "ok": true,
            "created": false,
            "commit": {"sha": "abc123", "message": "nothing to commit"}
        }),
    )]);

    let response = server
        .client()
        .git_commit(&GitCommitRequest {
            project_id: "project".into(),
            worktree_id: Some("worktree".into()),
            message: "ship selected changes".to_string(),
            stage_all: false,
            selected_files: vec!["src/lib.rs".to_string()],
            selected_hunks: vec![GitHunkSelectionDto { path: "src/main.rs".to_string(), index: 2 }],
            selected_lines: vec![GitLineSelectionDto { path: "src/new.rs".to_string(), line: 7 }],
            selected_old_lines: vec![GitLineSelectionDto {
                path: "src/old.rs".to_string(),
                line: 3,
            }],
        })
        .await
        .expect("commit response");

    assert!(response.ok);
    assert!(!response.created);
    assert_eq!(response.commit.sha, "abc123");
    assert_eq!(response.commit.message, "nothing to commit");
    let request = server.take_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/git/commit");
    assert_eq!(
        request.body,
        json!({
            "project_id": "project",
            "worktree_id": "worktree",
            "message": "ship selected changes",
            "stage_all": false,
            "selected_files": ["src/lib.rs"],
            "selected_hunks": [{"path": "src/main.rs", "index": 2}],
            "selected_lines": [{"path": "src/new.rs", "line": 7}],
            "selected_old_lines": [{"path": "src/old.rs", "line": 3}]
        })
    );
    server.join();
}

#[tokio::test]
async fn git_command_omits_timeout_and_decodes_command_failure() {
    let server = test_support::server(vec![(
        200,
        json!({
            "success": false,
            "code": 128,
            "stdout": "",
            "stderr": "fatal: not a git repository"
        }),
    )]);

    let response = server
        .client()
        .git_command(&GitCommandRequest {
            directory: "/repo with spaces".to_string(),
            args: Vec::new(),
            timeout_secs: None,
        })
        .await
        .expect("command response");

    assert!(!response.success);
    assert_eq!(response.code, Some(128));
    assert!(response.stdout.is_empty());
    assert_eq!(response.stderr, "fatal: not a git repository");
    let request = server.take_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/git/command");
    assert_eq!(request.body, json!({"directory": "/repo with spaces", "args": []}));
    server.join();
}

#[tokio::test]
async fn git_merge_decodes_already_merged_response() {
    let server = test_support::server(vec![(
        200,
        json!({
            "ok": true,
            "source_branch": "feature",
            "target_branch": "main",
            "workspace": "/repo",
            "already_merged": true,
            "message": "already up to date"
        }),
    )]);

    let response = server
        .client()
        .git_merge(&GitMergeRequest {
            project_id: "project".into(),
            source_branch: "feature".to_string(),
            target_branch: "main".to_string(),
        })
        .await
        .expect("merge response");

    assert!(response.ok);
    assert!(response.already_merged);
    assert_eq!(response.source_branch, "feature");
    assert_eq!(response.target_branch, "main");
    assert_eq!(response.workspace, "/repo");
    assert_eq!(response.message, "already up to date");
    let request = server.take_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/git/merge");
    assert_eq!(
        request.body,
        json!({
            "project_id": "project",
            "source_branch": "feature",
            "target_branch": "main"
        })
    );
    server.join();
}

#[tokio::test]
async fn git_commit_surfaces_non_success_status() {
    let server = test_support::server(vec![(500, json!({"message": "commit failed"}))]);

    let error = server
        .client()
        .git_commit(&GitCommitRequest {
            project_id: "project".into(),
            worktree_id: None,
            message: "ship it".to_string(),
            stage_all: true,
            selected_files: Vec::new(),
            selected_hunks: Vec::new(),
            selected_lines: Vec::new(),
            selected_old_lines: Vec::new(),
        })
        .await
        .expect_err("status error");

    assert!(error.contains("gateway request failed: 500"));
    let request = server.take_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/git/commit");
    server.join();
}

#[tokio::test]
async fn git_command_surfaces_response_decode_error() {
    let server = test_support::server_raw(vec![(200, "not-json".to_string())]);

    let error = server
        .client()
        .git_command(&GitCommandRequest {
            directory: "/repo".to_string(),
            args: vec!["status".to_string()],
            timeout_secs: Some(0),
        })
        .await
        .expect_err("decode error");

    assert!(!error.is_empty());
    let request = server.take_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/git/command");
    assert_eq!(request.body["timeout_secs"], 0);
    server.join();
}
