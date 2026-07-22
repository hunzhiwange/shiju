//! apply_patch 工具的单元测试模块
//!
//! 本模块提供对 `ApplyPatchTool` 的全面测试覆盖，验证补丁应用功能的正确性。
//! 主要测试场景包括：
//! - 工具 schema 的正确性验证
//! - 文件添加操作
//!
//! 测试使用临时目录进行隔离，确保不会影响实际工作空间。

use super::super::*;
use crate::app::agent::security::{AutonomyLevel, SecurityPolicy};
use crate::app::agent::tools::context::scope_tool_use_context;
use crate::app::agent::tools::{FileSnapshot, ToolUseContext};
use serde_json::json;
use tempfile::TempDir;
use vw_api_types::tools::ToolResultContentDto;

/// 创建测试用的安全策略
///
/// 构造一个受监督级别的安全策略，用于测试环境中的工具执行。
/// 该策略将工作空间限制在指定的临时目录中，确保测试的隔离性。
///
/// # 参数
///
/// - `workspace`: 测试用的工作空间目录路径
///
/// # 返回
///
/// 返回一个 `Arc<SecurityPolicy>` 智能指针，包含配置好的安全策略
fn test_security(workspace: std::path::PathBuf) -> Arc<SecurityPolicy> {
    Arc::new(SecurityPolicy {
        // 使用受监督级别，适用于测试环境
        autonomy: AutonomyLevel::Supervised,
        // 设置测试专用的工作空间目录
        workspace_dir: workspace,
        // 其他字段使用默认值
        ..SecurityPolicy::default()
    })
}

/// 测试 ApplyPatchTool 的 schema 是否为 object 类型
///
/// 验证工具返回的 JSON schema 具有正确的结构：
/// - 顶层类型为 "object"
/// - 包含 "properties" 字段
/// - "patchText" 属性存在且为 object 类型
#[test]
fn schema_is_object() {
    // 获取工具的 schema 定义
    let s = ApplyPatchTool::schema();

    // 验证 schema 顶层是 object 类型
    assert!(s.is_object());
    assert_eq!(s["type"], "object");

    // 验证包含 properties 字段
    assert!(s["properties"].is_object());

    // 验证 patchText 属性存在且类型正确
    assert!(s["properties"]["patchText"].is_object());
}

#[test]
fn tool_metadata_and_failure_results_are_stable() {
    let dir = TempDir::new().unwrap();
    let tool = ApplyPatchTool::new(test_security(dir.path().to_path_buf()));

    assert_eq!(tool.name(), "apply_patch");
    assert!(tool.description().contains("apply_patch"));
    assert_eq!(tool.parameters_schema(), ApplyPatchTool::schema());

    let failure = ApplyPatchTool::failure("blocked");
    assert!(!failure.success);
    assert_eq!(failure.output, "blocked");
    assert_eq!(failure.error.as_deref(), Some("blocked"));
}

#[tokio::test]
async fn apply_patch_rejects_invalid_empty_and_noop_inputs() {
    let dir = TempDir::new().unwrap();
    let tool = ApplyPatchTool::new(test_security(dir.path().to_path_buf()));

    let missing = tool.call(json!({})).await.unwrap_err();
    assert!(missing.to_string().contains("Missing or invalid 'patchText' parameter"));

    let wrong_type = tool.call(json!({"patchText": 7})).await.unwrap_err();
    assert!(wrong_type.to_string().contains("Missing or invalid 'patchText' parameter"));

    let empty = tool.call(json!({"patchText": "   "})).await.unwrap();
    assert!(!empty.is_success());
    assert!(empty.error_text().unwrap_or_default().contains("Missing patchText"));

    let noop = tool.call(json!({"patchText": "*** Begin Patch\n*** End Patch"})).await.unwrap();
    assert!(!noop.is_success());
    assert!(noop.error_text().unwrap_or_default().contains("empty patch"));
}

#[tokio::test]
async fn apply_patch_move_updates_destination_and_removes_source() {
    let dir = TempDir::new().unwrap();
    tokio::fs::write(dir.path().join("source.txt"), "old\n").await.unwrap();
    let tool = ApplyPatchTool::new(test_security(dir.path().to_path_buf()));

    let result = tool
        .call(json!({
            "patchText": "*** Begin Patch\n*** Update File: source.txt\n*** Move to: nested/target.txt\n@@\n-old\n+new\n*** End Patch"
        }))
        .await
        .unwrap();

    assert!(result.is_success(), "error: {:?}", result.error_text());
    assert_eq!(result.data["files"][0]["operation"], json!("move"));
    assert_eq!(result.data["files"][0]["from_path"], json!("source.txt"));
    assert_eq!(result.data["files"][0]["file"]["path"], json!("nested/target.txt"));
    assert!(!dir.path().join("source.txt").exists());
    assert_eq!(
        tokio::fs::read_to_string(dir.path().join("nested/target.txt")).await.unwrap(),
        "new\n"
    );
}

/// 测试应用添加文件的补丁功能
///
/// 该测试验证 ApplyPatchTool 能够正确处理 "Add File" 类型的补丁。
/// 测试流程：
/// 1. 创建临时测试目录
/// 2. 构造包含新文件内容的补丁
/// 3. 执行补丁应用
/// 4. 验证文件已正确创建且内容匹配
/// 5. 清理临时目录
#[tokio::test]
async fn apply_patch_add_file_succeeds() {
    let dir = std::env::temp_dir().join("vibewindow_test_tools_apply_patch_add");
    let _ = tokio::fs::remove_dir_all(&dir).await;
    tokio::fs::create_dir_all(&dir).await.unwrap();
    let tool = ApplyPatchTool::new(test_security(dir.clone()));

    let result = tool
        .call(json!({
            "patchText": "*** Begin Patch\n*** Add File: a.txt\n+hello\n*** End Patch"
        }))
        .await
        .unwrap();

    assert!(result.is_success(), "error: {:?}", result.error_text());
    assert_eq!(result.data["kind"], json!("apply_patch"));
    assert_eq!(result.data["files"][0]["operation"], json!("add"));
    assert_eq!(result.data["files"][0]["file"]["path"], json!("a.txt"));
    assert_eq!(
        result.render_hint.as_ref().and_then(|hint| hint.kind.as_deref()),
        Some("apply_patch")
    );
    assert!(matches!(
        result.content_blocks.first(),
        Some(ToolResultContentDto::StructuredPatch { hunks }) if !hunks.is_empty()
    ));

    let content = tokio::fs::read_to_string(dir.join("a.txt")).await.unwrap();
    assert_eq!(content, "hello\n");

    let _ = tokio::fs::remove_dir_all(&dir).await;
}

#[tokio::test]
async fn apply_patch_output_contains_changes_metadata() {
    let dir = std::env::temp_dir().join("vibewindow_test_tools_apply_patch_changes");
    let _ = tokio::fs::remove_dir_all(&dir).await;
    tokio::fs::create_dir_all(&dir).await.unwrap();
    tokio::fs::write(dir.join("a.txt"), "old\n").await.unwrap();

    let tool = ApplyPatchTool::new(test_security(dir.clone()));
    let result = tool
        .call(json!({
            "patchText": "*** Begin Patch\n*** Update File: a.txt\n@@\n-old\n+new\n*** End Patch"
        }))
        .await
        .unwrap();

    assert!(result.is_success(), "error: {:?}", result.error_text());
    assert_eq!(result.data["kind"], json!("apply_patch"));
    assert_eq!(result.data["files"][0]["operation"], json!("update"));
    assert_eq!(result.data["files"][0]["additions"], json!(1));
    assert_eq!(result.data["files"][0]["deletions"], json!(1));
    assert!(matches!(
        result.content_blocks.first(),
        Some(ToolResultContentDto::StructuredPatch { hunks }) if !hunks.is_empty()
    ));

    let _ = tokio::fs::remove_dir_all(&dir).await;
}

#[tokio::test]
async fn apply_patch_reports_partial_read_state_when_context_present() {
    let dir = std::env::temp_dir().join("vibewindow_test_tools_apply_patch_read_state");
    let _ = tokio::fs::remove_dir_all(&dir).await;
    tokio::fs::create_dir_all(&dir).await.unwrap();
    tokio::fs::write(dir.join("a.txt"), "old\nsecond\n").await.unwrap();

    let tool = ApplyPatchTool::new(test_security(dir.clone()));
    let context = Arc::new(ToolUseContext::new(
        "apply-patch-read-state",
        Some(dir.to_string_lossy().to_string()),
    ));
    {
        let read_state = context.read_state_handle();
        read_state.lock().unwrap_or_else(|error| error.into_inner()).note_read(
            Some(dir.as_path()),
            "a.txt",
            8,
            true,
            Some(1),
            Some(1),
            Some(FileSnapshot::from_text("old\nsecond\n")),
        );
    }

    let result = scope_tool_use_context(
        context.clone(),
        tool.call(json!({
            "patchText": "*** Begin Patch\n*** Update File: a.txt\n@@\n-old\n+new\n*** End Patch"
        })),
    )
    .await
    .unwrap();

    assert!(result.is_success(), "error: {:?}", result.error_text());
    assert_eq!(result.data["files"][0]["read_state"]["status"], json!("partial"));
    assert_eq!(result.data["files"][0]["read_state"]["offset"], json!(1));

    let mut snapshot = context.read_state_snapshot();
    let entry = snapshot.get(Some(dir.as_path()), "a.txt").expect("read state missing");
    assert_eq!(entry.snapshot, Some(FileSnapshot::from_text("new\nsecond\n")));

    let _ = tokio::fs::remove_dir_all(&dir).await;
}

#[tokio::test]
async fn apply_patch_invalidates_read_state_for_deleted_file() {
    let dir = std::env::temp_dir().join("vibewindow_test_tools_apply_patch_delete_state");
    let _ = tokio::fs::remove_dir_all(&dir).await;
    tokio::fs::create_dir_all(&dir).await.unwrap();
    tokio::fs::write(dir.join("a.txt"), "old\n").await.unwrap();

    let tool = ApplyPatchTool::new(test_security(dir.clone()));
    let context = Arc::new(ToolUseContext::new(
        "apply-patch-delete-state",
        Some(dir.to_string_lossy().to_string()),
    ));
    {
        let read_state = context.read_state_handle();
        read_state.lock().unwrap_or_else(|error| error.into_inner()).note_read(
            Some(dir.as_path()),
            "a.txt",
            4,
            false,
            None,
            None,
            Some(FileSnapshot::from_text("old\n")),
        );
    }

    let result = scope_tool_use_context(
        context.clone(),
        tool.call(json!({
            "patchText": "*** Begin Patch\n*** Delete File: a.txt\n*** End Patch"
        })),
    )
    .await
    .unwrap();

    assert!(result.is_success(), "error: {:?}", result.error_text());
    assert_eq!(result.data["files"][0]["operation"], json!("delete"));

    let mut snapshot = context.read_state_snapshot();
    assert!(snapshot.get(Some(dir.as_path()), "a.txt").is_none());

    let _ = tokio::fs::remove_dir_all(&dir).await;
}
