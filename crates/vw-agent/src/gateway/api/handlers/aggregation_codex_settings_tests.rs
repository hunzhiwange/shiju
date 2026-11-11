#[path = "aggregation_codex_settings_fixture.rs"]
mod fixture;
use crate::global_database::GlobalDatabase;
use axum::{
    Extension, Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
    middleware,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::os::unix::fs::PermissionsExt;
use tower::ServiceExt;
use vw_agent_apps::providers::{
    live::{commit::*, write_guard::lock_app},
    mode_state::StackState,
};
use vw_api_types::{
    agent_apps::{AppId, aggregation::*},
    id::{COMPANY_ID_HEADER, PLATFORM_ID_HEADER, ProjectId, ProjectScope, TenantScope},
};
const TOKEN: &str = "task43-fixture-token-with-32-characters";
struct Fixture {
    root: tempfile::TempDir,
    db: GlobalDatabase,
    tx: ProviderTransactions,
}
fn scope() -> ProjectScope {
    ProjectScope::new(TenantScope::new("platform", "company").unwrap(), ProjectId::from("project"))
        .unwrap()
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let private = root.path().join("private");
        std::fs::create_dir(&private).unwrap();
        std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o700)).unwrap();
        let db = GlobalDatabase::for_home(root.path()).unwrap();
        crate::gateway::settings_store::GatewaySettingsStore::new(db.clone()).unwrap();
        let tx =
            ProviderTransactions::new(&private, ExecutionHostId::new("host").unwrap()).unwrap();
        let mut guard = lock_app(AppId::Codex, root.path()).unwrap();
        tx.commit_project_checked(
            &scope(),
            &mut guard,
            &tx.state(AppId::Codex).unwrap(),
            true,
            &[],
            PendingTarget {
                mode: Some(ModeState {
                    mode: Mode::Proxy,
                    attached: true,
                    proxy_route: Some("alpha".into()),
                    contract: None,
                }),
                stack: Some(StackState {
                    enabled: true,
                    members: vec!["alpha".into(), "beta".into()],
                    keys: [("alpha".into(), "alpha".into()), ("beta".into(), "beta".into())].into(),
                }),
                ..Default::default()
            },
            || Ok(()),
        )
        .unwrap();
        drop(guard);
        std::fs::create_dir(root.path().join(".codex")).unwrap();
        std::fs::write(
            root.path().join(".codex/config.toml"),
            "model_catalog_json='shiju-model-catalog.json'\n[agents]\nmax_threads=3\n",
        )
        .unwrap();
        std::fs::write(
            root.path().join(".codex/shiju-model-catalog.json"),
            fixture::catalog().to_string(),
        )
        .unwrap();
        let f = Self { root, db, tx };
        f.policy(true, true, true);
        f
    }
    fn policy(&self, read: bool, write: bool, routing: bool) {
        let permits = json!([{"project_id":"project","app_id":"codex"},{"project_id":"other","app_id":"codex"},{"project_id":"project","app_id":"claude"}]);
        let p = self.root.path().join("access.json");
        std::fs::write(&p,json!({"principals":[{"id":"fixture","token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":[{"platform_id":"platform","company_id":"company","host_id":"host","provider_config_read":read,"provider_config_write":write,"aggregation_read":permits,"aggregation_write":if write{permits.clone()}else{json!([])},"routing_manage":routing,"routing_apps":permits}]}],"hosts":[{"id":"host","online":true,"binding":"local","home":self.root.path(),"credential_root":self.root.path().join("private"),"search_paths":[],"applications":self.root.path().join("Applications")}]}).to_string()).unwrap();
        std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    fn router(&self) -> Router {
        super::aggregation_codex_settings::router_with_policy(Some(
            self.root.path().join("access.json"),
        ))
        .layer(Extension(self.db.clone()))
        .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware))
    }
    async fn request(
        &self,
        app: &str,
        project: &str,
        token: bool,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let mut req=Request::builder().method(if body.is_some(){"PUT"}else{"GET"}).uri(if body.is_some(){format!("/agent-apps/{app}/aggregation/codex/settings")}else{format!("/agent-apps/{app}/aggregation/codex/settings?targetHostId=host&projectId={project}")}).header(PLATFORM_ID_HEADER,"platform").header(COMPANY_ID_HEADER,"company");
        if token {
            req = req.header("authorization", format!("Bearer {TOKEN}"));
        }
        let body = if let Some(mut b) = body {
            b["targetHostId"] = json!("host");
            b["projectId"] = json!(project);
            req = req.header("content-type", "application/json");
            Body::from(b.to_string())
        } else {
            Body::empty()
        };
        let resp = self.router().oneshot(req.body(body).unwrap()).await.unwrap();
        let status = resp.status();
        let bytes = to_bytes(resp.into_body(), 65536).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
    }
}
#[tokio::test]
async fn aggregation_t43_aggregation_codex_settings_preserves_contract() {
    let f = Fixture::new();
    let (status, v) = f.request("codex", "project", true, None).await;
    assert_eq!(status, 200, "{v}");
    let settings: CodexAggregationSettings = serde_json::from_value(v).unwrap();
    assert!(settings.requested.classic_tools);
    assert_eq!(settings.choices.len(), 1);
    let requested = json!({"subagentModelId":"ccs-beta/child","subagentEffort":"high","autoReviewModelId":"ccs-beta/child","classicTools":true});
    let (status, v) = f
        .request(
            "codex",
            "project",
            true,
            Some(json!({"expectedRevision":settings.revision,"requested":requested})),
        )
        .await;
    assert_eq!(status, 200, "{v}");
    assert_eq!(v["effective"], requested);
    assert_ne!(v["revision"], settings.revision);
    let config = std::fs::read_to_string(f.root.path().join(".codex/config.toml")).unwrap();
    assert!(config.contains("ccs-beta/child"));
    assert!(config.contains("max_threads=3") || config.contains("max_threads = 3"));
    let catalog: Value = serde_json::from_slice(
        &std::fs::read(f.root.path().join(".codex/shiju-model-catalog.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(catalog["models"][0]["auto_review_model_override"], "ccs-beta/child");
    assert_eq!(catalog["models"][1]["multi_agent_version"], "v1");
    assert!(f.tx.pending(AppId::Codex).unwrap().is_none());
    if let Some(destination) = std::env::var_os("SHIJU_T43_EXPORT") {
        std::fs::create_dir_all(&destination).unwrap();
        for name in ["config.toml", "shiju-model-catalog.json"] {
            std::fs::copy(
                f.root.path().join(".codex").join(name),
                std::path::Path::new(&destination).join(name),
            )
            .unwrap();
        }
    }
}

fn values() -> Value {
    json!({"subagentModelId":null,"subagentEffort":null,"autoReviewModelId":null,"classicTools":true})
}
#[tokio::test]
async fn aggregation_t43_denies_missing_auth_foreign_project_and_cross_application() {
    let f = Fixture::new();
    assert_eq!(f.request("codex", "project", false, None).await.0, 401);
    assert_eq!(f.request("codex", "other", true, None).await.0, 403);
    let (status, v) = f.request("claude", "project", true, None).await;
    assert_eq!(status, 422);
    assert_eq!(v["error"]["code"], "unsupported_app");
    f.policy(false, true, true);
    assert_eq!(f.request("codex", "project", true, None).await.0, 403);
}
#[tokio::test]
async fn aggregation_t43_rejects_unpublished_models_and_stale_revision_without_writes() {
    let f = Fixture::new();
    let (_, v) = f.request("codex", "project", true, None).await;
    let before = std::fs::read(f.root.path().join(".codex/config.toml")).unwrap();
    for id in ["primary", "ccs-missing/child", "ccs-claude-beta--child"] {
        let mut requested = values();
        requested["subagentModelId"] = json!(id);
        let (status, error) = f
            .request(
                "codex",
                "project",
                true,
                Some(json!({"expectedRevision":v["revision"],"requested":requested})),
            )
            .await;
        assert_eq!(status, 422, "{error}");
        assert_eq!(error["error"]["code"], "invalid_model");
    }
    assert_eq!(
        f.request(
            "codex",
            "project",
            true,
            Some(json!({"expectedRevision":"stale","requested":values()}))
        )
        .await
        .0,
        409
    );
    assert_eq!(std::fs::read(f.root.path().join(".codex/config.toml")).unwrap(), before);
    assert!(f.tx.state(AppId::Codex).unwrap().codex_aggregation_settings.is_none());
}
#[tokio::test]
async fn aggregation_t43_requires_write_and_attached_routing_permissions() {
    let f = Fixture::new();
    let (_, v) = f.request("codex", "project", true, None).await;
    for (write, routing) in [(false, true), (true, false)] {
        f.policy(true, write, routing);
        assert_eq!(
            f.request(
                "codex",
                "project",
                true,
                Some(json!({"expectedRevision":v["revision"],"requested":values()}))
            )
            .await
            .0,
            403
        );
    }
    assert!(f.tx.state(AppId::Codex).unwrap().codex_aggregation_settings.is_none());
}
#[tokio::test]
async fn aggregation_t43_normalizes_effort_clears_model_and_preserves_user_fields() {
    let f = Fixture::new();
    let (_, v) = f.request("codex", "project", true, None).await;
    let mut requested = values();
    requested["subagentModelId"] = json!("ccs-beta/child");
    requested["subagentEffort"] = json!("max");
    let (status, v) = f
        .request(
            "codex",
            "project",
            true,
            Some(json!({"expectedRevision":v["revision"],"requested":requested})),
        )
        .await;
    assert_eq!(status, 200, "{v}");
    assert_eq!(v["requested"]["subagentEffort"], Value::Null);
    let rejected = f
        .request(
            "codex",
            "project",
            true,
            Some(json!({"expectedRevision":v["revision"],"requested":requested})),
        )
        .await;
    assert_eq!(rejected.0, 422, "{}", rejected.1);
    assert_eq!(rejected.1["error"]["code"], "invalid_model");
    let mut requested = values();
    requested["subagentEffort"] = json!("high");
    let (status, v) = f
        .request(
            "codex",
            "project",
            true,
            Some(json!({"expectedRevision":v["revision"],"requested":requested})),
        )
        .await;
    assert_eq!(status, 200, "{v}");
    assert_eq!(v["requested"]["subagentEffort"], Value::Null);
    let config = std::fs::read_to_string(f.root.path().join(".codex/config.toml")).unwrap();
    assert!(!config.contains("default_subagent"));
    assert!(config.contains("max_threads"));
    assert_eq!(f.request("codex", "project", true, None).await.1["requested"], v["requested"]);
}
#[tokio::test]
async fn aggregation_t43_reports_forced_v2_and_can_disable_classic_projection() {
    let f = Fixture::new();
    std::fs::write(
        f.root.path().join(".codex/config.toml"),
        "model_catalog_json='shiju-model-catalog.json'\n[features]\nmulti_agent_v2=true\n",
    )
    .unwrap();
    let (_, v) = f.request("codex", "project", true, None).await;
    let mut requested = values();
    requested["subagentModelId"] = json!("ccs-beta/child");
    let (status, v) = f
        .request(
            "codex",
            "project",
            true,
            Some(json!({"expectedRevision":v["revision"],"requested":requested})),
        )
        .await;
    assert_eq!(status, 200, "{v}");
    assert_eq!(v["effective"]["classicTools"], false);
    assert!(v["notices"].as_array().unwrap().contains(&json!("multi_agent_v2_forced")));
    assert!(v["notices"].as_array().unwrap().contains(&json!("encrypted_task_cross_provider")));
    requested["classicTools"] = json!(false);
    let (status, v) = f
        .request(
            "codex",
            "project",
            true,
            Some(json!({"expectedRevision":v["revision"],"requested":requested})),
        )
        .await;
    assert_eq!(status, 200, "{v}");
    let catalog: Value = serde_json::from_slice(
        &std::fs::read(f.root.path().join(".codex/shiju-model-catalog.json")).unwrap(),
    )
    .unwrap();
    assert!(
        catalog["models"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r.get("multi_agent_version").is_none())
    );
}
#[tokio::test]
async fn aggregation_t43_invalid_payload_and_symlink_do_not_publish() {
    let f = Fixture::new();
    let (_, v) = f.request("codex", "project", true, None).await;
    assert_eq!(
        f.request(
            "codex",
            "project",
            true,
            Some(json!({"expectedRevision":v["revision"],"requested":values(),"path":"untrusted"}))
        )
        .await
        .0,
        400
    );
    let path = f.root.path().join(".codex/shiju-model-catalog.json");
    std::fs::remove_file(&path).unwrap();
    std::os::unix::fs::symlink(f.root.path().join("access.json"), path).unwrap();
    assert_eq!(f.request("codex", "project", true, None).await.0, 422);
    assert!(f.tx.state(AppId::Codex).unwrap().codex_aggregation_settings.is_none());
}

#[tokio::test]
async fn aggregation_t43_pending_intent_uses_existing_recovery_and_blocks_api() {
    use vw_agent_apps::providers::live::{
        stage::{FileChange, WholeFile},
        write_guard::{FilePrivacy, WriteError},
    };
    let f = Fixture::new();
    let config = f.root.path().join(".codex/config.toml");
    let before = std::fs::read(&config).unwrap();
    let patch = WholeFile::new(Some(&before), Some(b"model='planned'\n".to_vec()));
    let mut guard = lock_app(AppId::Codex, &f.root.path().join(".codex")).unwrap();
    let calls = std::cell::Cell::new(0);
    let result = f.tx.commit_project_checked(
        &scope(),
        &mut guard,
        &f.tx.state(AppId::Codex).unwrap(),
        false,
        &[FileChange {
            name: std::ffi::OsStr::new("config.toml"),
            privacy: FilePrivacy::Private,
            patch: &patch,
        }],
        PendingTarget {
            codex_aggregation_settings: Some(CodexSettingsValues::default()),
            ..Default::default()
        },
        || {
            calls.set(calls.get() + 1);
            if calls.get() == 2 { Err(WriteError::Conflict) } else { Ok(()) }
        },
    );
    assert_eq!(result, Err(WriteError::Conflict));
    assert_eq!(f.tx.pending(AppId::Codex).unwrap().unwrap().version, 8);
    drop(guard);
    let (status, v) = f.request("codex", "project", true, None).await;
    assert_eq!(status, 409, "{v}");
    assert_eq!(v["error"]["code"], "recovery_pending");
    assert_eq!(std::fs::read(&config).unwrap(), before);
    let mut guard = lock_app(AppId::Codex, &f.root.path().join(".codex")).unwrap();
    f.tx.recover_project(&scope(), &mut guard).unwrap();
    drop(guard);
    assert!(f.tx.pending(AppId::Codex).unwrap().is_none());
    assert!(f.tx.state(AppId::Codex).unwrap().codex_aggregation_settings.is_none());
    assert_eq!(std::fs::read(&config).unwrap(), before);
}

#[tokio::test]
async fn aggregation_t43_membership_change_retains_intent_and_revalidates_choices() {
    let f = Fixture::new();
    let (_, v) = f.request("codex", "project", true, None).await;
    let requested = json!({"subagentModelId":"ccs-beta/child","subagentEffort":"high","autoReviewModelId":"ccs-beta/child","classicTools":true});
    let (status, v) = f
        .request(
            "codex",
            "project",
            true,
            Some(json!({"expectedRevision":v["revision"],"requested":requested})),
        )
        .await;
    assert_eq!(status, 200, "{v}");
    for present in [false, true] {
        let state = f.tx.state(AppId::Codex).unwrap();
        let mut stack = state.stack.clone();
        stack.members =
            if present { vec!["alpha".into(), "beta".into()] } else { vec!["alpha".into()] };
        let mut guard = lock_app(AppId::Codex, f.root.path()).unwrap();
        f.tx.commit_project_checked(
            &scope(),
            &mut guard,
            &state,
            false,
            &[],
            PendingTarget { stack: Some(stack), ..Default::default() },
            || Ok(()),
        )
        .unwrap();
        drop(guard);
        let config = std::fs::read(f.root.path().join(".codex/config.toml")).unwrap();
        let catalog = std::fs::read(f.root.path().join(".codex/shiju-model-catalog.json")).unwrap();
        let (status, current) = f.request("codex", "project", true, None).await;
        assert_eq!(status, 200, "{current}");
        assert_eq!(current["requested"], requested);
        assert_eq!(current["choices"].as_array().unwrap().len(), usize::from(present));
        assert_eq!(
            current["effective"]["subagentModelId"],
            if present { json!("ccs-beta/child") } else { Value::Null }
        );
        assert_eq!(
            current["effective"]["autoReviewModelId"],
            if present { json!("ccs-beta/child") } else { Value::Null }
        );
        assert_eq!(std::fs::read(f.root.path().join(".codex/config.toml")).unwrap(), config);
        assert_eq!(
            std::fs::read(f.root.path().join(".codex/shiju-model-catalog.json")).unwrap(),
            catalog
        );
    }
}

#[tokio::test]
async fn aggregation_t43_native_write_failure_returns_target_failed_without_saving_intent() {
    let f = Fixture::new();
    let (_, v) = f.request("codex", "project", true, None).await;
    let dir = f.root.path().join(".codex");
    let before = std::fs::read(dir.join("config.toml")).unwrap();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o500)).unwrap();
    let response = f
        .request(
            "codex",
            "project",
            true,
            Some(json!({"expectedRevision":v["revision"],"requested":values()})),
        )
        .await;
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(response.0, 502, "{}", response.1);
    assert_eq!(response.1["error"]["code"], "target_failed");
    assert_eq!(std::fs::read(dir.join("config.toml")).unwrap(), before);
    assert!(f.tx.state(AppId::Codex).unwrap().codex_aggregation_settings.is_none());
}

#[tokio::test]
async fn aggregation_t43_reports_unavailable_and_unsupported_hosts() {
    let f = Fixture::new();
    let p = f.root.path().join("access.json");
    let mut policy: Value = serde_json::from_slice(&std::fs::read(&p).unwrap()).unwrap();
    policy["hosts"][0]["online"] = json!(false);
    std::fs::write(&p, policy.to_string()).unwrap();
    let (status, v) = f.request("codex", "project", true, None).await;
    assert_eq!(status, 503);
    assert_eq!(v["error"]["code"], "target_unavailable");
    policy["hosts"][0]["online"] = json!(true);
    policy["hosts"][0]["binding"] = json!("remote");
    std::fs::write(&p, policy.to_string()).unwrap();
    let (status, v) = f.request("codex", "project", true, None).await;
    assert_eq!(status, 422);
    assert_eq!(v["error"]["code"], "unsupported_target");
}

#[tokio::test]
async fn aggregation_t43_official_default_requires_verified_account_generation() {
    let f = Fixture::new();
    crate::gateway::settings_store::GatewaySettingsStore::new(f.db.clone())
        .unwrap()
        .upsert(
            scope().tenant_scope(),
            "app_providers.records",
            "\"codex\":alpha",
            &json!({"name":"Official","category":"official"}).to_string(),
        )
        .unwrap();
    let (status, v) = f.request("codex", "project", true, None).await;
    assert_eq!(status, 503, "{v}");
    assert_eq!(v["error"]["code"], "target_unavailable");
    assert!(f.tx.state(AppId::Codex).unwrap().codex_aggregation_settings.is_none());
}
