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
const TOKEN: &str = "task44-fixture-token-with-32-characters";
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
        std::fs::write(root.path().join(".codex/shiju-model-catalog.json"), "{}").unwrap();
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
        super::aggregation_clients::router_with_policy(Some(self.root.path().join("access.json")))
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
        let mut req=Request::builder().method(if body.is_some(){"POST"}else{"GET"}).uri(if body.is_some(){format!("/agent-apps/{app}/aggregation/codex/ack")}else{format!("/agent-apps/{app}/aggregation/codex/clients?targetHostId=host&projectId={project}")}).header(PLATFORM_ID_HEADER,"platform").header(COMPANY_ID_HEADER,"company");
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
async fn aggregation_t44_aggregation_clients_preserves_contract() {
    let f = Fixture::new();
    let path = f.root.path().join(".codex");
    let config = std::fs::read(path.join("config.toml")).unwrap();
    let (status, state) = f.request("codex", "project", true, None).await;
    assert_eq!(status, 200, "{state}");
    assert_eq!(state.as_object().unwrap().len(), 5);
    assert_eq!(state["unverified"], true);
    assert!(!path.join("shiju-codex-client-history.json").exists());
    assert!(!path.join("shiju-codex-client-ack.json").exists());
    let revision = state["revision"].clone();
    f.policy(true, false, false);
    assert_eq!(f.request("codex", "project", true, None).await.0, 200);
    assert_eq!(
        f.request("codex", "project", true, Some(json!({"expectedRevision":revision}))).await.0,
        403
    );
    assert!(!path.join("shiju-codex-client-ack.json").exists());
    f.policy(true, true, false);
    let (status, ack) =
        f.request("codex", "project", true, Some(json!({"expectedRevision":revision}))).await;
    assert_eq!(status, 200, "{ack}");
    assert_eq!(ack, state);
    let record: Value =
        serde_json::from_slice(&std::fs::read(path.join("shiju-codex-client-ack.json")).unwrap())
            .unwrap();
    assert_eq!(record, json!({"revision":revision}));
    assert_eq!(
        std::fs::metadata(path.join("shiju-codex-client-ack.json")).unwrap().permissions().mode()
            & 0o777,
        0o600
    );
    assert_eq!(std::fs::read(path.join("config.toml")).unwrap(), config);
    assert!(!path.join("shiju-codex-client-history.json").exists());
    std::fs::write(path.join("shiju-model-catalog.json"), "{\"changed\":true}").unwrap();
    assert_eq!(
        f.request("codex", "project", true, Some(json!({"expectedRevision":revision}))).await.0,
        409
    );
    assert_ne!(f.request("codex", "project", true, None).await.1["revision"], revision);
}
#[tokio::test]
async fn aggregation_t44_permissions_and_request_boundaries() {
    let f = Fixture::new();
    assert_eq!(f.request("codex", "project", false, None).await.0, 401);
    assert_eq!(f.request("codex", "other", true, None).await.0, 403);
    assert_eq!(f.request("claude", "project", true, None).await.0, 422);
    for body in [json!({"expectedRevision":""}), json!({"expectedRevision":"x", "daemonPid":12})] {
        assert_eq!(f.request("codex", "project", true, Some(body)).await.0, 400);
    }
    f.policy(false, true, false);
    assert_eq!(f.request("codex", "project", true, None).await.0, 403);
}
#[tokio::test]
async fn aggregation_t44_rejects_unsafe_ack_and_preserves_external_file() {
    let f = Fixture::new();
    let (_, state) = f.request("codex", "project", true, None).await;
    let external = f.root.path().join("external");
    std::fs::write(&external, "unchanged").unwrap();
    std::os::unix::fs::symlink(&external, f.root.path().join(".codex/shiju-codex-client-ack.json"))
        .unwrap();
    let (status, body) = f
        .request("codex", "project", true, Some(json!({"expectedRevision":state["revision"]})))
        .await;
    assert_eq!(status, 422, "{body}");
    assert_eq!(std::fs::read_to_string(external).unwrap(), "unchanged");
}
#[tokio::test]
async fn aggregation_t44_unreadable_identity_is_unverified_and_get_never_creates_home() {
    let f = Fixture::new();
    let codex = f.root.path().join(".codex");
    {
        let mut files =
            vw_agent_apps::providers::codex_catalog::CodexFiles::open(f.root.path()).unwrap();
        vw_agent_apps::aggregation::catalog_generation::observe(&mut files, &f.tx, &scope(), 1_000)
            .unwrap();
    }
    let (_, before) = f.request("codex", "project", true, None).await;
    assert_eq!(before["unverified"], false);
    let dir = codex.join("app-server-daemon");
    std::fs::create_dir(&dir).unwrap();
    std::fs::write(dir.join("daemon.pid"), "invalid-private-canary").unwrap();
    let (status, after) = f.request("codex", "project", true, None).await;
    assert_eq!(status, 200);
    assert_eq!(before["revision"], after["revision"]);
    assert_eq!(after["unverified"], true);
    assert!(!after.to_string().contains("private-canary"));
    std::fs::remove_dir_all(&codex).unwrap();
    assert_eq!(f.request("codex", "project", true, None).await.0, 422);
    assert!(!codex.exists());
}
struct Client(std::process::Child);
impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
#[tokio::test]
async fn aggregation_t44_real_codex_survives_ack_and_keeps_stale_flags() {
    use vw_agent_apps::{aggregation::catalog_generation, providers::codex_catalog::CodexFiles};
    let binary = std::env::var_os("T44_CODEX_BINARY")
        .expect("T44_CODEX_BINARY must name a real installed Codex CLI");
    let client_home = tempfile::tempdir().unwrap();
    std::fs::write(client_home.path().join("config.toml"), "cli_auth_credentials_store='file'\n")
        .unwrap();
    let mut client = Client(
        std::process::Command::new(binary)
            .arg("app-server")
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", client_home.path())
            .env("CODEX_HOME", client_home.path())
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap(),
    );
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    assert!(client.0.try_wait().unwrap().is_none());
    let table = std::process::Command::new("/bin/ps")
        .args(["-ww", "-Ao", "pid=,etime=,command="])
        .output()
        .unwrap();
    let table = String::from_utf8(table.stdout).unwrap();
    assert!(
        table
            .lines()
            .any(|line| line.split_whitespace().next() == Some(client.0.id().to_string().as_str())
                && line.contains("app-server"))
    );
    let f = Fixture::new();
    let now =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()
            as u64;
    {
        let mut files = CodexFiles::open(f.root.path()).unwrap();
        std::fs::write(
            f.root.path().join(".codex/auth.json"),
            r#"{"tokens":{"account_id":"fixture-one"}}"#,
        )
        .unwrap();
        catalog_generation::observe(&mut files, &f.tx, &scope(), 1000).unwrap();
        std::fs::write(
            f.root.path().join(".codex/auth.json"),
            r#"{"tokens":{"account_id":"fixture-two"}}"#,
        )
        .unwrap();
        catalog_generation::observe(&mut files, &f.tx, &scope(), now + 10_000).unwrap();
    }
    let (status, before) = f.request("codex", "project", true, None).await;
    assert_eq!(status, 200, "{before}");
    let typed: CodexClientsState = serde_json::from_value(before.clone()).unwrap();
    assert!(typed.others && typed.auth && !typed.unverified);
    let (status, after) =
        f.request("codex", "project", true, Some(json!({"expectedRevision":typed.revision}))).await;
    assert_eq!(status, 200, "{after}");
    assert_eq!(before, after);
    assert!(client.0.try_wait().unwrap().is_none());
    assert!(!after.to_string().contains("fixture-two"));
    {
        let mut files = CodexFiles::open(f.root.path()).unwrap();
        let current = f.tx.state(AppId::Codex).unwrap();
        let mut disabled = current.stack.clone();
        disabled.enabled = false;
        f.tx.commit_project_checked(
            &scope(),
            &mut files.guard,
            &current,
            true,
            &[],
            PendingTarget {
                mode: current.mode.clone(),
                stack: Some(disabled),
                ..Default::default()
            },
            || Ok(()),
        )
        .unwrap();
        std::fs::write(f.root.path().join(".codex/config.toml"), "").unwrap();
        std::fs::remove_file(f.root.path().join(".codex/auth.json")).unwrap();
        catalog_generation::observe(&mut files, &f.tx, &scope(), now + 11_000).unwrap();
    }
    let (status, disabled) = f.request("codex", "project", true, None).await;
    assert_eq!(status, 200, "{disabled}");
    assert_eq!(disabled["others"], true);
    assert_eq!(disabled["auth"], false);
    assert_eq!(disabled["unverified"], false);
    {
        let mut files = CodexFiles::open(f.root.path()).unwrap();
        let current = f.tx.state(AppId::Codex).unwrap();
        let mut enabled = current.stack.clone();
        enabled.enabled = true;
        f.tx.commit_project_checked(
            &scope(),
            &mut files.guard,
            &current,
            true,
            &[],
            PendingTarget {
                mode: current.mode.clone(),
                stack: Some(enabled),
                ..Default::default()
            },
            || Ok(()),
        )
        .unwrap();
        std::fs::write(
            f.root.path().join(".codex/config.toml"),
            "model_catalog_json='/never-read/foreign.json'\n",
        )
        .unwrap();
        catalog_generation::observe(&mut files, &f.tx, &scope(), now + 12_000).unwrap();
    }
    let (status, external) = f.request("codex", "project", true, None).await;
    assert_eq!(status, 200, "{external}");
    assert_eq!(external["others"], false);
    assert_eq!(external["auth"], false);
    {
        let mut files = CodexFiles::open(f.root.path()).unwrap();
        std::fs::write(
            f.root.path().join(".codex/auth.json"),
            r#"{"tokens":{"account_id":"fixture-three"}}"#,
        )
        .unwrap();
        catalog_generation::observe(&mut files, &f.tx, &scope(), now + 13_000).unwrap();
    }
    let (status, external_login) = f.request("codex", "project", true, None).await;
    assert_eq!(status, 200, "{external_login}");
    assert_eq!(external_login["others"], true);
    assert_eq!(external_login["auth"], true);
    assert!(client.0.try_wait().unwrap().is_none());
    println!("Real isolated Codex detected; acknowledgement preserved process and stale hints.");
}
#[tokio::test]
async fn aggregation_t44_explicit_write_grant_and_target_failures() {
    let f = Fixture::new();
    let (_, state) = f.request("codex", "project", true, None).await;
    let path = f.root.path().join("access.json");
    let mut policy: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    policy["principals"][0]["grants"][0]["aggregation_write"] = json!([]);
    std::fs::write(&path, policy.to_string()).unwrap();
    assert_eq!(
        f.request("codex", "project", true, Some(json!({"expectedRevision":state["revision"]})))
            .await
            .0,
        403
    );
    policy["hosts"][0]["online"] = json!(false);
    std::fs::write(&path, policy.to_string()).unwrap();
    assert_eq!(f.request("codex", "project", true, None).await.0, 503);
    f.policy(true, true, false);
    std::fs::write(f.root.path().join(".codex/config.toml"), "invalid=[").unwrap();
    let (status, body) = f.request("codex", "project", true, None).await;
    assert_eq!(status, 422, "{body}");
    assert!(!f.root.path().join(".codex/shiju-codex-client-ack.json").exists());
}
#[tokio::test]
async fn aggregation_t44_acknowledgement_suppresses_only_current_native_reminder() {
    use vw_agent_apps::{
        aggregation::{catalog_ack, stale_clients::ClientsState},
        providers::codex_catalog::CodexFiles,
    };
    let f = Fixture::new();
    let (_, state) = f.request("codex", "project", true, None).await;
    assert_eq!(
        f.request("codex", "project", true, Some(json!({"expectedRevision":state["revision"]})))
            .await
            .0,
        200
    );
    let state: CodexClientsState = serde_json::from_value(state).unwrap();
    {
        let files = CodexFiles::open(f.root.path()).unwrap();
        let clients = ClientsState {
            revision: state.revision,
            daemon: state.daemon,
            others: state.others,
            auth: state.auth,
            unverified: state.unverified,
        };
        assert!(catalog_ack::reminder(&files, &f.tx, &scope(), &clients).unwrap().is_none());
    }
    std::fs::write(f.root.path().join(".codex/shiju-model-catalog.json"), "{\"new\":true}")
        .unwrap();
    let (_, state) = f.request("codex", "project", true, None).await;
    let state: CodexClientsState = serde_json::from_value(state).unwrap();
    let files = CodexFiles::open(f.root.path()).unwrap();
    let clients = ClientsState {
        revision: state.revision,
        daemon: state.daemon,
        others: state.others,
        auth: state.auth,
        unverified: state.unverified,
    };
    assert!(catalog_ack::reminder(&files, &f.tx, &scope(), &clients).unwrap().is_some());
}
#[test]
fn aggregation_t44_snapshot_distinguishes_external_catalog_without_reading_it() {
    use vw_agent_apps::{aggregation::catalog_generation, providers::codex_catalog::CodexFiles};
    let f = Fixture::new();
    let files = CodexFiles::open(f.root.path()).unwrap();
    assert!(!catalog_generation::read(&files, &f.tx, &scope()).unwrap().catalog_is_external());
    std::fs::write(f.root.path().join(".codex/config.toml"), "").unwrap();
    assert!(!catalog_generation::read(&files, &f.tx, &scope()).unwrap().catalog_is_external());
    std::fs::write(
        f.root.path().join(".codex/config.toml"),
        "model_catalog_json='/never-read/foreign.json'\n",
    )
    .unwrap();
    assert!(catalog_generation::read(&files, &f.tx, &scope()).unwrap().catalog_is_external());
}
