use super::super::test_support;
use vw_api_types::{agent_apps::AppId, app_providers::env_diagnose::*, id::TenantScope};
#[tokio::test]
async fn cc04_env_diagnose_sdk_encodes_target_and_redacts_denial() {
    let server = test_support::server_with_tenant_scope(
        vec![(403, serde_json::json!({"secret":"hidden"}))],
        TenantScope::new("p", "a").unwrap(),
    );
    let request = EnvDiagnoseRequest { target_host_id: "host /?&".into() };
    let error = server
        .client()
        .provider_env_diagnose(AppId::Claude, &request, "fixture-config-read-token-over-32")
        .await
        .unwrap_err();
    assert!(matches!(error, crate::AgentAppsReadError::Http { status: 403, .. }));
    assert!(!error.to_string().contains("hidden"));
    let captured = server.take_request();
    assert_eq!(captured.path, "/v1/app-providers/claude/environment/diagnose");
    assert_eq!(captured.method, "POST");
    assert_eq!(captured.body, serde_json::to_value(request).unwrap());
    assert_eq!(captured.header("x-company-id"), Some("a"));
    server.join();
}
