#![cfg(target_os = "macos")]
use axum::{Router, middleware};
use sha2::{Digest, Sha256};
use std::{os::unix::fs::PermissionsExt, path::Path};
use vw_api_types::{agent_apps::AppId, app_providers::preview::*, id::TenantScope};
const TOKEN: &str = "fixture-config-reader-token-over-32-chars";
pub(super) fn policy(root: &Path) -> std::path::PathBuf {
    std::fs::set_permissions(root, std::fs::Permissions::from_mode(0o700)).unwrap();
    let path = root.join("access.json");
    let value = serde_json::json!({
        "principals":[{"id":"reader","token_sha256":hex::encode(Sha256::digest(TOKEN)),
            "grants":[{"platform_id":"p","company_id":"a","host_id":"host","provider_config_read":true},
                      {"platform_id":"p","company_id":"a","host_id":"metadata-only"},
                      {"platform_id":"p","company_id":"a","host_id":"offline","provider_config_read":true},
                      {"platform_id":"p","company_id":"a","host_id":"remote","provider_config_read":true}]}],
        "hosts":[{"id":"host","online":true,"binding":"local","home":root,"search_paths":[],"applications":root.join("Applications")},
                 {"id":"metadata-only","online":true,"binding":"local","home":root,"search_paths":[],"applications":root.join("Applications")},
                 {"id":"offline","online":false,"binding":"local","home":root,"search_paths":[],"applications":root.join("Applications")},
                 {"id":"remote","online":true,"binding":"remote","home":root,"search_paths":[],"applications":root.join("Applications")}]
    });
    std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    path
}

#[tokio::test]
async fn cc04_preview_sdk_api_projects_without_writing_and_denies_permission() {
    use vw_gateway_client::{GatewayClient, GatewayEndpoint};
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join(".claude")).unwrap();
    let config = root.path().join(".claude/settings.json");
    let before = br#"{"hooks":{"keep":true},"env":{"ANTHROPIC_API_KEY":"live-secret"}}"#;
    std::fs::write(&config, before).unwrap();
    let router = Router::new()
        .nest("/v1", super::preview::router_with_policy(Some(policy(root.path()))))
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
    let mut request = ProviderPreviewRequest {
        request_options: None,
        target_host_id: "host".into(),
        directory_override: None,
        settings: serde_json::json!({"env":{"ANTHROPIC_API_KEY":"draft-secret","ANTHROPIC_MODEL":"m"},"hooks":{"keep":false}}),
        category: None,
        provider_id: None,
        meta: None,
    };
    let response = client.app_provider_preview(AppId::Claude, &request, TOKEN).await.unwrap();
    assert_eq!(response.settings["hooks"]["keep"], true);
    assert!(response.inactive.contains(&vec!["hooks".into()]));
    assert!(!serde_json::to_string(&response).unwrap().contains("secret"));
    // The same SDK/HTTP/executor chain exercises all four supported application branches.
    for (app, directory, name, text, draft, expected) in [
        (
            AppId::Gemini,
            ".gemini",
            "settings.json",
            r#"{"ui":{"compact":true}}"#,
            serde_json::json!({"env":{"GEMINI_API_KEY":"gemini-secret"},"config":{"ui":{"compact":false}}}),
            vec!["config", "ui"],
        ),
        (
            AppId::Codex,
            ".codex",
            "config.toml",
            "[projects.fixture]\nkeep=true\n",
            serde_json::json!({"auth":{},"config":"model_provider='ollama'\n[projects.fixture]\nkeep=false\n"}),
            vec!["config", "projects", "fixture"],
        ),
        (
            AppId::GrokBuild,
            ".grok",
            "config.toml",
            "[ui]\ncompact=true\n",
            serde_json::json!({"config":"[models]\ndefault='fixture'\n[model.fixture]\nmodel='m'\nname='fixture'\nbase_url='https://fixture.test'\napi_key='grok-secret'\napi_backend='responses'\ncontext_window=1000\n[ui]\ncompact=false\n"}),
            vec!["config", "ui", "compact"],
        ),
    ] {
        let directory = root.path().join(directory);
        std::fs::create_dir(&directory).unwrap();
        let file = directory.join(name);
        std::fs::write(&file, text).unwrap();
        let mut draft_request = request.clone();
        draft_request.settings = draft;
        let view = client.app_provider_preview(app, &draft_request, TOKEN).await.unwrap();
        assert!(view.unavailable.is_none());
        assert!(view.parse_error.is_none());
        assert!(!view.changes.is_empty());
        assert!(
            view.inactive
                .iter()
                .any(|path| path.iter().map(String::as_str).eq(expected.iter().copied()))
        );
        assert!(!serde_json::to_string(&view).unwrap().contains("secret"));
        assert_eq!(std::fs::read_to_string(&file).unwrap(), text);
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 1);
    }
    for (host, status) in
        [("metadata-only", 403), ("missing", 403), ("offline", 503), ("remote", 501)]
    {
        request.target_host_id = host.into();
        assert!(
            matches!(client.app_provider_preview(AppId::Claude,&request,TOKEN).await.unwrap_err(),vw_gateway_client::AgentAppsReadError::Http{status:actual,..} if actual==status)
        );
    }
    assert_eq!(std::fs::read(&config).unwrap(), before);
    assert_eq!(std::fs::read_dir(config.parent().unwrap()).unwrap().count(), 1);
    server.abort();
    let _ = server.await;
}

#[tokio::test]
async fn cc04_preview_existing_record_and_owner_are_read_only_and_tenant_scoped() {
    use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
    use vw_agent_apps::providers::live::commit::{
        AppState, ExecutionHostId, ProviderTransactions, ScopedProvider,
    };
    use vw_gateway_client::{GatewayClient, GatewayEndpoint};
    let root = tempfile::tempdir().unwrap();
    let private = root.path().join("private");
    std::fs::create_dir(&private).unwrap();
    std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o700)).unwrap();
    let policy_path = policy(root.path());
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&policy_path).unwrap()).unwrap();
    value["hosts"][0]["credential_root"] = serde_json::json!(private);
    std::fs::write(&policy_path, serde_json::to_vec(&value).unwrap()).unwrap();
    let tenant = TenantScope::new("p", "a").unwrap();
    let db = GlobalDatabase::for_home(root.path()).unwrap();
    let store = GatewaySettingsStore::new(db.clone()).unwrap();
    let saved = serde_json::json!({"category":"custom","settingsConfig":{"env":{"ANTHROPIC_API_KEY":"stored-secret","ANTHROPIC_MODEL":"stored-model"}}});
    store.upsert(&tenant, "app_providers.records", "\"claude\":saved", &saved.to_string()).unwrap();
    let transactions =
        ProviderTransactions::new(&private, ExecutionHostId::new("host").unwrap()).unwrap();
    let state = AppState {
        pointer: Some(ScopedProvider { tenant: tenant.clone(), id: "saved".into() }),
        ..Default::default()
    };
    let state_path = private.join("provider-live/aG9zdA/claude.state.json");
    std::fs::write(&state_path, serde_json::to_vec(&state).unwrap()).unwrap();
    let state_before = std::fs::read(&state_path).unwrap();
    let db_before = std::fs::read(db.sqlite_path().unwrap()).unwrap();
    let router = Router::new()
        .nest("/v1", super::preview::router_with_policy(Some(policy_path)))
        .layer(axum::Extension(db.clone()))
        .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let endpoint = GatewayEndpoint::new("127.0.0.1", addr.port());
    let client = GatewayClient::new(endpoint.clone().with_tenant_scope(tenant.clone())).unwrap();
    let req = ProviderPreviewRequest {
        request_options: None,
        target_host_id: "host".into(),
        provider_id: Some("saved".into()),
        meta: None,
        settings: serde_json::Value::Null,
        category: None,
        directory_override: None,
    };
    let view = client.app_provider_preview(AppId::Claude, &req, TOKEN).await.unwrap();
    assert!(view.unavailable.is_none());
    assert_eq!(view.settings["env"]["ANTHROPIC_API_KEY"], "[redacted]");
    assert!(!serde_json::to_string(&view).unwrap().contains("stored-secret"));
    assert_eq!(std::fs::read(&state_path).unwrap(), state_before);
    assert_eq!(std::fs::read(db.sqlite_path().unwrap()).unwrap(), db_before);
    assert!(transactions.pending(AppId::Claude).unwrap().is_none());
    assert_eq!(
        store.get(&tenant, "app_providers.records", "\"claude\":saved").unwrap().unwrap(),
        saved.to_string()
    );
    let foreign =
        GatewayClient::new(endpoint.with_tenant_scope(TenantScope::new("p", "b").unwrap()))
            .unwrap();
    assert!(matches!(
        foreign.app_provider_preview(AppId::Claude, &req, TOKEN).await.unwrap_err(),
        vw_gateway_client::AgentAppsReadError::Http { status: 403, .. }
    ));
    assert!(matches!(
        client.app_provider_preview(AppId::Claude, &req, "invalid").await.unwrap_err(),
        vw_gateway_client::AgentAppsReadError::Http { status: 401, .. }
    ));
    assert!(!root.path().join(".claude").exists());
    let pending = vw_agent_apps::providers::live::commit::Pending {
        provider_record: None,
        project_preimage: None,
        version: 1,
        host: ExecutionHostId::new("host").unwrap(),
        app: AppId::Claude,
        tenant: tenant.clone(),
        directory_device: 0,
        directory_inode: 0,
        target: Default::default(),
        files: vec![],
        published: false,
    };
    let pending_path = state_path.with_file_name("claude.pending.json");
    let pending_bytes = serde_json::to_vec(&pending).unwrap();
    std::fs::write(&pending_path, &pending_bytes).unwrap();
    let view = client.app_provider_preview(AppId::Claude, &req, TOKEN).await.unwrap();
    assert_eq!(view.unavailable, Some(PreviewUnavailable::PendingTransaction));
    assert_eq!(std::fs::read(&pending_path).unwrap(), pending_bytes);
    assert_eq!(std::fs::read(&state_path).unwrap(), state_before);
    std::fs::remove_file(&pending_path).unwrap();
    let mut other_owner = state.clone();
    other_owner.pointer.as_mut().unwrap().tenant = TenantScope::new("p", "b").unwrap();
    let other_bytes = serde_json::to_vec(&other_owner).unwrap();
    std::fs::write(&state_path, &other_bytes).unwrap();
    let view = client.app_provider_preview(AppId::Claude, &req, TOKEN).await.unwrap();
    assert_eq!(view.unavailable, Some(PreviewUnavailable::Ownership));
    assert!(view.settings.is_null());
    assert_eq!(std::fs::read(&state_path).unwrap(), other_bytes);

    server.abort();
    let _ = server.await;
}
#[tokio::test]
async fn cc04_preview_registered_route_requires_gateway_authentication() {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
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
                .uri("/app-providers/claude/config/preview")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"targetHostId":"host","settings":{}}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
