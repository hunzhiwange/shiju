use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
    middleware,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::os::unix::fs::PermissionsExt;
use tower::ServiceExt;
use vw_agent_apps::providers::live::{commit::*, write_guard::lock_app};
use vw_api_types::{
    agent_apps::AppId,
    id::{COMPANY_ID_HEADER, PLATFORM_ID_HEADER, ProjectId, ProjectScope, TenantScope},
};
const TOKEN: &str = "aggregation-t48-private-fixture-bearer-token";
struct Fixture {
    dir: tempfile::TempDir,
    policy: std::path::PathBuf,
    tx: ProviderTransactions,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let root = dir.path().join("private");
        std::fs::create_dir(&root).unwrap();
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
        let tx = ProviderTransactions::new(&root, ExecutionHostId::new("host").unwrap()).unwrap();
        let scope = ProjectScope::new(
            TenantScope::new("platform", "company").unwrap(),
            ProjectId::from("project"),
        )
        .unwrap();
        let mut guard = lock_app(AppId::Pi, dir.path()).unwrap();
        tx.commit_project_checked(
            &scope,
            &mut guard,
            &tx.state(AppId::Pi).unwrap(),
            true,
            &[],
            PendingTarget::default(),
            || Ok(()),
        )
        .unwrap();
        drop(guard);
        let native = dir.path().join(".pi/agent");
        std::fs::create_dir_all(&native).unwrap();
        std::fs::write(
            native.join("settings.json"),
            br#"{"defaultProvider":"native","defaultModel":"selected"}"#,
        )
        .unwrap();
        std::fs::write(
            native.join("auth.json"),
            br#"{"native":{"type":"api_key","key":"synthetic-only"}}"#,
        )
        .unwrap();
        let policy = dir.path().join("access.json");
        let f = Self { dir, policy, tx };
        f.policy(true);
        f
    }
    fn policy(&self, write: bool) {
        let permits =
            json!([{"project_id":"project","app_id":"pi"},{"project_id":"other","app_id":"pi"}]);
        std::fs::write(&self.policy,json!({"principals":[{"id":"editor","token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":[{"platform_id":"platform","company_id":"company","host_id":"host","provider_config_read":true,"provider_config_write":write,"aggregation_read":permits,"aggregation_write":if write {permits} else {json!([])}}]}],"hosts":[{"id":"host","online":true,"binding":"local","home":self.dir.path(),"credential_root":self.dir.path().join("private"),"applications":self.dir.path().join("Applications"),"search_paths":[]}]}).to_string()).unwrap();
        std::fs::set_permissions(&self.policy, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    fn router(&self) -> Router {
        super::pi::router_with_policy(Some(self.policy.clone()))
            .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware))
    }
    async fn request(&self, method: &str, body: Value, token: bool) -> (StatusCode, Value) {
        let path = if method == "POST" {
            "/app-providers/pi/config/state"
        } else {
            "/app-providers/pi/fixture/directory-entry"
        };
        let mut req = Request::builder()
            .method(method)
            .uri(path)
            .header(PLATFORM_ID_HEADER, "platform")
            .header(COMPANY_ID_HEADER, "company")
            .header("content-type", "application/json");
        if token {
            req = req.header("authorization", format!("Bearer {TOKEN}"));
            if body.get("projectId").is_none() {
                req = req.header("x-agent-apps-token", TOKEN);
            }
        }
        let scoped = body.get("projectId").is_some();
        let response =
            self.router().oneshot(req.body(Body::from(body.to_string())).unwrap()).await.unwrap();
        if scoped {
            assert_eq!(response.headers().get("cache-control").unwrap(), "no-store");
        }
        let status = response.status();
        let bytes = to_bytes(response.into_body(), 2 * 1024 * 1024).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
    }
    async fn read(&self) -> Value {
        let (status, value) =
            self.request("POST", json!({"targetHostId":"host","projectId":"project"}), true).await;
        assert_eq!(status, StatusCode::OK, "{value}");
        value
    }
    fn native(&self, name: &str) -> Vec<u8> {
        std::fs::read(self.dir.path().join(".pi/agent").join(name)).unwrap()
    }
}
fn profile(model: &str) -> Value {
    json!({"name":"Fixture","declaration":{"api":"openai-completions","baseUrl":"https://fixture.invalid/v1","models":[{"id":model,"name":model,"contextWindow":16000,"maxTokens":2048}]}})
}
#[tokio::test]
async fn aggregation_t48_aggregation_pi_preserves_contract() {
    let f = Fixture::new();
    let settings = f.native("settings.json");
    let auth = f.native("auth.json");
    let initial = f.read().await;
    assert_eq!(initial["defaultProviderId"], "native");
    assert_eq!(initial["defaultModelId"], "selected");
    let mut revision = initial["revision"].clone();
    for (kind, model) in [("insert", "first"), ("replace", "second"), ("remove", "")] {
        let mut change = json!({"kind":kind});
        if kind != "remove" {
            change["profile"] = profile(model);
        }
        let (status,state)=f.request("PUT",json!({"targetHostId":"host","projectId":"project","expectedRevision":revision,"change":change}),true).await;
        assert_eq!(status, StatusCode::OK, "{state}");
        revision = state["revision"].clone();
        assert_eq!(state["defaultProviderId"], "native");
        assert_eq!(f.native("settings.json"), settings);
        assert_eq!(f.native("auth.json"), auth);
        let models: Value = serde_json::from_slice(&f.native("models.json")).unwrap();
        if kind == "remove" {
            assert!(models["providers"].get("fixture").is_none());
        } else {
            assert_eq!(models["providers"]["fixture"]["models"][0]["id"], model);
        }
    }
    assert_eq!(
        f.tx.state(AppId::Pi).unwrap().project_scope.unwrap().project_id().as_ref(),
        "project"
    );
}
#[tokio::test]
async fn aggregation_t48_rejects_unauthorized_and_cross_project_access() {
    let f = Fixture::new();
    for (host, project, token, status) in [
        ("host", "project", false, StatusCode::UNAUTHORIZED),
        ("foreign", "project", true, StatusCode::FORBIDDEN),
        ("host", "missing", true, StatusCode::FORBIDDEN),
        ("host", "other", true, StatusCode::FORBIDDEN),
    ] {
        assert_eq!(
            f.request("POST", json!({"targetHostId":host,"projectId":project}), token).await.0,
            status
        );
    }
    let revision = f.read().await["revision"].clone();
    f.policy(false);
    assert_eq!(f.request("PUT",json!({"targetHostId":"host","projectId":"project","expectedRevision":revision,"change":{"kind":"remove"}}),true).await.0,StatusCode::FORBIDDEN);
    assert!(!f.dir.path().join(".pi/agent/models.json").exists());
}
#[tokio::test]
async fn aggregation_t48_revision_and_strict_entry_semantics() {
    let f = Fixture::new();
    let first = f.read().await;
    let revision = first["revision"].clone();
    let request = |revision: Value, change: Value| json!({"targetHostId":"host","projectId":"project","expectedRevision":revision,"change":change});
    assert_eq!(
        f.request(
            "PUT",
            request(revision.clone(), json!({"kind":"replace","profile":profile("first")})),
            true
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        f.request("PUT", request(revision.clone(), json!({"kind":"remove"})), true).await.0,
        StatusCode::OK
    );
    assert!(!f.dir.path().join(".pi/agent/models.json").exists());
    let (_, inserted) = f
        .request(
            "PUT",
            request(revision.clone(), json!({"kind":"insert","profile":profile("first")})),
            true,
        )
        .await;
    let before = f.native("models.json");
    assert_eq!(
        f.request("PUT", request(revision, json!({"kind":"remove"})), true).await.0,
        StatusCode::CONFLICT
    );
    assert_eq!(f.native("models.json"), before);
    assert_eq!(
        f.request(
            "PUT",
            request(
                inserted["revision"].clone(),
                json!({"kind":"insert","profile":profile("first")})
            ),
            true
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let mut secret = profile("first");
    secret["declaration"]["apiKey"] = json!("synthetic-secret");
    assert_eq!(
        f.request(
            "PUT",
            request(inserted["revision"].clone(), json!({"kind":"replace","profile":secret})),
            true
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(f.native("models.json"), before);
    std::fs::write(
        f.dir.path().join(".pi/agent/settings.json"),
        br#"{"defaultProvider":"native","defaultModel":"external"}"#,
    )
    .unwrap();
    assert_eq!(
        f.request("PUT", request(inserted["revision"].clone(), json!({"kind":"remove"})), true)
            .await
            .0,
        StatusCode::CONFLICT
    );
    assert_ne!(f.read().await["revision"], inserted["revision"]);
}
#[tokio::test]
async fn aggregation_t48_unknown_fields_and_legacy_scope_bypass_are_rejected() {
    let f = Fixture::new();
    for extra in ["path", "accountId", "defaultModelId"] {
        let mut query = json!({"targetHostId":"host","projectId":"project"});
        query[extra] = json!("injected");
        assert_eq!(f.request("POST", query, true).await.0, StatusCode::BAD_REQUEST);
    }
    assert_eq!(
        f.request("POST", json!({"targetHostId":"host"}), true).await.0,
        StatusCode::CONFLICT
    );
    let revision = f.read().await["revision"].clone();
    for change in [
        json!({"kind":"remove","profile":profile("first")}),
        json!({"kind":"login"}),
        json!({"kind":"insert","profile":profile("first"),"path":"injected"}),
    ] {
        assert_eq!(f.request("PUT",json!({"targetHostId":"host","projectId":"project","expectedRevision":revision,"change":change}),true).await.0,StatusCode::BAD_REQUEST);
    }
}
#[tokio::test]
async fn aggregation_t48_preserves_other_providers_and_native_secrets() {
    let f = Fixture::new();
    let pre=br#"{"providers":{"other": { "future": true },"fixture":{"baseUrl":"https://fixture.invalid/v1","api":"openai-completions","apiKey":"synthetic-existing","models":[{"id":"first","name":"first","compat":{"supportsDeveloperRole":true}}]}},"future":17}"#;
    std::fs::write(f.dir.path().join(".pi/agent/models.json"), pre).unwrap();
    let revision = f.read().await["revision"].clone();
    let (status,response)=f.request("PUT",json!({"targetHostId":"host","projectId":"project","expectedRevision":revision,"change":{"kind":"replace","profile":profile("first")}}),true).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!response.to_string().contains("synthetic-existing"));
    let bytes = f.native("models.json");
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["providers"]["fixture"]["apiKey"], "synthetic-existing");
    assert_eq!(value["providers"]["fixture"]["models"][0]["compat"]["supportsDeveloperRole"], true);
    assert_eq!(value["future"], 17);
    let other = br#""other": { "future": true }"#;
    assert!(bytes.windows(other.len()).any(|w| w == other));
}
#[tokio::test]
async fn aggregation_t48_rejects_unbound_and_unsafe_native_targets() {
    let f = Fixture::new();
    std::os::unix::fs::symlink(
        f.dir.path().join(".pi/agent/auth.json"),
        f.dir.path().join(".pi/agent/models.json"),
    )
    .unwrap();
    assert_eq!(
        f.request("POST", json!({"targetHostId":"host","projectId":"project"}), true).await.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    std::fs::remove_file(f.dir.path().join(".pi/agent/models.json")).unwrap();
    // A genuinely unused transaction owner is not claimed by a read.
    let fresh = f.dir.path().join("fresh");
    std::fs::create_dir(&fresh).unwrap();
    std::fs::set_permissions(&fresh, std::fs::Permissions::from_mode(0o700)).unwrap();
    let tx = ProviderTransactions::new(&fresh, ExecutionHostId::new("host").unwrap()).unwrap();
    let mut policy: Value = serde_json::from_slice(&std::fs::read(&f.policy).unwrap()).unwrap();
    policy["hosts"][0]["credential_root"] = json!(fresh);
    std::fs::write(&f.policy, policy.to_string()).unwrap();
    assert_eq!(
        f.request("POST", json!({"targetHostId":"host","projectId":"project"}), true).await.0,
        StatusCode::CONFLICT
    );
    assert!(tx.state(AppId::Pi).unwrap().project_scope.is_none());
}
#[tokio::test]
async fn aggregation_t48_pending_transaction_is_not_recovered_by_directory_api() {
    use vw_agent_apps::providers::live::{
        stage::FileChange,
        write_guard::{FilePrivacy, WriteError},
    };
    let f = Fixture::new();
    let revision = f.read().await["revision"].clone();
    let mut guard = lock_app(AppId::Pi, f.dir.path()).unwrap();
    let patch = |_: Option<&[u8]>| Ok(b"staged".to_vec());
    let changes = [FileChange {
        name: std::ffi::OsStr::new("staged-native"),
        privacy: FilePrivacy::Private,
        patch: &patch,
    }];
    let scope = ProjectScope::new(
        TenantScope::new("platform", "company").unwrap(),
        ProjectId::from("project"),
    )
    .unwrap();
    let calls = std::cell::Cell::new(0);
    assert_eq!(
        f.tx.commit_project_checked(
            &scope,
            &mut guard,
            &f.tx.state(AppId::Pi).unwrap(),
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
    let pending = f.tx.pending(AppId::Pi).unwrap().unwrap();
    for method in ["POST", "PUT"] {
        let mut body = json!({"targetHostId":"host","projectId":"project"});
        if method == "PUT" {
            body["expectedRevision"] = revision.clone();
            body["change"] = json!({"kind":"remove"});
        }
        let (status, response) = f.request(method, body, true).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(response["error"]["code"], "recovery_pending");
    }
    assert_eq!(
        serde_json::to_value(f.tx.pending(AppId::Pi).unwrap().unwrap()).unwrap(),
        serde_json::to_value(pending).unwrap()
    );
    assert!(!f.dir.path().join("staged-native").exists());
}
#[tokio::test]
async fn aggregation_t48_host_directory_override_controls_native_target() {
    let f = Fixture::new();
    let custom = f.dir.path().join("custom-pi");
    std::fs::create_dir(&custom).unwrap();
    std::fs::write(
        custom.join("settings.json"),
        br#"{"defaultProvider":"override","defaultModel":"custom"}"#,
    )
    .unwrap();
    let mut policy: Value = serde_json::from_slice(&std::fs::read(&f.policy).unwrap()).unwrap();
    policy["hosts"][0]["pi_directory"] = json!(custom);
    std::fs::write(&f.policy, policy.to_string()).unwrap();
    let state = f.read().await;
    assert_eq!(state["defaultProviderId"], "override");
    let (status,_)=f.request("PUT",json!({"targetHostId":"host","projectId":"project","expectedRevision":state["revision"],"change":{"kind":"insert","profile":profile("custom")}}),true).await;
    assert_eq!(status, StatusCode::OK);
    assert!(custom.join("models.json").exists());
    assert!(!f.dir.path().join(".pi/agent/models.json").exists());
}
#[tokio::test]
async fn aggregation_t48_unavailable_host_and_wrong_tenant_fail_without_changes() {
    let f = Fixture::new();
    let settings = f.native("settings.json");
    let response = f
        .router()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/app-providers/pi/config/state")
                .header(PLATFORM_ID_HEADER, "platform")
                .header(COMPANY_ID_HEADER, "foreign")
                .header("authorization", format!("Bearer {TOKEN}"))
                .header("content-type", "application/json")
                .body(Body::from(json!({"targetHostId":"host","projectId":"project"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let mut policy: Value = serde_json::from_slice(&std::fs::read(&f.policy).unwrap()).unwrap();
    policy["hosts"][0]["online"] = json!(false);
    std::fs::write(&f.policy, policy.to_string()).unwrap();
    let (status, response) =
        f.request("POST", json!({"targetHostId":"host","projectId":"project"}), true).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(response["error"]["code"], "target_unavailable");
    assert_eq!(f.native("settings.json"), settings);
    assert!(!f.dir.path().join(".pi/agent/models.json").exists());
}
#[tokio::test]
async fn aggregation_t48_legacy_native_operations_cannot_bypass_project_binding() {
    let f = Fixture::new();
    let settings = f.native("settings.json");
    let auth = f.native("auth.json");
    for path in ["config/import", "fixture/form/read", "fixture/add", "fixture/remove-live"] {
        let response = f
            .router()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/app-providers/pi/{path}"))
                    .header(PLATFORM_ID_HEADER, "platform")
                    .header(COMPANY_ID_HEADER, "company")
                    .header("x-agent-apps-token", TOKEN)
                    .header("content-type", "application/json")
                    .body(Body::from(json!({"targetHostId":"host"}).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT, "{path}");
    }
    assert_eq!(f.native("settings.json"), settings);
    assert_eq!(f.native("auth.json"), auth);
    assert!(!f.dir.path().join(".pi/agent/models.json").exists());
}
