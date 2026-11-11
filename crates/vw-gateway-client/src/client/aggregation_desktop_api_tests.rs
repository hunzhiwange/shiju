use super::{AgentAppsSettingsError, GatewayClient, test_support};
use crate::endpoint::GatewayAuth;
use serde_json::json;
use vw_api_types::{agent_apps::aggregation::*, id::TenantScope};

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

#[tokio::test]
async fn aggregation_t53_status_preserves_modes_installation_and_recovery() {
    for (installation, mode, recovery) in [
        ("installed", "official", "none"),
        ("installed", "direct_gateway", "restored"),
        ("not_installed", "routing", "pending"),
        ("unsupported", "aggregation", "failed"),
    ] {
        let body = json!({"revision":"r1","installation":installation,"configured":true,"mode":mode,"appliedProviderId":"profile","notices":["restart_desktop"],"recovery":recovery});
        let server = test_support::server(vec![(200, body.clone())]);
        let status = client(&server).aggregation_desktop_status(&scope()).await.unwrap();
        assert_eq!(serde_json::to_value(status).unwrap(), body);
        let request = server.take_request();
        assert_eq!(request.method, "GET");
        let url = reqwest::Url::parse(&format!("{}{}", server.base_url(), request.path)).unwrap();
        assert_eq!(url.path(), "/v1/agent-apps/claude-desktop/profile/status");
        let query = url.query_pairs().collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(query["targetHostId"], scope().target_host_id);
        assert_eq!(query["projectId"], scope().project_id.as_ref());
        assert_eq!(request.body, serde_json::Value::Null);
        assert_eq!(request.header("authorization"), Some("Bearer fixture-bearer"));
        server.join();
    }
}

#[tokio::test]
async fn aggregation_t53_mapping_errors_preserve_status_code_and_recovery() {
    for (status, code) in [
        (401, "unauthenticated"),
        (403, "forbidden"),
        (409, "revision_conflict"),
        (422, "unsupported_target"),
        (502, "recovery_failed"),
        (503, "target_unavailable"),
        (504, "target_timeout"),
    ] {
        let body = json!({"error":{"code":code,"message":code,"details":{"recovery":"failed"}}});
        let server = test_support::server(vec![(status, body.clone())]);
        let change: DesktopMappingChange = serde_json::from_value(json!({"targetHostId":"host","projectId":"project","expectedRevision":"r1","mapping":{"mode":"proxy","routes":{}}})).unwrap();
        let error =
            client(&server).aggregation_desktop_mapping_save("profile", &change).await.unwrap_err();
        assert_eq!(error, AgentAppsSettingsError::Http { status, body });
        assert_eq!(server.take_request().method, "PUT");
        server.join();
    }
}

#[tokio::test]
async fn aggregation_t53_mapping_rejects_path_identities_before_transport() {
    let client = GatewayClient::new(crate::GatewayEndpoint::new("127.0.0.1", 1)).unwrap();
    for id in ["", "../escape", "a/b", "a?query", "%2e%2e", "中文"] {
        let error = client.aggregation_desktop_mapping(id, &scope()).await.unwrap_err();
        assert!(
            matches!(error, AgentAppsSettingsError::Transport(ref message) if message == "invalid provider identity")
        );
    }
}

#[tokio::test]
async fn aggregation_t53_malformed_response_and_redirect_never_become_success() {
    let server = test_support::server_raw(vec![(200, "not-json".into()), (307, "redirect".into())]);
    let client = client(&server);
    assert!(matches!(
        client.aggregation_desktop_status(&scope()).await,
        Err(AgentAppsSettingsError::Decode(_))
    ));
    assert!(matches!(
        client.aggregation_desktop_mapping("profile", &scope()).await,
        Err(AgentAppsSettingsError::Http { status: 307, .. })
    ));
    server.take_request();
    server.take_request();
    server.join();
}
