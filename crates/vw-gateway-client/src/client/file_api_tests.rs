//! 验证结构化文件接口与旧版目录接口的路径和载荷契约。
//!
//! 测试覆盖跨平台分隔符规整、根目录编码、可选工作树上下文及错误传播。

use serde_json::json;

use crate::client::test_support;

#[tokio::test]
async fn workspace_api_uses_file_search_and_git_routes() {
    let server = test_support::server(vec![
        (200, json!([{"path":"src/main.rs","lineNumber":3,"line":"needle","start":0,"end":6}])),
        (200, json!(["src/main.rs"])),
        (
            200,
            json!([{"name":"main.rs","path":"src/main.rs","absolute":"/repo/src/main.rs","type":"file","ignored":false}]),
        ),
        (200, json!({"type":"text","content":"needle","encoding":"utf-8"})),
        (200, json!([{"path":"src/main.rs","added":1,"removed":0,"status":"modified"}])),
        (200, json!({"ok":true,"created":true,"commit":{"sha":"abc123","message":"ship it"}})),
        (200, json!({"success":true,"code":0,"stdout":"clean","stderr":""})),
        (
            200,
            json!({"ok":true,"source_branch":"feature","target_branch":"main","workspace":"/repo","already_merged":false,"message":"merged"}),
        ),
        (403, json!({"message":"workspace denied"})),
    ]);
    let client = server.client();

    client
        .file_find_text_in_directory(Some("/repo with spaces"), "needle/value")
        .await
        .expect("text search");
    client
        .file_find_in_directory(
            Some("/repo with spaces"),
            "main /rs",
            Some(false),
            Some("file"),
            Some(7),
        )
        .await
        .expect("file search");
    client
        .directory_file_list(Some("/repo with spaces"), "src /nested")
        .await
        .expect("directory list");
    client
        .file_content_in_directory(Some("/repo with spaces"), "src /main.rs")
        .await
        .expect("file content");
    client.file_status_in_directory(Some("/repo with spaces")).await.expect("file status");
    client
        .git_commit(&vw_api_types::git::GitCommitRequest {
            project_id: "project /one".into(),
            worktree_id: None,
            message: "ship it".to_string(),
            stage_all: true,
            selected_files: Vec::new(),
            selected_hunks: Vec::new(),
            selected_lines: Vec::new(),
            selected_old_lines: Vec::new(),
        })
        .await
        .expect("git commit");
    client
        .git_command(&vw_api_types::git::GitCommandRequest {
            directory: "/repo with spaces".to_string(),
            args: vec!["status".to_string(), "--short".to_string()],
            timeout_secs: Some(5),
        })
        .await
        .expect("git command");
    client
        .git_merge(&vw_api_types::git::GitMergeRequest {
            project_id: "project /one".into(),
            source_branch: "feature".to_string(),
            target_branch: "main".to_string(),
        })
        .await
        .expect("git merge");
    let denied = client
        .file_content_in_directory(Some("/repo with spaces"), "denied /file")
        .await
        .expect_err("Gateway rejection must propagate");
    assert!(denied.contains("403"));
    assert!(denied.contains("workspace denied"));

    let expected = [
        ("GET", "/v1/find?directory=%2Frepo+with+spaces&pattern=needle%2Fvalue"),
        (
            "GET",
            "/v1/find/file?directory=%2Frepo+with+spaces&query=main+%2Frs&dirs=false&type=file&limit=7",
        ),
        ("GET", "/v1/file?directory=%2Frepo+with+spaces&path=src+%2Fnested"),
        ("GET", "/v1/file/content?directory=%2Frepo+with+spaces&path=src+%2Fmain.rs"),
        ("GET", "/v1/file/status?directory=%2Frepo+with+spaces"),
        ("POST", "/v1/git/commit"),
        ("POST", "/v1/git/command"),
        ("POST", "/v1/git/merge"),
        ("GET", "/v1/file/content?directory=%2Frepo+with+spaces&path=denied+%2Ffile"),
    ];
    for (method, path) in expected {
        let request = server.take_request();
        assert_eq!(request.method, method);
        assert_eq!(request.path, path);
    }
    server.join();
}
