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
const TOKEN: &str = "task45-fixture-token-with-32-characters";
struct Fixture {
    root: tempfile::TempDir,
    db: GlobalDatabase,
    tx: ProviderTransactions,
    router: std::sync::OnceLock<Router>,
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
        std::fs::write(root.path().join(".codex/shiju-model-catalog.json"), "{}").unwrap();
        let f = Self { root, db, tx, router: std::sync::OnceLock::new() };
        f.policy(true, true, true);
        f
    }
    fn policy(&self, read: bool, write: bool, routing: bool) {
        let permits = json!([{"project_id":"project","app_id":"codex"},{"project_id":"other","app_id":"codex"},{"project_id":"project","app_id":"claude"}]);
        let p = self.root.path().join("access.json");
        std::fs::write(&p,json!({"principals":[{"id":"fixture","token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":[{"platform_id":"platform","company_id":"company","host_id":"host","provider_config_read":read,"provider_config_write":write,"aggregation_codex_restart":if read {permits.clone()}else{json!([])},"aggregation_read":permits,"aggregation_write":if write{permits.clone()}else{json!([])},"routing_manage":routing,"routing_apps":permits}]}],"hosts":[{"id":"host","online":true,"binding":"local","home":self.root.path(),"credential_root":self.root.path().join("private"),"search_paths":[std::path::Path::new(&std::env::var("T45_CODEX_BINARY").expect("real Codex CLI required")).parent().unwrap()],"applications":self.root.path().join("Applications")}]}).to_string()).unwrap();
        std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    fn router(&self) -> Router {
        self.router
            .get_or_init(|| {
                super::app_providers::operations::router_with_policy(Some(
                    self.root.path().join("access.json"),
                ))
                .merge(super::aggregation_clients::router_with_policy(Some(
                    self.root.path().join("access.json"),
                )))
                .layer(Extension(self.db.clone()))
                .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware))
            })
            .clone()
    }
    async fn request(
        &self,
        app: &str,
        project: &str,
        token: bool,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let mut req=Request::builder().method(if body.is_some(){"POST"}else{"GET"}).uri(if body.is_some(){format!("/agent-apps/{app}/aggregation/codex/restart")}else{format!("/agent-apps/{app}/aggregation/codex/clients?targetHostId=host&projectId={project}")}).header(PLATFORM_ID_HEADER,"platform").header(COMPANY_ID_HEADER,"company");
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
async fn aggregation_t45_aggregation_restart_preserves_contract() {
    let f = Fixture::new();
    let (status, body) = f
        .request(
            "codex",
            "project",
            true,
            Some(json!({"expectedRevision":"x","requestId":"restart-one","confirmed":false})),
        )
        .await;
    assert_eq!(status, 422, "{body}");
    assert_eq!(body["error"]["code"], "confirmation_required");
}

impl Fixture {
    async fn operation(
        &self,
        id: &str,
        method: &str,
        project: &str,
        token: bool,
    ) -> (StatusCode, Value) {
        let mut req = Request::builder()
            .method(method)
            .uri(format!("/app-providers/operations/{id}?targetHostId=host&projectId={project}"))
            .header(PLATFORM_ID_HEADER, "platform")
            .header(COMPANY_ID_HEADER, "company");
        if token {
            req = req.header("authorization", format!("Bearer {TOKEN}"));
        }
        let resp = self.router().oneshot(req.body(Body::empty()).unwrap()).await.unwrap();
        let status = resp.status();
        let bytes = to_bytes(resp.into_body(), 65536).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }
    async fn terminal(&self, id: &str) -> Value {
        for _ in 0..200 {
            let (status, p) = self.operation(id, "GET", "project", true).await;
            assert_eq!(status, 200, "{p}");
            if matches!(p["status"].as_str(), Some("completed" | "failed" | "cancelled")) {
                return p;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        panic!("restart task did not settle")
    }
    async fn revision(&self) -> Value {
        let (status, p) = self.request("codex", "project", true, None).await;
        assert_eq!(status, 200, "{p}");
        p["revision"].clone()
    }
    async fn restart(&self, revision: Value, id: &str) -> (StatusCode, Value) {
        self.request(
            "codex",
            "project",
            true,
            Some(json!({"expectedRevision":revision,"requestId":id,"confirmed":true})),
        )
        .await
    }
}
#[tokio::test]
async fn aggregation_t45_not_running_replays_and_independent_permissions() {
    let f = Fixture::new();
    let revision = f.revision().await;
    f.policy(true, false, false); // independent restart permission, no config write or routing grant
    let before = std::fs::read(f.root.path().join(".codex/config.toml")).unwrap();
    let (status, p) = f.restart(revision.clone(), "restart-one").await;
    assert_eq!(status, 202, "{p}");
    let id = p["operationId"].as_str().unwrap();
    let terminal = f.terminal(id).await;
    assert_eq!(terminal["status"], "completed");
    assert_eq!(terminal["aggregationOutcome"], json!({"kind":"restarted","result":"not_running"}));
    assert_eq!(f.restart(revision.clone(), "restart-one").await.1["operationId"], id);
    assert_eq!(
        f.request(
            "codex",
            "project",
            true,
            Some(json!({"expectedRevision":revision,"requestId":"restart-one","confirmed":false}))
        )
        .await
        .1["error"]["code"],
        "request_conflict"
    );

    assert_eq!(
        f.restart(json!("changed"), "restart-one").await.1["error"]["code"],
        "request_conflict"
    );
    // Replay precedes a fresh revision check, but is always subject to current authorization.
    std::fs::write(f.root.path().join(".codex/shiju-model-catalog.json"), "{\"changed\":true}")
        .unwrap();
    assert_eq!(f.restart(revision.clone(), "restart-one").await.0, 202);
    assert_eq!(f.restart(revision, "restart-two").await.0, 409);
    assert_eq!(f.operation(id, "GET", "other", true).await.0, 404);
    assert_eq!(f.operation(id, "GET", "project", false).await.0, 401);
    assert_eq!(f.operation(id, "DELETE", "project", true).await.1["status"], "completed");
    let policy_path = f.root.path().join("access.json");
    let mut policy: Value = serde_json::from_slice(&std::fs::read(&policy_path).unwrap()).unwrap();
    policy["principals"][0]["id"] = json!("other-actor");
    std::fs::write(&policy_path, policy.to_string()).unwrap();
    assert_eq!(f.operation(id, "GET", "project", true).await.0, 404);
    assert_eq!(f.operation(id, "DELETE", "project", true).await.0, 404);
    f.policy(false, true, true); // config write does not confer restart, polling, cancellation or replay
    assert_eq!(f.operation(id, "GET", "project", true).await.0, 403);
    assert_eq!(f.operation(id, "DELETE", "project", true).await.0, 403);
    assert_eq!(f.restart(json!("any"), "restart-one").await.0, 403);
    assert_eq!(std::fs::read(f.root.path().join(".codex/config.toml")).unwrap(), before);
    assert!(!f.root.path().join(".codex/app-server-daemon").exists());
    assert_eq!(f.tx.state(AppId::Codex).unwrap().project_scope, Some(scope()));
}
#[tokio::test]
async fn aggregation_t45_request_boundaries_reject_arbitrary_process_control() {
    let f = Fixture::new();
    let revision = f.revision().await;
    let body = json!({"expectedRevision":revision,"requestId":"one","confirmed":true});
    for (field, value) in [
        ("command", json!("kill")),
        ("pid", json!(123)),
        ("path", json!("/tmp/codex")),
        ("environment", json!({"CODEX_HOME":"/tmp"})),
        ("args", json!([])),
    ] {
        let mut b = body.clone();
        b[field] = value;
        assert_eq!(f.request("codex", "project", true, Some(b)).await.0, 400);
    }
    let mut missing = body.clone();
    missing.as_object_mut().unwrap().remove("confirmed");
    assert_eq!(f.request("codex", "project", true, Some(missing)).await.0, 400);
    assert_eq!(f.request("codex", "project", false, Some(body.clone())).await.0, 401);
    assert_eq!(f.request("codex", "other", true, Some(body.clone())).await.0, 403);
    assert_eq!(
        f.request("claude", "project", true, Some(body)).await.1["error"]["code"],
        "unsupported_app"
    );
}
#[tokio::test]
async fn aggregation_t45_invalid_daemon_record_reports_failure_without_side_effects() {
    let f = Fixture::new();
    let revision = f.revision().await;
    let daemon = f.root.path().join(".codex/app-server-daemon");
    std::fs::create_dir(&daemon).unwrap();
    std::fs::write(daemon.join("daemon.pid"), "{broken}").unwrap();
    let (status, p) = f.restart(revision, "broken").await;
    assert_eq!(status, 202, "{p}");
    let terminal = f.terminal(p["operationId"].as_str().unwrap()).await;
    assert_eq!(terminal["status"], "failed");
    assert_eq!(
        terminal["aggregationOutcome"],
        json!({"kind":"failed","status":422,"code":"unsupported_target","recovery":"none"})
    );
    let text = terminal.to_string();
    assert!(!text.contains(f.root.path().to_str().unwrap()));
    assert!(!text.contains("broken}"));
    assert_eq!(std::fs::read_to_string(daemon.join("daemon.pid")).unwrap(), "{broken}");
}

#[tokio::test]
async fn aggregation_t45_real_unmanaged_codex_is_never_restarted() {
    use std::process::{Command, Stdio};
    struct OwnedChild(std::process::Child);
    impl Drop for OwnedChild {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let f = Fixture::new();
    std::fs::write(f.root.path().join(".codex/config.toml"), "cli_auth_credentials_store='file'\n")
        .unwrap();
    let revision = f.revision().await;
    let binary = std::env::var("T45_CODEX_BINARY").expect("real installed Codex CLI required");
    let mut child = OwnedChild(
        Command::new(binary)
            .arg("app-server")
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", f.root.path())
            .env("CODEX_HOME", f.root.path().join(".codex"))
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let mut info = std::mem::MaybeUninit::<libc::proc_bsdinfo>::uninit();
    let size = std::mem::size_of::<libc::proc_bsdinfo>() as i32;
    // SAFETY: buffer has the exact requested native structure size for this owned child.
    assert_eq!(
        unsafe {
            libc::proc_pidinfo(
                child.0.id() as i32,
                libc::PROC_PIDTBSDINFO,
                1,
                info.as_mut_ptr().cast(),
                size,
            )
        },
        size
    );
    // SAFETY: proc_pidinfo initialized the complete structure above.
    let info = unsafe { info.assume_init() };
    let dir = f.root.path().join(".codex/app-server-daemon");
    std::fs::create_dir(&dir).unwrap();
    std::fs::write(dir.join("daemon.pid"),json!({"pid":child.0.id(),"processIdentity":{"startSeconds":info.pbi_start_tvsec,"startMicroseconds":info.pbi_start_tvusec}}).to_string()).unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert!(child.0.try_wait().unwrap().is_none(), "real isolated CLI must remain alive");
    let (status, p) = f.restart(revision, "unmanaged").await;
    assert_eq!(status, 202, "{p}");
    let terminal = f.terminal(p["operationId"].as_str().unwrap()).await;
    assert_eq!(terminal["status"], "failed");
    assert_eq!(terminal["aggregationOutcome"]["code"], "target_unavailable");
    assert!(child.0.try_wait().unwrap().is_none(), "unmanaged Codex was affected");
    println!("real isolated ordinary Codex survived refused restart; only fixture child is reaped");
}
