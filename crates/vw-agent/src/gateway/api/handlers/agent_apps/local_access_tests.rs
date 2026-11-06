use super::access::*;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use vw_api_types::{agent_apps::AppLifecycleAction, id::TenantScope};

#[test]
fn local_apps_generated_grant_uses_existing_authorization_and_rejects_other_scopes() {
    let home = tempfile::tempdir().unwrap();
    let credential = vw_shared::local_apps::initialize(home.path(), "p", "c").unwrap();
    let policy = vw_shared::local_apps::policy_path(home.path(), "p", "c");
    let mut headers = HeaderMap::new();
    headers.insert("x-agent-apps-token", HeaderValue::from_str(&credential.token).unwrap());
    let tenant = TenantScope::new("p", "c").unwrap();
    let host = vw_shared::local_apps::HOST_ID;
    assert!(authorize(Some(&policy), &headers, &tenant, host).is_ok());
    assert!(authorize_preferences(Some(&policy), &headers, &tenant, host, None, true).is_ok());
    assert!(authorize_launch_session(Some(&policy), &headers, &tenant, host).is_ok());
    for action in
        [AppLifecycleAction::Install, AppLifecycleAction::Update, AppLifecycleAction::Uninstall]
    {
        assert!(authorize_action(Some(&policy), &headers, &tenant, host, Some(action)).is_ok());
    }
    assert_eq!(
        authorize(Some(&policy), &headers, &TenantScope::new("p", "other").unwrap(), host).err(),
        Some(StatusCode::FORBIDDEN)
    );
    assert_eq!(
        authorize(Some(&policy), &headers, &tenant, "other-host").err(),
        Some(StatusCode::FORBIDDEN)
    );
    assert_eq!(
        authorize(Some(&policy), &HeaderMap::new(), &tenant, host).err(),
        Some(StatusCode::UNAUTHORIZED)
    );
    assert_eq!(
        authorize(Some(&home.path().join("explicit-missing-policy")), &headers, &tenant, host)
            .err(),
        Some(StatusCode::SERVICE_UNAVAILABLE)
    );
}

#[tokio::test]
async fn local_apps_default_router_loads_new_private_grant_without_restart_or_env() {
    use axum::{body::Body, http::Request, middleware};
    use tower::ServiceExt;
    if let Some(home) = std::env::var_os("SHIJU_LOCAL_APPS_TEST_CHILD") {
        assert!(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").is_none());
        let app = crate::gateway::router::handler_router()
            .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware));
        let request = |token: &str| {
            Request::builder()
                .uri("/agent-apps/catalog?targetHostId=desktop-local")
                .header("x-platform-id", "p")
                .header("x-company-id", "c")
                .header("x-agent-apps-token", token)
                .body(Body::empty())
                .unwrap()
        };
        assert_eq!(
            app.clone().oneshot(request(&"0".repeat(64))).await.unwrap().status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
        let credentials =
            vw_shared::local_apps::initialize(std::path::Path::new(&home), "p", "c").unwrap();
        assert_eq!(
            app.clone().oneshot(request(&credentials.token)).await.unwrap().status(),
            StatusCode::OK
        );
        assert_eq!(
            app.oneshot(request(&"0".repeat(64))).await.unwrap().status(),
            StatusCode::UNAUTHORIZED
        );
        return;
    }
    let home = tempfile::tempdir().unwrap();
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "gateway::api::handlers::agent_apps::local_access_tests::local_apps_default_router_loads_new_private_grant_without_restart_or_env", "--nocapture"])
        .env("SHIJU_LOCAL_APPS_TEST_CHILD", home.path())
        .env("VIBEWINDOW_TEST_HOME", home.path())
        .env_remove("SHIJU_AGENT_APPS_ACCESS_FILE")
        .status().unwrap();
    assert!(status.success());
}
