use crate::client::test_support;
use serde_json::json;
use vw_api_types::agent_apps_skills::claude_api::*;

#[tokio::test]
async fn cp12_claude_api_success() {
    let scope = ClaudeConfigQuery { project_id: "project-a".into(), target_host_id: "host-a".into() };
    let server = test_support::server(vec![
        (200, json!({"scope":scope,"status":{"state":"valid","primaryApiKeyPresent":true,"managed":true}})),
        (200, json!({"scope":scope,"changed":true})),
        (200, json!({"scope":scope,"changed":false})),
    ]);
    let status = server.client().claude_config_status(&scope, "test-token").await.unwrap();
    assert!(matches!(status.status, ClaudeConfigStatus::Valid { managed: true, .. }));
    let read = server.take_request();
    assert_eq!(read.method, "GET");
    assert_eq!(read.header("x-agent-apps-token"), Some("test-token"));
    let request = ClaudeConfigRequest { scope, action: ClaudeConfigAction::Disable };
    assert!(server.client().claude_config_marker(&request, "test-token").await.unwrap().changed);
    assert_eq!(server.take_request().method, "POST");
    assert!(!server.client().claude_onboarding(&request, "test-token").await.unwrap().changed);
    assert_eq!(server.take_request().method, "POST");
    server.join();
}

#[tokio::test]
async fn cp12_claude_api_does_not_echo_error_payload() {
    let server = test_support::server(vec![(403, json!({"credential":"private-value"}))]);
    let scope = ClaudeConfigQuery { project_id: "project-a".into(), target_host_id: "host-a".into() };
    assert_eq!(server.client().claude_config_status(&scope, "test-token").await.unwrap_err(),
        "Claude control HTTP 403");
    server.take_request();
    server.join();
}
