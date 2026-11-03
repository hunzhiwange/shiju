//! Batch 工具聚合执行测试。
//!
//! 使用一个最小 echo 工具验证 batch 能顺序分发子调用，同时拒绝递归调用自身，
//! 防止工具编排形成无限嵌套。

use super::super::*;
use super::MAX_CALLS;
use async_trait::async_trait;
use serde_json::json;

struct EchoTool;
struct FailingTool;
struct ErrorTool;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for EchoTool {
    fn name(&self) -> &str {
        "echo"
    }

    fn description(&self) -> &str {
        "Echo input"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({"type": "object"})
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let value = args.get("value").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        Ok(ToolResult { success: true, output: value, error: None })
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for FailingTool {
    fn name(&self) -> &str {
        "failing"
    }

    fn description(&self) -> &str {
        "Returns a ToolResult failure"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({"type": "object"})
    }

    async fn execute(&self, _args: serde_json::Value) -> anyhow::Result<ToolResult> {
        Ok(ToolResult { success: false, output: "fallback failure".into(), error: None })
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for ErrorTool {
    fn name(&self) -> &str {
        "error"
    }

    fn description(&self) -> &str {
        "Returns an anyhow error"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({"type": "object"})
    }

    async fn execute(&self, _args: serde_json::Value) -> anyhow::Result<ToolResult> {
        anyhow::bail!("boom")
    }
}

fn build_tool() -> BatchTool {
    let tools: Arc<Vec<Arc<dyn Tool>>> =
        Arc::new(vec![Arc::new(EchoTool), Arc::new(FailingTool), Arc::new(ErrorTool)]);
    BatchTool::new(tools)
}

#[tokio::test]
async fn batch_executes_calls() {
    let tool = build_tool();
    let result = tool
        .execute(json!({
            "tool_calls": [
                {"tool": "echo", "args": {"value": "a"}},
                {"tool": "echo", "parameters": {"value": "b"}}
            ]
        }))
        .await
        .unwrap();

    assert!(result.success);
    assert!(result.output.contains("全部 2 个工具均执行成功"));
    assert!(result.output.contains("a"));
    assert!(result.output.contains("b"));
}

#[tokio::test]
async fn batch_rejects_recursive_call() {
    let tool = build_tool();
    // batch 调用 batch 会形成不可控递归，必须在调度前拒绝。
    let result = tool
        .execute(json!({
            "tool_calls": [
                {"tool": "batch", "args": {}}
            ]
        }))
        .await
        .unwrap();

    assert!(result.success);
    assert!(result.output.contains("不允许在 batch 中调用"));
}

#[test]
fn schema_and_registry_helpers_are_stable() {
    let tool = build_tool();

    assert_eq!(tool.name(), "batch");
    assert!(!tool.description().trim().is_empty());
    assert_eq!(BatchTool::schema()["required"][0], "tool_calls");
    assert_eq!(tool.available_tool_names(), vec!["echo", "failing", "error"]);
    assert!(tool.find_tool("echo").is_some());
    assert!(tool.find_tool("missing").is_none());
}

#[tokio::test]
async fn batch_accepts_aliases_and_default_empty_args() {
    let tool = build_tool();
    let result = tool
        .execute(json!({
            "calls": [
                {"tool": "echo"}
            ]
        }))
        .await
        .unwrap();

    assert!(result.success);
    assert!(result.output.contains("全部 1 个工具均执行成功"));
}

#[tokio::test]
async fn batch_reports_invalid_empty_unknown_and_failed_calls() {
    let tool = build_tool();

    let invalid = tool.execute(json!({"tool_calls": "nope"})).await.unwrap();
    assert!(!invalid.success);
    assert!(invalid.error.unwrap().contains("batch 输入参数无效"));

    let empty = tool.execute(json!({"tool_calls": []})).await.unwrap();
    assert!(!empty.success);
    assert!(empty.error.unwrap().contains("至少提供一个工具调用"));

    let mixed = tool
        .execute(json!({
            "toolCalls": [
                {"tool": "missing", "args": {}},
                {"tool": "failing", "args": {}},
                {"tool": "error", "args": {}}
            ]
        }))
        .await
        .unwrap();

    assert!(mixed.success);
    assert!(mixed.output.contains("已成功执行 0/3 个工具"));
    assert!(mixed.output.contains("不在注册表中"));
    assert!(mixed.output.contains("fallback failure"));
    assert!(mixed.output.contains("工具执行失败: boom"));
}

#[tokio::test]
async fn batch_discards_calls_over_the_limit() {
    let tool = build_tool();
    let calls = (0..=MAX_CALLS)
        .map(|idx| json!({"tool": "echo", "args": {"value": format!("call-{idx}")}}))
        .collect::<Vec<_>>();

    let result = tool.execute(json!({ "tool_calls": calls })).await.unwrap();

    assert!(result.success);
    assert!(result.output.contains("batch 最多允许 25 个工具调用"));
    assert!(result.output.contains("call-24"));
    assert!(!result.output.contains("call-25\n\n"));
}

#[tokio::test]
async fn batch_documented_example_executes_with_registered_tool_ids() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("Cargo.toml"), "example manifest").unwrap();
    let security = Arc::new(crate::app::agent::security::SecurityPolicy {
        workspace_dir: dir.path().to_path_buf(),
        ..Default::default()
    });
    let tool = BatchTool::new(Arc::new(vec![Arc::new(FileReadTool::new(security))]));
    let description = tool.description();
    let start = description.find("{\"tool_calls\"").expect("object payload example");
    let payload = description[start..].lines().next().unwrap();
    let args = serde_json::from_str(payload).unwrap();
    let result = tool.execute(args).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("全部 1 个工具均执行成功"));
}

#[test]
fn batch_schema_declares_nested_args() {
    let schema = BatchTool::schema();
    assert_eq!(schema["properties"]["tool_calls"]["items"]["properties"]["args"]["type"], "object");
}
