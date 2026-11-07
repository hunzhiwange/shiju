use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware,
};
use tower::ServiceExt;
use vw_api_types::id::TenantScope;
#[tokio::test]
async fn cc04_env_clean_registered_route_requires_gateway_authentication() {
    let app = crate::gateway::router::handler_router()
        .layer(axum::Extension(TenantScope::new("p", "a").unwrap()))
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
                .uri("/app-providers/claude/environment/clean")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"targetHostId":"host","selections":[]}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
#[cfg(target_os = "macos")]
mod execution {
    use axum::{Router, middleware};
    use sha2::{Digest, Sha256};
    use std::{fs, os::unix::fs::PermissionsExt, path::Path};
    use vw_api_types::{
        agent_apps::AppId,
        app_providers::{env_clean::*, env_diagnose::*},
        id::TenantScope,
    };
    use vw_gateway_client::{AgentAppsReadError, GatewayClient, GatewayEndpoint};
    const TOKEN: &str = "fixture-config-cleaner-token-over-32-chars";
    fn policy(home: &Path, private: &Path) -> std::path::PathBuf {
        fs::set_permissions(home, fs::Permissions::from_mode(0o700)).unwrap();
        let path = home.join("access.json");
        let value = serde_json::json!({
            "principals":[{"id":"cleaner","token_sha256":hex::encode(Sha256::digest(TOKEN)),
                "grants":[{"platform_id":"p","company_id":"a","host_id":"host","provider_config_read":true,"provider_config_write":true},
                          {"platform_id":"p","company_id":"a","host_id":"read-only","provider_config_read":true},
                          {"platform_id":"p","company_id":"a","host_id":"alias","provider_config_read":true,"provider_config_write":true}]}],
            "hosts":[{"id":"host","online":true,"binding":"local","home":home,"credential_root":private,"search_paths":[],"applications":home.join("Applications")},
                     {"id":"read-only","online":true,"binding":"local","home":home,"credential_root":private,"search_paths":[],"applications":home.join("Applications")},
                     {"id":"alias","online":true,"binding":"local","home":home,"credential_root":private,"search_paths":[],"applications":home.join("Applications")}]
        });
        fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        path
    }
    fn denied(result: Result<EnvCleanResult, AgentAppsReadError>, status: u16) {
        assert!(
            matches!(result.unwrap_err(), AgentAppsReadError::Http { status: actual, body } if actual == status && body.is_null())
        );
    }
    #[tokio::test]
    async fn cc04_env_clean_sdk_api_backups_selection_and_write_permissions() {
        let home = tempfile::tempdir().unwrap();
        let private = tempfile::tempdir().unwrap();
        fs::set_permissions(private.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let path = home.path().join(".zshrc");
        let content = "# keep\nexport ANTHROPIC_API_KEY=selected-secret\nexport ANTHROPIC_API_KEY=unselected-secret\nOTHER=keep\n";
        fs::write(&path, content).unwrap();
        fs::write(home.path().join(".profile"), "ANTHROPIC_API_KEY=other-source\n").unwrap();
        let access = policy(home.path(), private.path());
        let router = Router::new()
            .nest(
                "/v1",
                super::super::env_diagnose::router_with_policy(Some(access.clone()))
                    .merge(super::super::env_clean::router_with_policy(Some(access))),
            )
            .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        let client = GatewayClient::new(
            GatewayEndpoint::new("127.0.0.1", port)
                .with_tenant_scope(TenantScope::new("p", "a").unwrap()),
        )
        .unwrap();
        let diagnosis = client
            .provider_env_diagnose(
                AppId::Claude,
                &EnvDiagnoseRequest { target_host_id: "host".into() },
                TOKEN,
            )
            .await
            .unwrap();
        let selection = diagnosis
            .entries
            .iter()
            .find(|e| e.source == EnvSource::UserShell { file: UserShellFile::Zshrc, line: 2 })
            .unwrap()
            .cleanup
            .clone()
            .unwrap();
        assert!(!serde_json::to_string(&diagnosis).unwrap().contains("selected-secret"));
        let mut request = EnvCleanRequest {
            target_host_id: "read-only".into(),
            selections: vec![selection.clone()],
        };
        denied(client.provider_env_clean(AppId::Claude, &request, TOKEN).await, 403);
        request.target_host_id = "alias".into();
        denied(client.provider_env_clean(AppId::Claude, &request, TOKEN).await, 409);
        request.target_host_id = "host".into();
        denied(client.provider_env_clean(AppId::Claude, &request, "invalid").await, 401);
        let foreign = GatewayClient::new(
            GatewayEndpoint::new("127.0.0.1", port)
                .with_tenant_scope(TenantScope::new("p", "b").unwrap()),
        )
        .unwrap();
        denied(foreign.provider_env_clean(AppId::Claude, &request, TOKEN).await, 403);
        denied(client.provider_env_clean(AppId::Omp, &request, TOKEN).await, 422);
        let mut forged = request.clone();
        forged.selections[0].base_digest = "forged".into();
        denied(client.provider_env_clean(AppId::Claude, &forged, TOKEN).await, 409);
        assert_eq!(fs::read_to_string(&path).unwrap(), content);
        assert_eq!(fs::read_dir(private.path()).unwrap().count(), 0);
        let result = client.provider_env_clean(AppId::Claude, &request, TOKEN).await.unwrap();
        assert_eq!(result.removed_definitions, 1);
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            content.replace("export ANTHROPIC_API_KEY=selected-secret\n", "")
        );
        assert_eq!(
            fs::read_to_string(home.path().join(".profile")).unwrap(),
            "ANTHROPIC_API_KEY=other-source\n"
        );
        let backup =
            private.path().join("environment-backups").join(format!("{}.json", result.backup_id));
        let saved: serde_json::Value = serde_json::from_slice(&fs::read(&backup).unwrap()).unwrap();
        assert_eq!(saved["files"][0]["original"], content);
        assert_eq!(fs::metadata(backup).unwrap().permissions().mode() & 0o777, 0o600);
        assert!(!serde_json::to_string(&result).unwrap().contains("secret"));
        denied(client.provider_env_clean(AppId::Claude, &request, TOKEN).await, 409);
        // A newly diagnosed definition cannot be removed using a stale file base.
        let diagnosis = client
            .provider_env_diagnose(
                AppId::Claude,
                &EnvDiagnoseRequest { target_host_id: "host".into() },
                TOKEN,
            )
            .await
            .unwrap();
        request.selections = vec![
            diagnosis
                .entries
                .iter()
                .find(|e| {
                    matches!(e.source, EnvSource::UserShell { file: UserShellFile::Zshrc, .. })
                })
                .unwrap()
                .cleanup
                .clone()
                .unwrap(),
        ];
        fs::write(&path, "ANTHROPIC_API_KEY=external-edit\n").unwrap();
        denied(client.provider_env_clean(AppId::Claude, &request, TOKEN).await, 409);
        assert_eq!(fs::read_to_string(&path).unwrap(), "ANTHROPIC_API_KEY=external-edit\n");
        assert_eq!(fs::read_dir(private.path().join("environment-backups")).unwrap().count(), 1);
        server.abort();
        let _ = server.await;
    }
}
