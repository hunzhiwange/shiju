//! Real policy, tenant middleware, HTTP/SDK and isolated native files; no user credentials.
use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
use axum::{Extension, Router, middleware};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    os::unix::fs::PermissionsExt,
    path::Path,
    sync::{Arc, Mutex},
};
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{operations::*, preview::ProviderPreviewRequest},
    id::TenantScope,
    task::TaskStatus,
};
use vw_gateway_client::{GatewayClient, GatewayEndpoint};
const TOKEN: &str = "fixture-security-authorized-token-over-32-chars";
const KEY: &str = "fixture-security-nested-key-753";
const COOKIE: &str = "fixture-security-cookie-951";
const NUMERIC_SECRET: u64 = 987654321012345;

#[derive(Clone, Default)]
struct Logs(Arc<Mutex<Vec<u8>>>);
impl tracing::Subscriber for Logs {
    fn max_level_hint(&self) -> Option<tracing::metadata::LevelFilter> {
        Some(tracing::metadata::LevelFilter::TRACE)
    }
    fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
    fn event(&self, event: &tracing::Event<'_>) {
        struct Visitor<'a>(&'a mut Vec<u8>);
        impl tracing::field::Visit for Visitor<'_> {
            fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                use std::io::Write;
                write!(self.0, "{}={:?};", field.name(), value).unwrap();
            }
        }
        event.record(&mut Visitor(&mut self.0.lock().unwrap()));
    }
    fn enter(&self, _: &tracing::span::Id) {}
    fn exit(&self, _: &tracing::span::Id) {}
}

fn safe(text: &str) {
    for secret in
        [TOKEN.to_string(), KEY.to_string(), COOKIE.to_string(), NUMERIC_SECRET.to_string()]
    {
        assert!(!text.contains(&secret), "credential reached observable output");
    }
}
fn tree(root: &Path) -> BTreeMap<std::path::PathBuf, Option<Vec<u8>>> {
    fn visit(root: &Path, dir: &Path, result: &mut BTreeMap<std::path::PathBuf, Option<Vec<u8>>>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                result.insert(path.strip_prefix(root).unwrap().to_owned(), None);
                visit(root, &path, result);
            } else {
                result.insert(
                    path.strip_prefix(root).unwrap().to_owned(),
                    Some(std::fs::read(path).unwrap()),
                );
            }
        }
    }
    let mut result = BTreeMap::new();
    visit(root, root, &mut result);
    result
}
struct Fixture {
    root: tempfile::TempDir,
    db: GlobalDatabase,
    url: String,
    client: GatewayClient,
    server: tokio::task::JoinHandle<()>,
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
        std::fs::create_dir(root.path().join("private")).unwrap();
        std::fs::set_permissions(
            root.path().join("private"),
            std::fs::Permissions::from_mode(0o700),
        )
        .unwrap();
        let policy = root.path().join("access.json");
        std::fs::write(&policy, json!({"principals":[{"id":"security","token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":[{"platform_id":"p","company_id":"a","host_id":"host","provider_config_read":true,"provider_config_write":true}]}],"hosts":[{"id":"host","online":true,"binding":"local","home":root.path(),"credential_root":root.path().join("private"),"search_paths":[],"applications":root.path().join("Applications")}]}).to_string()).unwrap();
        std::fs::set_permissions(&policy, std::fs::Permissions::from_mode(0o600)).unwrap();
        let db = GlobalDatabase::for_home(root.path()).unwrap();
        let mut router = Router::new();
        // These are the production subrouters with explicit policy injection; no replacement handlers.
        for make in [
            super::preview::router_with_policy,
            super::operations::router_with_policy,
            super::claude::router_with_policy,
            super::codex_catalog::router_with_policy,
            super::gemini::router_with_policy,
            super::grok::router_with_policy,
            super::claude_desktop::router_with_policy,
            super::opencode::router_with_policy,
            super::openclaw::router_with_policy,
            super::hermes::router_with_policy,
            super::pi::router_with_policy,
            super::omp::router_with_policy,
            super::mcode::router_with_policy,
            super::kimi::router_with_policy,
            super::dsh::router_with_policy,
            super::agy::router_with_policy,
            super::catalog::router_with_policy,
            super::paths::router_with_policy,
            super::mode_state::router_with_policy,
            super::env_diagnose::router_with_policy,
            super::env_clean::router_with_policy,
            super::env_restore::router_with_policy,
            super::editor_merge::router_with_policy,
            super::snippets::router_with_policy,
            super::import::router_with_policy,
            super::desktop_mapping::router_with_policy,
        ] {
            router = router.merge(make(Some(policy.clone())));
        }
        router = router
            .route("/app-providers/{app}/{id}", axum::routing::get(super::read))
            .layer(Extension(db.clone()))
            .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            axum::serve(listener, Router::new().nest("/v1", router)).await.unwrap()
        });
        let client = GatewayClient::new(
            GatewayEndpoint::new("127.0.0.1", port)
                .with_tenant_scope(TenantScope::new("p", "a").unwrap()),
        )
        .unwrap();
        Self { root, db, url: format!("http://127.0.0.1:{port}/v1"), client, server }
    }
    fn write(&self, name: &str, value: &Value) {
        let path = self.root.path().join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, value.to_string()).unwrap();
    }
    async fn request(
        &self,
        method: &str,
        path: &str,
        company: &str,
        token: &str,
        body: Value,
    ) -> (u16, String) {
        let response = reqwest::Client::new()
            .request(method.parse().unwrap(), format!("{}{path}", self.url))
            .header("x-platform-id", "p")
            .header("x-company-id", company)
            .header("x-agent-apps-token", token)
            .json(&body)
            .send()
            .await
            .unwrap();
        let status = response.status().as_u16();
        let text = response.text().await.unwrap();
        safe(&text);
        (status, text)
    }
}

#[tokio::test]
async fn cc04_api_security_rejects_cross_scope_mutation() {
    let f = Fixture::new().await;
    f.write(".claude/settings.json", &json!({"env":{"ANTHROPIC_API_KEY":KEY}}));
    f.write(".pi/agent/models.json", &json!({"providers":{"saved":{"apiKey":KEY,"models":[]}}}));
    let before = tree(f.root.path());
    // Every shipped native app adapter receives a valid target DTO, reaching its authorization branch.
    for app in [
        "claude",
        "codex",
        "gemini",
        "grokbuild",
        "claude-desktop",
        "opencode",
        "openclaw",
        "hermes",
        "pi",
        "omp",
        "mcode",
        "kimi",
        "dsh",
        "agy",
    ] {
        let action = if ["opencode", "openclaw", "hermes", "pi", "omp", "mcode", "kimi", "dsh"]
            .contains(&app)
        {
            "add"
        } else {
            "switch"
        };
        for (company, host, token, status) in
            [("b", "host", TOKEN, 403), ("a", "other", TOKEN, 403), ("a", "host", "invalid", 401)]
        {
            assert_eq!(
                f.request(
                    "POST",
                    &format!("/app-providers/{app}/saved/{action}"),
                    company,
                    token,
                    json!({"targetHostId":host})
                )
                .await
                .0,
                status,
                "{app}"
            );
        }
    }
    for (path, extra) in [
        ("/app-providers/pi/catalog/import", json!({"trigger":"manual"})),
        (
            "/app-providers/operations",
            json!({"requestId":"denied","action":{"kind":"import","app":"pi","trigger":"manual"}}),
        ),
        ("/app-providers/claude/environment/clean", json!({"selections":[]})),
        ("/app-providers/claude/environment/restore", json!({"backupId":"missing"})),
        (
            "/app-providers/claude/saved/editor/save",
            json!({"context":"missing","edits":[],"choice":"refuse"}),
        ),
    ] {
        let mut body = extra;
        body["targetHostId"] = json!("host");
        assert_eq!(f.request("POST", path, "b", TOKEN, body).await.0, 403, "{path}");
    }
    assert_eq!(tree(f.root.path()), before, "rejected requests changed files or database");
    let policy = f.root.path().join("access.json");
    let mut access: Value = serde_json::from_slice(&std::fs::read(&policy).unwrap()).unwrap();
    access["principals"][0]["grants"][0]["provider_config_write"] = json!(false);
    std::fs::write(&policy, access.to_string()).unwrap();
    let before = tree(f.root.path());
    for app in [
        "claude",
        "codex",
        "gemini",
        "grokbuild",
        "claude-desktop",
        "opencode",
        "openclaw",
        "hermes",
        "pi",
        "omp",
        "mcode",
        "kimi",
        "dsh",
        "agy",
    ] {
        let action = if ["opencode", "openclaw", "hermes", "pi", "omp", "mcode", "kimi", "dsh"]
            .contains(&app)
        {
            "add"
        } else {
            "switch"
        };
        assert_eq!(
            f.request(
                "POST",
                &format!("/app-providers/{app}/saved/{action}"),
                "a",
                TOKEN,
                json!({"targetHostId":"host"})
            )
            .await
            .0,
            403,
            "read-only {app}"
        );
    }
    assert_eq!(tree(f.root.path()), before, "read-only grant allowed mutation");
}

#[tokio::test]
async fn cc04_api_security_nested_secrets_never_reach_responses() {
    let logs = Logs::default();
    let _guard = tracing::subscriber::set_default(logs.clone());
    tracing::info!("security capture active");
    let f = Fixture::new().await;
    let settings = json!({"env":{"ANTHROPIC_API_KEY":KEY},"hooks":[{"headers":{"aUtHoRiZaTiOn":NUMERIC_SECRET,"Cookie":{(COOKIE): true}}}],"plugins":[{"password":KEY,"tokens":[KEY]}]});
    f.write(".claude/settings.json", &settings);
    let req = ProviderPreviewRequest {
        target_host_id: "host".into(),
        directory_override: None,
        settings: json!({"env":{"ANTHROPIC_API_KEY":KEY}}),
        category: None,
        provider_id: None,
        meta: None,
        request_options: None,
    };
    let before = tree(f.root.path());
    let view = f.client.app_provider_preview(AppId::Claude, &req, TOKEN).await.unwrap();
    safe(&serde_json::to_string(&view).unwrap());
    assert_eq!(view.settings["hooks"][0]["headers"]["aUtHoRiZaTiOn"], "[redacted]");
    assert_eq!(view.settings["hooks"][0]["headers"]["Cookie"], "[redacted]");
    assert_eq!(tree(f.root.path()), before);
    let text = String::from_utf8(logs.0.lock().unwrap().clone()).unwrap();
    assert!(text.contains("security capture active"));
    safe(&text);
}

#[tokio::test]
async fn cc04_api_security_reads_errors_and_progress_are_safe() {
    let f = Fixture::new().await;
    f.write(".pi/agent/models.json", &json!({"providers":{"saved":{"apiKey":KEY,"models":[]}}}));
    let request = ProviderOperationRequest {
        target_host_id: "host".into(),
        request_id: "security-import".into(),
        action: ProviderOperationAction::Import {
            app: AppId::Pi,
            trigger: vw_api_types::app_providers::import::ImportTrigger::Manual,
        },
    };
    let first = f.client.provider_operation_start(&request, TOKEN).await.unwrap();
    safe(&serde_json::to_string(&first).unwrap());
    let id = first.operation_id.0;
    let last = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let progress = f.client.provider_operation_get(&id, "host", TOKEN).await.unwrap();
            safe(&serde_json::to_string(&progress).unwrap());
            if !matches!(progress.status, TaskStatus::Queued | TaskStatus::Running) {
                break progress;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(last.status, TaskStatus::Completed);
    let store = GatewaySettingsStore::new(f.db.clone()).unwrap();
    assert!(
        store
            .get(&TenantScope::new("p", "a").unwrap(), "app_providers.records", "\"pi\":saved")
            .unwrap()
            .unwrap()
            .contains(KEY)
    );
    for path in [
        "/app-providers/pi/saved",
        "/app-providers/pi",
        "/app-providers/pi/catalog/state?targetHostId=host",
        "/app-providers/pi/mode/state?targetHostId=host",
    ] {
        assert_eq!(f.request("GET", path, "a", TOKEN, Value::Null).await.0, 200, "{path}");
    }
    let before = tree(f.root.path());
    for method in ["GET", "DELETE"] {
        assert_eq!(
            f.request(
                method,
                &format!("/app-providers/operations/{id}?targetHostId=host"),
                "b",
                TOKEN,
                Value::Null
            )
            .await
            .0,
            403
        );
    }
    assert_eq!(f.request("GET", "/app-providers/pi/missing", "a", TOKEN, Value::Null).await.0, 404);
    assert_eq!(tree(f.root.path()), before);
    // Malformed native source must expose a logical parse location, never a source excerpt.
    std::fs::create_dir(f.root.path().join(".claude")).unwrap();
    std::fs::write(
        f.root.path().join(".claude/settings.json"),
        format!("{{\"secret\":\"{KEY}\", bad"),
    )
    .unwrap();
    let (status, text) = f
        .request(
            "POST",
            "/app-providers/claude/config/preview",
            "a",
            TOKEN,
            json!({"targetHostId":"host","settings":{}}),
        )
        .await;
    assert_eq!(status, 200);
    assert!(serde_json::from_str::<Value>(&text).unwrap()["parseError"].is_object());
}

#[tokio::test]
async fn cc04_api_security_registered_routes_deny_missing_gateway_auth_before_storage() {
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use tower::ServiceExt;
    let f = Fixture::new().await;
    let before = tree(f.root.path());
    let app = crate::gateway::router::handler_router()
        .layer(Extension(f.db.clone()))
        .layer(Extension(TenantScope::new("p", "a").unwrap()))
        .layer(middleware::from_fn(|req, next| async {
            crate::gateway::middleware::basic_auth_middleware_with_credentials(
                req,
                next,
                Some("gateway-fixture"),
                "security",
            )
            .await
        }));
    for (method, path) in [
        ("GET", "/app-providers/claude"),
        ("GET", "/app-providers/claude/saved"),
        ("POST", "/app-providers/claude/saved"),
        ("PUT", "/app-providers/claude/saved"),
        ("PATCH", "/app-providers/claude/saved"),
        ("DELETE", "/app-providers/claude/saved"),
        ("POST", "/app-providers/claude/saved/duplicate"),
        ("GET", "/app-providers/claude/presentation/entries"),
        ("PUT", "/app-providers/claude/saved/presentation"),
        ("GET", "/app-providers/claude/saved/endpoints"),
        ("GET", "/app-providers/claude/saved/icon"),
        ("GET", "/universal-providers"),
        ("PUT", "/universal-providers/saved"),
        ("POST", "/universal-providers/saved/sync"),
        ("POST", "/app-providers/claude/config/paths"),
        ("POST", "/app-providers/claude/config/preview"),
        ("GET", "/app-providers/claude/config/snippet"),
        ("POST", "/app-providers/claude/saved/editor/open"),
        ("POST", "/app-providers/claude/saved/editor/save"),
        ("GET", "/app-providers/claude/mode/state"),
        ("POST", "/app-providers/claude/mode/apply"),
        ("POST", "/app-providers/claude/environment/diagnose"),
        ("POST", "/app-providers/claude/environment/clean"),
        ("POST", "/app-providers/claude/environment/restore"),
        ("POST", "/app-providers/pi/catalog/import"),
        ("POST", "/app-providers/operations"),
        ("GET", "/app-providers/operations/saved"),
        ("DELETE", "/app-providers/operations/saved"),
        ("POST", "/app-providers/claude-desktop/providers/import-claude"),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(path)
                    .header("content-type", "application/json")
                    .body(Body::from(json!({"secret":KEY}).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 401, "{method} {path}");
        safe(
            &String::from_utf8(to_bytes(response.into_body(), 4096).await.unwrap().to_vec())
                .unwrap(),
        );
    }
    assert_eq!(tree(f.root.path()), before);
}
