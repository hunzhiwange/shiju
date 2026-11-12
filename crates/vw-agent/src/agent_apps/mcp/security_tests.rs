//! Exercise the production HTTP router, stores and host paths in an isolated process.
use super::McpCatalogStore;
use crate::{gateway::agent_apps_diagnostic_jobs::DiagnosticJobs, global_database::GlobalDatabase};
use axum::{
    Extension, Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command, time::Duration};
use tower::ServiceExt;
use vw_api_types::{TenantScope, mcp_management::McpCatalogScope};

const ENV: &str = "SHIJU_MCP_TASK51_ROOT";
const A: &str = "fixture-t51-member-a-token-at-least-32-bytes";
const PEER: &str = "fixture-t51-same-tenant-peer-token-at-least-32-bytes";
const B: &str = "fixture-t51-member-b-token-at-least-32-bytes";
const ENV_SECRET: &str = "fixture-t51-private-env-token";
const HEADER_SECRET: &str = "fixture-t51-private-header-token";

// The child process installs a subscriber so the parent checks actual trace events too.
struct FixtureLogs;
impl tracing::Subscriber for FixtureLogs {
    fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, values: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        values.record(&mut self.visitor());
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _: &tracing::span::Id, values: &tracing::span::Record<'_>) {
        values.record(&mut self.visitor());
    }
    fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
    fn event(&self, event: &tracing::Event<'_>) {
        event.record(&mut self.visitor());
    }
    fn enter(&self, _: &tracing::span::Id) {}
    fn exit(&self, _: &tracing::span::Id) {}
}
impl FixtureLogs {
    fn visitor(&self) -> impl tracing::field::Visit {
        struct Fields;
        impl tracing::field::Visit for Fields {
            fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                eprintln!("{field}={value:?}");
            }
        }
        Fields
    }
}

async fn send(
    app: &Router,
    method: &str,
    uri: &str,
    company: &str,
    token: &str,
    payload: Value,
) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("authorization", "Bearer dynamic-acceptance-skey")
                .header("x-platform-id", "p")
                .header("x-company-id", company)
                .header("x-agent-apps-token", token)
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 65536).await.unwrap();
    let body = String::from_utf8(bytes.to_vec()).unwrap();
    for secret in [A, PEER, B, ENV_SECRET, HEADER_SECRET] {
        assert!(!body.contains(secret), "HTTP response disclosed a planted token");
    }
    (status, serde_json::from_str(&body).unwrap_or(Value::Null))
}
fn private_dir(path: &Path) {
    fs::create_dir_all(path).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
}

#[tokio::test]
async fn cc_mcp_t51_foreign_tenants_accounts_and_job_owners_cannot_cross_boundaries() {
    let Some(root) = std::env::var_os(ENV) else {
        let root = tempfile::tempdir().unwrap();
        private_dir(root.path());
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                concat!(
                    module_path!(),
                    "::cc_mcp_t51_foreign_tenants_accounts_and_job_owners_cannot_cross_boundaries"
                )
                .split_once("::")
                .unwrap()
                .1,
                "--nocapture",
            ])
            .env(ENV, root.path())
            .env("SHIJU_AGENT_APPS_ACCESS_FILE", root.path().join("access.json"))
            .output()
            .unwrap();
        let logs = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        for secret in [A, PEER, B, ENV_SECRET, HEADER_SECRET] {
            assert!(!logs.contains(secret), "process logs disclosed a planted token");
        }
        assert!(output.status.success(), "isolated security regression failed: {logs}");
        assert!(root.path().join("verified").exists(), "isolated test must execute assertions");
        return;
    };
    tracing::subscriber::set_global_default(FixtureLogs).unwrap();
    let root = Path::new(&root);
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}/mcp", listener.local_addr().unwrap());
    let mut hosts = Vec::new();
    let mut principals = Vec::new();
    let mut installations = Vec::new();
    for (company, token) in [("a", A), ("b", B)] {
        let home = root.join(company);
        for dir in [home.join("bin"), home.join("private"), home.join(".codex")] {
            private_dir(&dir);
        }
        let cli = home.join("bin/codex");
        fs::write(&cli, "#!/bin/sh\ntouch \"$(dirname \"$0\")/../started\"\nprintf '1.2.3\\n'\n")
            .unwrap();
        fs::set_permissions(&cli, fs::Permissions::from_mode(0o700)).unwrap();
        fs::hard_link(&cli, home.join("bin/claude")).unwrap();
        let env = vw_agent_apps::apps::discovery::read::LocalReadEnvironment {
            host_id: company.into(),
            home: home.clone(),
            applications: home.join("Applications"),
            search_paths: vec![home.join("bin")],
        };
        let id = vw_agent_apps::apps::discovery::read::read(
            vw_api_types::agent_apps::AppId::Codex,
            &env,
        )
        .unwrap()
        .installations
        .remove(0)
        .id;
        let claude = vw_agent_apps::apps::discovery::read::read(
            vw_api_types::agent_apps::AppId::Claude,
            &env,
        )
        .unwrap()
        .installations
        .remove(0)
        .id;
        installations.push((id, claude));
        fs::remove_file(home.join("started")).unwrap();
        fs::write(home.join(".codex/config.toml"), format!("[mcp_servers.external]\ncommand = '/usr/bin/true'\nenv = {{ ACCESS_TOKEN = '{ENV_SECRET}' }}\n[mcp_servers.remote]\nurl = '{url}'\nhttp_headers = {{ Authorization = '{HEADER_SECRET}' }}\n")).unwrap();
        hosts.push(json!({"id":company,"online":true,"binding":"local","home":home,"applications":env.applications,"search_paths":env.search_paths,"credential_root":home.join("private")}));
        let grant = json!({"platform_id":"p","company_id":company,"host_id":company,"mcp_catalog_write":true,"provider_config_read":true,"provider_config_write":true,"auth_account_read":["claude_official"],"mcp_probe":[{"project_id":"project","app_id":"codex","record_id":"remote","connection_revision":1,"endpoint_sha256":hex::encode(Sha256::digest(url.as_bytes()))}]});
        principals.push(json!({"id":company,"token_sha256":hex::encode(Sha256::digest(token)),"grants":[grant.clone()]}));
        if company == "a" {
            principals.push(json!({"id":"peer","token_sha256":hex::encode(Sha256::digest(PEER)),"grants":[grant]}));
        }
    }
    fs::write(root.join("access.json"), json!({"principals":principals,"hosts":hosts}).to_string())
        .unwrap();
    fs::set_permissions(root.join("access.json"), fs::Permissions::from_mode(0o600)).unwrap();
    let db = GlobalDatabase::new(root.join("catalog.sqlite3")).unwrap();
    let store = McpCatalogStore::new(db.clone()).unwrap();
    // Real account directories, with the same account ID owned independently by each member.
    let mut account_files = Vec::new();
    for company in ["a", "b"] {
        use crate::agent_apps::auth_accounts::{
            AuthAccountMetadata, AuthAccountScope, AuthAccountStore,
        };
        use vw_agent_apps::auth::credential_file::{AccountCredential, CredentialFileStore};
        use vw_api_types::{TimestampMs, agent_apps::AppId, agent_apps_auth::AuthService};
        let tenant = TenantScope::new("p", company).unwrap();
        let private = root.join(company).join("private");
        let credentials = CredentialFileStore::new(&private).unwrap();
        let owner = credentials.for_owner(company).unwrap();
        owner.register_claude_directory(&tenant, company, "owned").unwrap();
        let lease = owner.lease(&tenant, company, AppId::Claude, "owned", false).unwrap();
        let AccountCredential::ClaudeDirectory(directory) = lease.read().unwrap() else {
            panic!("directory credential expected")
        };
        fs::write(directory.path().join(".claude.json"), json!({"mcpServers":{"private":{"command":"/usr/bin/true","env":{"ACCESS_TOKEN":ENV_SECRET},"headers":{"Authorization":HEADER_SECRET}}}}).to_string()).unwrap();
        let account_path = directory.path().join(".claude.json");
        account_files.push((account_path.clone(), fs::read(&account_path).unwrap()));
        AuthAccountStore::new(db.clone(), &private)
            .unwrap()
            .insert_metadata(
                &AuthAccountScope {
                    tenant: &tenant,
                    host: company,
                    principal: company,
                    service: AuthService::ClaudeOfficial,
                },
                "owned",
                &AuthAccountMetadata::new("fixture", TimestampMs(1)).unwrap(),
            )
            .unwrap();
    }
    let app = crate::gateway::dynamic_tools_acceptance_router()
        .layer(Extension(db))
        .layer(Extension(DiagnosticJobs::open(root).unwrap()));
    let catalog_uri = "/v1/agent-apps/mcp?targetHostId=a";
    let payload = json!({"targetHostId":"a","id":"local","connection":{"transport":"stdio","command":"/usr/bin/touch","args":[root.join("executed")]}});
    assert_eq!(
        send(&app, "POST", "/v1/agent-apps/mcp", "a", A, payload.clone()).await.0,
        StatusCode::CREATED
    );
    let scope_a = McpCatalogScope::Tenant { scope: TenantScope::new("p", "a").unwrap() };
    let scope_b = McpCatalogScope::Tenant { scope: TenantScope::new("p", "b").unwrap() };
    let original = store.list(&scope_a).unwrap();
    let files = [root.join("a/.codex/config.toml"), root.join("b/.codex/config.toml")];
    let before = files.each_ref().map(|file| fs::read(file).unwrap());
    let binding = json!({"target":{"appId":"codex","installationId":installations[0].0,"targetHostId":"a"},"expectedRevision":1,"enabled":true});
    let checks = "/v1/agent-apps/mcp/checks?projectId=project&targetHostId=a&appId=codex";
    for (company, token) in [("b", A), ("a", B)] {
        for (method, uri, body) in [
            ("GET", catalog_uri, Value::Null),
            ("POST", "/v1/agent-apps/mcp", payload.clone()),
            ("PUT", "/v1/agent-apps/mcp/local/bindings", binding.clone()),
            ("POST", checks, json!({"serverIds":["remote"]})),
            (
                "POST",
                "/v1/agent-apps/mcp/resync",
                json!({"targetHostId":"a","targets":[{"appId":"codex","installationId":installations[0].0}]}),
            ),
        ] {
            assert_eq!(
                send(&app, method, uri, company, token, body).await.0,
                StatusCode::FORBIDDEN,
                "foreign member must be rejected before side effects: {method} {uri}"
            );
        }
    }
    assert_eq!(store.list(&scope_a).unwrap(), original);
    assert!(store.list(&scope_b).unwrap().is_empty());
    assert_eq!(files.each_ref().map(|file| fs::read(file).unwrap()), before);
    assert!(!root.join("executed").exists());
    for company in ["a", "b"] {
        assert!(!root.join(company).join("started").exists());
    }
    assert_eq!(listener.accept().unwrap_err().kind(), std::io::ErrorKind::WouldBlock);
    // Metadata/native preview never executes the catalog stdio command or discloses secrets.
    assert_eq!(send(&app, "GET", catalog_uri, "a", A, Value::Null).await.0, StatusCode::OK);
    for (index, company, token) in [(0, "a", A), (1, "b", B)] {
        let native = format!(
            "/v1/agent-apps/mcp/native?targetHostId={company}&appId=codex&installationId={}",
            installations[index].0.0
        );
        assert_eq!(send(&app, "GET", &native, company, token, Value::Null).await.0, StatusCode::OK);
        let account = format!(
            "/v1/agent-apps/mcp/native?targetHostId={company}&appId=claude&installationId={}&accountId=owned",
            installations[index].1.0
        );
        let (status, preview) = send(&app, "GET", &account, company, token, Value::Null).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(preview["status"], "present", "owned account source must actually be parsed");
        if company == "a" {
            assert_eq!(
                send(&app, "GET", &account, company, PEER, Value::Null).await.0,
                StatusCode::NOT_FOUND,
                "an account ID cannot borrow another member's credential directory"
            );
        }
    }
    // A real loopback connection remains live after foreign query/cancel attempts.
    let (status, _) = send(&app, "POST", "/v1/agent-apps/mcp", "a", A,
        json!({"targetHostId":"a","projectId":"project","id":"remote","connection":{"transport":"streamable_http","url":url}})).await;
    // Project writes require a separate grant; establish the fixture through the real store.
    assert_eq!(status, StatusCode::FORBIDDEN);
    let project = McpCatalogScope::Project {
        scope: vw_api_types::ProjectScope::new(
            TenantScope::new("p", "a").unwrap(),
            "project".into(),
        )
        .unwrap(),
    };
    store
        .create(
            &project,
            vw_agent_apps::mcp::catalog::McpCatalogDraft {
                id: "remote".into(),
                name: None,
                description: None,
                homepage: None,
                documentation: None,
                tags: vec![],
                connection: vw_api_types::mcp_management::McpConnectionDefinition::StreamableHttp {
                    url: url.clone(),
                    headers: Default::default(),
                },
            },
        )
        .unwrap();
    let observed = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let count = observed.clone();
    let server = tokio::spawn(async move {
        axum::serve(
            tokio::net::TcpListener::from_std(listener).unwrap(),
            Router::new().fallback(move || {
                let count = count.clone();
                async move {
                    count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    std::future::pending::<StatusCode>().await
                }
            }),
        )
        .await
        .unwrap();
    });
    let (status, job) = send(&app, "POST", checks, "a", A, json!({"serverIds":["remote"]})).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let job_uri = format!(
        "/v1/agent-apps/diagnostics/jobs/{}?projectId=project&targetHostId=a&appId=codex",
        job["taskId"].as_str().unwrap()
    );
    let cancel_uri = job_uri.replace("?", "/cancel?");
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    while observed.load(std::sync::atomic::Ordering::SeqCst) == 0 {
        assert!(tokio::time::Instant::now() < deadline, "loopback connection must start");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    for (company, token) in [("b", B), ("a", PEER)] {
        assert_eq!(
            send(
                &app,
                "GET",
                &job_uri.replace("targetHostId=a", &format!("targetHostId={company}")),
                company,
                token,
                Value::Null
            )
            .await
            .0,
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            send(
                &app,
                "POST",
                &cancel_uri.replace("targetHostId=a", &format!("targetHostId={company}")),
                company,
                token,
                Value::Null
            )
            .await
            .0,
            StatusCode::NOT_FOUND
        );
    }
    let (status, live) = send(&app, "GET", &job_uri, "a", A, Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(live["state"]["status"], "running");
    assert_eq!(send(&app, "POST", &cancel_uri, "a", A, Value::Null).await.0, StatusCode::OK);
    loop {
        let (_, body) = send(&app, "GET", &job_uri, "a", A, Value::Null).await;
        if body["state"]["status"] == "cancelled" {
            break;
        }
        assert!(tokio::time::Instant::now() < deadline, "connection cancellation deadline");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(
        observed.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "poll/cancel cannot replay transport work"
    );
    let persisted = fs::read_to_string(root.join("diagnostics.json")).unwrap();
    for secret in [A, PEER, B, ENV_SECRET, HEADER_SECRET] {
        assert!(!persisted.contains(secret));
    }
    server.abort();
    let _ = server.await;
    let (status, bound) =
        send(&app, "PUT", "/v1/agent-apps/mcp/local/bindings", "a", A, binding).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(bound["items"][0]["nativeOutcome"]["status"], "applied");
    assert!(!root.join("executed").exists(), "publishing stdio configuration cannot execute it");
    // Remove only the managed entry; the operation must repair real native state.
    fs::write(&files[0], &before[0]).unwrap();
    // Both members have identical write grants. Only the submitting member owns this operation.
    let (status, submitted) = send(&app,"POST","/v1/agent-apps/mcp/resync","a",A,json!({"targetHostId":"a","targets":[{"appId":"codex","installationId":installations[0].0}]})).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let operation = format!(
        "/v1/agent-apps/operations/{}?targetHostId=a",
        submitted["operationId"].as_str().unwrap()
    );
    for (company, token) in [("b", B), ("a", PEER)] {
        for method in ["GET", "DELETE"] {
            assert_eq!(
                send(&app, method, &operation, company, token, Value::Null).await.0,
                StatusCode::NOT_FOUND,
                "resync must bind the exact submitting member"
            );
        }
        let events = operation.replace("?", "/events?");
        assert_eq!(
            send(&app, "GET", &events, company, token, Value::Null).await.0,
            StatusCode::NOT_FOUND
        );
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    loop {
        let (status, body) = send(&app, "GET", &operation, "a", A, Value::Null).await;
        assert_eq!(status, StatusCode::OK);
        if body["phase"] == "terminal" {
            assert_eq!(body["outcome"], "succeeded");
            assert_eq!(
                body["targets"][0]["distribution"]["items"][0]["nativeOutcome"]["status"],
                "applied"
            );
            break;
        }
        assert!(tokio::time::Instant::now() < deadline, "resync deadline");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(send(&app, "DELETE", &operation, "a", A, Value::Null).await.0, StatusCode::OK);
    assert!(fs::read_to_string(&files[0]).unwrap().contains("mcp_servers.local"));
    assert_eq!(fs::read(&files[1]).unwrap(), before[1]);
    assert!(!root.join("executed").exists());
    for (path, expected) in account_files {
        assert_eq!(fs::read(path).unwrap(), expected);
    }
    fs::write(root.join("verified"), b"").unwrap();
}
