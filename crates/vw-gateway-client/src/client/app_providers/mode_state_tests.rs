use vw_api_types::{agent_apps::AppId, app_providers::mode_state::*};
#[tokio::test]
async fn cc04_mode_state_sdk_scopes_query_and_discards_sensitive_error() {
    let status = serde_json::json!({"mode":"direct","directProvider":"saved","inUseProvider":"saved","proxyRoute":null,"attached":false,"hasContract":true,"stack":{"enabled":false,"active":false,"members":[]},"decision":"skipRewrite","routeRuntime":"unavailable"});
    let server = super::super::test_support::server_with_tenant_scope(
        vec![(200, status), (503, serde_json::json!({"key":"fake-sensitive"}))],
        vw_api_types::id::TenantScope::new("p", "c").unwrap(),
    );
    let q = ModeStateQuery {
        target_host_id: "host".into(),
        provider_id: Some("saved".into()),
        mode: ProviderMode::Direct,
    };
    let view = server.client().provider_mode_state(AppId::Claude, &q, "fake-token").await.unwrap();
    assert_eq!(view.decision, Some(RewriteDecision::SkipRewrite));
    let request = server.take_request();
    assert_eq!(request.method, "GET");
    assert_eq!(
        request.path,
        "/v1/app-providers/claude/mode/state?targetHostId=host&providerId=saved&mode=direct"
    );
    let error =
        server.client().provider_mode_state(AppId::Claude, &q, "fake-token").await.unwrap_err();
    assert!(!format!("{error:?}").contains("fake-sensitive"));
    server.take_request();
    server.join();
}
