//! 用固定 HTTP 响应验证数据网关路径、个人凭证与敏感字段过滤。
use super::super::account_tests::fixture;
use super::*;
use serde_json::json;
use vw_gateway_client::ShijuConnectorAuthorizationStatus;
fn envelope(data: serde_json::Value) -> String {
    json!({"code": 200, "data": data}).to_string()
}
fn connector() -> serde_json::Value {
    json!({"platform_id": 1, "platform_code": "sample", "platform_name": "示例连接器", "company_platform_id": 7, "authorization_available": true,
        "account_binding_variables": [{"name": "token", "type": "password", "required": true, "description": "密钥"}],
        "binding": {"third_party_account_id": "user", "third_party_account_name": "用户", "binding_config": {"token": "must-not-leak"}, "status": "enabled"}})
}
#[tokio::test]
async fn connector_personal_routes_authenticate_and_redact_passwords() {
    let flow = "a".repeat(32);
    let (base, requests) = fixture(vec![
        envelope(json!([connector()])),
        envelope(json!([connector()])),
        envelope(
            json!({"flow_id": flow, "authorization_url": "https://provider.example/authorize", "poll_interval_seconds": 2, "expires_in_seconds": 90}),
        ),
        envelope(json!({"status": "approved", "binding_config": {"token": "must-not-leak"}})),
    ]);
    let client = ShijuAccountClient::with_base_urls(&base, &format!("{base}/sj/")).unwrap();
    let connectors = client.connectors("personal-credential").await.unwrap();
    assert!(connectors[0].binding.as_ref().unwrap().binding_config.is_empty());
    let challenge = client
        .authorize_connector(
            "personal-credential",
            &ShijuConnectorAuthorizeRequest { company_platform_id: 7 },
        )
        .await
        .unwrap();
    let poll = client.poll_connector("personal-credential", &challenge.flow_id).await.unwrap();
    assert_eq!(poll.status, ShijuConnectorAuthorizationStatus::Approved);
    assert!(!format!("{connectors:?} {challenge:?} {poll:?}").contains("must-not-leak"));
    let requests = requests.join().unwrap();
    assert!(requests[0].starts_with("GET /sj/data/api/v1/me/account-binding-connectors "));
    assert!(requests[2].starts_with("POST /sj/data/api/v1/me/account-binding-authorizations "));
    assert!(
        requests[3]
            .starts_with(&format!("GET /sj/data/api/v1/me/account-binding-authorizations/{flow} "))
    );
    for request in requests {
        assert!(request.to_lowercase().contains("x-api-key: personal-credential"));
        assert!(!request.to_lowercase().contains("cookie:"));
    }
}
#[tokio::test]
async fn connector_save_uses_current_user_and_drops_response_secrets() {
    let binding = connector()["binding"].clone();
    let (base, requests) = fixture(vec![envelope(json!([connector()])), envelope(binding)]);
    let client = ShijuAccountClient::with_base_urls(&base, &base).unwrap();
    let request = ShijuConnectorSaveRequest {
        company_platform_id: 7,
        third_party_account_id: "user".into(),
        third_party_account_name: "用户".into(),
        binding_config: serde_json::from_value(json!({"token": "new-secret"})).unwrap(),
    };
    let saved = client.save_connector_binding("personal-credential", &request).await.unwrap();
    assert!(saved.binding_config.is_empty());
    assert!(!format!("{request:?}").contains("new-secret"));
    let requests = requests.join().unwrap();
    assert!(requests[1].starts_with("PUT /data/api/v1/me/account-bindings "));
    assert!(requests[1].contains("new-secret"));
    assert!(!requests[1].contains("user_id"));
}
#[tokio::test]
async fn connector_rejects_invalid_flow_before_network() {
    let client =
        ShijuAccountClient::with_base_urls("http://127.0.0.1:1", "http://127.0.0.1:1").unwrap();
    for flow in ["../other", "", "https://untrusted.example/"] {
        assert!(
            client
                .poll_connector("credential", flow)
                .await
                .unwrap_err()
                .to_string()
                .contains("flow ID")
        );
    }
}
#[tokio::test]
async fn connector_rejects_unsafe_authorization_url() {
    let (base, requests) = fixture(vec![
        envelope(json!([connector()])),
        envelope(
            json!({"flow_id": "a".repeat(32), "authorization_url": "file:///tmp/payload", "poll_interval_seconds": 1, "expires_in_seconds": 60}),
        ),
    ]);
    let client = ShijuAccountClient::with_base_urls(&base, &base).unwrap();
    assert!(
        client
            .authorize_connector(
                "credential",
                &ShijuConnectorAuthorizeRequest { company_platform_id: 7 }
            )
            .await
            .is_err()
    );
    requests.join().unwrap();
}
#[tokio::test]
async fn connector_rejects_missing_required_variable_before_save() {
    let (base, requests) = fixture(vec![envelope(json!([connector()]))]);
    let client = ShijuAccountClient::with_base_urls(&base, &base).unwrap();
    let request = ShijuConnectorSaveRequest {
        company_platform_id: 7,
        third_party_account_id: "user".into(),
        third_party_account_name: String::new(),
        binding_config: Default::default(),
    };
    assert!(client.save_connector_binding("credential", &request).await.is_err());
    assert_eq!(requests.join().unwrap().len(), 1);
}
#[tokio::test]
async fn connector_upstream_errors_do_not_echo_secrets() {
    let (base, requests) =
        fixture(vec![json!({"code": 400, "message": "upstream-secret", "data": null}).to_string()]);
    let client = ShijuAccountClient::with_base_urls(&base, &base).unwrap();
    let error = client.connectors("credential").await.unwrap_err();
    assert!(!error.to_string().contains("upstream-secret"));
    requests.join().unwrap();
}
