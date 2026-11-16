use axum::{Extension, middleware};
use vw_api_transport::id::TenantScope;
#[tokio::test]
async fn cc04_claude_registered_route_requires_gateway_authentication() {
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;
    let app = crate::gateway::router::handler_router()
        .layer(Extension(TenantScope::new("p", "a").unwrap()))
        .layer(middleware::from_fn(|req, next| async {
            crate::gateway::middleware::basic_auth_middleware_with_credentials(
                req,
                next,
                Some("secret"),
                "test",
            )
            .await
        }));
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/app-providers/claude/saved/switch")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"targetHostId":"host"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 401);
}
