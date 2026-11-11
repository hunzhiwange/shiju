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
const FIXTURE_ENV: &str = "SHIJU_MCP_TASK36_TEST_ROOT";

async fn send(
    app: &Router,
    company: &str,
    authorization: Option<&str>,
    member: bool,
    payload: &Value,
    bulk: bool,
) -> (StatusCode, Value) {
    let mut request = Request::builder()
        .method(if bulk { "POST" } else { "PUT" })
        .uri(if bulk { "/v1/agent-apps/mcp/bulk" } else { "/v1/agent-apps/mcp/fixture/bindings" })
        .header("content-type", "application/json")
        .header("x-platform-id", "p")
        .header("x-company-id", company);
    if let Some(authorization) = authorization {
        request = request.header("authorization", authorization);
    }
    if member {
        request = request.header("x-agent-apps-token", TOKEN);
    }
    let response =
        app.clone().oneshot(request.body(Body::from(payload.to_string())).unwrap()).await.unwrap();
    let status = response.status();
    if matches!(status, StatusCode::OK | StatusCode::CONFLICT | StatusCode::BAD_REQUEST) {
        assert_eq!(response.headers().get("cache-control").unwrap(), "no-store");
    }
    let body = to_bytes(response.into_body(), 65536).await.unwrap();
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

#[tokio::test]
async fn cc_mcp_t36_bind_and_bulk_apply_real_native_results() {
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
                concat!(module_path!(), "::cc_mcp_t36_bind_and_bulk_apply_real_native_results")
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
        assert!(status.success(), "MCP deletion router fixture failed");
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
    let mut payload = json!({"target":{"appId":"codex","installationId":installation.id,"targetHostId":"host"},"enabled":true,"expectedRevision":record.record.revision});
    for (router, auth) in [(&app, BASIC), (&client, SKEY)] {
        assert_eq!(
            send(router, "a", None, true, &payload, false).await.0,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            send(router, "b", Some(auth), true, &payload, false).await.0,
            StatusCode::FORBIDDEN
        );
    }
    let mut unknown = payload.clone();
    unknown["target"]["appId"] = json!("unknown");
    assert_eq!(
        send(&app, "a", Some(BASIC), true, &unknown, false).await.0,
        StatusCode::BAD_REQUEST
    );
    let mut denied = payload.clone();
    denied["projectId"] = json!("other");
    assert_eq!(send(&app, "a", Some(BASIC), true, &denied, false).await.0, StatusCode::FORBIDDEN);
    assert!(!native.exists());
    assert_eq!(store.list(&scope).unwrap(), [record]);
    let (status, body) = send(&app, "a", Some(BASIC), true, &payload, false).await;
    assert_eq!(status, StatusCode::OK, "binding route must publish actual native configuration");
    assert_eq!(body["outcome"], "succeeded");
    assert_eq!(body["items"][0]["nativeOutcome"]["status"], "applied");
    assert!(fs::read_to_string(&native).unwrap().contains("fixture"));
    let current = body["items"][0]["record"].clone();
    payload.as_object_mut().unwrap().remove("expectedRevision");
    payload["records"] = json!([current, {"id":"missing","revision":1}]);
    payload["enabled"] = json!(false);
    let (status, body) = send(&client, "a", Some(SKEY), true, &payload, true).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["outcome"], "partially_succeeded");
    assert_eq!(body["items"][0]["nativeOutcome"]["status"], "applied");
    assert_eq!(body["items"][1]["desiredCommitted"], false);
    assert_eq!(body["items"][1]["nativeOutcome"]["code"], "not_found");
    assert!(!fs::read_to_string(&native).unwrap().contains("mcp_servers.fixture"));
    fs::write(root.join("verified"), b"").unwrap();
}
