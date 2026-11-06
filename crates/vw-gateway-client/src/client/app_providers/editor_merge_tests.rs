use super::super::test_support;
use vw_api_types::{agent_apps::AppId, app_providers::editor_merge::*, id::TenantScope};
#[tokio::test]
async fn cc04_editor_merge_sdk_uses_bound_route_and_conceals_errors() {
    let server = test_support::server_with_tenant_scope(
        vec![(403, serde_json::json!({"secret":"hidden"}))],
        TenantScope::new("p", "a").unwrap(),
    );
    let error = server
        .client()
        .app_provider_editor_open(
            AppId::Claude,
            "saved",
            &EditorOpen { target_host_id: "host".into() },
            "fixture-editor-token-at-least-32-chars",
        )
        .await
        .unwrap_err();
    assert!(!error.to_string().contains("hidden"));
    let request = server.take_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/app-providers/claude/saved/editor/open");
    assert_eq!(request.body, serde_json::json!({"targetHostId":"host"}));
    server.join();
}
