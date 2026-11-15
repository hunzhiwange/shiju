//! Sleep 工具测试。
//!
//! 覆盖正常等待和超长等待阻断，确保工具不会无界占用执行资源。

use super::SleepTool;
use tool_types::Tool;
use serde_json::json;
use std::time::Instant;

#[tokio::test]
async fn sleep_waits_for_requested_duration() {
    let tool = SleepTool::new();
    let start = Instant::now();
    let result = tool
        .execute(json!({
            "duration_ms": 5
        }))
        .await
        .expect("sleep should succeed");

    assert!(result.success);
    assert!(start.elapsed().as_millis() >= 5);
}

#[tokio::test]
async fn sleep_rejects_excessive_duration() {
    let tool = SleepTool::new();
    let error = tool
        .execute(json!({
            "duration_ms": 60001
        }))
        .await
        .expect_err("sleep should reject excessive duration");

    assert!(error.to_string().contains("exceeds"));
}
