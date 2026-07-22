use super::*;
use axum::body::{Body, to_bytes};
use axum::extract::{Path, Query};
use axum::http::{HeaderMap, Method, Request, StatusCode};
use tower::ServiceExt;

fn json_request(method: Method, uri: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(axum::http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("request should build")
}

async fn response_json(response: axum::response::Response) -> serde_json::Value {
    let body = to_bytes(response.into_body(), usize::MAX).await.expect("body should be readable");
    serde_json::from_slice(&body).expect("response should be json")
}

#[test]
fn router_builds_with_unit_state() {
    let _ = router::<()>();
}

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
async fn provider_list_returns_defaults_and_sorted_connections() {
    let temp = tempfile::tempdir().expect("tempdir");

    let Json(response) = provider_list(
        Query(InstanceQuery { directory: Some(temp.path().to_string_lossy().to_string()) }),
        HeaderMap::new(),
    )
    .await
    .expect("providers should list");

    let mut expected_connected = response
        .all
        .iter()
        .filter(|provider| provider.key.as_deref().is_some_and(|key| !key.trim().is_empty()))
        .map(|provider| provider.id.clone())
        .collect::<Vec<_>>();
    expected_connected.sort();

    let expected_defaults = response
        .all
        .iter()
        .filter_map(|provider_info| {
            let models = provider::provider::sort(
                provider_info.models.values().cloned().collect::<Vec<_>>(),
            );
            models.first().map(|model| (provider_info.id.clone(), model.id.clone()))
        })
        .collect::<HashMap<_, _>>();

    assert_eq!(response.connected, expected_connected);
    assert_eq!(response.default, expected_defaults);
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

#[tokio::test]
async fn router_auth_methods_returns_empty_json_object() {
    let response = router::<()>()
        .oneshot(
            Request::builder()
                .uri("/provider/auth")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("route should respond");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response_json(response).await, serde_json::json!({}));
}

#[tokio::test]
async fn router_oauth_authorize_returns_null_authorization() {
    let response = router::<()>()
        .oneshot(json_request(
            Method::POST,
            "/provider/custom/oauth/authorize",
            serde_json::json!({"method": 7}),
        ))
        .await
        .expect("route should respond");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response_json(response).await, serde_json::Value::Null);
}

#[tokio::test]
async fn router_oauth_callback_accepts_absent_code_and_maps_provider_error() {
    let response = router::<()>()
        .oneshot(json_request(
            Method::POST,
            "/provider/custom/oauth/callback",
            serde_json::json!({"method": 1}),
        ))
        .await
        .expect("route should respond");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn router_oauth_routes_reject_invalid_json_shapes() {
    let app = router::<()>();
    let missing_method = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/provider/custom/oauth/authorize",
            serde_json::json!({}),
        ))
        .await
        .expect("route should respond");
    let non_numeric_method = app
        .oneshot(json_request(
            Method::POST,
            "/provider/custom/oauth/callback",
            serde_json::json!({"method": "first", "code": null}),
        ))
        .await
        .expect("route should respond");

    assert_eq!(missing_method.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(non_numeric_method.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn router_rejects_wrong_method_and_unknown_oauth_action() {
    let app = router::<()>();
    let wrong_method = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/provider/custom/oauth/authorize")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("route should respond");
    let unknown_action = app
        .oneshot(json_request(
            Method::POST,
            "/provider/custom/oauth/unknown",
            serde_json::json!({"method": 0}),
        ))
        .await
        .expect("route should respond");

    assert_eq!(wrong_method.status(), StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(unknown_action.status(), StatusCode::NOT_FOUND);
}
