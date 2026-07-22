use super::*;

use axum::body::Body;
use axum::http::Request;
use axum::http::StatusCode;
use axum::http::header;
use tower::ServiceExt;
use vw_api_types::id::{COMPANY_ID_HEADER, PLATFORM_ID_HEADER};

fn tenant_request(path: &str) -> Request<Body> {
    Request::get(path)
        .header(PLATFORM_ID_HEADER, "platform-test")
        .header(COMPANY_ID_HEADER, "company-test")
        .body(Body::empty())
        .unwrap()
}

#[tokio::test]
async fn handler_router_serves_unversioned_handler_routes() {
    let response = handler_router()
        .oneshot(Request::get("/global/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn build_router_serves_unversioned_handler_routes() {
    let response =
        build_router(Vec::new()).oneshot(tenant_request("/global/health")).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn build_router_serves_v1_handler_routes() {
    let response =
        build_router(Vec::new()).oneshot(tenant_request("/v1/global/health")).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn build_router_applies_cors_to_unversioned_handler_routes() {
    let origin = "https://ui.example.test";
    let request = Request::get("/global/health")
        .header(header::ORIGIN, origin)
        .header(PLATFORM_ID_HEADER, "platform-test")
        .header(COMPANY_ID_HEADER, "company-test")
        .body(Body::empty())
        .unwrap();

    let response = build_router(vec![origin.to_string()]).oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN).unwrap(), origin);
}

#[tokio::test]
async fn build_router_registers_v1_desktop_service_route() {
    let response = build_router(Vec::new())
        .oneshot(tenant_request("/v1/desktop/service/status"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn build_router_allows_whitelisted_cors_origin() {
    let origin = "https://ui.example.test";
    let request = Request::get("/v1/global/health")
        .header(header::ORIGIN, origin)
        .header(PLATFORM_ID_HEADER, "platform-test")
        .header(COMPANY_ID_HEADER, "company-test")
        .body(Body::empty())
        .unwrap();

    let response = build_router(vec![origin.to_string()]).oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN).unwrap(), origin);
}

#[tokio::test]
async fn build_router_allows_default_localhost_cors_origin() {
    let origin = "http://localhost:5173";
    let request = Request::get("/v1/global/health")
        .header(header::ORIGIN, origin)
        .header(PLATFORM_ID_HEADER, "platform-test")
        .header(COMPANY_ID_HEADER, "company-test")
        .body(Body::empty())
        .unwrap();

    let response = build_router(Vec::new()).oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN).unwrap(), origin);
}

#[tokio::test]
async fn build_router_answers_whitelisted_cors_preflight() {
    let origin = "https://ui.example.test";
    let request = Request::builder()
        .method("OPTIONS")
        .uri("/v1/global/health")
        .header(header::ORIGIN, origin)
        .header(header::ACCESS_CONTROL_REQUEST_METHOD, "PATCH")
        .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "x-test-header")
        .body(Body::empty())
        .unwrap();

    let response = build_router(vec![origin.to_string()]).oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN).unwrap(), origin);
    assert!(response.headers().contains_key(header::ACCESS_CONTROL_ALLOW_METHODS));
    assert!(response.headers().contains_key(header::ACCESS_CONTROL_ALLOW_HEADERS));
}

#[tokio::test]
async fn build_router_omits_cors_header_for_unlisted_origin() {
    let request = Request::get("/v1/global/health")
        .header(header::ORIGIN, "https://blocked.example.test")
        .header(PLATFORM_ID_HEADER, "platform-test")
        .header(COMPANY_ID_HEADER, "company-test")
        .body(Body::empty())
        .unwrap();

    let response = build_router(Vec::new()).oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert!(!response.headers().contains_key(header::ACCESS_CONTROL_ALLOW_ORIGIN));
}

#[tokio::test]
async fn build_router_adds_cors_header_to_tenant_rejection() {
    let origin = "https://ui.example.test";
    let request = Request::get("/v1/global/health")
        .header(header::ORIGIN, origin)
        .body(Body::empty())
        .unwrap();

    let response = build_router(vec![origin.to_string()]).oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(response.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN).unwrap(), origin);
}

#[tokio::test]
async fn build_router_returns_not_found_for_unknown_v1_route() {
    let response = build_router(Vec::new()).oneshot(tenant_request("/v1/not-found")).await.unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn build_router_does_not_mount_handlers_under_repeated_v1_prefix() {
    let response =
        build_router(Vec::new()).oneshot(tenant_request("/v1/v1/global/health")).await.unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn handler_router_reports_method_not_allowed_for_registered_route() {
    let response = handler_router()
        .oneshot(Request::post("/global/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn build_router_rejects_client_routes_without_tenant_headers() {
    let response = build_router(Vec::new())
        .oneshot(Request::get("/v1/global/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn build_router_rejects_unversioned_routes_without_tenant_headers() {
    let response = build_router(Vec::new())
        .oneshot(Request::get("/global/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
