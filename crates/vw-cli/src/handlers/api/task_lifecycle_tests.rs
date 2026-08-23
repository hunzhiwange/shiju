//! `api task-lifecycle` 六个 workflow 路由测试。

use std::sync::mpsc::TryRecvError;

use serde_json::{Value, json};

use super::session_tests::{assert_request, run_command, spawn_server};

#[tokio::test(flavor = "current_thread")]
async fn api_task_lifecycle_commands_cover_workflow_crud_and_reset() {
    let workflow = json!({"id":"wf/one","label":"One","statuses":[]});
    let body = json!({"workflow":workflow.clone()});
    let input = body.to_string();
    let responses = vec![json!([workflow.clone()]), workflow.clone(), workflow.clone(), workflow.clone(), json!({"id":"wf/one","deleted":true}), workflow];
    let (port, requests) = spawn_server(responses);
    let port = port.to_string();
    let common = ["--host", "127.0.0.1", "--port", port.as_str()];
    for command in [
        &["task-lifecycle", "create", "--input", "-"][..],
        &["task-lifecycle", "update", "wf/one", "--input", "-"][..],
        &["task-lifecycle", "delete", "wf/one"][..],
        &["task-lifecycle", "reset", "wf/one"][..],
    ] {
        let error = run_command(&common, command, input.as_bytes()).await.expect_err("write requires --yes");
        assert!(error.contains("--yes"));
        assert!(matches!(requests.try_recv(), Err(TryRecvError::Empty)));
    }
    run_command(&common, &["task-lifecycle", "list"], b"").await.unwrap();
    assert_request(&requests, "GET", "/v1/task-lifecycle/workflows", Value::Null);
    run_command(&common, &["task-lifecycle", "get", "wf/one"], b"").await.unwrap();
    assert_request(&requests, "GET", "/v1/task-lifecycle/workflows/wf%2Fone", Value::Null);

    let yes = ["--host", "127.0.0.1", "--port", port.as_str(), "--yes"];
    run_command(&yes, &["task-lifecycle", "create", "--input", "-"], input.as_bytes()).await.unwrap();
    assert_request(&requests, "POST", "/v1/task-lifecycle/workflows", body.clone());
    run_command(&yes, &["task-lifecycle", "update", "wf/one", "--input", "-"], input.as_bytes()).await.unwrap();
    assert_request(&requests, "PUT", "/v1/task-lifecycle/workflows/wf%2Fone", body);
    run_command(&yes, &["task-lifecycle", "delete", "wf/one"], b"").await.unwrap();
    assert_request(&requests, "DELETE", "/v1/task-lifecycle/workflows/wf%2Fone", json!({}));
    run_command(&yes, &["task-lifecycle", "reset", "wf/one"], b"").await.unwrap();
    assert_request(&requests, "POST", "/v1/task-lifecycle/workflows/wf%2Fone/reset", json!({}));
}
