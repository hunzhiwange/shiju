use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command};

use axum::{
    Extension, Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tower::ServiceExt;
use vw_api_types::{TenantScope, mcp_management::McpCatalogScope};

use crate::{agent_apps::mcp::McpCatalogStore, global_database::GlobalDatabase};

const TOKEN: &str = "fixture-token-for-the-mcp-catalog-writer";
const BASIC: &str = "Basic dGVzdDpzZWNyZXQ=";
const SKEY: &str = "Bearer dynamic-acceptance-skey";
const FIXTURE_ENV: &str = "SHIJU_MCP_TASK38_TEST_ROOT";

async fn send(
    app: &Router,
    method: &str,
    uri: &str,
    company: &str,
    auth: Option<&str>,
    payload: &Value,
) -> (StatusCode, Value) {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .header("x-platform-id", "p")
        .header("x-company-id", company)
        .header("x-agent-apps-token", TOKEN);
    if let Some(auth) = auth {
        request = request.header("authorization", auth);
    }
    let response =
        app.clone().oneshot(request.body(Body::from(payload.to_string())).unwrap()).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 65536).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

#[tokio::test]
async fn cc_mcp_t38_resync_repairs_native_and_reuses_owned_operations() {
    let Some(root) = std::env::var_os(FIXTURE_ENV) else {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let path = root.path().join("access.json");
        let hosts = ["host", "read-only", "project-only"].map(|id| {
            json!({
                "id":id,"online":true,"binding":"local","home":root.path(),
                "applications":root.path().join("Applications"),"search_paths":[root.path().join("bin")],"credential_root":root.path().join("private")
            })
        });
        let policy = json!({
            "principals":[{"id":"writer","token_sha256":hex::encode(Sha256::digest(TOKEN)),
                "grants":[{"platform_id":"p","company_id":"a","host_id":"host",
                    "mcp_catalog_write":true,"provider_config_write":true,"mcp_catalog_write_projects":["project"]},
                    {"platform_id":"p","company_id":"a","host_id":"read-only"},
                    {"platform_id":"p","company_id":"a","host_id":"project-only",
                    "mcp_catalog_write_projects":["project"]}]}],
            "hosts": hosts
        });
        fs::write(&path, policy.to_string()).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        let status = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                concat!(
                    module_path!(),
                    "::cc_mcp_t38_resync_repairs_native_and_reuses_owned_operations"
                )
                .split_once("::")
                .unwrap()
                .1,
                "--nocapture",
            ])
            .env(FIXTURE_ENV, root.path())
            .env("SHIJU_AGENT_APPS_ACCESS_FILE", &path)
            .env("VIBEWINDOW_SERVER_USERNAME", "test")
            .env("VIBEWINDOW_SERVER_PASSWORD", "secret")
            .status()
            .unwrap();
        assert!(status.success(), "MCP resync router fixture failed");
        assert!(root.path().join("verified").is_file(), "isolated test must actually run");
        return;
    };
    let root = Path::new(&root);
    let database = GlobalDatabase::new(root.join("catalog.sqlite3")).unwrap();
    let store = McpCatalogStore::new(database.clone()).unwrap();
    let app = crate::gateway::router::build_router(Vec::new()).layer(Extension(database.clone()));
    let client = crate::gateway::dynamic_tools_acceptance_router().layer(Extension(database));
    let scope = McpCatalogScope::Tenant { scope: TenantScope::new("p", "a").unwrap() };
    let env = vw_agent_apps::apps::discovery::read::LocalReadEnvironment {
        host_id: "host".into(),
        home: root.into(),
        search_paths: vec![root.join("bin")],
        applications: root.join("Applications"),
    };
    fs::create_dir_all(root.join("bin")).unwrap();
    fs::create_dir_all(root.join("private")).unwrap();
    fs::set_permissions(root.join("private"), fs::Permissions::from_mode(0o700)).unwrap();
    let binary = root.join("bin/codex");
    fs::write(&binary, "#!/bin/sh\nprintf '1.2.3\\n'\n").unwrap();
    fs::set_permissions(&binary, fs::Permissions::from_mode(0o700)).unwrap();
    let installation =
        vw_agent_apps::apps::discovery::read::read(vw_api_types::agent_apps::AppId::Codex, &env)
            .unwrap()
            .installations
            .remove(0);
    let record = store
        .create(
            &scope,
            vw_agent_apps::mcp::catalog::McpCatalogDraft {
                id: "fixture".into(),
                name: None,
                description: None,
                homepage: None,
                documentation: None,
                tags: vec![],
                connection: vw_api_types::mcp_management::McpConnectionDefinition::Stdio {
                    command: "fixture".into(),
                    args: vec![],
                    env: Default::default(),
                    cwd: None,
                },
            },
        )
        .unwrap();
    let native = root.join(".codex/config.toml");
    let mut binding = json!({"target":{"appId":"codex","installationId":installation.id,"targetHostId":"host"},"enabled":true,"expectedRevision":record.record.revision});
    let uri = "/v1/agent-apps/mcp/resync";
    let payload = json!({"targetHostId":"host", "targets":[{"appId":"codex","installationId":installation.id},{"appId":"gemini","installationId":"missing"}]});
    for (router, auth) in [(&app, BASIC), (&client, SKEY)] {
        assert_eq!(
            send(router, "POST", uri, "a", None, &payload).await.0,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            send(router, "POST", uri, "b", Some(auth), &payload).await.0,
            StatusCode::FORBIDDEN,
            "resync must enforce tenant authorization"
        );
    }
    let mut denied = payload.clone();
    denied["targetHostId"] = json!("read-only");
    assert_eq!(send(&app, "POST", uri, "a", Some(BASIC), &denied).await.0, StatusCode::FORBIDDEN);
    denied = payload.clone();
    denied["projectId"] = json!("other");
    assert_eq!(send(&app, "POST", uri, "a", Some(BASIC), &denied).await.0, StatusCode::FORBIDDEN);
    let mut invalid = payload.clone();
    invalid["targets"][0]["path"] = json!("/untrusted");
    assert_eq!(
        send(&app, "POST", uri, "a", Some(BASIC), &invalid).await.0,
        StatusCode::BAD_REQUEST
    );
    assert!(!native.exists());
    // The host contract does not create an absent application directory. HTTP 200
    // preserves the saved desired binding and its explicit projection failure.
    let (status, pending) =
        send(&app, "PUT", "/v1/agent-apps/mcp/fixture/bindings", "a", Some(BASIC), &binding).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(pending["outcome"], "failed", "{pending}");
    assert_eq!(pending["items"][0]["desiredCommitted"], true);
    assert_eq!(pending["items"][0]["nativeOutcome"]["status"], "failed");
    assert_eq!(pending["items"][0]["nativeOutcome"]["code"], "target_unavailable");
    assert_eq!(
        serde_json::to_value(&store.list(&scope).unwrap()[0].record).unwrap(),
        pending["items"][0]["record"]
    );
    assert!(!native.exists());
    binding["expectedRevision"] = pending["items"][0]["record"]["revision"].clone();
    fs::create_dir_all(native.parent().unwrap()).unwrap();
    let (status, bound) =
        send(&app, "PUT", "/v1/agent-apps/mcp/fixture/bindings", "a", Some(BASIC), &binding).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        bound["items"][0]["nativeOutcome"]["status"], "applied",
        "binding fixture must publish native intent: {bound}"
    );
    fs::write(&native, "# externally removed managed entry\n").unwrap();
    let desired = store.list(&scope).unwrap();
    let (status, submitted) = send(&app, "POST", uri, "a", Some(BASIC), &payload).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let id = submitted["operationId"].as_str().unwrap();
    let operation_uri = format!("/v1/agent-apps/operations/{id}?targetHostId=host");
    let mut completed = Value::Null;
    for _ in 0..1000 {
        let (status, body) =
            send(&app, "GET", &operation_uri, "a", Some(BASIC), &Value::Null).await;
        assert_eq!(status, StatusCode::OK);
        if body["phase"] == "terminal" {
            completed = body;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert_eq!(completed["status"], "failed");
    assert_eq!(completed["outcome"], "partially_succeeded");
    assert_eq!(completed["targets"][0]["appId"], "codex");
    assert_eq!(
        completed["targets"][0]["distribution"]["items"][0]["nativeOutcome"]["status"],
        "applied"
    );
    assert_eq!(completed["targets"][1]["code"], "not_found");
    assert!(fs::read_to_string(&native).unwrap().contains("fixture"));
    assert_eq!(store.list(&scope).unwrap(), desired);
    assert!(!completed.to_string().contains(root.to_str().unwrap()));
    assert_eq!(
        send(&app, "GET", &operation_uri, "b", Some(BASIC), &Value::Null).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        send(&app, "DELETE", &operation_uri, "b", Some(BASIC), &Value::Null).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        send(&app, "GET", &operation_uri, "a", None, &Value::Null).await.0,
        StatusCode::UNAUTHORIZED
    );
    let events_uri = format!("/v1/agent-apps/operations/{id}/events?targetHostId=host");
    assert_eq!(send(&app, "GET", &events_uri, "a", Some(BASIC), &Value::Null).await.1, completed);
    let before = fs::read(&native).unwrap();
    let (status, cancelled) =
        send(&app, "DELETE", &operation_uri, "a", Some(BASIC), &Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(cancelled, completed);
    assert_eq!(fs::read(&native).unwrap(), before);
    // The bootstrap skey path submits the same production operation contract.
    let (status, submitted) = send(&client, "POST", uri, "a", Some(SKEY), &payload).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let uri = format!(
        "/v1/agent-apps/operations/{}?targetHostId=host",
        submitted["operationId"].as_str().unwrap()
    );
    let (status, cancelled) = send(&client, "DELETE", &uri, "a", Some(SKEY), &Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(cancelled["phase"], "terminal");
    assert_eq!(cancelled["status"], "cancelled");
    assert_eq!(cancelled["outcome"], "cancelled");
    assert_eq!(fs::read(&native).unwrap(), before);
    assert_eq!(store.list(&scope).unwrap(), desired);
    fs::write(root.join("verified"), b"").unwrap();
}
