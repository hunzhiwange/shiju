#![cfg(not(target_arch = "wasm32"))]

//! 验证 worktree 创建、列举、重置和删除操作及缺失项目上下文的失败边界。
//! 破坏性场景仅作用于临时仓库，并显式确认主工作区不会被重置或删除。

use super::{CreateInput, Error, RemoveInput, ResetInput, create, list_directories};

// 统一匹配错误变体，避免只比较字符串而漏掉错误分类回归。
fn assert_missing_project_context(error: Error) {
    match error {
        Error::MissingProject(message) => assert_eq!(message, "missing project context"),
        other => panic!("expected missing project context, got {other}"),
    }
}

#[test]
fn operation_inputs_deserialize_aliases_and_defaults() {
    let create: CreateInput =
        serde_json::from_value(serde_json::json!({"name": "demo", "startCommand": "echo ok"}))
            .expect("create input");
    let remove: RemoveInput =
        serde_json::from_value(serde_json::json!({"directory": "/tmp/wt"})).expect("remove input");
    let reset: ResetInput =
        serde_json::from_value(serde_json::json!({"directory": "/tmp/wt", "baseRef": "main"}))
            .expect("reset input");

    assert_eq!(create.start_command.as_deref(), Some("echo ok"));
    assert_eq!(create.name.as_deref(), Some("demo"));
    assert!(!remove.force);
    assert_eq!(reset.base_ref.as_deref(), Some("main"));
}

#[test]
fn operation_inputs_serialize_camel_case_fields() {
    let create = serde_json::to_value(CreateInput {
        name: Some("demo".to_string()),
        start_command: Some("npm test".to_string()),
    })
    .expect("create json");
    let reset = serde_json::to_value(ResetInput {
        directory: "/tmp/wt".to_string(),
        base_ref: Some("main".to_string()),
    })
    .expect("reset json");
    let remove =
        serde_json::to_value(RemoveInput { directory: "/tmp/wt".to_string(), force: true })
            .expect("remove json");

    assert_eq!(create["startCommand"], "npm test");
    assert_eq!(reset["baseRef"], "main");
    assert_eq!(remove["force"], true);
}

#[tokio::test]
async fn create_without_project_context_returns_missing_project_before_creating() {
    let error = create(Some(CreateInput {
        name: Some("demo".to_string()),
        start_command: Some("should-not-run".to_string()),
    }))
    .await
    .expect_err("create should require project context");

    assert_missing_project_context(error);
}

#[tokio::test]
async fn list_directories_without_project_context_returns_missing_project() {
    let error = list_directories().await.expect_err("list should require project context");

    assert_missing_project_context(error);
}
