use app_routing_services::RoutingListener;
use axum::{Router, http::StatusCode};
use serde_json::json;
use tokio::sync::mpsc;
use vw_api_transport::id::{TenantScope, ProjectScope};
#[tokio::test]
async fn aggregation_t12_queue_publication_rejects_unauthorized_scope() {
    use crate::server::lifecycle_api::RoutingControl;
    use super::configure_with_policy;
    use axum::{
        Extension,
        body::Body,
        http::{HeaderMap, Request},
        routing::get,
    };
    use sha2::{Digest, Sha256};
    use std::{os::unix::fs::PermissionsExt, sync::Arc};
    use tower::ServiceExt;
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let path = root.path().join("policy.json");
    let policy = json!({
        "hosts": [{"id":"host", "online":true, "binding":"local", "home":root.path(),
            "search_paths":[], "applications":root.path().join("Applications"), "credential_root":root.path()}],
        "principals": [{"id":"manager", "token_sha256":hex::encode(Sha256::digest("fixture-queue-publication-manager-token")),
            "grants":[{"platform_id":"platform", "company_id":"company", "host_id":"host",
                "routing_read":true, "routing_manage":true, "provider_config_write":true,
                "routing_apps":[{"project_id":"project", "app_id":"codex"}]}]}]
    });
    std::fs::write(&path, policy.to_string()).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let listener = Arc::new(RoutingListener::with_database(agent_database::GlobalDatabase::for_home(root.path()).unwrap()));
    let (send, mut receive) = mpsc::unbounded_channel();
    let router = configure_with_policy(
        Router::new().route(
            "/",
            get(move |Extension(control): Extension<RoutingControl>| async move {
                send.send(control).unwrap();
            }),
        ),
        &listener,
        path,
        agent_database::GlobalDatabase::for_home(root.path()).unwrap(),
    )
    .await;
    assert_eq!(router.oneshot(Request::new(Body::empty())).await.unwrap().status(), StatusCode::OK);
    let control = receive.try_recv().unwrap();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let mut headers = HeaderMap::new();
    headers
        .insert("x-agent-apps-token", "fixture-queue-publication-manager-token".parse().unwrap());
    for (tenant, project, host) in [
        (TenantScope::new("platform", "other").unwrap(), "project", "host"),
        (tenant.clone(), "other", "host"),
        (tenant.clone(), "project", "other"),
    ] {
        let scope = ProjectScope::new(tenant, project.into()).unwrap();
        assert_eq!(
            control
                .publish_failover_route(
                    &headers,
                    &scope,
                    host,
                    None,
                    |_, _| panic!("unauthorized preparation"),
                    |_, _, _, _, _| panic!("unauthorized commit")
                )
                .err(),
            Some(StatusCode::FORBIDDEN)
        );
    }
}
