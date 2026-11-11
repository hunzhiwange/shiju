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
const FIXTURE_ENV: &str = "SHIJU_MCP_TASK37_TEST_ROOT";

async fn send(
    app: &Router,
    company: &str,
    authorization: Option<&str>,
    member: bool,
    payload: &Value,
) -> (StatusCode, Value) {
    let mut request = Request::builder()
        .method("POST")
        .uri("/v1/agent-apps/mcp/import")
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
async fn cc_mcp_t37_import_preserves_partial_results_and_authorization() {
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
                    "mcp_catalog_write":true,"provider_config_read":true,"mcp_catalog_write_projects":["project"]},
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
                    "::cc_mcp_t37_import_preserves_partial_results_and_authorization"
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
        assert!(status.success(), "MCP import router fixture failed");
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
    fs::create_dir_all(root.join(".codex")).unwrap();
    let native = root.join(".codex/config.toml");
    fs::write(&native, "[mcp_servers.shared]\ncommand = 'fixture'\n").unwrap();
    let target = vw_api_types::mcp_management::McpAppTarget {
        app_id: vw_api_types::agent_apps::AppId::Codex,
        installation_id: vw_api_types::agent_apps::AppInstallationId(
            "previous-installation".into(),
        ),
        target_host_id: "host".into(),
    };
    let seeded = store.import_from_apps(
        &scope,
        &[crate::agent_apps::mcp::McpImportSource {
            target,
            native: vw_agent_apps::mcp::import::NativeMcpSource::HostHome(root),
        }],
    );
    assert!(matches!(
        &seeded[0].entries.as_ref().unwrap()[0].outcome,
        Ok(crate::agent_apps::mcp::McpImportOutcome::Added(_))
    ));
    let original = "[mcp_servers.shared]\ncommand = 'fixture'\n[mcp_servers.added]\ncommand = 'fixture'\n[mcp_servers.bad]\ncommand = 42\n";
    fs::write(&native, original).unwrap();
    let payload = json!({"sources":[{"target":{"appId":"codex","installationId":installation.id,"targetHostId":"host"}}]});
    for (router, auth) in [(&app, BASIC), (&client, SKEY)] {
        assert_eq!(send(router, "a", None, true, &payload).await.0, StatusCode::UNAUTHORIZED);
        assert_eq!(send(router, "b", Some(auth), true, &payload).await.0, StatusCode::FORBIDDEN);
    }
    assert_eq!(send(&app, "a", Some(BASIC), false, &payload).await.0, StatusCode::UNAUTHORIZED);
    let mut mixed = payload.clone();
    mixed["sources"].as_array_mut().unwrap().push(json!({"target":{"appId":"codex","installationId":installation.id,"targetHostId":"read-only"}}));
    assert_eq!(send(&app, "a", Some(BASIC), true, &mixed).await.0, StatusCode::FORBIDDEN);
    assert_eq!(store.list(&scope).unwrap().len(), 1);
    let (status, body) = send(&app, "a", Some(BASIC), true, &payload).await;
    assert_eq!(status, StatusCode::OK, "import route must return per-entry results");
    assert_eq!(body["sources"][0]["target"], payload["sources"][0]["target"]);
    assert_eq!(fs::read_to_string(&native).unwrap(), original);
    assert!(!body.to_string().contains(root.to_str().unwrap()));
    let entries = body["sources"][0]["entries"].as_array().unwrap();
    let outcome = |id: &str| entries.iter().find(|entry| entry["id"] == id).unwrap();
    assert_eq!(outcome("added")["status"], "added");
    assert_eq!(outcome("shared")["status"], "merged");
    assert_eq!(outcome("bad")["code"], "invalid_definition");
    assert_eq!(outcome("added")["record"]["id"], "added");
    assert_eq!(outcome("added")["record"]["revision"], 1);
    assert_eq!(outcome("shared")["record"]["revision"], 2);
    let normalize = |mut records: Vec<vw_api_types::mcp_management::McpCatalogRecord>| {
        records.sort_by_key(|record| record.record.id.0.clone());
        records
    };
    let persisted = normalize(
        McpCatalogStore::new(GlobalDatabase::new(root.join("catalog.sqlite3")).unwrap())
            .unwrap()
            .list(&scope)
            .unwrap(),
    );
    let ids = persisted.iter().map(|record| record.record.id.0.as_str()).collect::<Vec<_>>();
    assert_eq!(ids, ["added", "shared"]);
    let conflicting =
        "[mcp_servers.shared]\ncommand = 'different'\n[mcp_servers.added]\ncommand = 'fixture'\n";
    fs::write(&native, conflicting).unwrap();
    let (status, body) = send(&client, "a", Some(SKEY), true, &payload).await;
    assert_eq!(status, StatusCode::OK);
    let entries = body["sources"][0]["entries"].as_array().unwrap();
    assert_eq!(entries.iter().find(|entry| entry["id"] == "shared").unwrap()["code"], "conflict");
    assert_eq!(entries.iter().find(|entry| entry["id"] == "added").unwrap()["status"], "unchanged");
    assert_eq!(fs::read_to_string(&native).unwrap(), conflicting);
    assert_eq!(normalize(store.list(&scope).unwrap()), persisted);
    let mut denied = payload.clone();
    denied["projectId"] = json!("other");
    assert_eq!(send(&app, "a", Some(BASIC), true, &denied).await.0, StatusCode::FORBIDDEN);
    let current = fs::read(&native).unwrap();
    let mut missing = payload.clone();
    missing["sources"][0]["target"]["installationId"] = json!("missing");
    let (status, body) = send(&app, "a", Some(BASIC), true, &missing).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["sources"][0]["code"], "not_found");
    assert_eq!(fs::read(&native).unwrap(), current);
    assert_eq!(normalize(store.list(&scope).unwrap()), persisted);
    assert!(!body.to_string().contains(root.to_str().unwrap()));
    fs::write(root.join("verified"), b"").unwrap();
}
