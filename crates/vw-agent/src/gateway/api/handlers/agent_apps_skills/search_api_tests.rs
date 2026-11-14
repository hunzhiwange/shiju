use axum::{Extension, body::Body, http::{Request, StatusCode}};
use tower::ServiceExt;
use vw_api_types::{id::TenantScope, agent_apps_skills::search_api::MarketSearchFailure};

#[tokio::test]
async fn cp12_search_api_rejects_without_side_effects() {
    // Invalid queries are rejected before policy, storage or outbound HTTP is accessed.
    for query in ["query=&limit=20&offset=0", "query=rust&limit=101&offset=0",
        "query=rust&limit=20&offset=10001", "query=rust&limit=0&offset=0"] {
        let app = super::search_api::router()
            .layer(Extension(TenantScope::new("platform-a", "company-a").unwrap()));
        let request = Request::builder().uri(format!(
            "/agent-apps/skills/search?projectId=project-a&targetHostId=host-a&{query}"))
            .body(Body::empty()).unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let bytes = axum::body::to_bytes(response.into_body(), 4096).await.unwrap();
        assert_eq!(serde_json::from_slice::<MarketSearchFailure>(&bytes).unwrap(),
            MarketSearchFailure::InvalidInput);
    }
}
