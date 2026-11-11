//! Real HTTP, policy, native files and persistent transaction behavior.
#![cfg(target_os = "macos")]
use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
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
use vw_agent_apps::providers::live::{
    commit::{ExecutionHostId, OmoSelection, PendingTarget, ProviderTransactions},
    write_guard::lock_app,
};
use vw_api_types::{
    agent_apps::AppId,
    id::{ProjectId, ProjectScope, TenantScope},
};
const TOKEN: &str = "t47-synthetic-bearer-credential-000000000000";
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
        std::fs::create_dir(root.path().join("private")).unwrap();
        std::fs::set_permissions(
            root.path().join("private"),
            std::fs::Permissions::from_mode(0o700),
        )
        .unwrap();
        let db =
            GlobalDatabase::new(vw_config_types::paths::global_database_path(root.path())).unwrap();
        let store = GatewaySettingsStore::new(db.clone()).unwrap();
        for (id, category) in [("standard", "omo"), ("slim", "omo-slim")] {
            store.upsert(&tenant(),"app_providers.records",&format!("\"opencode\":{id}"),&json!({"name":id,"category":category,"settingsConfig":{"agents":{"builder":{"model":"vendor/model"}},"categories":{"quick":{"model":"vendor/fast"}},"user_option":true}}).to_string()).unwrap();
        }
        let tx = ProviderTransactions::new(
            &root.path().join("private"),
            ExecutionHostId::new("host").unwrap(),
        )
        .unwrap();
        let mut guard = lock_app(AppId::OpenCode, root.path()).unwrap();
        tx.commit_project_checked(
            &scope("project"),
            &mut guard,
            &tx.state(AppId::OpenCode).unwrap(),
            true,
            &[],
            PendingTarget::default(),
            || Ok(()),
        )
        .unwrap();
        drop(guard);
        std::fs::create_dir(root.path().join(".omo")).unwrap();
        std::fs::write(
            root.path().join(".omo/omo.jsonc"),
            b"{ // preserved\n\"[codex]\": {\"keep\":true}\n}\n",
        )
        .unwrap();
        let policy = root.path().join("access.json");
        let f = Self { root, db, tx, policy };
        f.permissions(true, true, true, true);
        f
    }
    fn permissions(&self, read: bool, agg_read: bool, write: bool, agg_write: bool) {
        let permits = json!([{"project_id":"project","app_id":"opencode"},{"project_id":"other","app_id":"opencode"}]);
        std::fs::write(&self.policy,json!({"principals":[{"id":"operator","token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":[{"platform_id":"platform","company_id":"company","host_id":"host","provider_config_read":read,"provider_config_write":write,"aggregation_read":if agg_read{permits.clone()}else{json!([])},"aggregation_write":if agg_write{permits}else{json!([])}}]}],"hosts":[{"id":"host","online":true,"binding":"local","home":self.root.path(),"credential_root":self.root.path().join("private"),"applications":self.root.path().join("Applications"),"search_paths":[]}]}).to_string()).unwrap();
        std::fs::set_permissions(&self.policy, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    fn router(&self) -> Router {
        super::aggregation_omo::router_with_policy(Some(self.policy.clone()))
            .layer(Extension(self.db.clone()))
            .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware))
    }
    async fn request(
        &self,
        variant: &str,
        action: Option<&str>,
        token: Option<&str>,
        project: &str,
        extra: Value,
    ) -> (StatusCode, Value) {
        let path = match action {
            Some(action) => format!("/agent-apps/opencode/{variant}/{action}"),
            None => format!("/agent-apps/opencode/{variant}?targetHostId=host&projectId={project}"),
        };
        let mut request = Request::builder()
            .method(if action.is_some() { "POST" } else { "GET" })
            .uri(path)
            .header("x-platform-id", "platform")
            .header("x-company-id", "company");
        if let Some(token) = token {
            request = request.header("authorization", format!("Bearer {token}"));
        }
        let body = if action.is_some() {
            let mut body = json!({"targetHostId":"host","projectId":project});
            for (k, v) in extra.as_object().unwrap() {
                body[k] = v.clone();
            }
            request = request.header("content-type", "application/json");
            Body::from(body.to_string())
        } else {
            Body::empty()
        };
        let response = self.router().oneshot(request.body(body).unwrap()).await.unwrap();
        assert_eq!(response.headers().get("cache-control").unwrap(), "no-store");
        let status = response.status();
        let bytes = to_bytes(response.into_body(), 65536).await.unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains(TOKEN));
        (status, serde_json::from_slice(&bytes).unwrap())
    }
    async fn read(&self, variant: &str) -> Value {
        let (status, value) = self.request(variant, None, Some(TOKEN), "project", json!({})).await;
        assert_eq!(status, 200, "{value}");
        value
    }
    async fn mutate(
        &self,
        variant: &str,
        action: &str,
        revision: &Value,
        provider: Option<&str>,
    ) -> (StatusCode, Value) {
        let mut body = json!({"expectedRevision":revision});
        if let Some(id) = provider {
            body["providerId"] = json!(id);
        }
        self.request(variant, Some(action), Some(TOKEN), "project", body).await
    }
}
#[tokio::test]
async fn aggregation_t47_aggregation_omo_preserves_contract() {
    let f = Fixture::new();
    for variant in ["omo", "omo-slim"] {
        let (status, _) = f.request(variant, None, None, "project", json!({})).await;
        assert_eq!(status, 401);
        let initial = f.read(variant).await;
        assert!(initial["currentProviderId"].is_null());
        let provider = if variant == "omo" { "standard" } else { "slim" };
        let (status, applied) =
            f.mutate(variant, "apply", &initial["revision"], Some(provider)).await;
        assert_eq!(status, 200, "{applied}");
        assert_eq!(applied["currentProviderId"], provider);
        assert_eq!(applied["agents"]["builder"]["model"], "vendor/model");
        if variant == "omo-slim" {
            assert!(applied.get("categories").is_none());
            assert!(applied["otherFields"].get("categories").is_none());
        } else {
            assert_eq!(applied["categories"]["quick"]["model"], "vendor/fast");
        }
        assert_eq!(f.read(variant).await, applied);
        let (status, disabled) = f.mutate(variant, "disable", &applied["revision"], None).await;
        assert_eq!(status, 200, "{disabled}");
        assert!(disabled["currentProviderId"].is_null());
        let (status, again) = f.mutate(variant, "disable", &disabled["revision"], None).await;
        assert_eq!(status, 200, "{again}");
        assert_eq!(disabled, again);
    }
    let raw = std::fs::read_to_string(f.root.path().join(".omo/omo.jsonc")).unwrap();
    assert!(raw.contains("// preserved"));
    assert!(raw.contains("\"[codex]\": {\"keep\":true}"));
    assert_eq!(f.tx.state(AppId::OpenCode).unwrap().omo, Some(OmoSelection::Disabled));
    assert!(f.tx.pending(AppId::OpenCode).unwrap().is_none());
}
#[tokio::test]
async fn aggregation_t47_scope_and_permissions_precede_writes() {
    let f = Fixture::new();
    let state = f.read("omo").await;
    let before = std::fs::read(f.root.path().join(".omo/omo.jsonc")).unwrap();
    for variant in ["omo", "omo-slim"] {
        for action in ["apply", "disable"] {
            let extra = if action == "apply" {
                json!({"expectedRevision":state["revision"],"providerId":"standard"})
            } else {
                json!({"expectedRevision":state["revision"]})
            };
            for (project, token, status) in [
                ("project", None, 401),
                ("other", Some(TOKEN), 403),
                ("ungranted", Some(TOKEN), 403),
            ] {
                assert_eq!(
                    f.request(variant, Some(action), token, project, extra.clone()).await.0,
                    status
                );
            }
        }
    }
    for (read, ar, write, aw) in [(true, true, false, true), (true, true, true, false)] {
        f.permissions(read, ar, write, aw);
        assert_eq!(f.mutate("omo", "apply", &state["revision"], Some("standard")).await.0, 403);
    }
    for (read, ar) in [(false, true), (true, false)] {
        f.permissions(read, ar, true, true);
        assert_eq!(f.request("omo", None, Some(TOKEN), "project", json!({})).await.0, 403);
    }
    assert_eq!(std::fs::read(f.root.path().join(".omo/omo.jsonc")).unwrap(), before);
}
#[tokio::test]
async fn aggregation_t47_stale_revision_and_variants_are_explicit_conflicts() {
    let f = Fixture::new();
    let initial = f.read("omo").await;
    let slim_revision = f.read("omo-slim").await["revision"].clone();
    assert_ne!(initial["revision"], slim_revision);
    assert_eq!(f.mutate("omo-slim", "apply", &initial["revision"], Some("slim")).await.0, 409);

    assert_eq!(f.mutate("omo", "apply", &json!("stale"), Some("standard")).await.0, 409);
    let (status, active) = f.mutate("omo", "apply", &initial["revision"], Some("standard")).await;
    assert_eq!(status, 200, "{active}");
    let slim = f.read("omo-slim").await;
    let (status, error) = f.mutate("omo-slim", "apply", &slim["revision"], Some("slim")).await;
    assert_eq!(status, 409);
    assert_eq!(error["error"]["code"], "variant_conflict");
    let (status, inactive) = f.mutate("omo-slim", "disable", &slim["revision"], None).await;
    assert_eq!(status, 200, "{inactive}");
    assert_eq!(f.read("omo").await["currentProviderId"], "standard");
    assert_eq!(f.mutate("omo", "disable", &initial["revision"], None).await.0, 409);
    let revision = f.read("omo").await["revision"].clone();
    std::fs::write(f.root.path().join(".omo/omo.jsonc"), b"{\"[codex]\":{\"changed\":true}}")
        .unwrap();
    assert_eq!(f.mutate("omo", "disable", &revision, None).await.0, 409);
}
#[tokio::test]
async fn aggregation_t47_profiles_and_unknown_inputs_are_rejected() {
    let f = Fixture::new();
    let state = f.read("omo").await;
    assert_eq!(f.mutate("omo", "apply", &state["revision"], Some("slim")).await.0, 422);
    assert_eq!(f.mutate("omo", "apply", &state["revision"], Some("missing")).await.0, 404);
    assert_eq!(f.request("omo",Some("apply"),Some(TOKEN),"project",json!({"expectedRevision":state["revision"],"providerId":"standard","path":"/tmp/untrusted"})).await.0,400);
    let store = GatewaySettingsStore::new(f.db.clone()).unwrap();
    store
        .upsert(
            &tenant(),
            "app_providers.records",
            "\"opencode\":standard",
            &json!({"category":"omo","settingsConfig":{"apiKey":"synthetic-private"}}).to_string(),
        )
        .unwrap();
    assert_eq!(f.mutate("omo", "apply", &state["revision"], Some("standard")).await.0, 409);
    let state = f.read("omo").await;
    assert_eq!(f.mutate("omo", "apply", &state["revision"], Some("standard")).await.0, 422);
}
#[tokio::test]
async fn aggregation_t47_corrupt_and_unsafe_native_inputs_fail_closed() {
    let f = Fixture::new();
    let state = f.read("omo").await;
    std::fs::write(f.root.path().join(".omo/omo.jsonc"), b"{broken").unwrap();
    assert_eq!(f.request("omo", None, Some(TOKEN), "project", json!({})).await.0, 422);
    assert_eq!(f.mutate("omo", "disable", &state["revision"], None).await.0, 422);
    std::fs::remove_file(f.root.path().join(".omo/omo.jsonc")).unwrap();
    std::os::unix::fs::symlink(&f.policy, f.root.path().join(".omo/omo.jsonc")).unwrap();
    assert_eq!(f.request("omo", None, Some(TOKEN), "project", json!({})).await.0, 503);
}
#[tokio::test]
async fn aggregation_t47_read_is_observational_and_legacy_slots_are_not_claimed() {
    let f = Fixture::new();
    let path = f.root.path().join("private/provider-live/aG9zdA/opencode.state.json");
    let before = std::fs::read(&path).unwrap();
    let native = std::fs::read(f.root.path().join(".omo/omo.jsonc")).unwrap();
    f.read("omo").await;
    f.read("omo-slim").await;
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(std::fs::read(f.root.path().join(".omo/omo.jsonc")).unwrap(), native);
    assert!(!f.root.path().join(".config").exists());
    let mut legacy: Value = serde_json::from_slice(&before).unwrap();
    legacy.as_object_mut().unwrap().remove("project_scope");
    std::fs::write(&path, legacy.to_string()).unwrap();
    let before = std::fs::read(&path).unwrap();
    for variant in ["omo", "omo-slim"] {
        assert_eq!(f.request(variant, None, Some(TOKEN), "project", json!({})).await.0, 409);
    }
    assert_eq!(std::fs::read(&path).unwrap(), before);
}
#[tokio::test]
async fn aggregation_t47_missing_unified_write_is_explicitly_unsupported() {
    let f = Fixture::new();
    std::fs::remove_file(f.root.path().join(".omo/omo.jsonc")).unwrap();
    let state = f.read("omo").await;
    let (status, error) = f.mutate("omo", "apply", &state["revision"], Some("standard")).await;
    assert_eq!(status, 422);
    assert_eq!(error["error"]["code"], "unsupported_target");
    assert!(!f.root.path().join(".omo/omo.jsonc").exists());
    assert!(f.tx.state(AppId::OpenCode).unwrap().omo.is_none());
}
#[tokio::test]
async fn aggregation_t47_pending_selection_recovers_with_the_same_native_transaction() {
    use vw_agent_apps::providers::live::{
        stage::{FileChange, WholeFile},
        write_guard::{FilePrivacy, WriteError},
    };
    let f = Fixture::new();
    let config = f.root.path().join(".omo/omo.jsonc");
    let before = std::fs::read(&config).unwrap();
    let patch = WholeFile::new(Some(&before), Some(b"{\"[opencode]\":{\"agents\":{}}}".to_vec()));
    let changes = [FileChange {
        name: std::ffi::OsStr::new("omo.jsonc"),
        privacy: FilePrivacy::Shared,
        patch: &patch,
    }];
    let mut guard = lock_app(AppId::OpenCode, &f.root.path().join(".omo")).unwrap();
    let calls = std::cell::Cell::new(0);
    assert_eq!(
        f.tx.commit_project_checked(
            &scope("project"),
            &mut guard,
            &f.tx.state(AppId::OpenCode).unwrap(),
            false,
            &changes,
            PendingTarget {
                omo: Some(OmoSelection::Standard("standard".into())),
                ..Default::default()
            },
            || {
                calls.set(calls.get() + 1);
                if calls.get() == 2 { Err(WriteError::Conflict) } else { Ok(()) }
            }
        ),
        Err(WriteError::Conflict)
    );
    drop(guard);
    let pending = f.tx.pending(AppId::OpenCode).unwrap().unwrap();
    assert_eq!(pending.version, 9);
    assert!(f.tx.state(AppId::OpenCode).unwrap().omo.is_none());
    let (status, error) = f.request("omo", None, Some(TOKEN), "project", json!({})).await;
    assert_eq!(status, 409);
    assert_eq!(error["error"]["code"], "recovery_pending");
    assert_eq!(std::fs::read(&config).unwrap(), before);
    // Simulate a crash immediately after the durable publication mark, before rename.
    let journal = f.root.path().join("private/provider-live/aG9zdA/opencode.pending.json");
    let mut intent: Value = serde_json::from_slice(&std::fs::read(&journal).unwrap()).unwrap();
    intent["published"] = json!(true);
    std::fs::write(&journal, intent.to_string()).unwrap();
    let mut guard = lock_app(AppId::OpenCode, &f.root.path().join(".omo")).unwrap();
    f.tx.recover_project(&scope("project"), &mut guard).unwrap();
    drop(guard);
    assert_eq!(f.read("omo").await["currentProviderId"], "standard");
    assert!(f.tx.pending(AppId::OpenCode).unwrap().is_none());
}
#[tokio::test]
async fn aggregation_t47_unavailable_targets_are_distinct_from_invalid_configuration() {
    let f = Fixture::new();
    let initial = f.read("omo").await;
    let before = std::fs::read(f.root.path().join(".omo/omo.jsonc")).unwrap();
    assert_eq!(f.request("omo",Some("apply"),Some(TOKEN),"project",json!({"targetHostId":"other","expectedRevision":initial["revision"],"providerId":"standard"})).await.0,403);
    for (binding, online, status, code) in
        [("remote", true, 422, "unsupported_target"), ("local", false, 503, "target_unavailable")]
    {
        f.permissions(true, true, true, true);
        let mut policy: Value = serde_json::from_slice(&std::fs::read(&f.policy).unwrap()).unwrap();
        policy["hosts"][0]["binding"] = json!(binding);
        policy["hosts"][0]["online"] = json!(online);
        std::fs::write(&f.policy, policy.to_string()).unwrap();
        let (actual, error) = f.request("omo", None, Some(TOKEN), "project", json!({})).await;
        assert_eq!(actual, status);
        assert_eq!(error["error"]["code"], code);
    }
    assert_eq!(std::fs::read(f.root.path().join(".omo/omo.jsonc")).unwrap(), before);
}
#[tokio::test]
async fn aggregation_t47_missing_native_file_can_disable_saved_current_but_not_legacy_files() {
    let f = Fixture::new();
    let initial = f.read("omo").await;
    let (status, applied) = f.mutate("omo", "apply", &initial["revision"], Some("standard")).await;
    assert_eq!(status, 200, "{applied}");
    let native = f.root.path().join(".omo/omo.jsonc");
    std::fs::remove_file(&native).unwrap();
    let missing = f.read("omo").await;
    assert_eq!(missing["currentProviderId"], "standard");
    let (status, disabled) = f.mutate("omo", "disable", &missing["revision"], None).await;
    assert_eq!(status, 200, "a missing native file is already disabled: {disabled}");
    assert!(disabled["currentProviderId"].is_null());
    assert!(!native.exists());
    assert_eq!(f.tx.state(AppId::OpenCode).unwrap().omo, Some(OmoSelection::Disabled));
    let slim = f.read("omo-slim").await;
    assert_eq!(f.mutate("omo-slim", "apply", &slim["revision"], Some("slim")).await.0, 200);

    // An existing legacy file, even an empty one, still needs its own writer.
    let legacy = Fixture::new();
    std::fs::remove_file(legacy.root.path().join(".omo/omo.jsonc")).unwrap();
    let directory = legacy.root.path().join(".config/opencode");
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("oh-my-openagent.jsonc");
    std::fs::write(&path, b"{}").unwrap();
    let before = legacy.read("omo").await;
    let (status, error) = legacy.mutate("omo", "disable", &before["revision"], None).await;
    assert_eq!(status, 422);
    assert_eq!(error["error"]["code"], "unsupported_target");
    assert_eq!(std::fs::read(path).unwrap(), b"{}");
}
#[tokio::test]
async fn aggregation_t47_untracked_opposite_configuration_requires_explicit_disable() {
    let f = Fixture::new();
    let directory = f.root.path().join(".config/opencode");
    std::fs::create_dir_all(&directory).unwrap();
    let slim_path = directory.join("oh-my-opencode-slim.jsonc");
    std::fs::write(&slim_path, b"{}").unwrap();
    let standard = f.read("omo").await;
    let (status, error) = f.mutate("omo", "apply", &standard["revision"], Some("standard")).await;
    assert_eq!(status, 409, "existing opposite configuration must be stopped explicitly: {error}");
    assert_eq!(error["error"]["code"], "variant_conflict");
    let slim = f.read("omo-slim").await;
    let (status, disabled) = f.mutate("omo-slim", "disable", &slim["revision"], None).await;
    assert_eq!(status, 200, "{disabled}");
    assert!(!slim_path.exists());
    let standard = f.read("omo").await;
    assert_eq!(f.mutate("omo", "apply", &standard["revision"], Some("standard")).await.0, 200);

    let f = Fixture::new();
    std::fs::write(f.root.path().join(".omo/omo.jsonc"), b"{\"[opencode]\":{\"agents\":{}}}")
        .unwrap();
    let slim = f.read("omo-slim").await;
    let (status, error) = f.mutate("omo-slim", "apply", &slim["revision"], Some("slim")).await;
    assert_eq!(status, 409, "{error}");
    assert_eq!(error["error"]["code"], "variant_conflict");
    let standard = f.read("omo").await;
    assert_eq!(f.mutate("omo", "disable", &standard["revision"], None).await.0, 200);
    let slim = f.read("omo-slim").await;
    assert_eq!(f.mutate("omo-slim", "apply", &slim["revision"], Some("slim")).await.0, 200);
}
