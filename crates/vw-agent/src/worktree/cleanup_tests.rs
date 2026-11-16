//! Real runtime status conversion at the worktree host boundary.
#[test]
fn worktree_host_busy_snapshot_includes_busy_and_retry_only() {
    use crate::session::status::Info;
    use std::collections::{HashMap, HashSet};
    let busy = super::super::host_adapter::busy_sessions(HashMap::from([
        ("busy".into(), Info::Busy),
        ("retry".into(), Info::Retry { attempt: 1, message: "retry".into(), next: 100 }),
        ("idle".into(), Info::Idle),
    ]));
    assert_eq!(busy, HashSet::from(["busy".into(), "retry".into()]));
}

#[tokio::test]
async fn worktree_host_router_mounts_cleanup_and_requires_tenant() {
    use axum::{body::Body, http::{Request, StatusCode}};
    use tower::ServiceExt;
    let response = crate::gateway::api::handlers::project::router::<()>()
        .oneshot(Request::builder()
            .uri("/projects/missing/worktree-cleanup-status")
            .body(Body::empty()).unwrap())
        .await.unwrap();
    assert_ne!(response.status(), StatusCode::NOT_FOUND);
    assert!(!response.status().is_success());
}
