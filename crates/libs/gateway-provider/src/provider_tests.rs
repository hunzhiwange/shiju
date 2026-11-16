// 覆盖提供商目录、认证方式与 OAuth 路由契约。

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

// OAuth 请求形状与路由委托分开验证，避免协议校验被后端未实现状态掩盖。
#[test]
fn router_builds_with_unit_state() {
    let _ = router::<()>();
}

// 拒绝路径除状态码外还检查状态保持不变，防止错误响应掩盖已发生的副作用。

#[tokio::test]
async fn provider_list_returns_defaults_and_sorted_connections() {
    let temp = tempfile::tempdir().expect("tempdir");

    let Json(response) = provider_list(
        Extension(TenantScope::default()),
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
