use axum::http::StatusCode;
use {
    vw_api_transport::id::TenantScope,
};

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
                .body(Body::empty())
                .unwrap()
        };
        assert_eq!(
            app.clone().oneshot(request(&"0".repeat(64))).await.unwrap().status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
        let credentials =
            local_app_auth::initialize(std::path::Path::new(&home), "p", "c").unwrap();
        assert_eq!(
            app.clone().oneshot(request(&credentials.token)).await.unwrap().status(),
            StatusCode::OK
        );
        assert_eq!(
            app.oneshot(request(&"0".repeat(64))).await.unwrap().status(),
            StatusCode::UNAUTHORIZED
        );
        verify_local_auth(std::path::Path::new(&home), &credentials.token).await;
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

async fn verify_local_auth(home: &std::path::Path, app_token: &str) {
    use axum::{body::Body, extract::ConnectInfo, http::Request};
    use std::net::SocketAddr;
    use tower::ServiceExt;
    use {gateway_endpoint::{GatewayEndpoint}, {gateway_app_auth::AgentAppsAuthCredentials, gateway_app_auth::AgentAppsAuthError, vw_gateway_client::GatewayClient, vw_api_app_auth::{AuthErrorCode, AuthFlowId, AuthService}}};

    let router = crate::gateway::api::handlers::agent_apps_auth_read::router(None, "test", None);
    let credential = local_app_auth::initialize_auth(home, "p", "c").unwrap();
    assert_eq!(credential.token, app_token);
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server_router = router.clone();
    let server = tokio::spawn(async move {
        axum::serve(listener, server_router.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap();
    });
    let client = GatewayClient::new(
        GatewayEndpoint::new("127.0.0.1", port)
            .with_tenant_scope(TenantScope::new("p", "c").unwrap()),
    )
    .unwrap();
    let auth = AgentAppsAuthCredentials { username: "", password: "", principal_token: app_token };
    let host = local_app_auth::HOST_ID;
    let service = AuthService::CodexOauth;
    assert!(client.agent_apps_auth_accounts(host, service, &auth).await.unwrap().is_empty());
    // An unknown flow reaches the authorized handler without calling a provider.
    let unknown_flow = AuthFlowId(uuid::Uuid::new_v4().to_string());
    assert!(matches!(
        client.agent_apps_auth_poll(host, service, &auth, &unknown_flow).await,
        Err(AgentAppsAuthError::Http { status: 404, code: Some(AuthErrorCode::NotFound), .. })
    ));

    let request = |peer: &str, forwarded: bool, company: &str, token: &str| {
        let mut builder = Request::builder()
            .uri("/v1/agent-apps/auth/accounts?host_id=desktop-local&service=codex_oauth")
            .extension(ConnectInfo(peer.parse::<SocketAddr>().unwrap()))
            .header("x-platform-id", "p")
            .header("x-company-id", company)
            ;
        if forwarded {
            builder = builder.header("x-forwarded-for", "127.0.0.1");
        }
        builder.body(Body::empty()).unwrap()
    };
    let other = local_app_auth::initialize_auth(home, "p", "other").unwrap();
    assert_ne!(other.token, app_token);
    for input in [
        request("192.0.2.1:1234", false, "c", app_token),
        request("127.0.0.1:1234", true, "c", app_token),
        request("127.0.0.1:1234", false, "other", app_token),
        request("127.0.0.1:1234", false, "c", &"0".repeat(64)),
    ] {
        assert_eq!(router.clone().oneshot(input).await.unwrap().status(), StatusCode::UNAUTHORIZED);
    }
    for managed in [
        crate::gateway::api::handlers::agent_apps_auth_read::router(Some("secret"), "test", None),
        crate::gateway::api::handlers::agent_apps_auth_read::router(
            None,
            "test",
            Some(home.join("admin-policy.json")),
        ),
    ] {
        let input = request("127.0.0.1:1234", false, "c", app_token);
        assert_eq!(managed.oneshot(input).await.unwrap().status(), StatusCode::UNAUTHORIZED);
    }
    server.abort();
}
