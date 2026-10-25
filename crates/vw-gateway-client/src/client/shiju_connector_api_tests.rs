//! 校验个人连接器传输路径、凭证头和可空变量列表。
use super::*;
use crate::client::{SHIJU_CREDENTIAL_HEADER, test_support};
use serde_json::json;
#[test]
fn connector_accepts_null_variables_without_binding() {
    let connector: ShijuConnector = serde_json::from_value(json!({"platform_id": 1, "platform_code": "sample", "platform_name": "连接器", "company_platform_id": 7, "authorization_available": false, "account_binding_variables": null})).unwrap();
    assert!(connector.account_binding_variables.is_empty());
    assert!(connector.binding.is_none());
}
#[tokio::test]
async fn connector_client_uses_sensitive_gateway_routes() {
    let flow = "a".repeat(32);
    let binding = json!({"third_party_account_id": "account", "third_party_account_name": "用户", "binding_config": {}, "status": "enabled"});
    let server = test_support::server(vec![
        (200, json!([])),
        (200, binding),
        (
            200,
            json!({"flow_id": flow, "authorization_url": "https://provider.example/authorize", "poll_interval_seconds": 1, "expires_in_seconds": 60}),
        ),
        (200, json!({"status": "approved"})),
    ]);
    let credential = ShijuCredential::new("private-connector-key").unwrap();
    let client = server.client();
    assert!(client.shiju_connectors(&credential).await.unwrap().is_empty());
    let request = ShijuConnectorSaveRequest {
        company_platform_id: 7,
        third_party_account_id: "account".into(),
        third_party_account_name: "用户".into(),
        binding_config: serde_json::from_value(json!({"token": "private-binding-secret"})).unwrap(),
    };
    client.shiju_connector_save(&credential, &request).await.unwrap();
    let challenge = client.shiju_connector_authorize(&credential, 7).await.unwrap();
    assert_eq!(
        client.shiju_connector_poll(&credential, &challenge.flow_id).await.unwrap().status,
        ShijuConnectorAuthorizationStatus::Approved
    );
    for (method, path) in [
        ("GET", "/v1/shiju/connectors".to_string()),
        ("PUT", "/v1/shiju/connectors/binding".to_string()),
        ("POST", "/v1/shiju/connectors/authorizations".to_string()),
        ("POST", "/v1/shiju/connectors/authorizations/poll".to_string()),
    ] {
        let recorded = server.take_request();
        assert_eq!(recorded.method, method);
        assert_eq!(recorded.path, path);
        assert_eq!(recorded.header(SHIJU_CREDENTIAL_HEADER), Some("private-connector-key"));
        assert!(!recorded.path.contains(&flow));
        if path.ends_with("/poll") {
            assert_eq!(recorded.body["flow_id"], flow);
        }
        if method == "PUT" {
            assert_eq!(recorded.body["binding_config"]["token"], "private-binding-secret");
        }
    }
    assert!(!format!("{request:?} {challenge:?}").contains("private-binding-secret"));
    assert!(!format!("{challenge:?}").contains(&flow));
    server.join();
}
