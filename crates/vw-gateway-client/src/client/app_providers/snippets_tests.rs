use super::super::test_support;
use vw_api_types::{agent_apps::AppId, app_providers::snippets::*, id::TenantScope};
#[tokio::test]
async fn cc04_snippets_sdk_uses_routes_and_conceals_error_payloads() {
    let server = test_support::server_with_tenant_scope(
        vec![(403, serde_json::json!({"secret":"fixture-key"}))],
        TenantScope::new("p", "a").unwrap(),
    );
    let error = server
        .client()
        .app_provider_snippet_save(
            AppId::Claude,
            &SnippetWrite { target_host_id: "host".into(), content: "{}".into() },
            "fixture-token",
        )
        .await
        .unwrap_err();
    assert!(!error.to_string().contains("fixture-key"));
    let request = server.take_request();
    assert_eq!(request.method, "PUT");
    assert_eq!(request.path, "/v1/app-providers/claude/config/snippet");
    assert_eq!(request.body, serde_json::json!({"targetHostId":"host","content":"{}"}));
    server.join();
}
