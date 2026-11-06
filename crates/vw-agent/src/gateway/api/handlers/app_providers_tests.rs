use crate::global_database::GlobalDatabase;
use vw_api_types::app_providers::{AppProviderWrite, ProviderProtocol};
fn profile(key: &str) -> AppProviderWrite {
    AppProviderWrite {
        request_options: None,
        name: "fixture".into(),
        protocol: ProviderProtocol::Anthropic,
        api_key: key.into(),
        base_url: "https://provider.example.test".into(),
        model: "fixture-model".into(),
    }
}
#[tokio::test]
async fn cc_apps_provider_launch_isolation_api_write_read_auth_and_redaction() {
    use axum::{
        Extension, Router,
        body::{Body, to_bytes},
        http::{Request, StatusCode},
        middleware,
    };
    use tower::ServiceExt;
    let home = tempfile::tempdir().unwrap();
    let app = Router::new()
        .nest("/v1", crate::gateway::router::handler_router())
        .layer(Extension(GlobalDatabase::for_home(home.path()).unwrap()))
        .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware))
        .layer(middleware::from_fn(|req, next| async {
            crate::gateway::middleware::basic_auth_middleware_with_credentials(
                req,
                next,
                Some("secret"),
                "test",
            )
            .await
        }));
    let request = |method: &str, company: &str, auth: &str, body: String| {
        Request::builder()
            .method(method)
            .uri("/v1/app-providers/claude/selected")
            .header("authorization", auth)
            .header("x-platform-id", "p")
            .header("x-company-id", company)
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap()
    };
    let payload = serde_json::to_string(&profile("fixture-key-only")).unwrap();
    let response = app
        .clone()
        .oneshot(request("PUT", "a", "Basic YmFkOmJhZA==", payload.clone()))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let response =
        app.clone().oneshot(request("PUT", "a", "Basic dGVzdDpzZWNyZXQ=", payload)).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 65536).await.unwrap();
    assert!(!String::from_utf8_lossy(&body).contains("fixture-key-only"));
    for (company, status) in [("a", StatusCode::OK), ("b", StatusCode::NOT_FOUND)] {
        let response = app
            .clone()
            .oneshot(request("GET", company, "Basic dGVzdDpzZWNyZXQ=", String::new()))
            .await
            .unwrap();
        assert_eq!(response.status(), status);
        assert!(
            !String::from_utf8_lossy(&to_bytes(response.into_body(), 65536).await.unwrap())
                .contains("fixture-key-only")
        );
    }
    let response = app
        .oneshot(request(
            "PUT",
            "a",
            "Basic dGVzdDpzZWNyZXQ=",
            "{\"apiKey\":\"fixture-key-only\",\"unknown\":1}".into(),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(
        !String::from_utf8_lossy(&to_bytes(response.into_body(), 65536).await.unwrap())
            .contains("fixture-key-only")
    );
}

#[tokio::test]
async fn cc_apps_provider_launch_isolation_sdk_http_sqlite_projection_process_roundtrip() {
    use axum::{Extension, Router, middleware};
    use vw_api_types::{agent_apps::*, app_providers::ProviderLaunchRequest, id::TenantScope};
    use vw_gateway_client::{GatewayAuth, GatewayClient, GatewayEndpoint};
    let home = tempfile::tempdir().unwrap();
    let database = GlobalDatabase::for_home(home.path()).unwrap();
    let router = Router::new()
        .nest("/v1", crate::gateway::router::handler_router())
        .layer(Extension(database.clone()))
        .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware))
        .layer(middleware::from_fn({
            let guard = crate::security::pairing::PairingGuard::new(true, &["secret".into()]);
            move |req: axum::http::Request<axum::body::Body>, next: axum::middleware::Next| {
                let guard = guard.clone();
                async move {
                    use axum::response::IntoResponse;
                    let key =
                        crate::gateway::api::auth::extract_auth_skey(req.headers()).unwrap_or("");
                    if !guard.is_authenticated(key) {
                        return axum::http::StatusCode::UNAUTHORIZED.into_response();
                    }
                    next.run(req).await
                }
            }
        }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let tenant = TenantScope::new("p", "a").unwrap();
    let client = GatewayClient::new(
        GatewayEndpoint::new("127.0.0.1", port)
            .with_tenant_scope(tenant.clone())
            .with_auth(GatewayAuth { skey: Some("secret".into()) }),
    )
    .unwrap();
    let summary = client
        .app_provider_save(AppId::Claude, "selected", &profile("fixture-key-only"))
        .await
        .unwrap();
    assert!(summary.configured);
    assert_eq!(client.app_provider_get(AppId::Claude, "selected").await.unwrap(), summary);
    let root = tempfile::tempdir().unwrap();
    let program = root.path().join("claude");
    std::fs::write(&program,"#!/bin/sh\ntest \"$ANTHROPIC_API_KEY\" = fixture-key-only || exit 22\nprintf 'selected' > actual-provider\nprintf '%s' \"$CLAUDE_CONFIG_DIR\" > private-path\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let request = ProviderLaunchRequest {
        provider_id: "selected".into(),
        launch: LaunchRequest {
            app_id: AppId::Claude,
            installation_id: AppInstallationId("observed".into()),
            cwd: root.path().to_str().unwrap().into(),
            args: vec![],
            skip_permissions: false,
        },
    };
    let installation = AppInstallation {
        id: request.launch.installation_id.clone(),
        app_id: AppId::Claude,
        target_host_id: "host".into(),
        environment: if cfg!(target_os = "macos") {
            AppInstallationEnvironment::Macos
        } else if cfg!(windows) {
            AppInstallationEnvironment::Windows
        } else {
            AppInstallationEnvironment::Linux
        },
        path: program.to_str().unwrap().into(),
        real_path: None,
        source: AppInstallationSource::Native,
        is_path_default: true,
        version: None,
        runtime_status: AppInstallationRuntimeStatus::Runnable,
    };
    tokio::task::spawn_blocking(move || {
        let store = crate::app_providers::AppProviderStore::new(database).unwrap();
        for environment in [
            AppInstallationEnvironment::Windows,
            AppInstallationEnvironment::Wsl { distribution: "Ubuntu".into() },
        ] {
            let mut foreign = installation.clone();
            foreign.environment = environment;
            #[cfg(not(windows))]
            assert!(
                store
                    .prepare_launch(
                        &tenant,
                        &request,
                        "host",
                        &[foreign],
                        &[root.path().into()],
                        false,
                        root.path()
                    )
                    .is_err()
            );
        }
        store
            .prepare_launch(
                &tenant,
                &request,
                "host",
                &[installation],
                &[root.path().into()],
                false,
                root.path(),
            )
            .unwrap()
            .run()
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(root.path().join("actual-provider")).unwrap(),
            "selected"
        );
        let private = std::fs::read_to_string(root.path().join("private-path")).unwrap();
        assert!(!std::path::Path::new(&private).exists());
    })
    .await
    .unwrap();
    server.abort();
    let _ = server.await;
}
