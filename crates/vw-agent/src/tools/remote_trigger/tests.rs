//! 远端触发工具测试。
//!
//! 当前运行时没有稳定远端触发后端，因此测试锁定显式不支持的返回语义。

use super::RemoteTriggerTool;
use crate::app::agent::tools::Tool;
use serde_json::{Value, json};

#[test]
fn remote_trigger_exposes_stable_metadata() {
    let tool = RemoteTriggerTool::new();

    assert_eq!(tool.name(), "RemoteTrigger");
    assert_eq!(
        tool.description(),
        "Trigger a remote task or environment action. Returns an explicit error when the runtime has no stable remote-trigger backend configured."
    );
}

#[test]
fn remote_trigger_schema_requires_target_and_rejects_extra_properties() {
    let schema = RemoteTriggerTool::new().parameters_schema();

    assert_eq!(schema["type"], "object");
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(schema["properties"]["target"]["type"], "string");
    assert_eq!(schema["properties"]["payload"]["type"], "object");
    assert_eq!(schema["required"], json!(["target"]));
}

#[test]
fn remote_trigger_spec_marks_strict_non_concurrent_write_capability() {
    let tool = RemoteTriggerTool::new();
    let schema = tool.parameters_schema();
    let spec = tool.spec();

    assert_eq!(spec.id, "RemoteTrigger");
    assert_eq!(spec.display_name, "RemoteTrigger");
    assert_eq!(spec.name, "RemoteTrigger");
    assert_eq!(spec.description, tool.description());
    assert_eq!(spec.input_schema, schema);
    assert_eq!(spec.parameters, schema);
    assert_eq!(spec.aliases, vec!["remote_trigger".to_string()]);
    assert!(!spec.read_only);
    assert!(!spec.destructive);
    assert!(!spec.concurrency_safe);
    assert!(!spec.requires_user_interaction);
    assert!(spec.strict);
}

#[tokio::test]
async fn remote_trigger_without_payload_returns_structured_unsupported_result() {
    let tool = RemoteTriggerTool::new();
    let result = tool
        .execute(json!({
            "target": "prod-eu"
        }))
        .await
        .expect("remote trigger should return structured result");

    assert!(!result.success);
    assert_eq!(result.error.as_deref(), Some("RemoteTrigger is not supported in this runtime"));
    let detail: Value = serde_json::from_str(&result.output).expect("output should contain JSON");
    assert_eq!(
        detail,
        json!({
            "target": "prod-eu",
            "payload": null,
            "reason": "remote trigger backend is not configured in this runtime"
        })
    );
}

#[tokio::test]
async fn remote_trigger_preserves_payload_in_unsupported_detail() {
    let result = RemoteTriggerTool::new()
        .execute(json!({
            "target": "staging-worker",
            "payload": {
                "job": "refresh-index",
                "attempt": 2
            }
        }))
        .await
        .expect("remote trigger should return structured result");

    assert!(!result.success);
    let detail: Value = serde_json::from_str(&result.output).expect("output should contain JSON");
    assert_eq!(
        detail,
        json!({
            "target": "staging-worker",
            "payload": {
                "job": "refresh-index",
                "attempt": 2
            },
            "reason": "remote trigger backend is not configured in this runtime"
        })
    );
}

#[tokio::test]
async fn remote_trigger_rejects_missing_target() {
    let error = RemoteTriggerTool::new()
        .execute(json!({"payload": {"job": "refresh-index"}}))
        .await
        .expect_err("target is required");

    assert!(error.to_string().contains("missing field `target`"));
}

#[tokio::test]
async fn remote_trigger_rejects_non_string_target() {
    let error = RemoteTriggerTool::new()
        .execute(json!({"target": 42}))
        .await
        .expect_err("target must be a string");

    let message = error.to_string();
    assert!(message.contains("invalid type"));
    assert!(message.contains("string"));
}
