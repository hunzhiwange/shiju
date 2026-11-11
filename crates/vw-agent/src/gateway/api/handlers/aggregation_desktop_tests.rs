use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
use axum::{
    Extension, Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
    middleware,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, os::unix::fs::PermissionsExt};
use tower::ServiceExt;
use vw_agent_apps::providers::{claude_desktop::ClaudeDesktopFiles, live::commit::*};
use vw_api_types::{
    agent_apps::AppId,
    id::{COMPANY_ID_HEADER, PLATFORM_ID_HEADER, ProjectScope, TenantScope},
};
const TOKEN: &str = "fixture-desktop-manager-token-longer-than-32";
struct Fixture {
    root: tempfile::TempDir,
    db: GlobalDatabase,
    tx: ProviderTransactions,
    policy: std::path::PathBuf,
}
fn tenant() -> TenantScope {
    TenantScope::new("p", "c").unwrap()
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        fs::create_dir(root.path().join("private")).unwrap();
        fs::set_permissions(root.path().join("private"), fs::Permissions::from_mode(0o700))
            .unwrap();
        fs::create_dir_all(root.path().join("Applications/Claude.app")).unwrap();
        let tx = ProviderTransactions::new(
            &root.path().join("private"),
            ExecutionHostId::new("host").unwrap(),
        )
        .unwrap();
        let mut files = ClaudeDesktopFiles::open(root.path()).unwrap();
        tx.commit_project_checked(
            &ProjectScope::new(tenant(), "project".into()).unwrap(),
            &mut files.guard,
            &tx.state(AppId::ClaudeDesktop).unwrap(),
            true,
            &[],
            PendingTarget::default(),
            || Ok(()),
        )
        .unwrap();
        drop(files);
        let db = GlobalDatabase::for_home(root.path()).unwrap();
        GatewaySettingsStore::new(db.clone()).unwrap().upsert(&tenant(),"app_providers.records","\"claude-desktop\":alpha",&json!({"name":"Alpha","category":"custom","settingsConfig":{"env":{"ANTHROPIC_BASE_URL":"https://fixture.invalid","ANTHROPIC_AUTH_TOKEN":"synthetic-private-token"}},"meta":{"claudeDesktopMode":"proxy","claudeDesktopModelRoutes":{"claude-sonnet-5":{"model":"model-one"}}}}).to_string()).unwrap();
        let policy = root.path().join("access.json");
        fs::write(&policy,json!({"principals":[{"id":"manager","token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":[{"platform_id":"p","company_id":"c","host_id":"host","provider_config_read":true,"provider_config_write":true,"aggregation_read":[{"project_id":"project","app_id":"claude-desktop"},{"project_id":"other","app_id":"claude-desktop"}],"aggregation_write":[{"project_id":"project","app_id":"claude-desktop"}]}]}],"hosts":[{"id":"host","online":true,"binding":"local","home":root.path(),"credential_root":root.path().join("private"),"search_paths":[],"applications":root.path().join("SystemApplications")}]}).to_string()).unwrap();
        fs::set_permissions(&policy, fs::Permissions::from_mode(0o600)).unwrap();
        Self { root, db, tx, policy }
    }
    fn router(&self) -> Router {
        super::app_providers::desktop_mapping::router_with_policy(Some(self.policy.clone()))
            .merge(super::aggregation_desktop::router_with_policy(Some(self.policy.clone())))
            .layer(Extension(self.db.clone()))
            .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware))
    }
    async fn request(
        &self,
        method: &str,
        path: &str,
        body: Value,
        authorized: bool,
    ) -> (StatusCode, Value) {
        let mut req = Request::builder()
            .method(method)
            .uri(path)
            .header(PLATFORM_ID_HEADER, "p")
            .header(COMPANY_ID_HEADER, "c")
            .header("content-type", "application/json");
        if authorized {
            req = req.header("authorization", format!("Bearer {TOKEN}"));
        }
        let response =
            self.router().oneshot(req.body(Body::from(body.to_string())).unwrap()).await.unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), 256 * 1024).await.unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains("synthetic-private-token"));
        (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
    }
}
#[tokio::test]
async fn aggregation_t46_aggregation_desktop_preserves_contract() {
    let f = Fixture::new();
    let (status, view) = f
        .request(
            "POST",
            "/app-providers/claude-desktop/alpha/form/read",
            json!({"targetHostId":"host","projectId":"project"}),
            true,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{view}");
    assert!(view["revision"].as_str().is_some_and(|s| !s.is_empty()));
    assert_eq!(view["form"]["mapping"]["routes"]["claude-sonnet-5"]["model"], "model-one");
    let before = f.tx.state(AppId::ClaudeDesktop).unwrap();
    let (status, profile) = f
        .request(
            "GET",
            "/agent-apps/claude-desktop/profile/status?targetHostId=host&projectId=project",
            Value::Null,
            true,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{profile}");
    assert_eq!(profile["installation"], "installed");
    assert_eq!(profile["mode"], "official");
    assert_eq!(profile["recovery"], "none");
    assert!(f.tx.state(AppId::ClaudeDesktop).unwrap() == before);
    assert!(
        !f.root
            .path()
            .join("Library/Application Support/Claude/claude_desktop_config.json")
            .exists()
    );
}

#[path = "aggregation_desktop_routing_tests.rs"]
mod routing_tests;

#[tokio::test]
async fn aggregation_t46_mapping_revisions_permissions_and_installation_are_enforced() {
    let f = Fixture::new();
    let form_path = "/app-providers/claude-desktop/alpha/form/read";
    let mapping_path = "/app-providers/claude-desktop/alpha/model-mapping";
    let query = json!({"targetHostId":"host","projectId":"project"});
    assert_eq!(
        f.request("POST", form_path, query.clone(), false).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        f.request("POST", form_path, json!({"targetHostId":"host","projectId":"other"}), true)
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        f.request("POST", form_path, json!({"targetHostId":"foreign","projectId":"project"}), true)
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    let (status, view) = f.request("POST", form_path, query.clone(), true).await;
    assert_eq!(status, 200, "{view}");
    let settings = GatewaySettingsStore::new(f.db.clone()).unwrap();
    let before =
        settings.get(&tenant(), "app_providers.records", "\"claude-desktop\":alpha").unwrap();
    let mapping = json!({"mode":"proxy","routes":{"claude-sonnet-5":{"model":"model-two","labelOverride":"Updated model"}}});
    let change = json!({"targetHostId":"host","projectId":"project","expectedRevision":view["revision"],"mapping":mapping});
    let mut stale = change.clone();
    stale["expectedRevision"] = json!("stale");
    assert_eq!(f.request("PUT", mapping_path, stale, true).await.0, StatusCode::CONFLICT);
    let mut invalid = change.clone();
    invalid["mapping"]["routes"]["claude-sonnet-5"]["model"] = json!("bad model");
    assert_eq!(
        f.request("PUT", mapping_path, invalid, true).await.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let mut unknown = change.clone();
    unknown["path"] = json!("/outside");
    assert_eq!(f.request("PUT", mapping_path, unknown, true).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(
        settings.get(&tenant(), "app_providers.records", "\"claude-desktop\":alpha").unwrap(),
        before
    );
    let (status, saved) = f.request("PUT", mapping_path, change.clone(), true).await;
    assert_eq!(status, 200, "{saved}");
    assert_eq!(saved["form"]["mapping"]["routes"]["claude-sonnet-5"]["model"], "model-two");
    assert_ne!(saved["revision"], view["revision"]);
    let row: Value = serde_json::from_str(
        &settings
            .get(&tenant(), "app_providers.records", "\"claude-desktop\":alpha")
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(row["meta"]["claudeDesktopModelRoutes"]["claude-sonnet-5"]["model"], "model-two");
    assert!(f.tx.pending(AppId::ClaudeDesktop).unwrap().is_none());
    assert_eq!(f.request("PUT", mapping_path, change, true).await.0, StatusCode::CONFLICT);
    let routes=(0..200).map(|index|(format!("claude-sonnet-5-{index:03}"),json!({"model":format!("{}{index:03}","m".repeat(250)),"labelOverride":"L".repeat(256)}))).collect::<serde_json::Map<_,_>>();
    let large = json!({"targetHostId":"host","projectId":"project","expectedRevision":saved["revision"],"mapping":{"mode":"proxy","routes":routes}});
    assert!(serde_json::to_vec(&large).unwrap().len() > 64 * 1024);
    let (status, large_saved) = f.request("PUT", mapping_path, large, true).await;
    assert_eq!(status, 200, "{large_saved}");
    assert_eq!(large_saved["form"]["mapping"]["routes"].as_object().unwrap().len(), 200);
    assert!(
        !f.root
            .path()
            .join("Library/Application Support/Claude/claude_desktop_config.json")
            .exists()
    );
    fs::remove_dir(f.root.path().join("Applications/Claude.app")).unwrap();
    let (status, profile) = f
        .request(
            "GET",
            "/agent-apps/claude-desktop/profile/status?targetHostId=host&projectId=project",
            Value::Null,
            true,
        )
        .await;
    assert_eq!(status, 200);
    assert_eq!(profile["installation"], "not_installed");
    let (status,_)=f.request("PUT",mapping_path,json!({"targetHostId":"host","projectId":"project","expectedRevision":saved["revision"],"mapping":mapping}),true).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        !f.root
            .path()
            .join("Library/Application Support/Claude/claude_desktop_config.json")
            .exists()
    );
}

#[tokio::test]
async fn aggregation_t46_mapping_retry_recovers_only_the_owned_durable_record() {
    use vw_agent_apps::providers::live::{
        record_publication::{ProviderRecordChange, ProviderRecordParticipant},
        write_guard::{WriteError, lock_app},
    };
    let f = Fixture::new();
    let store = crate::app_providers::AppProviderStore::new(f.db.clone()).unwrap();
    struct FailFinish<'a>(&'a crate::app_providers::AppProviderStore);
    impl ProviderRecordParticipant for FailFinish<'_> {
        fn reserve(&self, change: &ProviderRecordChange) -> Result<(), WriteError> {
            self.0.reserve(change)
        }
        fn finish(&self, _: &ProviderRecordChange, _: bool) -> Result<(), WriteError> {
            Err(WriteError::Io)
        }
    }
    let before = store.publication_record(&tenant(), AppId::ClaudeDesktop, "alpha").unwrap();
    let mut after: Value = serde_json::from_str(&before).unwrap();
    after["meta"]["claudeDesktopModelRoutes"]["claude-sonnet-5"]["model"] =
        json!("recovered-model");
    let project = ProjectScope::new(tenant(), "project".into()).unwrap();
    let change = ProviderRecordChange {
        operation_id: "fixture-storage-failure".into(),
        host: ExecutionHostId::new("host").unwrap(),
        project: project.clone(),
        app: AppId::ClaudeDesktop,
        provider_id: "alpha".into(),
        before,
        after: after.to_string(),
        dependencies: vec![],
    };
    let mut guard = lock_app(AppId::ClaudeDesktop, f.root.path()).unwrap();
    let error = f.tx.commit_project_record(
        &project,
        &mut guard,
        &f.tx.state(AppId::ClaudeDesktop).unwrap(),
        &[],
        PendingTarget::default(),
        &change,
        &FailFinish(&store),
        || Ok(()),
    );
    assert!(matches!(error, Err(WriteError::Io)));
    drop(guard);
    assert!(f.tx.pending(AppId::ClaudeDesktop).unwrap().is_some_and(|pending| pending.published));
    let (status, profile) = f
        .request(
            "GET",
            "/agent-apps/claude-desktop/profile/status?targetHostId=host&projectId=project",
            Value::Null,
            true,
        )
        .await;
    assert_eq!(status, 200, "{profile}");
    assert_eq!(profile["recovery"], "pending");
    assert!(f.tx.pending(AppId::ClaudeDesktop).unwrap().is_some());
    let mapping = json!({"mode":"proxy","routes":{"claude-sonnet-5":{"model":"new-intent"}}});
    let request = json!({"targetHostId":"host","projectId":"other","expectedRevision":"retry","mapping":mapping});
    assert_eq!(
        f.request("PUT", "/app-providers/claude-desktop/alpha/model-mapping", request, true)
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert!(f.tx.pending(AppId::ClaudeDesktop).unwrap().is_some());
    let request = json!({"targetHostId":"host","projectId":"project","expectedRevision":"retry","mapping":mapping});
    assert_eq!(
        f.request("PUT", "/app-providers/claude-desktop/alpha/model-mapping", request, true)
            .await
            .0,
        StatusCode::CONFLICT
    );
    assert!(f.tx.pending(AppId::ClaudeDesktop).unwrap().is_none());
    let row: Value = serde_json::from_str(
        &store.publication_record(&tenant(), AppId::ClaudeDesktop, "alpha").unwrap(),
    )
    .unwrap();
    assert_eq!(
        row["meta"]["claudeDesktopModelRoutes"]["claude-sonnet-5"]["model"],
        "recovered-model"
    );
    assert_eq!(f.tx.state(AppId::ClaudeDesktop).unwrap().project_scope, Some(project));
    assert!(
        !f.root
            .path()
            .join("Library/Application Support/Claude/claude_desktop_config.json")
            .exists()
    );
}

#[test]
fn aggregation_t46_attached_candidate_requires_a_live_desktop_publication() {
    use crate::{
        app_routing::RoutingListener, gateway::aggregation_resync::RecordPublicationContext,
    };
    use std::sync::Arc;
    let f = Fixture::new();
    let before = f.tx.state(AppId::ClaudeDesktop).unwrap();
    let mut files = ClaudeDesktopFiles::open(f.root.path()).unwrap();
    let project = ProjectScope::new(tenant(), "project".into()).unwrap();
    f.tx.commit_project_checked(
        &project,
        &mut files.guard,
        &before,
        false,
        &[],
        PendingTarget {
            stack: Some(vw_agent_apps::providers::mode_state::StackState {
                enabled: true,
                members: vec!["alpha".into()],
                keys: [("primary".into(), "alpha".into())].into(),
            }),
            mode: Some(ModeState {
                mode: Mode::Proxy,
                attached: true,
                proxy_route: Some("alpha".into()),
                contract: None,
            }),
            ..Default::default()
        },
        || Ok(()),
    )
    .unwrap();
    drop(files);
    let expected = f.tx.state(AppId::ClaudeDesktop).unwrap();
    let mut candidate = expected.clone();
    candidate.stack.keys.remove("primary");
    candidate.stack.keys.insert("secondary".into(), "alpha".into());
    let snapshot = GatewaySettingsStore::read_existing(
        &f.db,
        &tenant(),
        "app_providers.records",
        "\"claude-desktop\":alpha",
    )
    .unwrap();
    let context = RecordPublicationContext {
        database: f.db.clone(),
        project,
        host: "host".into(),
        app: AppId::ClaudeDesktop,
        home: f.root.path().into(),
        root: f.root.path().join("private"),
        routing: Some(Arc::new(RoutingListener::default())),
        expected: expected.clone(),
        principal: "manager".into(),
        revision: "unused-before-listener-validation".into(),
        failure: Default::default(),
    };
    assert_eq!(context.publish_candidate(&candidate), Err(StatusCode::SERVICE_UNAVAILABLE));
    assert!(f.tx.state(AppId::ClaudeDesktop).unwrap() == expected);
    assert!(f.tx.pending(AppId::ClaudeDesktop).unwrap().is_none());
    assert_eq!(
        GatewaySettingsStore::read_existing(
            &f.db,
            &tenant(),
            "app_providers.records",
            "\"claude-desktop\":alpha"
        )
        .unwrap(),
        snapshot
    );
}
