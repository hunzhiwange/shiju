//! Only real host route assembly remains here; provider behavior lives in its crate.
#[tokio::test]
async fn app_provider_host_route_requires_basic_auth() {
    use axum::{Router, body::Body, http::{Request, StatusCode}, middleware};
    use tower::ServiceExt;
    let app = Router::new()
        .nest("/v1", crate::gateway::router::handler_router())
        .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware))
        .layer(middleware::from_fn(|req, next| async {
            crate::gateway::middleware::basic_auth_middleware_with_credentials(req, next, Some("secret"), "test").await
        }));
    for (auth, expected) in [("Basic YmFkOmJhZA==", StatusCode::UNAUTHORIZED), ("Basic dGVzdDpzZWNyZXQ=", StatusCode::BAD_REQUEST)] {
        // Missing tenant is rejected before provider storage is consulted.
        let response = app.clone().oneshot(Request::get("/v1/app-providers/claude/selected")
            .header("authorization", auth).body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(response.status(), expected);
    }
}
