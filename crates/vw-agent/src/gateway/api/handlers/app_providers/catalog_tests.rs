use axum::{
    Extension,
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;
use vw_api_transport::id::TenantScope;
#[tokio::test]
async fn cc04_catalog_api_lists_only_authenticated_tenant_and_application() {
    let root = tempfile::tempdir().unwrap();
    let database = crate::global_database::GlobalDatabase::for_home(root.path()).unwrap();
    let app = crate::gateway::router::handler_router()
        .layer(Extension(database))
        .layer(Extension(TenantScope::new("p", "a").unwrap()))
        .layer(axum::middleware::from_fn(|req, next| async {
            crate::gateway::middleware::basic_auth_middleware_with_credentials(
                req,
                next,
                Some("secret"),
                "test",
            )
            .await
        }));
    let response = app
        .clone()
        .oneshot(Request::builder().uri("/app-providers/claude").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/app-providers/claude")
                .header("authorization", "Basic dGVzdDpzZWNyZXQ=")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
