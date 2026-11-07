use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware,
};
use tower::ServiceExt;
use vw_api_types::id::TenantScope;
#[tokio::test]
async fn cc04_env_restore_registered_route_requires_gateway_authentication() {
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
                .uri("/app-providers/claude/environment/restore")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"targetHostId":"host","backupId":"opaque"}"#))
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
        app_providers::{env_clean::*, env_diagnose::*, env_restore::*},
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
    fn denied(result: Result<EnvRestoreResult, AgentAppsReadError>, status: u16) {
        assert!(
            matches!(result.unwrap_err(), AgentAppsReadError::Http { status: actual, body } if actual == status && body.is_null())
        );
    }
    #[tokio::test]
    async fn cc04_env_restore_sdk_api_restores_selected_definition_and_enforces_authority() {
        let home = tempfile::tempdir().unwrap();
        let private = tempfile::tempdir().unwrap();
        fs::set_permissions(private.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let path = home.path().join(".zshrc");
        let content = "# keep\nexport ANTHROPIC_API_KEY=selected-secret\nANTHROPIC_API_KEY=other\nOTHER=keep\n";
        fs::write(&path, content).unwrap();
        let access = policy(home.path(), private.path());
        let router = Router::new()
            .nest(
                "/v1",
                super::super::env_diagnose::router_with_policy(Some(access.clone()))
                    .merge(super::super::env_clean::router_with_policy(Some(access.clone())))
                    .merge(super::super::env_restore::router_with_policy(Some(access))),
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
        let cleaned = client
            .provider_env_clean(
                AppId::Claude,
                &EnvCleanRequest { target_host_id: "host".into(), selections: vec![selection] },
                TOKEN,
            )
            .await
            .unwrap();
        let replacement = fs::read_to_string(&path).unwrap();
        let backup =
            private.path().join("environment-backups").join(format!("{}.json", cleaned.backup_id));
        let saved_before = fs::read(&backup).unwrap();
        let mut request =
            EnvRestoreRequest { target_host_id: "read-only".into(), backup_id: cleaned.backup_id };
        denied(client.provider_env_restore(AppId::Claude, &request, TOKEN).await, 403);
        request.target_host_id = "alias".into();
        denied(client.provider_env_restore(AppId::Claude, &request, TOKEN).await, 409);
        request.target_host_id = "host".into();
        denied(client.provider_env_restore(AppId::Claude, &request, "wrong-token").await, 401);
        let foreign = GatewayClient::new(
            GatewayEndpoint::new("127.0.0.1", port)
                .with_tenant_scope(TenantScope::new("p", "b").unwrap()),
        )
        .unwrap();
        denied(foreign.provider_env_restore(AppId::Claude, &request, TOKEN).await, 403);
        denied(client.provider_env_restore(AppId::Omp, &request, TOKEN).await, 422);
        denied(client.provider_env_restore(AppId::Codex, &request, TOKEN).await, 409);
        let mut forged = request.clone();
        forged.backup_id = "../outside".into();
        denied(client.provider_env_restore(AppId::Claude, &forged, TOKEN).await, 400);
        forged.backup_id = "0".repeat(64);
        denied(client.provider_env_restore(AppId::Claude, &forged, TOKEN).await, 409);
        assert_eq!(fs::read_to_string(&path).unwrap(), replacement);
        assert_eq!(fs::read(&backup).unwrap(), saved_before);
        let result = client.provider_env_restore(AppId::Claude, &request, TOKEN).await.unwrap();
        assert_eq!(
            result.items,
            vec![EnvRestoreItem { file: UserShellFile::Zshrc, status: EnvRestoreStatus::Restored }]
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), content);
        assert!(!serde_json::to_string(&result).unwrap().contains("secret"));
        let retry = client.provider_env_restore(AppId::Claude, &request, TOKEN).await.unwrap();
        assert_eq!(retry.items[0].status, EnvRestoreStatus::AlreadyRestored);
        fs::write(&path, "OTHER=external\n").unwrap();
        let stale = client.provider_env_restore(AppId::Claude, &request, TOKEN).await.unwrap();
        assert_eq!(stale.items[0].status, EnvRestoreStatus::Conflict);
        assert_eq!(fs::read_to_string(&path).unwrap(), "OTHER=external\n");
        assert_eq!(fs::read_dir(private.path().join("environment-backups")).unwrap().count(), 1);
        server.abort();
        let _ = server.await;
    }
}
