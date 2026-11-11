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
const FIXTURE_ENV: &str = "SHIJU_MCP_TASK35_TEST_ROOT";

async fn delete(
    app: &Router,
    company: &str,
    authorization: Option<&str>,
    member: bool,
    payload: &Value,
) -> (StatusCode, Value) {
    let mut request = Request::delete("/v1/agent-apps/mcp/fixture")
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
async fn cc_mcp_t35_delete_revokes_each_target_and_retries_only_failures() {
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
                    "::cc_mcp_t35_delete_revokes_each_target_and_retries_only_failures"
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
    let codex =
        "[mcp_servers.fixture]\ncommand = \"fixture\"\n[mcp_servers.other]\ncommand = \"other\"\n";
    let gemini = r#"{"mcpServers":{"fixture":{"command":"fixture"},"other":{"command":"other"}}}"#;
    for (name, directory, file, bytes) in
        [("codex", ".codex", "config.toml", codex), ("gemini", ".gemini", "settings.json", gemini)]
    {
        let binary = root.join("bin").join(name);
        fs::write(&binary, "#!/bin/sh\nprintf '1.2.3\\n'\n").unwrap();
        fs::set_permissions(binary, fs::Permissions::from_mode(0o700)).unwrap();
        fs::create_dir_all(root.join(directory)).unwrap();
        fs::write(root.join(directory).join(file), bytes).unwrap();
    }
    for app_id in [vw_api_types::agent_apps::AppId::Codex, vw_api_types::agent_apps::AppId::Gemini]
    {
        let installation = vw_agent_apps::apps::discovery::read::read(app_id, &env)
            .unwrap()
            .installations
            .remove(0);
        let target = vw_api_types::mcp_management::McpAppTarget {
            app_id,
            installation_id: installation.id,
            target_host_id: "host".into(),
        };
        let reports = store.import_from_apps(
            &scope,
            &[crate::agent_apps::mcp::McpImportSource {
                target,
                native: vw_agent_apps::mcp::import::NativeMcpSource::HostHome(root),
            }],
        );
        assert!(reports[0].entries.as_ref().unwrap().iter().all(|entry| entry.outcome.is_ok()));
    }
    let record = store
        .list(&scope)
        .unwrap()
        .into_iter()
        .find(|record| record.record.id.0 == "fixture")
        .unwrap();
    let payload = json!({"targetHostId":"host","expectedRevision":record.record.revision});
    for (router, auth) in [(&app, BASIC), (&client, SKEY)] {
        assert_eq!(delete(router, "a", None, true, &payload).await.0, StatusCode::UNAUTHORIZED);
        assert_eq!(
            delete(router, "a", Some(auth), false, &payload).await.0,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(delete(router, "b", Some(auth), true, &payload).await.0, StatusCode::FORBIDDEN);
    }
    let mut denied = payload.clone();
    denied["targetHostId"] = json!("read-only");
    assert_eq!(delete(&app, "a", Some(BASIC), true, &denied).await.0, StatusCode::FORBIDDEN);
    let mut stale = payload.clone();
    stale["expectedRevision"] = json!(record.record.revision + 1);
    assert_eq!(delete(&app, "a", Some(BASIC), true, &stale).await.0, StatusCode::CONFLICT);
    assert_eq!(store.list(&scope).unwrap().len(), 2);
    assert_eq!(fs::read_to_string(root.join(".codex/config.toml")).unwrap(), codex);
    assert_eq!(fs::read_to_string(root.join(".gemini/settings.json")).unwrap(), gemini);
    fs::write(root.join(".gemini/settings.json"), "{broken").unwrap();
    let (status, body) = delete(&app, "a", Some(BASIC), true, &payload).await;
    assert_eq!(status, StatusCode::OK, "authorized DELETE must invoke native revocation");
    assert_eq!(body["outcome"], "partially_succeeded");
    let items = body["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    assert!(items.iter().all(|item| item["desiredCommitted"] == true && item["enabled"] == false));
    assert_eq!(items[0]["nativeOutcome"]["status"], "applied");
    assert_eq!(items[1]["nativeOutcome"]["status"], "failed");
    assert_eq!(items[1]["nativeOutcome"]["code"], "invalid_definition");
    assert!(!body.to_string().contains(root.to_str().unwrap()));
    let cleaned = fs::read_to_string(root.join(".codex/config.toml")).unwrap();
    assert!(!cleaned.contains("mcp_servers.fixture"));
    assert!(cleaned.contains("mcp_servers.other"));
    assert_eq!(fs::read_to_string(root.join(".gemini/settings.json")).unwrap(), "{broken");
    assert_eq!(store.list(&scope).unwrap().len(), 1);
    // A completed target is terminal: a later external replacement must survive retry.
    fs::write(root.join(".codex/config.toml"), codex).unwrap();
    fs::write(root.join(".gemini/settings.json"), gemini).unwrap();
    let (status, body) = delete(&client, "a", Some(SKEY), true, &payload).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["outcome"], "succeeded");
    assert_eq!(body["items"][0]["nativeOutcome"]["status"], "unchanged");
    assert_eq!(fs::read_to_string(root.join(".codex/config.toml")).unwrap(), codex);
    let remaining: Value =
        serde_json::from_slice(&fs::read(root.join(".gemini/settings.json")).unwrap()).unwrap();
    assert!(remaining["mcpServers"].get("fixture").is_none());
    assert!(remaining["mcpServers"].get("other").is_some());
    let project_scope = McpCatalogScope::Project {
        scope: vw_api_types::ProjectScope::new(
            TenantScope::new("p", "a").unwrap(),
            "project".into(),
        )
        .unwrap(),
    };
    let project_record = store
        .create(
            &project_scope,
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
    let mut project_payload = json!({"targetHostId":"project-only", "projectId":"other",
        "expectedRevision": project_record.record.revision});
    assert_eq!(
        delete(&app, "a", Some(BASIC), true, &project_payload).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(store.list(&project_scope).unwrap().len(), 1);
    project_payload["projectId"] = json!("project");
    let (status, body) = delete(&app, "a", Some(BASIC), true, &project_payload).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["outcome"], "succeeded");
    assert!(body["items"].as_array().unwrap().is_empty());
    assert!(store.list(&project_scope).unwrap().is_empty());
    assert_eq!(store.list(&scope).unwrap().len(), 1);
    let mut invalid = payload.clone();
    invalid["companyId"] = json!("b");
    assert_eq!(delete(&app, "a", Some(BASIC), true, &invalid).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(fs::read_to_string(root.join(".codex/config.toml")).unwrap(), codex);
    fs::write(root.join("verified"), b"").unwrap();
}
