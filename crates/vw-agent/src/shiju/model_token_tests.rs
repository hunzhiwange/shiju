use super::super::account_tests::{fixture, fixture_with_status};
use super::*;

#[tokio::test]
async fn shiju_model_token_exchanges_login_credential_without_request_body() {
    let (base, requests) = fixture(vec![
        serde_json::json!({
            "success": true,
            "data": {"key": "issued-model-key", "endpoint_url": MODEL_ENDPOINT}
        })
        .to_string(),
    ]);
    let client = ShijuAccountClient::with_base_urls(&base, &format!("{base}/sj/")).unwrap();
    let token = client.model_token("passport-key").await.unwrap();
    assert_eq!(token.key.expose(), "issued-model-key");
    assert_eq!(token.endpoint_url, MODEL_ENDPOINT);
    assert!(!format!("{token:?}").contains("issued-model-key"));
    let requests = requests.join().unwrap();
    assert!(requests[0].starts_with("PUT /sj/model/api/runtime/model-token HTTP/1.1\r\n"));
    assert!(requests[0].to_ascii_lowercase().contains("x-api-key: passport-key\r\n"));
    assert!(!requests[0].to_ascii_lowercase().contains("authorization:"));
    assert!(requests[0].ends_with("\r\n\r\n"));
}

#[tokio::test]
async fn shiju_model_token_defaults_missing_endpoint_and_rejects_untrusted_responses() {
    for (data, succeeds) in [
        (serde_json::json!({"key": "model-key"}), true),
        (
            serde_json::json!({"key": "model-key", "endpoint_url": "https://router.shiju.cloud/"}),
            true,
        ),
        (serde_json::json!({"key": ""}), false),
        (serde_json::json!({"key": "bad\nkey"}), false),
        (serde_json::json!({"key": "model-key", "endpoint_url": "https://evil.example/v1"}), false),
        (
            serde_json::json!({"key": "model-key", "endpoint_url": "http://router.shiju.cloud/v1"}),
            false,
        ),
    ] {
        let (base, requests) =
            fixture(vec![serde_json::json!({"success": true, "data": data}).to_string()]);
        let client = ShijuAccountClient::with_base_urls(&base, &base).unwrap();
        assert_eq!(client.model_token("passport-key").await.is_ok(), succeeds);
        requests.join().unwrap();
    }
}

#[tokio::test]
async fn shiju_model_token_preserves_auth_status_and_redacts_upstream_error_body() {
    for status in ["401 Unauthorized", "403 Forbidden", "302 Found", "500 Internal Server Error"] {
        let (base, requests) = fixture_with_status(vec![(
            status.to_string(),
            "passport-key issued-model-key sensitive upstream payload".to_string(),
        )]);
        let client = ShijuAccountClient::with_base_urls(&base, &base).unwrap();
        let error = client.model_token("passport-key").await.unwrap_err();
        assert_eq!(error.status().unwrap().as_u16(), status[..3].parse::<u16>().unwrap());
        assert!(!error.to_string().contains("passport-key"));
        assert!(!error.to_string().contains("issued-model-key"));
        assert!(!error.to_string().contains("payload"));
        requests.join().unwrap();
    }
}
