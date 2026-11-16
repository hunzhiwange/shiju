// 验证路由层对认证、租户上下文及各类端点的装配。

// 同一路由会从处理器路由和完整应用两条装配路径验证，防止注册遗漏。
use super::*;

use axum::body::Body;
use axum::http::Request;
use axum::http::StatusCode;
use axum::http::header;
use tower::ServiceExt;
use {
    vw_api_transport::id::COMPANY_ID_HEADER,
    vw_api_transport::id::PLATFORM_ID_HEADER,
};

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
async fn build_router_registers_v1_desktop_service_route() {
    let response = build_router(Vec::new())
        .oneshot(tenant_request("/v1/desktop/service/status"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn build_router_registers_v1_shiju_app_routes() {
    for path in [
        "/v1/shiju/apps/catalog",
        "/v1/shiju/apps",
        "/v1/shiju/apps/01K1APP0000000000000000001/launch",
    ] {
        let request = Request::post(path)
            .header(PLATFORM_ID_HEADER, "platform-test")
            .header(COMPANY_ID_HEADER, "company-test")
            .body(Body::empty())
            .unwrap();
        let response = build_router(Vec::new()).oneshot(request).await.unwrap();
        if path.ends_with("/launch") {
            assert_ne!(response.status(), StatusCode::NOT_FOUND, "{path}");
        } else {
            assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED, "{path}");
        }
    }
}

#[tokio::test]
async fn handler_router_reports_method_not_allowed_for_registered_route() {
    let response = handler_router()
        .oneshot(Request::post("/global/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}
