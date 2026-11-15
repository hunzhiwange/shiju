//! 验证工作流运行请求的默认步数、应用字段映射及状态名称。

use super::workflow::{WorkflowRunRequest, WorkflowRunStatus};
use serde_json::json;

// 空运行请求使用统一步数上限，并保持输入和内联定义为空。
#[test]
fn workflow_run_request_defaults_max_steps() {
    let request: WorkflowRunRequest = serde_json::from_value(json!({})).expect("request");

    assert_eq!(request.max_steps, 200);
    assert!(request.inputs.is_empty());
    assert_eq!(request.workflow_yaml, None);
    assert_eq!(WorkflowRunRequest::default().max_steps, 200);
}

// Rust 内部字段在协议上沿用应用标识和应用工作流名称，不泄露内部命名。
#[test]
fn workflow_run_request_uses_application_contract() {
    let request = WorkflowRunRequest {
        workflow_uuid: Some("app-uuid".to_string()),
        workflow_yaml: Some("workflow: {}".to_string()),
        ..WorkflowRunRequest::default()
    };

    let value = serde_json::to_value(request).expect("request");

    assert_eq!(value["application_uuid"], json!("app-uuid"));
    assert_eq!(value["application_workflow"], json!("workflow: {}"));
    assert!(value.get("workflow_uuid").is_none());
    assert!(value.get("workflow_yaml").is_none());
}

// 工作流运行状态以 snake_case 编码，供不同客户端稳定解析。
#[test]
fn workflow_status_uses_snake_case_contract() {
    let value = serde_json::to_value(WorkflowRunStatus::Succeeded).expect("status");

    assert_eq!(value, json!("succeeded"));
}
