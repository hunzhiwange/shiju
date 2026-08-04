use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;
use vw_gateway_client::SHIJU_CREDENTIAL_HEADER;

async fn response(request: Request<Body>) -> (StatusCode, Value) {
    let response = super::router_with_client::<()>(
        crate::shiju::account::ShijuAccountClient::with_base_urls(
            "http://127.0.0.1:9",
            "http://127.0.0.1:9",
        )
        .expect("client"),
    )
    .oneshot(request)
    .await
    .expect("response");
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.expect("body");
    let value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    (status, value)
}

#[tokio::test]
async fn shiju_account_routes_require_valid_sensitive_inputs() {
    let (status, body) = response(
        Request::post("/shiju/account/login/poll")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"state":"   "}"#))
            .expect("poll request"),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "SHIJU login state cannot be empty");

    let (status, body) = response(
        Request::get("/shiju/account/profile").body(Body::empty()).expect("profile request"),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "SHIJU credential header is required");

    let secret = "fixture-handler-secret";
    let (status, body) = response(
        Request::get("/shiju/account/profile")
            .header(SHIJU_CREDENTIAL_HEADER, "   ")
            .body(Body::empty())
            .expect("profile request"),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(!body.to_string().contains(secret));

    let (status, body) = response(
        Request::get("/shiju/account/me").body(Body::empty()).expect("current user request"),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "SHIJU credential header is required");

    for payload in
        [r#"{"name":"   ","username":"liuxiangmin"}"#, r#"{"name":"刘祥敏","username":"   "}"#]
    {
        let (status, _) = response(
            Request::put("/shiju/account/me")
                .header("content-type", "application/json")
                .header(SHIJU_CREDENTIAL_HEADER, "fixture-handler-secret")
                .body(Body::from(payload))
                .expect("current user update request"),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    let (status, _) = response(
        Request::put("/shiju/account/me")
            .header("content-type", "application/json")
            .header(SHIJU_CREDENTIAL_HEADER, "fixture-handler-secret")
            .body(Body::from(r#"{"name":"刘祥敏","username":"liuxiangmin","cookie":"forbidden"}"#))
            .expect("current user update request"),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn shiju_account_password_route_rejects_invalid_sensitive_inputs() {
    let (status, body) = response(
        Request::put("/shiju/account/me/password")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"old_password":"old-password-123","new_password":"new-password-456"}"#,
            ))
            .expect("password request"),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "SHIJU credential header is required");

    for payload in [
        r#"{"old_password":"   ","new_password":"new-password-456"}"#,
        r#"{"old_password":"old-password-123","new_password":"short1"}"#,
        r#"{"old_password":"old-password-123","new_password":"onlyletters"}"#,
        r#"{"old_password":"old-password-123","new_password":"12345678"}"#,
    ] {
        let (status, body) = response(
            Request::put("/shiju/account/me/password")
                .header("content-type", "application/json")
                .header(SHIJU_CREDENTIAL_HEADER, "fixture-handler-secret")
                .body(Body::from(payload))
                .expect("password request"),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(!body.to_string().contains("old-password-123"));
    }

    let (status, _) = response(
        Request::put("/shiju/account/me/password")
            .header("content-type", "application/json")
            .header(SHIJU_CREDENTIAL_HEADER, "fixture-handler-secret")
            .body(Body::from(
                r#"{"old_password":"old-password-123","new_password":"new-password-456","cookie":"forbidden"}"#,
            ))
            .expect("password request"),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[test]
fn shiju_account_upstream_errors_preserve_status_and_message() {
    for (status, message) in [
        (StatusCode::UNAUTHORIZED, "用户名或密码错误，请重新输入"),
        (StatusCode::SERVICE_UNAVAILABLE, "上游服务暂时不可用"),
    ] {
        let error = super::upstream_error(super::ShijuAccountError::Upstream {
            status,
            message: message.to_string(),
        });

        assert_eq!(error.status, status);
        assert_eq!(error.message, message);
    }
}
