use vw_api_app_provider_edit::import::*;
use vw_api_app_identity::{AppId};
#[tokio::test]
async fn cc04_import_sdk_scopes_trigger_and_discards_sensitive_error() {
    let server = crate::test_support::server_with_tenant_scope(
        vec![
            (200, serde_json::json!({"imported":2})),
            (422, serde_json::json!({"key":"fake-secret"})),
        ],
        vw_api_transport::TenantScope::new("p", "a").unwrap(),
    );
    let body = ProviderImport { target_host_id: "host".into(), trigger: ImportTrigger::Startup };
    assert_eq!(
        server.client().provider_import(AppId::Pi, &body, "fake-token").await.unwrap().imported,
        2
    );
    let r = server.take_request();
    assert_eq!(r.method, "POST");
    assert_eq!(r.path, "/v1/app-providers/pi/catalog/import");
    assert_eq!(r.body, serde_json::json!({"targetHostId":"host","trigger":"startup"}));
    let e = server.client().provider_import(AppId::Pi, &body, "fake-token").await.unwrap_err();
    assert!(!format!("{e:?}").contains("fake-secret"));
    server.take_request();
    server.join();
}
