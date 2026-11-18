//! Real Gateway pairing, native files and persistent OMO transactions.
#![cfg(target_os = "macos")]
use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
use axum::{
    Extension, Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
    middleware,
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::{Value, json};
use std::{os::unix::fs::DirBuilderExt, path::PathBuf};
use tower::ServiceExt;
use vw_agent_apps::providers::live::{
    commit::{ExecutionHostId, OmoSelection, PendingTarget, ProviderTransactions},
    write_guard::lock_app,
};
use {
    vw_api_app_identity::AppId, vw_api_transport::id::ProjectId,
    vw_api_transport::id::ProjectScope, vw_api_transport::id::TenantScope,
};
const SKEY: &str = "t47-fixture-gateway-skey-000000000000";
const INVALID_SKEY: &str = "t47-unpaired-gateway-skey";
const HOST: &str = local_app_auth::HOST_ID;
const PLATFORM: &str = "platform";
const COMPANY: &str = "company";

const ISOLATED_DIRS: [(&str, &str); 8] = [
    ("CODEX_HOME", ".codex"),
    ("XDG_CONFIG_HOME", ".config"),
    ("XDG_DATA_HOME", ".local/share"),
    ("XDG_CACHE_HOME", ".cache"),
    ("XDG_STATE_HOME", ".local/state"),
    ("XDG_RUNTIME_DIR", ".runtime"),
    ("VIBEWINDOW_CONFIG_DIR", ".config"),
    ("VIBEWINDOW_WORKSPACE", "workspace"),
];

fn run_in_isolated_process() -> bool {
    const ROOT: &str = "SHIJU_OMO_ACCEPTANCE_ROOT";
    if let Some(root) = std::env::var_os(ROOT).map(PathBuf::from) {
        let temp = std::env::temp_dir().canonicalize().unwrap();
        assert!(root != temp && root.starts_with(temp));
        assert_eq!(std::env::current_dir().unwrap(), root);
        for key in ["HOME", "CFFIXED_USER_HOME", "VIBEWINDOW_TEST_ROOT"] {
            assert_eq!(std::env::var_os(key).as_deref(), Some(root.as_os_str()));
        }
        for (key, suffix) in ISOLATED_DIRS {
            assert_eq!(std::env::var_os(key).as_deref(), Some(root.join(suffix).as_os_str()));
        }
        let paths = agent_paths::paths();
        assert_eq!(paths.home, root);
        for path in [&paths.config, &paths.data, &paths.cache, &paths.state] {
            assert!(path.starts_with(&root));
        }
        return false;
    }

    // The gateway resolves paths once per process. Give each case its own runtime.
    let root = tempfile::tempdir().unwrap();
    let path = root.path().canonicalize().unwrap();
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", std::thread::current().name().unwrap(), "--nocapture"])
        .current_dir(&path)
        .env_clear()
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
        .env("TMPDIR", std::env::temp_dir().canonicalize().unwrap())
        .env(ROOT, &path)
        .env("HOME", &path)
        .env("CFFIXED_USER_HOME", &path)
        .env("VIBEWINDOW_TEST_ROOT", &path)
        .envs(ISOLATED_DIRS.map(|(key, suffix)| (key, path.join(suffix))))
        .status()
        .unwrap();
    assert!(status.success(), "isolated OMO acceptance child failed: {status}");
    true
}
fn tenant() -> TenantScope {
    TenantScope::new(PLATFORM, COMPANY).unwrap()
}
fn scope(project: &str) -> ProjectScope {
    ProjectScope::new(tenant(), ProjectId::from(project)).unwrap()
}
struct Fixture {
    root: PathBuf,
    db: GlobalDatabase,
    tx: ProviderTransactions,
    credential_root: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = agent_paths::paths().home.clone();
        let credential_root = local_app_auth::auth_policy_path(&root, PLATFORM, COMPANY)
            .with_file_name("auth-credentials");
        std::fs::DirBuilder::new().recursive(true).mode(0o700).create(&credential_root).unwrap();
        let db = GlobalDatabase::new(config_runtime_paths::global_database_path(&root)).unwrap();
        let store = GatewaySettingsStore::new(db.clone()).unwrap();
        for (id, category) in [("standard", "omo"), ("slim", "omo-slim")] {
            store.upsert(&tenant(),"app_providers.records",&format!("\"opencode\":{id}"),&json!({"name":id,"category":category,"settingsConfig":{"agents":{"builder":{"model":"vendor/model"}},"categories":{"quick":{"model":"vendor/fast"}},"user_option":true}}).to_string()).unwrap();
        }
        let tx = ProviderTransactions::new(&credential_root, ExecutionHostId::new(HOST).unwrap())
            .unwrap();
        let mut guard = lock_app(AppId::OpenCode, &root).unwrap();
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
        std::fs::create_dir(root.join(".omo")).unwrap();
        std::fs::write(
            root.join(".omo/omo.jsonc"),
            b"{ // preserved\n\"[codex]\": {\"keep\":true}\n}\n",
        )
        .unwrap();
        Self { root, db, tx, credential_root }
    }

    fn transaction_file(&self, suffix: &str) -> PathBuf {
        self.credential_root
            .join("provider-live")
            .join(URL_SAFE_NO_PAD.encode(HOST))
            .join(format!("opencode.{suffix}.json"))
    }

    fn router(&self) -> Router {
        let state = agent_gateway::gateway_test_fixture::state::<(), (), ()>(
            Default::default(),
            true,
            &[SKEY.to_owned()],
        );
        super::super::aggregation_omo::router_with_policy(None)
            .layer(Extension(self.db.clone()))
            .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware))
            .layer(middleware::from_fn_with_state(
                state,
                agent_gateway::request_policy::skey_auth_middleware::<(), (), ()>,
            ))
    }
    async fn request(
        &self,
        variant: &str,
        action: Option<&str>,
        skey: Option<&str>,
        project: &str,
        extra: Value,
    ) -> (StatusCode, Value) {
        self.request_in_company(COMPANY, variant, action, skey, project, extra).await
    }

    async fn request_in_company(
        &self,
        company: &str,
        variant: &str,
        action: Option<&str>,
        skey: Option<&str>,
        project: &str,
        extra: Value,
    ) -> (StatusCode, Value) {
        let path = match action {
            Some(action) => format!("/agent-apps/opencode/{variant}/{action}"),
            None => {
                format!("/agent-apps/opencode/{variant}?targetHostId={HOST}&projectId={project}")
            }
        };
        let mut request = Request::builder()
            .method(if action.is_some() { "POST" } else { "GET" })
            .uri(path)
            .header("x-platform-id", PLATFORM)
            .header("x-company-id", company);
        if let Some(skey) = skey {
            request = request.header("authorization", format!("Bearer {skey}"));
        }
        let body = if action.is_some() {
            let mut body = json!({"targetHostId":HOST,"projectId":project});
            for (k, v) in extra.as_object().unwrap() {
                body[k] = v.clone();
            }
            request = request.header("content-type", "application/json");
            Body::from(body.to_string())
        } else {
            Body::empty()
        };
        let response = self.router().oneshot(request.body(body).unwrap()).await.unwrap();
        let status = response.status();
        if status != StatusCode::UNAUTHORIZED {
            assert_eq!(response.headers().get("cache-control").unwrap(), "no-store");
        }
        let bytes = to_bytes(response.into_body(), 65536).await.unwrap();
        let text = String::from_utf8_lossy(&bytes);
        assert!(!text.contains(SKEY));
        assert!(!text.contains(INVALID_SKEY));
        (status, serde_json::from_slice(&bytes).unwrap())
    }
    async fn read(&self, variant: &str) -> Value {
        let (status, value) = self.request(variant, None, Some(SKEY), "project", json!({})).await;
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
        self.request(variant, Some(action), Some(SKEY), "project", body).await
    }
}
#[tokio::test]
async fn aggregation_t47_aggregation_omo_preserves_contract() {
    if run_in_isolated_process() {
        return;
    }
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
    let raw = std::fs::read_to_string(f.root.join(".omo/omo.jsonc")).unwrap();
    assert!(raw.contains("// preserved"));
    assert!(raw.contains("\"[codex]\": {\"keep\":true}"));
    assert_eq!(f.tx.state(AppId::OpenCode).unwrap().omo, Some(OmoSelection::Disabled));
    assert!(f.tx.pending(AppId::OpenCode).unwrap().is_none());
}
#[tokio::test]
async fn aggregation_t47_scope_and_permissions_precede_writes() {
    if run_in_isolated_process() {
        return;
    }
    let f = Fixture::new();
    let tx_before = f.tx.state(AppId::OpenCode).unwrap();
    let before = std::fs::read(f.root.join(".omo/omo.jsonc")).unwrap();
    for variant in ["omo", "omo-slim"] {
        let state = f.read(variant).await;
        let provider = if variant == "omo" { "standard" } else { "slim" };
        for action in ["apply", "disable"] {
            let extra = if action == "apply" {
                json!({"expectedRevision":state["revision"],"providerId":provider})
            } else {
                json!({"expectedRevision":state["revision"]})
            };
            for (project, skey, status) in [
                ("project", None, 401),
                ("project", Some(INVALID_SKEY), 401),
                ("other", Some(SKEY), 403),
                ("ungranted", Some(SKEY), 403),
            ] {
                assert_eq!(
                    f.request(variant, Some(action), skey, project, extra.clone()).await.0,
                    status
                );
            }
            let (status, error) = f
                .request_in_company(
                    "other-company",
                    variant,
                    Some(action),
                    Some(SKEY),
                    "project",
                    extra,
                )
                .await;
            assert_eq!(status, 409, "{error}");
            assert_eq!(error["error"]["code"], "revision_conflict");
        }
        for skey in [None, Some(INVALID_SKEY)] {
            assert_eq!(f.request(variant, None, skey, "project", json!({})).await.0, 401);
        }
        assert_eq!(f.request(variant, None, Some(SKEY), "other", json!({})).await.0, 403);
        let (status, error) = f
            .request_in_company("other-company", variant, None, Some(SKEY), "project", json!({}))
            .await;
        assert_eq!(status, 409, "{error}");
        assert_eq!(error["error"]["code"], "revision_conflict");
    }
    assert!(
        f.tx.state(AppId::OpenCode).unwrap() == tx_before,
        "rejected requests must preserve the full OpenCode transaction state"
    );
    assert!(f.tx.pending(AppId::OpenCode).unwrap().is_none());
    assert!(!f.root.join(".config/opencode").exists());
    assert_eq!(std::fs::read(f.root.join(".omo/omo.jsonc")).unwrap(), before);
}
#[tokio::test]
async fn aggregation_t47_stale_revision_and_variants_are_explicit_conflicts() {
    if run_in_isolated_process() {
        return;
    }
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
    std::fs::write(f.root.join(".omo/omo.jsonc"), b"{\"[codex]\":{\"changed\":true}}").unwrap();
    assert_eq!(f.mutate("omo", "disable", &revision, None).await.0, 409);
}
#[tokio::test]
async fn aggregation_t47_profiles_and_unknown_inputs_are_rejected() {
    if run_in_isolated_process() {
        return;
    }
    let f = Fixture::new();
    let state = f.read("omo").await;
    assert_eq!(f.mutate("omo", "apply", &state["revision"], Some("slim")).await.0, 422);
    assert_eq!(f.mutate("omo", "apply", &state["revision"], Some("missing")).await.0, 404);
    assert_eq!(f.request("omo",Some("apply"),Some(SKEY),"project",json!({"expectedRevision":state["revision"],"providerId":"standard","path":"/tmp/untrusted"})).await.0,400);
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
    if run_in_isolated_process() {
        return;
    }
    let f = Fixture::new();
    let state = f.read("omo").await;
    std::fs::write(f.root.join(".omo/omo.jsonc"), b"{broken").unwrap();
    assert_eq!(f.request("omo", None, Some(SKEY), "project", json!({})).await.0, 422);
    assert_eq!(f.mutate("omo", "disable", &state["revision"], None).await.0, 422);
    std::fs::remove_file(f.root.join(".omo/omo.jsonc")).unwrap();
    let external = f.root.join("outside-omo.jsonc");
    std::fs::write(&external, b"{}").unwrap();
    std::os::unix::fs::symlink(&external, f.root.join(".omo/omo.jsonc")).unwrap();
    assert_eq!(f.request("omo", None, Some(SKEY), "project", json!({})).await.0, 503);
}
#[tokio::test]
async fn aggregation_t47_read_is_observational_and_legacy_slots_are_not_claimed() {
    if run_in_isolated_process() {
        return;
    }
    let f = Fixture::new();
    let path = f.transaction_file("state");
    let before = std::fs::read(&path).unwrap();
    let native = std::fs::read(f.root.join(".omo/omo.jsonc")).unwrap();
    f.read("omo").await;
    f.read("omo-slim").await;
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(std::fs::read(f.root.join(".omo/omo.jsonc")).unwrap(), native);
    assert!(!f.root.join(".config/opencode").exists());
    let mut legacy: Value = serde_json::from_slice(&before).unwrap();
    legacy.as_object_mut().unwrap().remove("project_scope");
    std::fs::write(&path, legacy.to_string()).unwrap();
    let before = std::fs::read(&path).unwrap();
    for variant in ["omo", "omo-slim"] {
        assert_eq!(f.request(variant, None, Some(SKEY), "project", json!({})).await.0, 409);
    }
    assert_eq!(std::fs::read(&path).unwrap(), before);
}
#[tokio::test]
async fn aggregation_t47_missing_unified_write_is_explicitly_unsupported() {
    if run_in_isolated_process() {
        return;
    }
    let f = Fixture::new();
    std::fs::remove_file(f.root.join(".omo/omo.jsonc")).unwrap();
    let state = f.read("omo").await;
    let (status, error) = f.mutate("omo", "apply", &state["revision"], Some("standard")).await;
    assert_eq!(status, 422);
    assert_eq!(error["error"]["code"], "unsupported_target");
    assert!(!f.root.join(".omo/omo.jsonc").exists());
    assert!(f.tx.state(AppId::OpenCode).unwrap().omo.is_none());
}
#[tokio::test]
async fn aggregation_t47_pending_selection_recovers_with_the_same_native_transaction() {
    if run_in_isolated_process() {
        return;
    }
    use vw_agent_apps::providers::live::{
        stage::{FileChange, WholeFile},
        write_guard::{FilePrivacy, WriteError},
    };
    let f = Fixture::new();
    let config = f.root.join(".omo/omo.jsonc");
    let before = std::fs::read(&config).unwrap();
    let patch = WholeFile::new(Some(&before), Some(b"{\"[opencode]\":{\"agents\":{}}}".to_vec()));
    let changes = [FileChange {
        name: std::ffi::OsStr::new("omo.jsonc"),
        privacy: FilePrivacy::Shared,
        patch: &patch,
    }];
    let mut guard = lock_app(AppId::OpenCode, &f.root.join(".omo")).unwrap();
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
    let (status, error) = f.request("omo", None, Some(SKEY), "project", json!({})).await;
    assert_eq!(status, 409);
    assert_eq!(error["error"]["code"], "recovery_pending");
    assert_eq!(std::fs::read(&config).unwrap(), before);
    // Simulate a crash immediately after the durable publication mark, before rename.
    let journal = f.transaction_file("pending");
    let mut intent: Value = serde_json::from_slice(&std::fs::read(&journal).unwrap()).unwrap();
    intent["published"] = json!(true);
    std::fs::write(&journal, intent.to_string()).unwrap();
    let mut guard = lock_app(AppId::OpenCode, &f.root.join(".omo")).unwrap();
    f.tx.recover_project(&scope("project"), &mut guard).unwrap();
    drop(guard);
    assert_eq!(f.read("omo").await["currentProviderId"], "standard");
    assert!(f.tx.pending(AppId::OpenCode).unwrap().is_none());
}
#[tokio::test]
async fn aggregation_t47_unavailable_targets_are_distinct_from_invalid_configuration() {
    if run_in_isolated_process() {
        return;
    }
    let f = Fixture::new();
    let initial = f.read("omo").await;
    let before = std::fs::read(f.root.join(".omo/omo.jsonc")).unwrap();
    for (target, expected, code) in
        [("other", 409, "revision_conflict"), ("../other", 400, "invalid_input")]
    {
        let (status, error) = f.request("omo", Some("apply"), Some(SKEY), "project", json!({
            "targetHostId": target, "expectedRevision": initial["revision"], "providerId": "standard"
        })).await;
        assert_eq!(status, expected, "{error}");
        assert_eq!(error["error"]["code"], code);
    }
    let backups = f.credential_root.join("backups");
    std::fs::remove_dir(&backups).unwrap();
    std::fs::write(&backups, b"fixture blocks backup directory creation").unwrap();
    let (status, error) = f.request("omo", None, Some(SKEY), "project", json!({})).await;
    assert_eq!(status, 503, "{error}");
    assert_eq!(error["error"]["code"], "target_unavailable");
    assert_eq!(std::fs::read(f.root.join(".omo/omo.jsonc")).unwrap(), before);
}
#[tokio::test]
async fn aggregation_t47_missing_native_file_can_disable_saved_current() {
    if run_in_isolated_process() {
        return;
    }
    let f = Fixture::new();
    let initial = f.read("omo").await;
    let (status, applied) = f.mutate("omo", "apply", &initial["revision"], Some("standard")).await;
    assert_eq!(status, 200, "{applied}");
    let native = f.root.join(".omo/omo.jsonc");
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
}
#[tokio::test]
async fn aggregation_t47_legacy_file_requires_its_own_writer() {
    if run_in_isolated_process() {
        return;
    }
    // An existing legacy file, even an empty one, still needs its own writer.
    let legacy = Fixture::new();
    std::fs::remove_file(legacy.root.join(".omo/omo.jsonc")).unwrap();
    let directory = legacy.root.join(".config/opencode");
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
async fn aggregation_t47_untracked_slim_configuration_requires_explicit_disable() {
    if run_in_isolated_process() {
        return;
    }
    let f = Fixture::new();
    let directory = f.root.join(".config/opencode");
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
}
#[tokio::test]
async fn aggregation_t47_untracked_standard_configuration_requires_explicit_disable() {
    if run_in_isolated_process() {
        return;
    }
    let f = Fixture::new();
    std::fs::write(f.root.join(".omo/omo.jsonc"), b"{\"[opencode]\":{\"agents\":{}}}").unwrap();
    let slim = f.read("omo-slim").await;
    let (status, error) = f.mutate("omo-slim", "apply", &slim["revision"], Some("slim")).await;
    assert_eq!(status, 409, "{error}");
    assert_eq!(error["error"]["code"], "variant_conflict");
    let standard = f.read("omo").await;
    assert_eq!(f.mutate("omo", "disable", &standard["revision"], None).await.0, 200);
    let slim = f.read("omo-slim").await;
    assert_eq!(f.mutate("omo-slim", "apply", &slim["revision"], Some("slim")).await.0, 200);
}
