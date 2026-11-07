use super::super::test_support;
use vw_api_types::{
    agent_apps::AppId, app_providers::import::ImportTrigger, app_providers::operations::*,
    id::TenantScope,
};
#[tokio::test]
async fn cc04_operations_sdk_scopes_requests_and_redacts_denial() {
    let server = test_support::server_with_tenant_scope(
        vec![(403, serde_json::json!({"secret":"hidden"}))],
        TenantScope::new("p", "a").unwrap(),
    );
    let req = ProviderOperationRequest {
        target_host_id: "host".into(),
        request_id: "request".into(),
        action: ProviderOperationAction::Import {
            app: AppId::Claude,
            trigger: ImportTrigger::Manual,
        },
    };
    let error = server
        .client()
        .provider_operation_start(&req, "fixture-token-over-32-chars")
        .await
        .unwrap_err();
    assert!(matches!(error, crate::AgentAppsReadError::Http { status: 403, .. }));
    assert!(!error.to_string().contains("hidden"));
    let captured = server.take_request();
    assert_eq!(captured.path, "/v1/app-providers/operations");
    assert_eq!(captured.method, "POST");
    assert_eq!(captured.header("x-company-id"), Some("a"));
    assert_eq!(captured.body, serde_json::to_value(req).unwrap());
    server.join();
}

#[tokio::test]
async fn cc04_operations_sdk_reads_and_cancels_only_the_scoped_operation() {
    let progress = serde_json::json!({"operationId":"opaque-id","requestId":"request","status":"running","phase":"settling","completed":1,"total":2,"cancellationRequested":true});
    let server = test_support::server_with_tenant_scope(
        vec![(200, progress.clone()), (200, progress)],
        TenantScope::new("p", "a").unwrap(),
    );
    let client = server.client();
    let read = client
        .provider_operation_get("opaque-id", "host name", "fixture-token-over-32-chars")
        .await
        .unwrap();
    let captured = server.take_request();
    assert_eq!(captured.method, "GET");
    assert_eq!(captured.path, "/v1/app-providers/operations/opaque-id?targetHostId=host+name");
    let cancelled = client
        .provider_operation_cancel("opaque-id", "host name", "fixture-token-over-32-chars")
        .await
        .unwrap();
    let captured = server.take_request();
    assert_eq!(captured.method, "DELETE");
    assert_eq!(captured.header("x-company-id"), Some("a"));
    assert_eq!(read, cancelled);
    assert_eq!(cancelled.phase, ProviderOperationPhase::Settling);
    server.join();
}
