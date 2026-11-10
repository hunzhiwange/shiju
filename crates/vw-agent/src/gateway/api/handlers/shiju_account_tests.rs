// 覆盖识炬账号路由的敏感输入校验与上游错误映射。

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

// 敏感字段在访问上游前完成校验，错误响应只保留允许公开的状态和消息。
#[tokio::test]
async fn shiju_account_routes_require_valid_sensitive_inputs() {
    let (status, body) =
        response(Request::put("/shiju/models/token").body(Body::empty()).unwrap()).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "SHIJU credential header is required");
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

#[tokio::test]
async fn connector_routes_require_credential_and_reject_identity_override() {
    let (status, _) =
        response(Request::get("/shiju/connectors").body(Body::empty()).unwrap()).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = response(
        Request::post("/shiju/connectors/authorizations/poll")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"flow_id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}"#))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    for (path, payload) in [
        (
            "/shiju/connectors/binding",
            r#"{"company_platform_id":7,"third_party_account_id":"account","third_party_account_name":"name","binding_config":{},"user_id":"other"}"#,
        ),
        ("/shiju/connectors/authorizations", r#"{"company_platform_id":7,"user_id":"other"}"#),
    ] {
        let method = if path.ends_with("binding") { "PUT" } else { "POST" };
        let (status, _) = response(
            Request::builder()
                .method(method)
                .uri(path)
                .header("content-type", "application/json")
                .header(SHIJU_CREDENTIAL_HEADER, "credential")
                .body(Body::from(payload))
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    }
}

#[tokio::test]
async fn shiju_monthly_quota_uses_employee_key_and_converts_upstream_budget() {
    use axum::routing::get;
    use axum::{Json, Router, serve};
    use serde_json::json;

    let upstream = Router::new().route(
        "/sj/model/api/runtime/model-usage",
        get(|headers: axum::http::HeaderMap, uri: axum::http::Uri| async move {
            assert_eq!(uri.query(), Some("include_monthly_quota=true"));
            let key = headers.get("x-api-key").unwrap().to_str().unwrap();
            let (limit, used, unit) = match key {
                "fixture-limited" => (50_000_000, 49_997_050, 500_000),
                "fixture-exhausted" => (50_000_000, 50_001_000, 500_000),
                "fixture-unlimited" => (0, 25_000_000, 500_000),
                "fixture-invalid" => (50_000_000, 0, 0),
                _ => panic!("unexpected fixture credential"),
            };
            Json(json!({
                "success": true,
                "data": {
                    "monthly_quota": limit,
                    "monthly_used_quota": used,
                    "monthly_reset_at": 1796054400,
                    "quota_per_unit": unit,
                    "unrelated": key,
                }
            }))
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let upstream_task = tokio::spawn(async move { serve(listener, upstream).await.unwrap() });
    let client =
        crate::shiju::account::ShijuAccountClient::with_base_urls(&base, &format!("{base}/sj/"))
            .unwrap();
    let gateway = super::router_with_client::<()>(client);
    for (credential, status, limit, remaining) in [
        (None, StatusCode::BAD_REQUEST, 0.0, None),
        (Some("fixture-limited"), StatusCode::OK, 100.0, Some(0.0059)),
        (Some("fixture-exhausted"), StatusCode::OK, 100.0, Some(0.0)),
        (Some("fixture-unlimited"), StatusCode::OK, 0.0, None),
        (Some("fixture-invalid"), StatusCode::INTERNAL_SERVER_ERROR, 0.0, None),
    ] {
        let mut request = Request::get("/shiju/account/monthly-quota");
        if let Some(credential) = credential {
            request = request.header(SHIJU_CREDENTIAL_HEADER, credential);
        }
        let response = gateway.clone().oneshot(request.body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(response.status(), status);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert!(!String::from_utf8_lossy(&body).contains("fixture-"));
        if status == StatusCode::OK {
            let quota: vw_gateway_client::ShijuMonthlyQuotaDto =
                serde_json::from_slice(&body).unwrap();
            assert_eq!(quota.limit, limit);
            assert_eq!(quota.remaining, remaining);
            assert_eq!(quota.reset_at, 1796054400);
        }
    }
    upstream_task.abort();
}
