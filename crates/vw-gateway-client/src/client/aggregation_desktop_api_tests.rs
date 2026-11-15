use super::{AgentAppsSettingsError, GatewayClient, test_support};
use crate::endpoint::GatewayAuth;
use serde_json::json;
use vw_api_app_aggregation::*;
use vw_api_transport::id::TenantScope;

fn scope() -> AggregationQuery {
    AggregationQuery { target_host_id: "host +&".into(), project_id: "project +&".into() }
}
fn form() -> serde_json::Value {
    json!({"revision":"desktop-r1","form":{"name":"Fixture","baseUrl":"https://fixture.invalid","official":false,"directEditable":false,"mapping":{"mode":"proxy","routes":{"claude-sonnet-5":{"model":"upstream-one","supports1m":true}}},"mappingEditable":true}})
}
fn client(server: &test_support::TestServer) -> GatewayClient {
    GatewayClient::new(
        server
            .client()
            .endpoint()
            .clone()
            .with_auth(GatewayAuth { skey: Some("fixture-bearer".into()) }),
    )
    .unwrap()
}

#[tokio::test]
async fn aggregation_t53_aggregation_desktop_api_preserves_contract() {
    let server = test_support::server_with_tenant_scope(
        vec![(200, form()), (200, form())],
        TenantScope::new("platform", "company").unwrap(),
    );
    let client = client(&server);
    let view = client.aggregation_desktop_mapping("profile", &scope()).await.unwrap();
    assert_eq!(view.revision, "desktop-r1");
    assert_eq!(view.form.mapping.routes["claude-sonnet-5"].model, "upstream-one");
    assert!(view.form.mapping.routes["claude-sonnet-5"].supports_1m);
    let read = server.take_request();
    assert_eq!(read.method, "POST");
    assert_eq!(read.path, "/v1/app-providers/claude-desktop/profile/form/read");
    assert_eq!(read.body, serde_json::to_value(scope()).unwrap());
    assert_eq!(read.header("authorization"), Some("Bearer fixture-bearer"));
    assert_eq!(read.header("x-platform-id"), Some("platform"));
    assert_eq!(read.header("x-company-id"), Some("company"));
    let change = DesktopMappingChange {
        target_host_id: scope().target_host_id,
        project_id: scope().project_id,
        expected_revision: view.revision,
        mapping: view.form.mapping,
    };
    let saved = client.aggregation_desktop_mapping_save("profile", &change).await.unwrap();
    assert_eq!(saved.revision, "desktop-r1");
    assert_eq!(saved.form.mapping, change.mapping);
    let write = server.take_request();
    assert_eq!(write.method, "PUT");
    assert_eq!(write.path, "/v1/app-providers/claude-desktop/profile/model-mapping");
    assert_eq!(write.body, serde_json::to_value(change).unwrap());
    assert_eq!(write.header("authorization"), Some("Bearer fixture-bearer"));
    server.join();
}
