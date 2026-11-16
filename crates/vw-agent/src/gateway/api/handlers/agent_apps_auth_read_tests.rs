//! Real host route assembly must not expose unversioned authorization aliases.
use axum::{body::Body, http::{Request, StatusCode}};
use tower::ServiceExt;

#[tokio::test]
async fn host_auth_read_rejects_unversioned_aliases() {
    for alias in ["/agent-apps/auth/accounts", "/api/agent-apps/auth/accounts"] {
        let response = crate::gateway::router::handler_router().oneshot(
            Request::builder().uri(format!("{alias}?host_id=host&service=codex_oauth"))
                .header("authorization", "Basic dGVzdDpzZWNyZXQ=")
                .header("x-agent-apps-token", "fixture-owner-token-aaaaaaaaaaaa")
                .header("x-platform-id", "p").header("x-company-id", "c")
                .body(Body::empty()).unwrap()
        ).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
