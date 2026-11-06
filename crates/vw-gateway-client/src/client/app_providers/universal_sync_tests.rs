use super::super::test_support;
use vw_api_types::{agent_apps::AppId, app_providers::universal_sync::*, id::TenantScope};
#[tokio::test]
async fn cc04_universal_sync_sdk_wire_and_redaction() {
    let server = test_support::server_with_tenant_scope(
        vec![
            (
                200,
                serde_json::json!([{"appId":"claude","providerId":"universal-claude-shared","catalog":{"status":"updated"},"live":"not_attempted"}]),
            ),
            (403, serde_json::json!({"message":"fixture-key /private/path"})),
        ],
        TenantScope::new("p", "a").unwrap(),
    );
    let req = UniversalSyncRequest { apps: vec![AppId::Claude] };
    let rows = server.client().universal_provider_sync("shared", &req).await.unwrap();
    assert_eq!(rows[0].live, UniversalLiveResult::NotAttempted);
    let wire = server.take_request();
    assert_eq!(wire.method, "POST");
    assert_eq!(wire.path, "/v1/universal-providers/shared/sync");
    assert_eq!(wire.header("x-company-id"), Some("a"));
    assert_eq!(wire.body, serde_json::json!({"apps":["claude"]}));
    let error = server.client().universal_provider_sync("shared", &req).await.unwrap_err();
    assert!(error.contains("403"));
    assert!(!error.contains("fixture-key") && !error.contains("/private"));
    server.take_request();
    assert!(server.client().universal_provider_sync("../bad", &req).await.is_err());
    server.join();
}
