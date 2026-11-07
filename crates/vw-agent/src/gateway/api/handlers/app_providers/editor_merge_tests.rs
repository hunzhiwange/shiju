use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
use axum::{Extension, Router, middleware};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::os::unix::fs::PermissionsExt;
use vw_agent_apps::providers::live::commit::{
    AppState, ExecutionHostId, ProviderTransactions, ScopedProvider,
};
use vw_api_types::{agent_apps::AppId, app_providers::editor_merge::*, id::TenantScope};
use vw_gateway_client::{GatewayClient, GatewayEndpoint};
const TOKEN: &str = "fixture-editor-token-with-at-least-32-chars";
struct Fixture {
    root: tempfile::TempDir,
    db: GlobalDatabase,
    client: GatewayClient,
    server: tokio::task::JoinHandle<()>,
    policy: std::path::PathBuf,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.server.abort();
    }
}
impl Fixture {
    async fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let private = root.path().join("private");
        std::fs::create_dir(&private).unwrap();
        std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o700)).unwrap();
        std::fs::create_dir(root.path().join(".claude")).unwrap();
        std::fs::write(
            root.path().join(".claude/settings.json"),
            br#"{"x":1,"untouched":"fixture-secret","env":{"ANTHROPIC_API_KEY":"fixture-key"}}"#,
        )
        .unwrap();
        let tenant = TenantScope::new("p", "a").unwrap();
        let db = GlobalDatabase::for_home(root.path()).unwrap();
        let store = GatewaySettingsStore::new(db.clone()).unwrap();
        for id in ["saved", "inactive"] {
            store.upsert(&tenant,"app_providers.records",&format!("\"claude\":{id}"),&json!({"category":"custom","settingsConfig":{"env":{"ANTHROPIC_API_KEY":"fixture-key"},"catalog":true}}).to_string()).unwrap();
        }
        let _tx =
            ProviderTransactions::new(&private, ExecutionHostId::new("host").unwrap()).unwrap();
        let state = AppState {
            pointer: Some(ScopedProvider { tenant: tenant.clone(), id: "saved".into() }),
            ..Default::default()
        };
        std::fs::write(
            private.join("provider-live/aG9zdA/claude.state.json"),
            serde_json::to_vec(&state).unwrap(),
        )
        .unwrap();
        let policy = root.path().join("access.json");
        std::fs::write(&policy,json!({"principals":[{"id":"editor","token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":[{"platform_id":"p","company_id":"a","host_id":"host","provider_config_read":true,"provider_config_write":true}]}],"hosts":[{"id":"host","online":true,"binding":"local","home":root.path(),"credential_root":private,"search_paths":[],"applications":root.path().join("Applications")}]}).to_string()).unwrap();
        std::fs::set_permissions(&policy, std::fs::Permissions::from_mode(0o600)).unwrap();
        let editor = super::editor_merge::Editor::new(Some(policy.clone()));
        let provider_routes = editor
            .clone()
            .router()
            .merge(super::operations::router_with_editor(Some(policy.clone()), editor))
            .merge(super::preview::router_with_policy(Some(policy.clone())));
        let router = Router::new()
            .nest("/v1", provider_routes)
            .layer(Extension(db.clone()))
            .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        let client =
            GatewayClient::new(GatewayEndpoint::new("127.0.0.1", port).with_tenant_scope(tenant))
                .unwrap();
        Self { root, db, client, server, policy }
    }
    fn file(&self) -> std::path::PathBuf {
        self.root.path().join(".claude/settings.json")
    }
    async fn open(&self, id: &str) -> EditorView {
        self.client
            .app_provider_editor_open(
                AppId::Claude,
                id,
                &EditorOpen { target_host_id: "host".into() },
                TOKEN,
            )
            .await
            .unwrap()
    }
    fn request(&self, view: &EditorView) -> EditorSave {
        EditorSave {
            target_host_id: "host".into(),
            context: view.context.clone(),
            edits: vec![EditorEdit {
                path: vec!["x".into()],
                value: EditorValue::Set { value: json!(2) },
            }],
            choice: EditorChoice::Refuse,
            resolution: None,
        }
    }
    async fn save(&self, id: &str, request: &EditorSave) -> EditorResult {
        self.client.app_provider_editor_save(AppId::Claude, id, request, TOKEN).await.unwrap()
    }
    fn record(&self, id: &str) -> String {
        GatewaySettingsStore::new(self.db.clone())
            .unwrap()
            .get(
                &TenantScope::new("p", "a").unwrap(),
                "app_providers.records",
                &format!("\"claude\":{id}"),
            )
            .unwrap()
            .unwrap()
    }
}
#[tokio::test]
async fn cc04_editor_merge_sdk_api_keeps_external_fields_and_requires_fresh_choice() {
    let f = Fixture::new().await;
    let view = f.open("saved").await;
    assert!(view.current);
    assert!(!serde_json::to_string(&view).unwrap().contains("fixture-secret"));
    let row = f.record("saved");
    let mut request = f.request(&view);
    std::fs::write(f.file(), br#"{"x":3,"untouched":"external-secret","z":true}"#).unwrap();
    let external = std::fs::read(f.file()).unwrap();
    let conflict = f.save("saved", &request).await;
    assert!(!conflict.saved);
    assert_eq!(conflict.conflicts, vec![vec!["x".to_string()]]);
    assert_eq!(std::fs::read(f.file()).unwrap(), external);
    request.choice = EditorChoice::KeepMine;
    request.resolution = conflict.resolution;
    // A second external edit invalidates the choice, even if the conflict key is the same.
    std::fs::write(f.file(), br#"{"x":4,"untouched":"external-secret","z":true}"#).unwrap();
    let renewed = f.save("saved", &request).await;
    assert!(!renewed.saved);
    assert_ne!(renewed.resolution, request.resolution);
    request.resolution = renewed.resolution;
    assert!(f.save("saved", &request).await.saved);
    let saved: Value = serde_json::from_slice(&std::fs::read(f.file()).unwrap()).unwrap();
    assert_eq!(saved, json!({"x":2,"untouched":"external-secret","z":true}));
    assert_eq!(row, f.record("saved"));
    assert!(
        f.client.app_provider_editor_save(AppId::Claude, "saved", &request, TOKEN).await.is_err()
    );
}
#[tokio::test]
async fn cc04_editor_merge_inactive_is_catalog_only_and_permission_denial_has_no_effect() {
    let f = Fixture::new().await;
    let view = f.open("inactive").await;
    assert!(!view.current);
    let bytes = std::fs::read(f.file()).unwrap();
    let mut req = f.request(&view);
    assert!(f.save("inactive", &req).await.saved);
    let row: Value = serde_json::from_str(&f.record("inactive")).unwrap();
    assert_eq!(row["settingsConfig"]["x"], 2);
    assert_eq!(std::fs::read(f.file()).unwrap(), bytes);
    let view = f.open("saved").await;
    req = f.request(&view);
    req.edits[0].path = vec!["env".into(), "ANTHROPIC_API_KEY".into()];
    assert!(f.client.app_provider_editor_save(AppId::Claude, "saved", &req, TOKEN).await.is_err());
    assert_eq!(std::fs::read(f.file()).unwrap(), bytes);
    req = f.request(&view);
    let mut policy: Value = serde_json::from_slice(&std::fs::read(&f.policy).unwrap()).unwrap();
    policy["principals"][0]["grants"][0]["provider_config_write"] = json!(false);
    std::fs::write(&f.policy, policy.to_string()).unwrap();
    let error =
        f.client.app_provider_editor_save(AppId::Claude, "saved", &req, TOKEN).await.unwrap_err();
    assert!(matches!(error, vw_gateway_client::AgentAppsReadError::Http { status: 403, .. }));
    assert_eq!(std::fs::read(f.file()).unwrap(), bytes);
}
#[tokio::test]
async fn cc04_editor_merge_keep_theirs_saves_only_other_user_edits() {
    let f = Fixture::new().await;
    let view = f.open("saved").await;
    let mut req = f.request(&view);
    req.edits.push(EditorEdit {
        path: vec!["user".into()],
        value: EditorValue::Set { value: json!(true) },
    });
    std::fs::write(f.file(), br#"{"x":3,"outside":true}"#).unwrap();
    let conflict = f.save("saved", &req).await;
    req.choice = EditorChoice::KeepTheirs;
    req.resolution = conflict.resolution;
    assert!(f.save("saved", &req).await.saved);
    assert_eq!(
        serde_json::from_slice::<Value>(&std::fs::read(f.file()).unwrap()).unwrap(),
        json!({"x":3,"outside":true,"user":true})
    );
}
#[tokio::test]
async fn cc04_editor_merge_rejects_wrong_binding_stale_record_and_invalid_projection() {
    let f = Fixture::new().await;
    let view = f.open("saved").await;
    let mut req = f.request(&view);
    let bytes = std::fs::read(f.file()).unwrap();
    assert!(
        f.client.app_provider_editor_save(AppId::Claude, "inactive", &req, TOKEN).await.is_err()
    );
    req.choice = EditorChoice::KeepMine;
    assert!(f.client.app_provider_editor_save(AppId::Claude, "saved", &req, TOKEN).await.is_err());
    req = f.request(&view);
    req.edits.push(EditorEdit {
        path: vec!["env".into()],
        value: EditorValue::Set { value: json!("invalid") },
    });
    assert!(f.client.app_provider_editor_save(AppId::Claude, "saved", &req, TOKEN).await.is_err());
    assert_eq!(std::fs::read(f.file()).unwrap(), bytes);
    let store = GatewaySettingsStore::new(f.db.clone()).unwrap();
    let tenant = TenantScope::new("p", "a").unwrap();
    let mut row: Value = serde_json::from_str(&f.record("saved")).unwrap();
    row["new_external"] = json!(true);
    store.upsert(&tenant, "app_providers.records", "\"claude\":saved", &row.to_string()).unwrap();
    req = f.request(&view);
    assert!(f.client.app_provider_editor_save(AppId::Claude, "saved", &req, TOKEN).await.is_err());
    assert_eq!(std::fs::read(f.file()).unwrap(), bytes);
    assert_eq!(f.record("saved"), row.to_string());
    // Invalid projection must fail at open, without returning a usable context or changing live.
    row["settingsConfig"]["apiBaseUrl"] = json!("invalid endpoint");
    store.upsert(&tenant, "app_providers.records", "\"claude\":saved", &row.to_string()).unwrap();
    assert!(
        f.client
            .app_provider_editor_open(
                AppId::Claude,
                "saved",
                &EditorOpen { target_host_id: "host".into() },
                TOKEN
            )
            .await
            .is_err()
    );
    assert_eq!(std::fs::read(f.file()).unwrap(), bytes);
}
#[tokio::test]
async fn cc04_editor_merge_symlink_and_state_change_are_rejected() {
    let f = Fixture::new().await;
    let view = f.open("saved").await;
    let req = f.request(&view);
    let before = std::fs::read(f.file()).unwrap();
    std::fs::rename(f.file(), f.root.path().join("original.json")).unwrap();
    std::os::unix::fs::symlink(f.root.path().join("original.json"), f.file()).unwrap();
    assert!(f.client.app_provider_editor_save(AppId::Claude, "saved", &req, TOKEN).await.is_err());
    assert_eq!(std::fs::read(f.file()).unwrap(), before);
    std::fs::remove_file(f.file()).unwrap();
    std::fs::rename(f.root.path().join("original.json"), f.file()).unwrap();
    let state_path = f.root.path().join("private/provider-live/aG9zdA/claude.state.json");
    let changed = AppState::default();
    std::fs::write(&state_path, serde_json::to_vec(&changed).unwrap()).unwrap();
    assert!(f.client.app_provider_editor_save(AppId::Claude, "saved", &req, TOKEN).await.is_err());
    assert_eq!(std::fs::read(f.file()).unwrap(), before);
    // A never-activated catalog does not provision transaction state or touch live.
    std::fs::remove_dir_all(f.root.path().join("private/provider-live")).unwrap();
    let inactive = f.open("inactive").await;
    assert!(f.save("inactive", &f.request(&inactive)).await.saved);
    assert!(!f.root.path().join("private/provider-live").exists());
    assert_eq!(std::fs::read(f.file()).unwrap(), before);
}
#[tokio::test]
async fn cc04_editor_merge_registered_route_requires_gateway_authentication() {
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;
    let app = crate::gateway::router::handler_router()
        .layer(Extension(TenantScope::new("p", "a").unwrap()))
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
                .uri("/app-providers/claude/saved/editor/open")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"targetHostId":"host"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 401);
}

use vw_api_types::{
    app_providers::{operations::*, preview::SavedProviderPreviewRequest},
    task::TaskStatus,
};

async fn wait_for_progress(
    f: &Fixture,
    id: &str,
    ready: impl Fn(&ProviderOperationProgress) -> bool,
) -> ProviderOperationProgress {
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let progress = f.client.provider_operation_get(id, "host", TOKEN).await.unwrap();
            if ready(&progress) {
                break progress;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("editor operation did not reach expected state")
}

fn operation(request_id: &str, input: EditorSave) -> ProviderOperationRequest {
    ProviderOperationRequest {
        target_host_id: "host".into(),
        request_id: request_id.into(),
        action: ProviderOperationAction::EditorSave {
            app: AppId::Claude,
            id: "saved".into(),
            input,
        },
    }
}

#[tokio::test]
async fn cc04_ui_preview_editor_operation_keeps_file_when_cancelled_and_saves_after_retry() {
    let f = Fixture::new().await;
    let view = f.open("saved").await;
    let file = std::fs::read(f.file()).unwrap();
    let hold = vw_agent_apps::providers::editor_merge::ClaudeFile::open(f.root.path()).unwrap();
    let request = operation("cancel-fixture", f.request(&view));
    let accepted = f.client.provider_operation_start(&request, TOKEN).await.unwrap();
    assert_eq!(accepted.request_id, "cancel-fixture");
    let id = accepted.operation_id.as_ref();
    let running = wait_for_progress(&f, id, |p| p.status == TaskStatus::Running).await;
    assert_eq!(running.phase, ProviderOperationPhase::Preparing);
    let same = f.client.provider_operation_start(&request, TOKEN).await.unwrap();
    assert_eq!(same.operation_id, accepted.operation_id);
    let cancelled = f.client.provider_operation_cancel(id, "host", TOKEN).await.unwrap();
    assert!(cancelled.cancellation_requested);
    assert_eq!(std::fs::read(f.file()).unwrap(), file);
    drop(hold);
    let terminal = wait_for_progress(&f, id, |p| p.phase == ProviderOperationPhase::Terminal).await;
    assert_eq!(terminal.status, TaskStatus::Cancelled);
    assert!(terminal.editor_outcome.is_none());
    assert_eq!(std::fs::read(f.file()).unwrap(), file);

    let retry = operation("retry-fixture", f.request(&view));
    let denied = f.client.provider_operation_start(&retry, "bad-token").await.unwrap_err();
    assert!(matches!(
        denied,
        vw_gateway_client::AgentAppsReadError::Http { status: 401 | 403, .. }
    ));
    assert_eq!(std::fs::read(f.file()).unwrap(), file);
    let mut external: Value = serde_json::from_slice(&file).unwrap();
    external["x"] = 3.into();
    let external = serde_json::to_vec(&external).unwrap();
    std::fs::write(f.file(), &external).unwrap();
    let accepted = f.client.provider_operation_start(&retry, TOKEN).await.unwrap();
    let terminal = wait_for_progress(&f, accepted.operation_id.as_ref(), |p| {
        p.phase == ProviderOperationPhase::Terminal
    })
    .await;
    assert_eq!(terminal.status, TaskStatus::Completed);
    let Some(ProviderEditorOutcome::Reviewed {
        result: EditorResult { saved: false, conflicts, resolution: Some(resolution) },
    }) = terminal.editor_outcome
    else {
        panic!("expected explicit conflict choice")
    };
    assert_eq!(conflicts, vec![vec!["x".to_owned()]]);
    assert_eq!(std::fs::read(f.file()).unwrap(), external);

    let mut input = f.request(&view);
    input.choice = EditorChoice::KeepMine;
    input.resolution = Some(resolution);
    let accepted = f
        .client
        .provider_operation_start(&operation("choose-fixture", input.clone()), TOKEN)
        .await
        .unwrap();
    let terminal = wait_for_progress(&f, accepted.operation_id.as_ref(), |p| {
        p.phase == ProviderOperationPhase::Terminal
    })
    .await;
    assert_eq!(terminal.status, TaskStatus::Completed);
    assert!(matches!(
        terminal.editor_outcome,
        Some(ProviderEditorOutcome::Reviewed { result: EditorResult { saved: true, .. } })
    ));
    let accepted =
        f.client.provider_operation_start(&operation("stale-fixture", input), TOKEN).await.unwrap();
    let terminal = wait_for_progress(&f, accepted.operation_id.as_ref(), |p| {
        p.phase == ProviderOperationPhase::Terminal
    })
    .await;
    assert_eq!(terminal.status, TaskStatus::Failed);
    assert_eq!(terminal.editor_outcome, Some(ProviderEditorOutcome::Rejected { status: 409 }));
    let saved: Value = serde_json::from_slice(&std::fs::read(f.file()).unwrap()).unwrap();
    assert_eq!(saved["x"], 2);
    assert_eq!(saved["untouched"], "fixture-secret");
}

#[tokio::test]
async fn cc04_ui_preview_saved_fallback_is_redacted_authorized_and_read_only() {
    let f = Fixture::new().await;
    let corrupt = b"broken native configuration";
    std::fs::write(f.file(), corrupt).unwrap();
    let request =
        SavedProviderPreviewRequest { target_host_id: "host".into(), provider_id: "saved".into() };
    let before = f.record("saved");
    let fallback =
        f.client.app_provider_saved_preview(AppId::Claude, &request, TOKEN).await.unwrap();
    assert_eq!(fallback.settings["catalog"], true);
    assert!(!fallback.settings.to_string().contains("fixture-key"));
    let denied = f
        .client
        .app_provider_saved_preview(AppId::Claude, &request, "bad-token")
        .await
        .unwrap_err();
    assert!(matches!(
        denied,
        vw_gateway_client::AgentAppsReadError::Http { status: 401 | 403, .. }
    ));
    assert_eq!(f.record("saved"), before);
    assert_eq!(std::fs::read(f.file()).unwrap(), corrupt);
}
