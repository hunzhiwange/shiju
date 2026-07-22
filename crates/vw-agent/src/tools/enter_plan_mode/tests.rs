//! 进入计划模式工具的行为测试。
//!
//! 覆盖状态写入、工具描述提示，以及计划模式下写入型工具被执行器拒绝的保护。

use super::EnterPlanModeTool;
use crate::app::agent::security::SecurityPolicy;
use crate::app::agent::tools::FileWriteTool;
use crate::app::agent::tools::Tool;
use crate::app::agent::tools::context::{ToolUseContext, scope_tool_use_context};
use crate::app::agent::tools::execute_tool_from_registry;
use serde_json::json;
use std::sync::Arc;
use tempfile::TempDir;
use vw_api_types::tools::ToolResultContentDto;

#[tokio::test]
async fn enter_plan_mode_sets_active_state() {
    let tool = EnterPlanModeTool::new();
    let context = Arc::new(ToolUseContext::new("enter-plan-mode", None));

    let result = scope_tool_use_context(
        context.clone(),
        tool.call(json!({
            "goal": "plan workflow tools",
            "note": "review constraints first"
        })),
    )
    .await
    .expect("tool should succeed");

    assert!(result.is_success());
    let state = context.plan_mode_state();
    assert!(state.active);
    assert_eq!(state.goal.as_deref(), Some("plan workflow tools"));
    assert_eq!(state.note.as_deref(), Some("review constraints first"));
    assert_eq!(
        result.data["message"].as_str(),
        Some(
            "Entered plan mode. You should now focus on exploring the codebase and designing an implementation approach."
        )
    );
    assert_eq!(result.data["instructions"].as_array().map(Vec::len), Some(6));
    assert!(
        result
            .model_result
            .as_str()
            .expect("model result should be text")
            .contains("DO NOT write or edit any files yet")
    );
    assert!(matches!(
        result.content_blocks.get(1),
        Some(ToolResultContentDto::Text { text })
            if text.contains("In plan mode, you should:")
    ));
}

#[tokio::test]
async fn enter_plan_mode_refreshes_existing_plan_details() {
    let tool = EnterPlanModeTool::new();
    let context = Arc::new(ToolUseContext::new("enter-plan-mode-refresh", None));
    context.enter_plan_mode(Some("old goal".to_string()), Some("old note".to_string()));

    let result = scope_tool_use_context(
        context.clone(),
        tool.call(json!({
            "goal": "revised goal",
            "note": "revised note"
        })),
    )
    .await
    .expect("re-entering plan mode should succeed");

    assert_eq!(result.data["already_active"], true);
    assert_eq!(result.data["goal"], "revised goal");
    assert_eq!(result.data["note"], "revised note");
    assert_eq!(
        result.render_hint.and_then(|hint| hint.summary),
        Some("Plan mode remains active".to_string())
    );
    let state = context.plan_mode_state();
    assert_eq!(state.goal.as_deref(), Some("revised goal"));
    assert_eq!(state.note.as_deref(), Some("revised note"));
}

#[tokio::test]
async fn enter_plan_mode_rejects_non_object_arguments() {
    let error = EnterPlanModeTool::new()
        .call(json!([]))
        .await
        .expect_err("array arguments should be rejected before context lookup");

    assert!(error.to_string().contains("invalid enter plan mode arguments"));
}

#[test]
fn enter_plan_mode_description_includes_usage_guidance() {
    let tool = EnterPlanModeTool::new();

    assert!(tool.description().contains("## When to Use This Tool"));
    assert!(tool.description().contains("## What Happens in Plan Mode"));
    assert!(tool.description().contains("AskUserQuestion"));
}

#[tokio::test]
async fn plan_mode_blocks_write_tools_in_executor() {
    let dir = TempDir::new().expect("tempdir should create");
    let context = Arc::new(ToolUseContext::new(
        "plan-mode-deny-write",
        Some(dir.path().to_string_lossy().to_string()),
    ));
    // 直接设置上下文状态，测试重点是执行器是否统一拦截写入工具。
    context.enter_plan_mode(Some("verify restrictions".to_string()), None);
    let mut security = SecurityPolicy::default();
    security.workspace_dir = dir.path().to_path_buf();
    let tools: Vec<Box<dyn Tool>> =
        vec![Box::new(EnterPlanModeTool::new()), Box::new(FileWriteTool::new(Arc::new(security)))];

    let error = execute_tool_from_registry(
        &tools,
        "write",
        json!({
            "filePath": "blocked.txt",
            "content": "nope"
        }),
        context,
    )
    .await
    .expect_err("plan mode should block file writes");

    assert!(error.message().contains("not allowed while plan mode is active"));
}
