use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
use axum::{
    Extension, Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
    middleware,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{os::unix::fs::PermissionsExt, path::Path};
use tower::ServiceExt;
use vw_agent_apps::providers::{
    live::{commit::*, write_guard::lock_app},
    mode_state::StackState,
};
use vw_api_types::{
    agent_apps::{AppId, aggregation::*},
    id::{COMPANY_ID_HEADER, PLATFORM_ID_HEADER, ProjectId, ProjectScope, TenantScope},
};
const TOKEN: &str = "aggregation-t39-local-test-bearer-token-32-bytes";
fn tenant() -> TenantScope {
    TenantScope::new("platform", "company").unwrap()
}
fn scope(project: &str) -> ProjectScope {
    ProjectScope::new(tenant(), ProjectId::from(project)).unwrap()
}
struct Fixture {
    root: tempfile::TempDir,
    db: GlobalDatabase,
    tx: ProviderTransactions,
    policy: std::path::PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let private = root.path().join("private");
        std::fs::create_dir(&private).unwrap();
        std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o700)).unwrap();
        let db = GlobalDatabase::for_home(root.path()).unwrap();
        let store = GatewaySettingsStore::new(db.clone()).unwrap();
        for (id, model) in [("alpha", "first"), ("beta", "second")] {
            store.upsert(&tenant(),"app_providers.records",&format!("\"claude\":{id}"),&json!({"name":id,"category":"custom",
                "settingsConfig":{"env":{"ANTHROPIC_BASE_URL":"https://fixture.invalid","ANTHROPIC_API_KEY":"synthetic-private-key","ANTHROPIC_MODEL":model}}}).to_string()).unwrap();
        }
        let tx =
            ProviderTransactions::new(&private, ExecutionHostId::new("host").unwrap()).unwrap();
        let mut guard = lock_app(AppId::Claude, root.path()).unwrap();
        tx.commit_project_checked(
            &scope("project"),
            &mut guard,
            &tx.state(AppId::Claude).unwrap(),
            true,
            &[],
            PendingTarget {
                pointer: Some("alpha".into()),
                mode: Some(ModeState {
                    mode: Mode::Direct,
                    attached: false,
                    proxy_route: Some("alpha".into()),
                    contract: None,
                }),
                stack: Some(StackState {
                    enabled: false,
                    members: vec!["alpha".into(), "beta".into()],
                    keys: [("alpha".into(), "alpha".into()), ("beta".into(), "beta".into())].into(),
                }),
                ..Default::default()
            },
            || Ok(()),
        )
        .unwrap();
        let policy = root.path().join("access.json");
        let f = Self { root, db, tx, policy };
        f.policy(true, true);
        f
    }
    fn policy(&self, read: bool, aggregation: bool) {
        let apps = if aggregation {
            json!([{"project_id":"project","app_id":"claude"},{"project_id":"other","app_id":"claude"},{"project_id":"project","app_id":"codex"},{ "project_id":"project", "app_id":"claude-desktop" },{"project_id":"project","app_id":"pi"}])
        } else {
            json!([])
        };
        std::fs::write(&self.policy,json!({"principals":[{"id":"reader","token_sha256":hex::encode(Sha256::digest(TOKEN)),
            "grants":[{"platform_id":"platform","company_id":"company","host_id":"host","provider_config_read":read,"aggregation_read":apps}]}],
            "hosts":[{"id":"host","online":true,"binding":"local","home":self.root.path(),"credential_root":self.root.path().join("private"),"search_paths":[],"applications":self.root.path().join("Applications")}]}).to_string()).unwrap();
        std::fs::set_permissions(&self.policy, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    fn router(&self) -> Router {
        super::aggregation_read::router_with_policy(Some(self.policy.clone()))
            .layer(Extension(self.db.clone()))
            .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware))
    }
    async fn request(
        &self,
        preview: bool,
        host: &str,
        project: &str,
        token: Option<&str>,
        extra: Value,
    ) -> (StatusCode, Value) {
        let mut builder = Request::builder()
            .method(if preview { "POST" } else { "GET" })
            .uri(if preview {
                "/agent-apps/claude/aggregation/preview".into()
            } else {
                format!(
                    "/agent-apps/claude/aggregation/state?targetHostId={host}&projectId={project}"
                )
            })
            .header(PLATFORM_ID_HEADER, "platform")
            .header(COMPANY_ID_HEADER, "company");
        if let Some(token) = token {
            builder = builder.header("authorization", format!("Bearer {token}"));
        }
        let body = if preview {
            let mut value = json!({"targetHostId":host,"projectId":project});
            for (key, val) in extra.as_object().unwrap() {
                value[key] = val.clone();
            }
            builder = builder.header("content-type", "application/json");
            Body::from(value.to_string())
        } else {
            Body::empty()
        };
        let response = self.router().oneshot(builder.body(body).unwrap()).await.unwrap();
        assert_eq!(response.headers().get("cache-control").unwrap(), "no-store");
        let status = response.status();
        let bytes = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
        let value: Value = serde_json::from_slice(&bytes).unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains("synthetic-private-key"));
        (status, value)
    }
    fn state_path(&self) -> std::path::PathBuf {
        self.root.path().join("private/provider-live/aG9zdA/claude.state.json")
    }
}
// SQLite SHM coordinates read locks, and is not durable application state.
// Keep its presence in the snapshot; compare every other file by content digest.
fn files(path: &Path) -> Vec<(std::path::PathBuf, String)> {
    let mut result = vec![];
    for entry in std::fs::read_dir(path).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            result.extend(files(&path));
        } else {
            let digest = if path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".sqlite3-shm"))
            {
                "sqlite-read-locks".into()
            } else {
                hex::encode(Sha256::digest(std::fs::read(&path).unwrap()))
            };
            result.push((path, digest));
        }
    }
    result.sort();
    result
}
#[tokio::test]
async fn aggregation_t39_aggregation_read_preserves_contract() {
    let f = Fixture::new();
    let before = files(f.root.path());
    let state = f.tx.state(AppId::Claude).unwrap();
    let (status, value) = f.request(false, "host", "project", Some(TOKEN), json!({})).await;
    assert_eq!(status, 200, "{value}");
    let current: AggregationState = serde_json::from_value(value).unwrap();
    assert_eq!(current.mode, AggregationMode::Direct);
    assert!(!current.attached);
    assert_eq!(current.default_provider_id.as_deref(), Some("alpha"));
    assert_eq!(current.direct_provider_id.as_deref(), Some("alpha"));
    assert!(current.members.iter().all(|member| member.models.is_empty()));
    assert_eq!(current.members.len(), 2);
    let (status, value) =
        f.request(true, "host", "project", Some(TOKEN), json!({"defaultProviderId":"beta"})).await;
    assert_eq!(status, 200, "{value}");
    let preview: AggregationPreview = serde_json::from_value(value).unwrap();
    assert_eq!(preview.default_provider_id.as_deref(), Some("beta"));
    assert_eq!(preview.members[0].provider_id, "beta");
    assert_eq!(preview.members[1].provider_id, "alpha");
    assert!(!preview.members[0].models.is_empty());
    assert!(preview.issues.is_empty());
    assert!(preview.preview_token.is_some());
    assert_eq!(current.revision, preview.revision);
    let token = preview.preview_token.unwrap();
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    let (payload, signature) = token.split_once('.').unwrap();
    assert_eq!(signature.len(), 64);
    let payload: Value = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(payload).unwrap()).unwrap();
    assert_eq!(payload["principal"], "reader");
    assert_eq!(payload["revision"], current.revision);
    assert_eq!(payload["defaultProviderId"], "beta");
    assert_eq!(payload["catalogPolicy"], "preserve");
    assert!(payload["expiresAt"].as_i64().unwrap() > chrono::Utc::now().timestamp());
    assert_eq!(files(f.root.path()), before);
    assert!(f.tx.state(AppId::Claude).unwrap() == state);
    assert!(f.tx.pending(AppId::Claude).unwrap().is_none());
}
#[tokio::test]
async fn aggregation_t39_denies_missing_credentials_host_and_real_foreign_project() {
    let f = Fixture::new();
    let before = files(f.root.path());
    for preview in [false, true] {
        for (host, project, token, expected) in [
            ("host", "project", None, 401),
            ("other", "project", Some(TOKEN), 403),
            ("host", "ungranted", Some(TOKEN), 403),
            ("host", "other", Some(TOKEN), 403),
            ("host", "project", Some("invalid-bearer"), 401),
        ] {
            let (status, _) = f.request(preview, host, project, token, json!({})).await;
            assert_eq!(status, expected);
        }
    }
    assert_eq!(files(f.root.path()), before);
    for (read, aggregation) in [(false, true), (true, false)] {
        f.policy(read, aggregation);
        assert_eq!(f.request(false, "host", "project", Some(TOKEN), json!({})).await.0, 403);
    }
}
#[tokio::test]
async fn aggregation_t39_unclaimed_state_and_pending_are_conflicts_without_recovery() {
    let f = Fixture::new();
    let before = std::fs::read(f.state_path()).unwrap();
    let mut state: Value = serde_json::from_slice(&before).unwrap();
    state.as_object_mut().unwrap().remove("project_scope");
    std::fs::write(f.state_path(), state.to_string()).unwrap();
    let snapshot = files(f.root.path());
    for preview in [false, true] {
        assert_eq!(f.request(preview, "host", "project", Some(TOKEN), json!({})).await.0, 409);
    }
    assert_eq!(files(f.root.path()), snapshot);
    std::fs::write(f.state_path(), before).unwrap();
    // Leave a real staged intent by rejecting the second validation, after
    // durable intent creation and before any native publication.
    use vw_agent_apps::providers::live::{
        stage::FileChange,
        write_guard::{FilePrivacy, WriteError},
    };
    let mut guard = lock_app(AppId::Claude, f.root.path()).unwrap();
    let patch = |_: Option<&[u8]>| Ok(b"planned".to_vec());
    let changes = [FileChange {
        name: std::ffi::OsStr::new("native-staged"),
        privacy: FilePrivacy::Private,
        patch: &patch,
    }];
    let calls = std::cell::Cell::new(0);
    assert_eq!(
        f.tx.commit_project_checked(
            &scope("project"),
            &mut guard,
            &f.tx.state(AppId::Claude).unwrap(),
            false,
            &changes,
            PendingTarget::default(),
            || {
                calls.set(calls.get() + 1);
                if calls.get() == 2 { Err(WriteError::Conflict) } else { Ok(()) }
            }
        ),
        Err(WriteError::Conflict)
    );
    drop(guard);
    let pending = f.state_path().with_file_name("claude.pending.json");
    assert!(f.tx.pending(AppId::Claude).unwrap().is_some());
    let snapshot = files(f.root.path());
    for preview in [false, true] {
        assert_eq!(f.request(preview, "host", "project", Some(TOKEN), json!({})).await.0, 409);
    }
    assert_eq!(files(f.root.path()), snapshot);
    assert!(pending.exists());
}
#[tokio::test]
async fn aggregation_t39_preview_blocks_invalid_default_and_failover_without_issuing_token() {
    let f = Fixture::new();
    let (_, value) = f
        .request(true, "host", "project", Some(TOKEN), json!({"defaultProviderId":"missing"}))
        .await;
    let preview: AggregationPreview = serde_json::from_value(value).unwrap();
    assert!(preview.preview_token.is_none());
    assert!(
        preview
            .issues
            .iter()
            .any(|issue| issue.code == PreviewIssueCode::InvalidDefault && issue.blocking)
    );
    let store = GatewaySettingsStore::new(f.db.clone()).unwrap();
    let key =
        serde_json::to_string(&(scope("project").project_id(), "host", AppId::Claude)).unwrap();
    store
        .upsert(
            &tenant(),
            "agent_apps.high_availability",
            &key,
            &json!({"version":1,"enabled":true,"members":[]}).to_string(),
        )
        .unwrap();
    let before = files(f.root.path());
    let (status, value) = f.request(true, "host", "project", Some(TOKEN), json!({})).await;
    assert_eq!(status, 200);
    let preview: AggregationPreview = serde_json::from_value(value).unwrap();
    assert!(preview.preview_token.is_none());
    assert!(preview.issues.iter().any(|issue| issue.code == PreviewIssueCode::FailoverEnabled));
    assert_eq!(files(f.root.path()), before);
}
#[tokio::test]
async fn aggregation_t39_revision_changes_with_native_inputs_and_rejects_unsafe_paths() {
    let f = Fixture::new();
    let (status, first) = f.request(false, "host", "project", Some(TOKEN), json!({})).await;
    assert_eq!(status, 200);
    std::fs::create_dir(f.root.path().join(".claude")).unwrap();
    let settings = f.root.path().join(".claude/settings.json");
    std::fs::write(
        &settings,
        br#"{ // preserved comment
"user": true,
}"#,
    )
    .unwrap();
    let (status, second) = f.request(false, "host", "project", Some(TOKEN), json!({})).await;
    assert_eq!(status, 200);
    assert_ne!(first["revision"], second["revision"]);
    assert_eq!(
        f.request(false, "host", "project", Some(TOKEN), json!({})).await.1["revision"],
        second["revision"]
    );
    std::fs::remove_file(&settings).unwrap();
    std::os::unix::fs::symlink(f.root.path().join("access.json"), &settings).unwrap();
    assert_eq!(f.request(true, "host", "project", Some(TOKEN), json!({})).await.0, 422);
}
#[tokio::test]
async fn aggregation_t39_rejects_unknown_preview_fields_and_non_codex_catalog_policy() {
    let f = Fixture::new();
    let before = files(f.root.path());
    for body in [
        json!({"path":"untrusted"}),
        json!({"catalogPolicy":"use_managed"}),
        json!({"catalogPolicy":"unexpected"}),
    ] {
        assert_eq!(f.request(true, "host", "project", Some(TOKEN), body).await.0, 400);
    }
    assert_eq!(files(f.root.path()), before);
}

#[tokio::test]
async fn aggregation_t39_codex_preview_uses_saved_profile_without_publishing_catalog() {
    use vw_api_types::app_providers::codex_catalog::CodexProfile;
    let f = Fixture::new();
    let profile = CodexProfile { name: "Codex relay".into(), config: "model='main'\nmodel_provider='relay'\n[model_providers.relay]\nbase_url='https://fixture.invalid/v1'\nwire_api='responses'\n".into(), api_key: Some("synthetic-private-key".into()), preserve_login: false };
    crate::app_providers::AppProviderStore::new(f.db.clone())
        .unwrap()
        .codex_save(&tenant(), "alpha", &profile, None, None)
        .unwrap();
    let mut guard = lock_app(AppId::Codex, f.root.path()).unwrap();
    f.tx.commit_project_checked(
        &scope("project"),
        &mut guard,
        &f.tx.state(AppId::Codex).unwrap(),
        true,
        &[],
        PendingTarget {
            pointer: Some("alpha".into()),
            mode: Some(ModeState {
                mode: Mode::Direct,
                attached: false,
                proxy_route: Some("alpha".into()),
                contract: None,
            }),
            stack: Some(StackState {
                enabled: false,
                members: vec!["alpha".into()],
                keys: [("alpha".into(), "alpha".into())].into(),
            }),
            ..Default::default()
        },
        || Ok(()),
    )
    .unwrap();
    drop(guard);
    let before = files(f.root.path());
    let request = Request::builder()
        .method("POST")
        .uri("/agent-apps/codex/aggregation/preview")
        .header(PLATFORM_ID_HEADER, "platform")
        .header(COMPANY_ID_HEADER, "company")
        .header("authorization", format!("Bearer {TOKEN}"))
        .header("content-type", "application/json")
        .body(Body::from(json!({"targetHostId":"host","projectId":"project"}).to_string()))
        .unwrap();
    let response = f.router().oneshot(request).await.unwrap();
    assert_eq!(response.status(), 200);
    let value: AggregationPreview =
        serde_json::from_slice(&to_bytes(response.into_body(), 64 * 1024).await.unwrap()).unwrap();
    assert!(value.issues.is_empty(), "{:?}", value.issues);
    assert!(value.preview_token.is_some());
    assert_eq!(value.members[0].models[0].id, "main");
    assert!(value.notices.contains(&AggregationNotice::CodexRouteOwnsCatalog));
    assert_eq!(files(f.root.path()), before);
    assert!(!f.root.path().join(".codex").exists());
}

#[tokio::test]
async fn aggregation_t39_foreign_tenant_state_is_forbidden_before_preview() {
    let f = Fixture::new();
    let mut state: Value = serde_json::from_slice(&std::fs::read(f.state_path()).unwrap()).unwrap();
    state["stack_tenant"] =
        serde_json::to_value(TenantScope::new("platform", "other-company").unwrap()).unwrap();
    std::fs::write(f.state_path(), state.to_string()).unwrap();
    let before = files(f.root.path());
    for preview in [false, true] {
        assert_eq!(f.request(preview, "host", "project", Some(TOKEN), json!({})).await.0, 403);
    }
    assert_eq!(files(f.root.path()), before);
}

#[tokio::test]
async fn aggregation_t39_corrupt_native_configuration_cannot_receive_preview_token() {
    let f = Fixture::new();
    std::fs::create_dir(f.root.path().join(".claude")).unwrap();
    std::fs::write(f.root.path().join(".claude/settings.json"), b"{broken-json").unwrap();
    let before = files(f.root.path());
    let (status, value) = f.request(true, "host", "project", Some(TOKEN), json!({})).await;
    assert_eq!(status, 422, "corrupt native configuration must be rejected before minting a token");
    assert_eq!(value["error"]["code"], "invalid_configuration");
    assert_eq!(files(f.root.path()), before);
}

#[tokio::test]
async fn aggregation_t39_desktop_preview_uses_native_mapping_without_creating_profile() {
    for (model, eligible) in [("claude-sonnet-5", false), ("vendor-model", true)] {
        let f = Fixture::new();
        let source = json!({"name":"Desktop relay","category":"custom","settingsConfig":{"env":{
            "ANTHROPIC_BASE_URL":"https://fixture.invalid","ANTHROPIC_AUTH_TOKEN":"synthetic-private-key","ANTHROPIC_MODEL":model}}});
        let row = vw_agent_apps::providers::desktop_mapping::import_row(&source).unwrap().unwrap();
        GatewaySettingsStore::new(f.db.clone())
            .unwrap()
            .upsert(
                &tenant(),
                "app_providers.records",
                "\"claude-desktop\":alpha",
                &row.to_string(),
            )
            .unwrap();
        let mut guard = lock_app(AppId::ClaudeDesktop, f.root.path()).unwrap();
        f.tx.commit_project_checked(
            &scope("project"),
            &mut guard,
            &f.tx.state(AppId::ClaudeDesktop).unwrap(),
            true,
            &[],
            PendingTarget {
                pointer: Some("alpha".into()),
                mode: Some(ModeState {
                    mode: Mode::Direct,
                    attached: false,
                    proxy_route: Some("alpha".into()),
                    contract: None,
                }),
                stack: Some(StackState {
                    enabled: false,
                    members: vec!["alpha".into()],
                    keys: [("alpha".into(), "alpha".into())].into(),
                }),
                ..Default::default()
            },
            || Ok(()),
        )
        .unwrap();
        drop(guard);
        let before = files(f.root.path());
        let request = Request::builder()
            .method("POST")
            .uri("/agent-apps/claude-desktop/aggregation/preview")
            .header(PLATFORM_ID_HEADER, "platform")
            .header(COMPANY_ID_HEADER, "company")
            .header("authorization", format!("Bearer {TOKEN}"))
            .header("content-type", "application/json")
            .body(Body::from(json!({"targetHostId":"host","projectId":"project"}).to_string()))
            .unwrap();
        let response = f.router().oneshot(request).await.unwrap();
        assert_eq!(response.status(), 200);
        let bytes = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains("synthetic-private-key"));
        let value: AggregationPreview = serde_json::from_slice(&bytes).unwrap();
        if eligible {
            assert!(value.issues.is_empty(), "{:?}", value.issues);
            assert!(value.preview_token.is_some());
            assert!(!value.members[0].models.is_empty());
        } else {
            assert!(value.preview_token.is_none());
            assert!(
                value
                    .issues
                    .iter()
                    .any(|issue| issue.code == PreviewIssueCode::InvalidDefault && issue.blocking)
            );
        }
        assert_eq!(files(f.root.path()), before);
        assert!(!f.root.path().join("Library").exists());
    }
}
