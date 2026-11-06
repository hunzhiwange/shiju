use super::super::test_support;
use vw_api_types::{agent_apps::AppId, app_providers::preview::*, id::TenantScope};
#[tokio::test]
async fn cc04_preview_sdk_encodes_target_and_redacts_denial() {
    let server = test_support::server_with_tenant_scope(
        vec![(403, serde_json::json!({"secret":"hidden"}))],
        TenantScope::new("p", "a").unwrap(),
    );
    let request = ProviderPreviewRequest {
        target_host_id: "host /?&".into(),
        directory_override: Some("~/custom".into()),
        settings: serde_json::json!({"apiKey":"fixture-secret"}),
        category: None,
        provider_id: None,
        meta: None,
    };
    let error = server
        .client()
        .app_provider_preview(AppId::Omp, &request, "fixture-config-read-token-over-32")
        .await
        .unwrap_err();
    assert!(matches!(error, crate::AgentAppsReadError::Http { status: 403, .. }));
    assert!(!error.to_string().contains("hidden"));
    let captured = server.take_request();
    assert_eq!(captured.path, "/v1/app-providers/omp/config/preview");
    assert_eq!(captured.method, "POST");
    assert_eq!(captured.body, serde_json::to_value(request).unwrap());
    assert_eq!(captured.header("x-company-id"), Some("a"));
    server.join();
}
