//! Request-local tenant propagation through the production middleware.
use crate::native_ui::session_tenant_scope;
use axum::{Router, routing::get, middleware::from_fn};
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use tower::ServiceExt;
use vw_api_transport::TenantScope;
async fn tenant_scope_probe() -> String {
    let tenant = session_store::ui_store::current_tenant_scope();
    format!("{}/{}", tenant.platform_id().as_ref(), tenant.company_id().as_ref())
}

fn tenant_scope_probe_app() -> Router {
    Router::new().route("/scope", get(tenant_scope_probe)).layer(from_fn(session_tenant_scope))
}

async fn scope_response(tenant: Option<TenantScope>) -> axum::response::Response {
    let mut request = Request::builder().uri("/scope").body(Body::empty()).expect("request");
    if let Some(tenant) = tenant {
        request.extensions_mut().insert(tenant);
    }
    tenant_scope_probe_app().oneshot(request).await.expect("scope response")
}

#[tokio::test]
async fn session_tenant_scope_forwards_each_request_scope() {
    let first = TenantScope::new("platform-a", "company-a").expect("first tenant");
    let second = TenantScope::new("platform-b", "company-b").expect("second tenant");

    let (first_response, second_response) =
        tokio::join!(scope_response(Some(first)), scope_response(Some(second)));
    assert_eq!(first_response.status(), StatusCode::OK);
    assert_eq!(second_response.status(), StatusCode::OK);

    let first_body = to_bytes(first_response.into_body(), usize::MAX).await.expect("first body");
    let second_body = to_bytes(second_response.into_body(), usize::MAX).await.expect("second body");
    assert_eq!(first_body.as_ref(), b"platform-a/company-a");
    assert_eq!(second_body.as_ref(), b"platform-b/company-b");
}

#[tokio::test]
async fn session_tenant_scope_rejects_requests_without_extension() {
    let response = scope_response(None).await;

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}
