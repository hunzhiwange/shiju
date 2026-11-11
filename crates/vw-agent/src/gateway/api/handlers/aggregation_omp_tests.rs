use axum::{
    Extension, Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
    middleware,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{os::unix::fs::PermissionsExt, path::PathBuf};
use tower::ServiceExt;
use vw_agent_apps::providers::live::{commit::*, write_guard::lock_app};
use vw_api_types::{
    agent_apps::AppId,
    id::{COMPANY_ID_HEADER, PLATFORM_ID_HEADER, ProjectId, ProjectScope, TenantScope},
};
const TOKEN: &str = "aggregation-t49-fixture-token-at-least-32-chars";
struct Fixture {
    root: tempfile::TempDir,
    policy: PathBuf,
    tx: ProviderTransactions,
}
fn scope(project: &str) -> ProjectScope {
    ProjectScope::new(TenantScope::new("p", "a").unwrap(), ProjectId::from(project)).unwrap()
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let private = root.path().join("private");
        std::fs::create_dir(&private).unwrap();
        std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o700)).unwrap();
        let tx =
            ProviderTransactions::new(&private, ExecutionHostId::new("host").unwrap()).unwrap();
        let mut guard = lock_app(AppId::Omp, root.path()).unwrap();
        tx.commit_project_checked(
            &scope("project"),
            &mut guard,
            &tx.state(AppId::Omp).unwrap(),
            true,
            &[],
            PendingTarget::default(),
            || Ok(()),
        )
        .unwrap();
        let policy = root.path().join("access.json");
        std::fs::write(&policy, json!({"principals":[{"id":"omp","token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":[{"platform_id":"p","company_id":"a","host_id":"host","provider_config_read":true,"provider_config_write":true,"aggregation_read":[{"project_id":"project","app_id":"omp"},{"project_id":"other","app_id":"omp"}]}]}],"hosts":[{"id":"host","online":true,"binding":"local","home":root.path(),"credential_root":private,"search_paths":[],"applications":root.path().join("Applications")}]}).to_string()).unwrap();
        std::fs::set_permissions(&policy, std::fs::Permissions::from_mode(0o600)).unwrap();
        let f = Self { root, policy, tx };
        std::fs::create_dir_all(f.dir()).unwrap();
        std::fs::write(f.dir().join("models.yml"), "# user header\nnotes: keep\nproviders:\n  other:\n    baseUrl: https://other.invalid # keep\n\n# footer\n").unwrap();
        std::fs::write(
            f.dir().join("config.yml"),
            "modelRoles:\n  default: builtin/default-model\n",
        )
        .unwrap();
        std::fs::write(f.dir().join("agent.db"), b"login fixture").unwrap();
        f
    }
    fn profile(&self) -> Option<String> {
        std::env::var("OMP_PROFILE")
            .ok()
            .or_else(|| std::env::var("PI_PROFILE").ok())
            .filter(|s| !s.is_empty() && s != "default")
    }
    fn dir(&self) -> PathBuf {
        match self.profile() {
            Some(p) => self.root.path().join(".omp/profiles").join(p).join("agent"),
            None => self.root.path().join(".omp/agent"),
        }
    }
    fn router(&self) -> Router {
        super::app_providers::omp::router_with_policy(Some(self.policy.clone()))
            .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware))
    }
    async fn call(
        &self,
        method: &str,
        path: &str,
        value: Value,
        token: bool,
    ) -> (StatusCode, Value) {
        let mut r = Request::builder()
            .method(method)
            .uri(path)
            .header(PLATFORM_ID_HEADER, "p")
            .header(COMPANY_ID_HEADER, "a")
            .header("content-type", "application/json");
        if token {
            r = if value.get("projectId").is_some() {
                r.header("authorization", format!("Bearer {TOKEN}"))
            } else {
                r.header("x-agent-apps-token", TOKEN)
            };
        }
        let response =
            self.router().oneshot(r.body(Body::from(value.to_string())).unwrap()).await.unwrap();
        if value.get("projectId").is_some() {
            assert_eq!(
                response.headers().get("cache-control").and_then(|v| v.to_str().ok()),
                Some("no-store")
            );
        }
        let status = response.status();
        let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
    }
    async fn read(&self) -> (StatusCode, Value) {
        self.call(
            "POST",
            "/app-providers/omp/config/state",
            json!({"targetHostId":"host","projectId":"project"}),
            true,
        )
        .await
    }
}
#[tokio::test]
async fn aggregation_t49_aggregation_omp_preserves_contract() {
    let f = Fixture::new();
    let snapshot = files(f.root.path());
    let (status, state) = f.read().await;
    assert_eq!(files(f.root.path()), snapshot);
    assert_eq!(status, StatusCode::OK, "{state}");
    assert_eq!(state["profile"], json!(f.profile()));
    assert_eq!(state["defaultProviderId"], "builtin");
    assert_eq!(state["defaultModelId"], "default-model");
    assert_eq!(state["members"], json!(["other"]));
    assert!(state["revision"].as_str().is_some_and(|s| !s.is_empty()));
}

impl Fixture {
    fn bytes(&self) -> Vec<u8> {
        std::fs::read(self.dir().join("models.yml")).unwrap()
    }
    fn grant(&self, read: bool, write: bool, aggregation_read: bool, aggregation_write: bool) {
        let mut v: Value = serde_json::from_slice(&std::fs::read(&self.policy).unwrap()).unwrap();
        let grant = &mut v["principals"][0]["grants"][0];
        grant["provider_config_read"] = json!(read);
        grant["provider_config_write"] = json!(write);
        let permits =
            json!([{"project_id":"project","app_id":"omp"},{"project_id":"other","app_id":"omp"}]);
        grant["aggregation_read"] = if aggregation_read { permits.clone() } else { json!([]) };
        grant["aggregation_write"] = if aggregation_write { permits } else { json!([]) };
        std::fs::write(&self.policy, v.to_string()).unwrap();
    }
    async fn change(&self, revision: &Value, change: Value) -> (StatusCode, Value) {
        self.call("PUT", "/app-providers/omp/managed/directory-entry", json!({"targetHostId":"host","projectId":"project","expectedRevision":revision,"change":change}), true).await
    }
}
fn insert(model: &str) -> Value {
    json!({"kind":"insert","profile":{"name":"Managed","declaration":{"baseUrl":"https://fixture.invalid/v1","api":"openai-completions","apiKey":"synthetic-test-key","models":[{"id":model}]}}})
}
#[tokio::test]
async fn aggregation_t49_writes_only_selected_profile_and_target_entry() {
    let f = Fixture::new();
    f.grant(true, true, true, true);
    let before = f.bytes();
    let inactive = if f.profile().is_some() {
        f.root.path().join(".omp/agent")
    } else {
        f.root.path().join(".omp/profiles/inactive/agent")
    };
    std::fs::create_dir_all(&inactive).unwrap();
    std::fs::write(inactive.join("models.yml"), b"# unrelated profile\n").unwrap();
    std::fs::write(
        f.dir().join("mcp.json"),
        b"{\"mcpServers\":{\"kept\":{\"command\":\"fixture\"}}}",
    )
    .unwrap();
    let original = f.read().await.1;
    let (status, added) = f.change(&original["revision"], insert("first")).await;
    assert_eq!(status, StatusCode::OK, "{added}");
    assert_eq!(added["members"], json!(["managed", "other"]));
    assert!(!added.to_string().contains("synthetic-test-key"));
    assert_eq!(
        std::fs::metadata(f.dir().join("models.yml")).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_ne!(added["revision"], original["revision"]);
    let written = String::from_utf8(f.bytes()).unwrap();
    assert!(written.starts_with("# user header\nnotes: keep\nproviders:\n  other:\n    baseUrl: https://other.invalid # keep\n"));
    assert!(written.ends_with("\n# footer\n"));
    let mut replace = insert("second");
    replace["kind"] = json!("replace");
    let (status, replaced) = f.change(&added["revision"], replace).await;
    assert_eq!(status, StatusCode::OK, "{replaced}");
    let native = vw_agent_apps::providers::omp::declarations(Some(&f.bytes())).unwrap();
    assert_eq!(native["managed"].declaration["models"][0]["id"], "second");
    let (status, removed) = f.change(&replaced["revision"], json!({"kind":"remove"})).await;
    assert_eq!(status, StatusCode::OK, "{removed}");
    assert_eq!(f.bytes(), before);
    let (status, again) = f.change(&removed["revision"], json!({"kind":"remove"})).await;
    assert_eq!(status, StatusCode::OK, "{again}");
    assert_eq!(again, removed);
    assert_eq!(std::fs::read(inactive.join("models.yml")).unwrap(), b"# unrelated profile\n");
    assert_eq!(
        std::fs::read(f.dir().join("config.yml")).unwrap(),
        b"modelRoles:\n  default: builtin/default-model\n"
    );
    assert_eq!(std::fs::read(f.dir().join("agent.db")).unwrap(), b"login fixture");
    assert_eq!(
        std::fs::read(f.dir().join("mcp.json")).unwrap(),
        b"{\"mcpServers\":{\"kept\":{\"command\":\"fixture\"}}}"
    );
    assert_eq!(f.tx.state(AppId::Omp).unwrap().project_scope, Some(scope("project")));
    assert!(f.tx.pending(AppId::Omp).unwrap().is_none());
}
#[tokio::test]
async fn aggregation_t49_denies_missing_grants_and_foreign_scope() {
    let f = Fixture::new();
    f.grant(true, true, true, true);
    let before = f.bytes();
    let original = f.read().await.1;
    for (host, project, token, status) in [
        ("host", "project", false, StatusCode::UNAUTHORIZED),
        ("elsewhere", "project", true, StatusCode::FORBIDDEN),
        ("host", "ungranted", true, StatusCode::FORBIDDEN),
        ("host", "other", true, StatusCode::FORBIDDEN),
    ] {
        assert_eq!(
            f.call(
                "POST",
                "/app-providers/omp/config/state",
                json!({"targetHostId":host,"projectId":project}),
                token
            )
            .await
            .0,
            status
        );
    }
    for (read, write, ar, aw) in [(false, true, true, true), (true, true, false, true)] {
        f.grant(read, write, ar, aw);
        assert_eq!(f.read().await.0, StatusCode::FORBIDDEN);
    }
    for (read, write, ar, aw) in [(true, false, true, true), (true, true, true, false)] {
        f.grant(read, write, ar, aw);
        assert_eq!(f.change(&original["revision"], insert("one")).await.0, StatusCode::FORBIDDEN);
    }
    f.grant(true, true, true, true);
    assert_eq!(
        f.call("POST", "/app-providers/omp/config/state", json!({"targetHostId":"host"}), true)
            .await
            .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        f.call(
            "POST",
            "/app-providers/omp/config/state",
            json!({"targetHostId":"host","projectId":"project","mcpServers":{}}),
            true
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(f.bytes(), before);
}
#[tokio::test]
async fn aggregation_t49_conflicts_and_invalid_yaml_leave_native_bytes_untouched() {
    let f = Fixture::new();
    f.grant(true, true, true, true);
    let before = f.bytes();
    let state = f.read().await.1;
    let (status, body) = f.change(&json!("stale"), insert("one")).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"]["code"], "revision_conflict");
    assert_eq!(f.bytes(), before);
    let mut replace = insert("one");
    replace["kind"] = json!("replace");
    assert_eq!(f.change(&state["revision"], replace).await.0, StatusCode::NOT_FOUND);
    let mut mcp = insert("one");
    mcp["profile"]["declaration"]["mcpServers"] = json!({"unsafe":{"command":"fixture"}});
    assert_eq!(f.change(&state["revision"], mcp).await.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(f.bytes(), before);
    let (status, added) = f.change(&state["revision"], insert("one")).await;
    assert_eq!(status, StatusCode::OK, "{added}");
    let bytes = f.bytes();
    assert_eq!(f.change(&added["revision"], insert("two")).await.0, StatusCode::CONFLICT);
    assert_eq!(f.bytes(), bytes);
    std::fs::write(f.dir().join("config.yml"), b"modelRoles:\n  default: builtin/changed\n")
        .unwrap();
    assert_eq!(
        f.change(&added["revision"], json!({"kind":"remove"})).await.0,
        StatusCode::CONFLICT
    );
    assert_eq!(f.bytes(), bytes);
    std::fs::write(
        f.dir().join("models.yml"),
        b"providers:\n  managed: &shared\n    apiKey: fixture\n",
    )
    .unwrap();
    let yaml = f.bytes();
    let state = f.read().await.1;
    assert_eq!(
        f.change(&state["revision"], json!({"kind":"remove"})).await.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(f.bytes(), yaml);
    std::fs::write(f.dir().join("config.yml"), b"modelRoles: [broken").unwrap();
    let (status, body) = f.read().await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["code"], "invalid_configuration");
}
#[tokio::test]
async fn aggregation_t49_unclaimed_slot_symlinks_and_unknown_fields_are_rejected() {
    let f = Fixture::new();
    f.grant(true, true, true, true);
    let state = f.read().await.1;
    let mut body = json!({"targetHostId":"host","projectId":"project","expectedRevision":state["revision"],"change":{"kind":"remove","mcpServers":{}}});
    assert_eq!(
        f.call("PUT", "/app-providers/omp/managed/directory-entry", body.clone(), true).await.0,
        StatusCode::BAD_REQUEST
    );
    body["change"] = json!({"kind":"remove"});
    body["profile"] = json!("other");
    assert_eq!(
        f.call("PUT", "/app-providers/omp/managed/directory-entry", body, true).await.0,
        StatusCode::BAD_REQUEST
    );
    let path = f.dir().join("models.yml");
    std::fs::rename(&path, f.dir().join("original.yml")).unwrap();
    std::os::unix::fs::symlink(f.dir().join("original.yml"), path).unwrap();
    assert_ne!(f.read().await.0, StatusCode::OK);
    // Restore and remove only this fixture's persisted binding to model a legacy slot.
    std::fs::remove_file(f.dir().join("models.yml")).unwrap();
    std::fs::rename(f.dir().join("original.yml"), f.dir().join("models.yml")).unwrap();
    let path = f.root.path().join("private/provider-live/aG9zdA/omp.state.json");
    let mut v: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    v.as_object_mut().unwrap().remove("project_scope");
    v.as_object_mut().unwrap().remove("projectScope");
    std::fs::write(path, v.to_string()).unwrap();
    assert_eq!(f.read().await.0, StatusCode::CONFLICT);
}
fn files(root: &std::path::Path) -> std::collections::BTreeMap<PathBuf, Vec<u8>> {
    fn visit(
        root: &std::path::Path,
        path: &std::path::Path,
        out: &mut std::collections::BTreeMap<PathBuf, Vec<u8>>,
    ) {
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(root, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(root).unwrap().to_path_buf(),
                    std::fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut out = std::collections::BTreeMap::new();
    visit(root, root, &mut out);
    out
}
#[tokio::test]
async fn aggregation_t49_pending_intent_is_not_recovered_by_read_or_write() {
    use vw_agent_apps::providers::live::{
        stage::FileChange,
        write_guard::{FilePrivacy, WriteError},
    };
    let f = Fixture::new();
    f.grant(true, true, true, true);
    let revision = f.read().await.1["revision"].clone();
    let mut guard = lock_app(AppId::Omp, f.root.path()).unwrap();
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
            &f.tx.state(AppId::Omp).unwrap(),
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
    assert!(f.tx.pending(AppId::Omp).unwrap().is_some());
    let snapshot = files(f.root.path());
    let (status, body) = f.read().await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"]["code"], "recovery_pending");
    assert_eq!(f.change(&revision, insert("one")).await.0, StatusCode::CONFLICT);
    assert_eq!(files(f.root.path()), snapshot);
}
