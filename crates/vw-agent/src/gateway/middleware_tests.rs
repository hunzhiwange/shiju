// 覆盖网关中间件的跨域、基础认证与租户请求头校验。
use super::*;

use axum::Extension;
use axum::Router;
use axum::body::Body;
use axum::http::StatusCode;
use axum::http::header;
use axum::middleware;
use axum::routing::get;
use base64::Engine;
use tower::util::ServiceExt;
use vw_api_types::id::{COMPANY_ID_HEADER, PLATFORM_ID_HEADER, TenantScope};

fn auth_value(username: &str, password: &str) -> String {
    let encoded =
        base64::engine::general_purpose::STANDARD.encode(format!("{username}:{password}"));
    format!("Basic {encoded}")
}

fn auth_test_router(password: Option<&'static str>, username: &'static str) -> Router {
    Router::new()
        .route(
            "/protected",
            get(|| async { StatusCode::NO_CONTENT }).post(|| async { StatusCode::CREATED }),
        )
        .layer(middleware::from_fn(move |req, next| {
            basic_auth_middleware_with_credentials(req, next, password, username)
        }))
}

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

#[test]
fn cors_origin_allowed_accepts_default_local_and_tauri_origins() {
    let whitelist = Vec::new();

    for origin in [
        "http://localhost:3000",
        "http://127.0.0.1:5173",
        "tauri://localhost",
        "http://tauri.localhost",
        "https://tauri.localhost",
    ] {
        let header = HeaderValue::from_str(origin).expect("origin should be a valid header");
        assert!(cors_origin_allowed(&header, &whitelist), "{origin} should be allowed");
    }
}

#[test]
fn cors_origin_allowed_accepts_exact_whitelist_match() {
    let whitelist = vec!["https://app.example.com".to_string()];
    let allowed = HeaderValue::from_static("https://app.example.com");
    let rejected = HeaderValue::from_static("https://api.example.com");

    assert!(cors_origin_allowed(&allowed, &whitelist));
    assert!(!cors_origin_allowed(&rejected, &whitelist));
}

#[test]
fn cors_origin_allowed_keeps_whitelist_matching_case_and_path_sensitive() {
    let whitelist = vec!["https://app.example.com/console".to_string()];

    for origin in ["https://APP.example.com/console", "https://app.example.com/console/"] {
        let header = HeaderValue::from_str(origin).expect("origin should be a valid header");
        assert!(!cors_origin_allowed(&header, &whitelist), "{origin} should be rejected");
    }
}

#[test]
fn cors_origin_allowed_rejects_unsupported_and_invalid_origins() {
    let whitelist = Vec::new();
    let unsupported = HeaderValue::from_static("https://localhost:3000");
    let invalid = HeaderValue::from_bytes(b"\xff").expect("opaque header bytes are accepted");

    assert!(!cors_origin_allowed(&unsupported, &whitelist));
    assert!(!cors_origin_allowed(&invalid, &whitelist));
}

#[test]
fn cors_origin_allowed_rejects_default_origin_near_matches() {
    let whitelist = Vec::new();

    for origin in [
        "http://localhost",
        "http://127.0.0.1",
        "tauri://localhost/",
        "http://tauri.localhost:80",
        "https://tauri.localhost/path",
        "TAURI://localhost",
    ] {
        let header = HeaderValue::from_str(origin).expect("origin should be a valid header");
        assert!(!cors_origin_allowed(&header, &whitelist), "{origin} should be rejected");
    }
}

#[tokio::test]
async fn basic_auth_allows_options_without_credentials() {
    let app = auth_test_router(Some("secret"), "alice");
    let request = Request::builder()
        .method(Method::OPTIONS)
        .uri("/protected")
        .body(Body::empty())
        .expect("request should build");

    let response = app.oneshot(request).await.expect("router should respond");

    assert_ne!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn basic_auth_allows_requests_when_password_is_not_configured() {
    let app = auth_test_router(None, "alice");
    let request = Request::builder()
        .method(Method::POST)
        .uri("/protected")
        .body(Body::empty())
        .expect("request should build");

    let response = app.oneshot(request).await.expect("router should respond");

    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn basic_auth_allows_matching_credentials() {
    let app = auth_test_router(Some("secret"), "alice");
    let request = Request::builder()
        .uri("/protected")
        .header(header::AUTHORIZATION, auth_value("alice", "secret"))
        .body(Body::empty())
        .expect("request should build");

    let response = app.oneshot(request).await.expect("router should respond");

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn basic_auth_uses_default_username_when_flag_username_is_absent() {
    let app = auth_test_router(Some("secret"), "vibewindow");
    let request = Request::builder()
        .uri("/protected")
        .header(header::AUTHORIZATION, auth_value("vibewindow", "secret"))
        .body(Body::empty())
        .expect("request should build");

    let response = app.oneshot(request).await.expect("router should respond");

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn basic_auth_rejects_missing_malformed_and_mismatched_credentials() {
    let cases = vec![
        None,
        Some("Bearer token".to_string()),
        Some(auth_value("alice", "secret").replacen("Basic ", "basic ", 1)),
        Some("Basic not-base64".to_string()),
        Some("Basic 6A==".to_string()),
        Some(auth_value("alice", "wrong")),
        Some(auth_value("bob", "secret")),
    ];

    for authorization in cases {
        let app = auth_test_router(Some("secret"), "alice");
        let mut request = Request::builder().uri("/protected");
        if let Some(value) = authorization.as_deref() {
            request = request.header(header::AUTHORIZATION, value);
        }
        let response = app
            .oneshot(request.body(Body::empty()).expect("request should build"))
            .await
            .expect("router should respond");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            response.headers().get(header::WWW_AUTHENTICATE),
            Some(&HeaderValue::from_static("Basic realm=\"vibewindow\""))
        );
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body should read");
        assert_eq!(body.as_ref(), br#"{"error":"unauthorized"}"#);
    }
}

#[tokio::test]
async fn basic_auth_rejects_non_utf8_authorization_header() {
    let app = auth_test_router(Some("secret"), "alice");
    let request = Request::builder()
        .uri("/protected")
        .header(
            header::AUTHORIZATION,
            HeaderValue::from_bytes(b"\xff").expect("opaque header bytes are accepted"),
        )
        .body(Body::empty())
        .expect("request should build");

    let response = app.oneshot(request).await.expect("router should respond");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        response.headers().get(header::WWW_AUTHENTICATE),
        Some(&HeaderValue::from_static("Basic realm=\"vibewindow\""))
    );
}

#[tokio::test]
async fn basic_auth_production_entry_allows_when_password_flag_is_absent() {
    let app = Router::new()
        .route("/protected", get(|| async { StatusCode::NO_CONTENT }))
        .layer(middleware::from_fn(basic_auth_middleware));
    let request =
        Request::builder().uri("/protected").body(Body::empty()).expect("request should build");

    let response = app.oneshot(request).await.expect("router should respond");

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
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
async fn tenant_scope_middleware_rejects_invalid_header_matrix() {
    let cases = [
        ("missing platform", None, Some("company-test")),
        ("missing company", Some("platform-test"), None),
        ("blank platform", Some("   "), Some("company-test")),
        ("blank company", Some("platform-test"), Some("\t")),
    ];

    for (name, platform, company) in cases {
        let mut request = Request::get("/scope");
        if let Some(platform) = platform {
            request = request.header(PLATFORM_ID_HEADER, platform);
        }
        if let Some(company) = company {
            request = request.header(COMPANY_ID_HEADER, company);
        }

        let response = tenant_test_router()
            .oneshot(request.body(Body::empty()).expect("request should build"))
            .await
            .expect("router should respond");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{name}");
    }
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
