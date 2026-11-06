use super::super::test_support;
use serde_json::json;
use vw_api_types::{agent_apps::AppId, app_providers::create::AppProviderCreate, id::TenantScope};
#[tokio::test]
async fn cc04_validation_sdk_transmits_write_only_options_and_sanitizes_errors() {
    let server = test_support::server_with_tenant_scope(
        vec![(400, json!({"secret":"fixture-private"}))],
        TenantScope::new("p", "a").unwrap(),
    );
    let request:AppProviderCreate=serde_json::from_value(json!({"name":"fixture","protocol":"anthropic","apiKey":{"action":"replace","value":"fixture-private"},"baseUrl":"https://fixture.test","model":"fixture","addToLive":false,"requestOptions":{"requestHeaders":{"Authorization":"fixture-private"}}})).unwrap();
    let error =
        server.client().app_provider_create(AppId::Claude, "fixture", &request).await.unwrap_err();
    assert!(error.contains("400"));
    assert!(!error.contains("fixture-private"));
    let wire = server.take_request();
    assert_eq!(wire.path, "/v1/app-providers/claude/fixture");
    assert_eq!(wire.body["requestOptions"]["requestHeaders"]["Authorization"], "fixture-private");
    server.join();
}

#[tokio::test]
async fn cc04_validation_sdk_save_never_exposes_raw_error_payload() {
    let server = test_support::server_with_tenant_scope(
        vec![(400, json!({"echo":"fixture-private-body"}))],
        TenantScope::new("p", "a").unwrap(),
    );
    let profile:vw_api_types::app_providers::AppProviderWrite=serde_json::from_value(json!({"name":"fixture","protocol":"anthropic","apiKey":"fixture-private","baseUrl":"https://fixture.test","model":"fixture","requestOptions":{"localProxyRequestOverrides":{"body":{"pluginData":"fixture-private-body"}}}})).unwrap();
    let error =
        server.client().app_provider_save(AppId::Claude, "fixture", &profile).await.unwrap_err();
    assert!(error.contains("400"));
    assert!(!error.contains("fixture-private"));
    assert_eq!(
        server.take_request().body["requestOptions"]["localProxyRequestOverrides"]["body"]["pluginData"],
        "fixture-private-body"
    );
    server.join();
}

#[derive(Clone, Default)]
struct Captured(std::sync::Arc<std::sync::Mutex<String>>);
impl tracing::Subscriber for Captured {
    fn enabled(&self, metadata: &tracing::Metadata<'_>) -> bool {
        metadata.target() == "vw_gateway_client"
    }
    fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
    fn event(&self, event: &tracing::Event<'_>) {
        struct Visitor<'a>(&'a mut String);
        impl tracing::field::Visit for Visitor<'_> {
            fn record_debug(&mut self, _: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                use std::fmt::Write;
                write!(self.0, "{value:?}").unwrap();
            }
        }
        event.record(&mut Visitor(&mut self.0.lock().unwrap()));
    }
    fn enter(&self, _: &tracing::span::Id) {}
    fn exit(&self, _: &tracing::span::Id) {}
}
#[tokio::test]
async fn cc04_validation_sdk_common_options_never_enter_domain_logs() {
    let captured = Captured::default();
    let _guard = tracing::subscriber::set_default(captured.clone());
    tracing::info!(target:"vw_gateway_client","capture-ready");
    let server = test_support::server_with_tenant_scope(
        vec![(400, json!({"echo":"fixture-private-body"})); 3],
        TenantScope::new("p", "a").unwrap(),
    );
    let request:AppProviderCreate=serde_json::from_value(json!({"name":"fixture","protocol":"anthropic","apiKey":{"action":"replace","value":"fixture-private-key"},"baseUrl":"https://fixture.test","model":"fixture","addToLive":false,"requestOptions":{"requestHeaders":{"X-Arbitrary":"fixture-private-header"},"localProxyRequestOverrides":{"body":{"arbitrary":"fixture-private-body"}}}})).unwrap();
    let profile:vw_api_types::app_providers::AppProviderWrite=serde_json::from_value(json!({"name":"fixture","protocol":"anthropic","apiKey":"fixture-private-key","baseUrl":"https://fixture.test","model":"fixture","requestOptions":request.request_options})).unwrap();
    let patch: vw_api_types::app_providers::update::AppProviderUpdate =
        serde_json::from_value(json!({"requestOptions":request.request_options})).unwrap();
    assert!(server.client().app_provider_create(AppId::Claude, "fixture", &request).await.is_err());
    server.take_request();
    assert!(server.client().app_provider_save(AppId::Claude, "fixture", &profile).await.is_err());
    server.take_request();
    assert!(server.client().app_provider_update(AppId::Claude, "fixture", &patch).await.is_err());
    server.take_request();
    server.join();
    let logs = captured.0.lock().unwrap();
    assert!(logs.contains("capture-ready"));
    assert!(!logs.contains("fixture-private"));
}
