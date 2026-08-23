//! 验证 agent-browser 命令响应、启动参数和安全拒绝边界。
//! 失败退出仍优先解析结构化响应；文件网址和未授权域名必须在启动命令前拒绝。

use super::*;
use crate::app::agent::security::SecurityPolicy;
use std::sync::Arc;

#[test]
fn agent_browser_launch_args_preserve_existing_and_add_automation_evasion() {
    let target_arg = "--disable-blink-features=AutomationControlled";

    let empty = agent_browser_launch_args(None);
    assert_eq!(empty, target_arg);

    let existing = "--window-size=1280,720 --headless=new";
    let merged = agent_browser_launch_args(Some(existing));
    assert_eq!(merged, format!("{existing} {target_arg}"));

    let already_configured = agent_browser_launch_args(Some(target_arg));
    assert_eq!(already_configured, target_arg);

    for args in [empty, merged, already_configured] {
        assert_eq!(
            args.split_whitespace().filter(|arg| *arg == target_arg).count(),
            1,
            "automation evasion argument must appear exactly once"
        );
    }
}

#[test]
fn agent_browser_response_deserializes_success_and_error_shapes() {
    let ok: AgentBrowserResponse =
        serde_json::from_str(r#"{"success":true,"data":{"title":"Example"},"error":null}"#)
            .unwrap();
    assert!(ok.success);
    assert_eq!(ok.data.unwrap()["title"], "Example");
    assert!(ok.error.is_none());

    let err: AgentBrowserResponse =
        serde_json::from_str(r#"{"success":false,"data":null,"error":"not found"}"#).unwrap();
    assert!(!err.success);
    assert_eq!(err.error.as_deref(), Some("not found"));
}

#[test]
fn agent_browser_response_defaults_omitted_optional_fields_to_none() {
    let response: AgentBrowserResponse = serde_json::from_str(r#"{"success":true}"#).unwrap();

    assert!(response.success);
    assert!(response.data.is_none());
    assert!(response.error.is_none());
}

#[test]
fn agent_browser_response_preserves_arbitrary_nested_data() {
    let response: AgentBrowserResponse = serde_json::from_str(
        r#"{"success":true,"data":{"items":[1,null,{"visible":false}],"label":"页面"}}"#,
    )
    .unwrap();

    assert_eq!(
        response.data,
        Some(serde_json::json!({
            "items": [1, null, { "visible": false }],
            "label": "页面"
        }))
    );
}

#[test]
fn agent_browser_response_ignores_unknown_cli_fields() {
    let response: AgentBrowserResponse = serde_json::from_str(
        r#"{"success":true,"data":null,"error":null,"version":"2","duration_ms":12}"#,
    )
    .unwrap();

    assert!(response.success);
    assert!(response.data.is_none());
    assert!(response.error.is_none());
}

#[test]
fn agent_browser_response_rejects_missing_success_flag() {
    let error = serde_json::from_str::<AgentBrowserResponse>(r#"{"data":null,"error":null}"#)
        .expect_err("success is required by the response protocol");

    assert!(error.to_string().contains("missing field `success`"));
}

#[test]
fn agent_browser_response_rejects_non_boolean_success_flag() {
    let error = serde_json::from_str::<AgentBrowserResponse>(
        r#"{"success":"yes","data":null,"error":null}"#,
    )
    .expect_err("success must be a JSON boolean");

    assert!(error.to_string().contains("invalid type"));
}

#[test]
fn agent_browser_response_rejects_non_string_error() {
    let error = serde_json::from_str::<AgentBrowserResponse>(
        r#"{"success":false,"data":null,"error":{"message":"not found"}}"#,
    )
    .expect_err("error must be a string or null");

    assert!(error.to_string().contains("invalid type"));
}

#[test]
fn response_from_output_prefers_json_payload_over_failed_exit_status() {
    let response = response_from_output(
        false,
        r#"{"success":true,"data":{"title":"Example"},"error":null}"#,
        "ignored stderr",
    );

    assert!(response.success);
    assert_eq!(response.data.unwrap()["title"], "Example");
    assert!(response.error.is_none());
}

#[test]
fn response_from_output_wraps_trimmed_non_json_stdout_on_success() {
    let response = response_from_output(true, "  legacy output\n", "");

    assert!(response.success);
    assert_eq!(response.data, Some(serde_json::json!({ "output": "legacy output" })));
    assert!(response.error.is_none());
}

#[test]
fn response_from_output_uses_trimmed_stderr_on_failure() {
    let response = response_from_output(false, "not json", "  command failed\n");

    assert!(!response.success);
    assert!(response.data.is_none());
    assert_eq!(response.error.as_deref(), Some("command failed"));
}

#[test]
fn response_from_output_preserves_empty_legacy_outputs() {
    let success = response_from_output(true, "", "ignored");
    assert!(success.success);
    assert_eq!(success.data, Some(serde_json::json!({ "output": "" })));
    assert!(success.error.is_none());

    let failure = response_from_output(false, "", "  \n");
    assert!(!failure.success);
    assert!(failure.data.is_none());
    assert_eq!(failure.error.as_deref(), Some(""));
}

#[tokio::test]
async fn execute_agent_browser_action_rejects_file_url_before_command_execution() {
    let tool = BrowserTool::new(
        Arc::new(SecurityPolicy::default()),
        vec!["example.com".to_string()],
        None,
    );

    let result = tool
        .execute_agent_browser_action(BrowserAction::Open {
            url: "file:///tmp/private.txt".to_string(),
        })
        .await;
    let Err(error) = result else {
        panic!("file URL should be rejected before agent-browser is executed");
    };

    assert!(error.to_string().contains("file:// URLs are not allowed"));
}

#[tokio::test]
// 网址授权在命令启动前完成，拒绝请求不得产生子进程或向外部浏览器发送参数。
async fn execute_agent_browser_action_rejects_unapproved_urls_before_command_execution() {
    let tool = BrowserTool::new(
        Arc::new(SecurityPolicy::default()),
        vec!["example.com".to_string()],
        None,
    );

    for url in ["https://example.net", "https://localhost", "https://other.example"] {
        let error = tool
            .execute_agent_browser_action(BrowserAction::Open { url: url.to_string() })
            .await
            .expect_err("unsafe URL should be rejected before agent-browser is executed");
        assert!(!error.to_string().is_empty());
    }
}
