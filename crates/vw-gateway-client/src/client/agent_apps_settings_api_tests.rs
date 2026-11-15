use super::test_support;

#[tokio::test]
async fn settings_facade_preserves_domain_error_and_request() {
    let body = serde_json::json!({"code": "denied"});
    let server = test_support::server(vec![(403, body.clone())]);
    let error: gateway_app_settings::AgentAppsSettingsError = server.client()
        .agent_apps_preferences_authorized("remote", "separate-grant")
        .await.unwrap_err();
    assert_eq!(error, super::AgentAppsSettingsError::Http { status: 403, body });
    let request = server.take_request();
    assert_eq!(request.method, "GET");
    assert_eq!(request.path, "/v1/agent-apps/preferences?targetHostId=remote");
    assert_eq!(request.header("x-agent-apps-token"), Some("separate-grant"));
    server.join();
}
