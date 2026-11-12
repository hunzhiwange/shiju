//! Host policy, installation identity and native filesystem isolation through the HTTP router.
use super::McpCatalogStore;
use crate::global_database::GlobalDatabase;
use axum::{
    Extension, Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command};
use tower::ServiceExt;
use vw_agent_apps::apps::discovery::read::{self, LocalReadEnvironment};
use vw_api_types::{TenantScope, agent_apps::AppId, mcp_management::McpCatalogScope};

const ROOT: &str = "SHIJU_MCP_TASK52_ROOT";
const TOKEN: &str = "fixture-t52-member-token-at-least-32-bytes";
const TEST: &str = "cc_mcp_t52_unavailable_hosts_never_fall_back_and_local_users_remain_isolated";

fn directory(path: &Path) {
    fs::create_dir_all(path).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
}
async fn send(app: &Router, method: &str, uri: &str, body: Value) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("authorization", "Bearer dynamic-acceptance-skey")
                .header("x-agent-apps-token", TOKEN)
                .header("x-platform-id", "p")
                .header("x-company-id", "company")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 65536).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}
fn target(host: &str, installation: &Value) -> Value {
    json!({"targetHostId":host,"appId":"codex","installationId":installation})
}
fn binding(target: Value, revision: u64, enabled: bool) -> Value {
    json!({"target":target,"expectedRevision":revision,"enabled":enabled})
}

#[tokio::test]
async fn cc_mcp_t52_unavailable_hosts_never_fall_back_and_local_users_remain_isolated() {
    let Some(root) = std::env::var_os(ROOT) else {
        let root = tempfile::tempdir().unwrap();
        directory(&root.path().join("desktop/.codex"));
        fs::write(root.path().join("desktop/.codex/config.toml"), b"# desktop sentinel\n").unwrap();
        let output = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", &format!("agent_apps::mcp::host_scope_tests::{TEST}"), "--nocapture"])
            .env(ROOT, root.path())
            .env("HOME", root.path().join("desktop"))
            .env("SHIJU_AGENT_APPS_ACCESS_FILE", root.path().join("access.json"))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "host regression failed: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(root.path().join("verified").is_file(), "child must execute behavior assertions");
        return;
    };
    let root = Path::new(&root);
    let mut hosts = Vec::new();
    let mut grants = Vec::new();
    let mut installations = Vec::new();
    for host in ["user-a", "user-b", "offline", "remote"] {
        let home = root.join(host);
        for path in [home.join("bin"), home.join("private"), home.join(".codex")] {
            directory(&path);
        }
        let binary = home.join("bin/codex");
        fs::write(&binary, "#!/bin/sh\ntouch \"$(dirname \"$0\")/../probed\"\nprintf '1.2.3\\n'\n")
            .unwrap();
        fs::set_permissions(&binary, fs::Permissions::from_mode(0o700)).unwrap();
        let env = LocalReadEnvironment {
            host_id: host.into(),
            home: home.clone(),
            search_paths: vec![home.join("bin")],
            applications: home.join("Applications"),
        };
        let installation = read::read(AppId::Codex, &env).unwrap().installations.remove(0);
        installations.push(json!(installation.id));
        fs::remove_file(home.join("probed")).unwrap();
        fs::write(home.join(".codex/config.toml"), format!("# {host} sentinel\n")).unwrap();
        hosts.push(json!({"id":host,"online":host!="offline","binding":if host=="remote" {"remote"} else {"local"},"home":home,"search_paths":env.search_paths,"applications":env.applications,"credential_root":home.join("private")}));
        grants.push(json!({"platform_id":"p","company_id":"company","host_id":host,"mcp_catalog_write":true,"provider_config_read":true,"provider_config_write":true}));
    }
    assert_ne!(installations[0], installations[1], "installation identity includes the host");
    let policy = root.join("access.json");
    fs::write(&policy, json!({"principals":[{"id":"member","token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":grants}],"hosts":hosts}).to_string()).unwrap();
    fs::set_permissions(&policy, fs::Permissions::from_mode(0o600)).unwrap();
    let database = GlobalDatabase::new(root.join("catalog.sqlite3")).unwrap();
    let store = McpCatalogStore::new(database.clone()).unwrap();
    let scope = McpCatalogScope::Tenant { scope: TenantScope::new("p", "company").unwrap() };
    let app = crate::gateway::dynamic_tools_acceptance_router().layer(Extension(database));
    let create = |host: &str| json!({"targetHostId":host,"id":"shared","connection":{"transport":"stdio","command":"/usr/bin/touch","args":[root.join("executed")]}});
    assert_eq!(
        send(&app, "POST", "/v1/agent-apps/mcp", create("user-a")).await.0,
        StatusCode::CREATED
    );
    let original = store.list(&scope).unwrap();
    let files = ["desktop", "user-a", "user-b", "offline", "remote"]
        .map(|host| root.join(host).join(".codex/config.toml"));
    let before = files.each_ref().map(|p| fs::read(p).unwrap());
    for (index, host, expected) in [
        (2, "offline", StatusCode::SERVICE_UNAVAILABLE),
        (3, "remote", StatusCode::NOT_IMPLEMENTED),
    ] {
        let target = target(host, &installations[index]);
        let native = format!(
            "/v1/agent-apps/mcp/native?targetHostId={host}&appId=codex&installationId={}",
            installations[index].as_str().unwrap()
        );
        for (method, uri, payload) in [
            ("GET", native.as_str(), Value::Null),
            ("POST", "/v1/agent-apps/mcp", create(host)),
            ("PUT", "/v1/agent-apps/mcp/shared/bindings", binding(target.clone(), 1, true)),
            ("POST", "/v1/agent-apps/mcp/import", json!({"sources":[{"target":target}]})),
            (
                "POST",
                "/v1/agent-apps/mcp/resync",
                json!({"targetHostId":host,"targets":[{"appId":"codex","installationId":installations[index]}]}),
            ),
        ] {
            let (status, body) = send(&app, method, uri, payload).await;
            assert_eq!(status, expected, "{host}: {method} {uri}: {body}");
            assert_eq!(body["code"], "target_unavailable");
        }
    }
    assert_eq!(
        store.list(&scope).unwrap(),
        original,
        "unavailable targets cannot commit desired changes"
    );
    assert_eq!(files.each_ref().map(|p| fs::read(p).unwrap()), before);
    for host in ["user-a", "user-b", "offline", "remote"] {
        assert!(
            !root.join(host).join("probed").exists(),
            "rejected requests cannot execute CLI discovery"
        );
    }
    // A valid local host cannot borrow another user's installation identity.
    let (status, _) = send(
        &app,
        "PUT",
        "/v1/agent-apps/mcp/shared/bindings",
        binding(target("user-b", &installations[0]), 1, true),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(store.list(&scope).unwrap(), original);
    // Same service, same tenant, distinct host bindings and physical native files.
    for (index, revision) in [(0, 1), (1, 2)] {
        let host = if index == 0 { "user-a" } else { "user-b" };
        let other = fs::read(&files[if index == 0 { 2 } else { 1 }]).unwrap();
        let (status, body) = send(
            &app,
            "PUT",
            "/v1/agent-apps/mcp/shared/bindings",
            binding(target(host, &installations[index]), revision, true),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["outcome"], "succeeded");
        assert_eq!(body["items"][0]["desiredCommitted"], true);
        assert_eq!(body["items"][0]["nativeOutcome"]["status"], "applied");
        assert!(fs::read_to_string(&files[index + 1]).unwrap().contains("mcp_servers.shared"));
        assert_eq!(fs::read(&files[if index == 0 { 2 } else { 1 }]).unwrap(), other);
    }
    for host in ["user-a", "user-b"] {
        let (status, body) =
            send(&app, "GET", &format!("/v1/agent-apps/mcp?targetHostId={host}"), Value::Null)
                .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["bindings"].as_array().unwrap().len(), 1);
        assert_eq!(body["bindings"][0]["target"]["targetHostId"], host);
    }
    let b = fs::read(&files[2]).unwrap();
    let (status, body) = send(
        &app,
        "PUT",
        "/v1/agent-apps/mcp/shared/bindings",
        binding(target("user-a", &installations[0]), 3, false),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["items"][0]["nativeOutcome"]["status"], "applied");
    assert!(!fs::read_to_string(&files[1]).unwrap().contains("mcp_servers.shared"));
    assert_eq!(fs::read(&files[2]).unwrap(), b, "disabling A must preserve B's same-name service");
    let persisted = store.list(&scope).unwrap();
    // Trusted local host whose configured HOME vanished cannot fall back to ambient HOME.
    fs::rename(root.join("user-a"), root.join("user-a-away")).unwrap();
    let (status, _) = send(
        &app,
        "PUT",
        "/v1/agent-apps/mcp/shared/bindings",
        binding(target("user-a", &installations[0]), 4, true),
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(!root.join("user-a").exists());
    assert_eq!(store.list(&scope).unwrap(), persisted);
    assert_eq!(fs::read(&files[0]).unwrap(), before[0], "ambient desktop HOME remains untouched");
    assert_eq!(fs::read(&files[2]).unwrap(), b);
    for index in [3, 4] {
        assert_eq!(fs::read(&files[index]).unwrap(), before[index]);
    }
    assert!(!root.join("executed").exists(), "binding configuration must not run stdio service");
    fs::write(root.join("verified"), b"").unwrap();
}
