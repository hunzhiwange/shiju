use super::super::test_support;
use vw_api_types::{agent_apps::AppId, app_providers::preview::*, id::TenantScope};
#[tokio::test]
async fn cc04_codex_sdk_uses_readonly_projection_and_conceals_errors() {
    let server = test_support::server_with_tenant_scope(
        vec![(400, serde_json::json!({"secret":"fake-response-key"}))],
        TenantScope::new("p", "a").unwrap(),
    );
    let request = ProviderPreviewRequest {
        target_host_id: "host".into(),
        directory_override: None,
        settings: serde_json::json!({"auth":{"OPENAI_API_KEY":"fake-route-key"},"config":"model='fixture'"}),
        category: None,
        provider_id: None,
        meta: None,
        request_options: None,
    };
    let error = server
        .client()
        .app_provider_preview(AppId::Codex, &request, "fixture-config-reader-token-over-32")
        .await
        .unwrap_err();
    assert!(matches!(error, crate::AgentAppsReadError::Http { status: 400, .. }));
    assert!(!error.to_string().contains("fake-response-key"));
    let captured = server.take_request();
    assert_eq!(captured.method, "POST");
    assert_eq!(captured.path, "/v1/app-providers/codex/config/preview");
    assert_eq!(captured.body, serde_json::to_value(request).unwrap());
    assert_eq!(captured.header("x-company-id"), Some("a"));
    server.join();
}
