use super::*;
use axum::Json;
use axum::extract::Path;
#[test]
fn oauth_request_bodies_deserialize() {
    let authorize: OAuthAuthorizeRequest =
        serde_json::from_value(serde_json::json!({"method": 2})).expect("authorize body");
    let callback: OAuthCallbackRequest = serde_json::from_value(serde_json::json!({
        "method": 2,
        "code": "abc"
    }))
    .expect("callback body");

    assert_eq!(authorize.method, 2);
    assert_eq!(callback.code.as_deref(), Some("abc"));
}

#[test]
fn oauth_callback_request_allows_missing_or_null_code() {
    let missing: OAuthCallbackRequest =
        serde_json::from_value(serde_json::json!({"method": 3})).expect("missing code is optional");
    let null: OAuthCallbackRequest = serde_json::from_value(serde_json::json!({
        "method": 4,
        "code": null
    }))
    .expect("null code is optional");

    assert_eq!(missing.method, 3);
    assert!(missing.code.is_none());
    assert_eq!(null.method, 4);
    assert!(null.code.is_none());
}

#[test]
fn oauth_requests_reject_missing_or_non_numeric_method() {
    assert!(serde_json::from_value::<OAuthAuthorizeRequest>(serde_json::json!({})).is_err());
    assert!(
        serde_json::from_value::<OAuthCallbackRequest>(serde_json::json!({
            "method": "first",
            "code": "abc"
        }))
        .is_err()
    );
}

#[test]
fn provider_list_response_serializes_public_contract() {
    let response = ProviderListResponse {
        all: Vec::new(),
        default: HashMap::from([("openai".to_string(), "gpt-test".to_string())]),
        connected: vec!["openai".to_string()],
    };

    assert_eq!(
        serde_json::to_value(response).expect("provider response should serialize"),
        serde_json::json!({
            "all": [],
            "default": {"openai": "gpt-test"},
            "connected": ["openai"]
        })
    );
}

#[tokio::test]
async fn provider_auth_methods_delegates_to_auth_module() {
    let Json(methods) = provider_auth_methods().await;

    assert!(methods.is_empty());
}

#[tokio::test]
async fn oauth_authorize_maps_empty_authorization() {
    let Json(result) = provider_oauth_authorize(
        Path("openai".to_string()),
        Json(OAuthAuthorizeRequest { method: 0 }),
    )
    .await
    .expect("authorize should succeed");

    assert!(result.is_none());
}

#[tokio::test]
async fn oauth_authorize_accepts_maximum_method_index_as_empty_authorization() {
    let Json(result) = provider_oauth_authorize(
        Path("boundary-provider".to_string()),
        Json(OAuthAuthorizeRequest { method: usize::MAX }),
    )
    .await
    .expect("placeholder authorization should accept any method index");

    assert!(result.is_none());
}

#[tokio::test]
async fn oauth_callback_maps_unsupported_to_bad_request() {
    let error = provider_oauth_callback(
        Path("openai".to_string()),
        Json(OAuthCallbackRequest { method: 0, code: Some("code".to_string()) }),
    )
    .await
    .expect_err("unsupported callback should be rejected");

    assert_eq!(error.status, axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(error.message, "unsupported auth method");
}
