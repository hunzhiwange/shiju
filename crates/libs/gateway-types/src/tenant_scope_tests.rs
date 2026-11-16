// 租户头解析与注入属于共享请求边界，不依赖宿主路由。
use super::*;
use axum::{Extension, Router, body::Body, http::{HeaderValue, StatusCode}, middleware, routing::get};
use tower::util::ServiceExt;

// 租户路由回显扩展中的作用域，用于证明请求头已完成解析和注入。
fn tenant_test_router() -> Router {
    Router::new()
        .route(
            "/scope",
            get(|Extension(scope): Extension<TenantScope>| async move {
                format!("{}/{}", scope.platform_id().as_ref(), scope.company_id().as_ref())
            }),
        )
        .layer(middleware::from_fn(tenant_scope_middleware))
}

#[tokio::test]
async fn tenant_scope_middleware_injects_single_non_empty_utf8_headers() {
    let request = Request::get("/scope")
        .header(PLATFORM_ID_HEADER, "platform-test")
        .header(COMPANY_ID_HEADER, "company-test")
        .body(Body::empty())
        .expect("request should build");

    let response = tenant_test_router().oneshot(request).await.expect("router should respond");
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should read");

    assert_eq!(body.as_ref(), b"platform-test/company-test");
}

#[tokio::test]
async fn tenant_scope_middleware_preserves_non_empty_header_whitespace() {
    let request = Request::get("/scope")
        .header(PLATFORM_ID_HEADER, " platform-test ")
        .header(COMPANY_ID_HEADER, " company-test ")
        .body(Body::empty())
        .expect("request should build");

    let response = tenant_test_router().oneshot(request).await.expect("router should respond");
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should read");

    assert_eq!(body.as_ref(), b" platform-test / company-test ");
}

#[tokio::test]
async fn tenant_scope_middleware_rejects_duplicate_and_non_utf8_headers() {
    let duplicate_request = Request::get("/scope")
        .header(PLATFORM_ID_HEADER, "platform-test")
        .header(PLATFORM_ID_HEADER, "platform-duplicate")
        .header(COMPANY_ID_HEADER, "company-test")
        .body(Body::empty())
        .expect("request should build");
    let duplicate_response =
        tenant_test_router().oneshot(duplicate_request).await.expect("router should respond");
    assert_eq!(duplicate_response.status(), StatusCode::BAD_REQUEST);

    let non_utf8_request = Request::get("/scope")
        .header(
            PLATFORM_ID_HEADER,
            HeaderValue::from_bytes(b"\xff").expect("opaque header should build"),
        )
        .header(COMPANY_ID_HEADER, "company-test")
        .body(Body::empty())
        .expect("request should build");
    let non_utf8_response =
        tenant_test_router().oneshot(non_utf8_request).await.expect("router should respond");
    assert_eq!(non_utf8_response.status(), StatusCode::BAD_REQUEST);
}

#[test]
fn required_tenant_header_reports_specific_invalid_value_errors() {
    let missing = HeaderMap::new();

    let mut duplicate = HeaderMap::new();
    duplicate.append(PLATFORM_ID_HEADER, HeaderValue::from_static("platform-test"));
    duplicate.append(PLATFORM_ID_HEADER, HeaderValue::from_static("platform-duplicate"));

    let mut non_utf8 = HeaderMap::new();
    non_utf8.insert(
        COMPANY_ID_HEADER,
        HeaderValue::from_bytes(b"\xff").expect("opaque header bytes are accepted"),
    );

    let mut blank = HeaderMap::new();
    blank.insert(PLATFORM_ID_HEADER, HeaderValue::from_static(" \t "));

    for (headers, name, expected_message) in [
        (&missing, COMPANY_ID_HEADER, "missing X-Company-ID header"),
        (&duplicate, PLATFORM_ID_HEADER, "X-Platform-ID header must have a single value"),
        (&non_utf8, COMPANY_ID_HEADER, "X-Company-ID header must be valid UTF-8"),
        (&blank, PLATFORM_ID_HEADER, "X-Platform-ID header must not be empty"),
    ] {
        let error = required_tenant_header(headers, name).expect_err("header should be rejected");

        assert_eq!(error.status, StatusCode::BAD_REQUEST);
        assert_eq!(error.message, expected_message);
    }
}

#[tokio::test]
async fn tenant_scope_middleware_bypasses_headers_before_method_rejection() {
    let request = Request::builder()
        .method(Method::OPTIONS)
        .uri("/scope")
        .body(Body::empty())
        .expect("request should build");

    let response = tenant_test_router().oneshot(request).await.expect("router should respond");

    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}
