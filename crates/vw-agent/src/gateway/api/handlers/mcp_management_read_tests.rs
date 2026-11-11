use std::{fs, net::TcpListener, os::unix::fs::PermissionsExt, path::Path, process::Command};

use axum::{
    Extension, Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tower::ServiceExt;
use vw_agent_apps::{
    apps::discovery::read::{LocalReadEnvironment, read},
    auth::credential_file::{AccountCredential, CredentialFileStore},
    mcp::catalog::McpCatalogDraft,
};
use vw_api_types::{
    TenantScope, TimestampMs,
    agent_apps::AppId,
    agent_apps_auth::AuthService,
    mcp_management::{McpCatalogScope, McpConnectionDefinition},
};

use crate::{
    agent_apps::{
        auth_accounts::{AuthAccountMetadata, AuthAccountScope, AuthAccountStore},
        mcp::McpCatalogStore,
    },
    global_database::GlobalDatabase,
};

const TOKEN: &str = "fixture-token-for-the-mcp-catalog-reader";
const CATALOG: &str = "/v1/agent-apps/mcp?targetHostId=host";
const PRESETS: &str = "/v1/agent-apps/mcp/presets?targetHostId=host";
const NATIVE: &str = "/v1/agent-apps/mcp/native?targetHostId=host";
const POLICY_ENV: &str = "SHIJU_MCP_TASK32_TEST_ROOT";

fn write_policy(root: &Path) {
    let path = root.join("access.json");
    let policy = json!({
        "principals": [{"id":"reader", "token_sha256":hex::encode(Sha256::digest(TOKEN)),
            "grants":[{"platform_id":"p","company_id":"a","host_id":"host",
                       "provider_config_read":true,"auth_account_read":["claude_official"]},
                      {"platform_id":"p","company_id":"a","host_id":"metadata-only"}]}],
        "hosts": [
            {"id":"host","online":true,"binding":"local","home":root,
             "search_paths":[root.join("bin")],"applications":root.join("Applications"),
             "credential_root":root.join("credentials")},
            {"id":"metadata-only","online":true,"binding":"local","home":root,
             "search_paths":[root.join("bin")],"applications":root.join("Applications")}
        ]
    });
    fs::write(&path, policy.to_string()).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
}

async fn get(
    app: &Router,
    uri: &str,
    company: &str,
    authorization: Option<&str>,
    member: bool,
) -> (StatusCode, String) {
    let mut request =
        Request::builder().uri(uri).header("x-platform-id", "p").header("x-company-id", company);
    if let Some(authorization) = authorization {
        request = request.header("authorization", authorization);
    }
    if member {
        request = request.header("x-agent-apps-token", TOKEN);
    }
    let response = app.clone().oneshot(request.body(Body::empty()).unwrap()).await.unwrap();
    (
        response.status(),
        String::from_utf8(to_bytes(response.into_body(), 65536).await.unwrap().to_vec()).unwrap(),
    )
}

#[tokio::test]
async fn cc_mcp_t32_gateway_reads_are_authorized_redacted_and_offline() {
    let Some(root) = std::env::var_os(POLICY_ENV) else {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        write_policy(root.path());
        let result = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "gateway::api::handlers::mcp_management_read_tests::cc_mcp_t32_gateway_reads_are_authorized_redacted_and_offline",
                "--nocapture",
            ])
            .env(POLICY_ENV, root.path())
            .env("SHIJU_AGENT_APPS_ACCESS_FILE", root.path().join("access.json"))
            .env("VIBEWINDOW_SERVER_USERNAME", "test")
            .env("VIBEWINDOW_SERVER_PASSWORD", "secret")
            .status()
            .unwrap();
        assert!(result.success(), "MCP read router fixture failed");
        assert!(root.path().join("verified").is_file(), "isolated test must actually run");
        return;
    };
    let root = Path::new(&root);
    let bin = root.join("bin");
    fs::create_dir(&bin).unwrap();
    let cli = bin.join("codex");
    let marker = root.join("started");
    fs::write(&cli, "#!/bin/sh\ntouch \"$(dirname \"$0\")/../started\"\nprintf 'codex 1.2.3\\n'\n")
        .unwrap();
    fs::set_permissions(&cli, fs::Permissions::from_mode(0o700)).unwrap();
    fs::hard_link(&cli, bin.join("claude")).unwrap();
    let environment = LocalReadEnvironment {
        host_id: "host".into(),
        home: root.into(),
        search_paths: vec![bin],
        applications: root.join("Applications"),
    };
    let installations = read(AppId::Codex, &environment).unwrap().installations;
    let claude = read(AppId::Claude, &environment).unwrap().installations.remove(0).id;
    assert!(marker.is_file(), "version probe fixture must detect a process launch");
    fs::remove_file(&marker).unwrap();
    let installation = &installations[0].id.0;
    let native = format!("{NATIVE}&appId=codex&installationId={installation}");
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}/private-fixture-url", listener.local_addr().unwrap());
    let config = root.join(".codex/config.toml");
    fs::create_dir(config.parent().unwrap()).unwrap();
    let contents = format!(
        "[mcp_servers.remote]\nurl = {endpoint:?}\nhttp_headers = {{ Authorization = 'fixture-header-value' }}\n\n[mcp_servers.local]\ncommand = '/bin/sh'\nargs = ['-c', 'touch {}']\nenv = {{ FIXTURE = 'fixture-env-value' }}\n",
        marker.display()
    );
    fs::write(&config, &contents).unwrap();

    let database = GlobalDatabase::new(root.join("catalog.sqlite3")).unwrap();
    let store = McpCatalogStore::new(database.clone()).unwrap();
    for (company, id) in [("a", "remote"), ("a", "local"), ("a", "missing"), ("b", "foreign")] {
        store
            .create(
                &McpCatalogScope::Tenant { scope: TenantScope::new("p", company).unwrap() },
                McpCatalogDraft {
                    id: id.into(),
                    name: None,
                    description: None,
                    homepage: None,
                    documentation: None,
                    tags: vec![],
                    connection: McpConnectionDefinition::StreamableHttp {
                        url: endpoint.clone(),
                        headers: Default::default(),
                    },
                },
            )
            .unwrap();
    }
    let credential_root = root.join("credentials");
    fs::create_dir(&credential_root).unwrap();
    fs::set_permissions(&credential_root, fs::Permissions::from_mode(0o700)).unwrap();
    let tenant = TenantScope::new("p", "a").unwrap();
    let accounts = AuthAccountStore::new(database.clone(), &credential_root).unwrap();
    let owner = AuthAccountScope {
        tenant: &tenant,
        host: "host",
        principal: "reader",
        service: AuthService::ClaudeOfficial,
    };
    let credentials = CredentialFileStore::new(&credential_root).unwrap();
    let sse_url = format!("{endpoint}/events");
    let profile = |principal: &str, id: &str| {
        let store = credentials.for_owner(principal).unwrap();
        store.register_claude_directory(&tenant, "host", id).unwrap();
        let lease = store.lease(&tenant, "host", AppId::Claude, id, false).unwrap();
        let AccountCredential::ClaudeDirectory(directory) = lease.read().unwrap() else {
            panic!("expected an isolated Claude account directory");
        };
        let path = directory.path().join(".claude.json");
        fs::write(
            &path,
            json!({
                "mcpServers":{"remote":{"type":"sse","url":sse_url,
                    "headers":{"Authorization":"fixture-profile-value"}}}
            })
            .to_string(),
        )
        .unwrap();
        accounts
            .insert_metadata(
                &AuthAccountScope { principal, ..owner },
                id,
                &AuthAccountMetadata::new("fixture-identity", TimestampMs(1)).unwrap(),
            )
            .unwrap();
        path
    };
    let profile = (profile("reader", "owned"), profile("other", "neighbor"));
    let app = crate::gateway::router::build_router(Vec::new()).layer(Extension(database.clone()));
    let client = crate::gateway::dynamic_tools_acceptance_router().layer(Extension(database));
    let basic = Some("Basic dGVzdDpzZWNyZXQ=");
    let bearer = Some("Bearer dynamic-acceptance-skey");

    let (status, body) = get(&app, CATALOG, "a", basic, true).await;
    assert_eq!(status, StatusCode::OK, "authorized catalog route must serve stored projections");
    let catalog: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(catalog["records"].as_array().unwrap().len(), 3);
    assert!(!body.contains("foreign") && !body.contains(&endpoint));
    let capabilities = catalog["capabilities"].as_array().unwrap();
    assert_eq!(capabilities.len(), 14);
    assert!(capabilities.iter().any(|row| row["appId"] == "agy" && row["sse"] == "unsupported"));

    let (status, body) = get(&app, PRESETS, "a", basic, true).await;
    assert_eq!(status, StatusCode::OK);
    let presets: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(presets["presets"].as_array().unwrap().len(), 5);
    assert!(presets["presets"].as_array().unwrap().iter().all(|preset| preset["enabled"] == false));

    let (status, body) = get(&app, &native, "a", basic, true).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let preview: Value = serde_json::from_str(&body).unwrap();
    for (id, status) in [("remote", "present"), ("local", "present"), ("missing", "missing")] {
        assert!(
            preview["entries"]
                .as_array()
                .unwrap()
                .iter()
                .any(|row| row["id"] == id && row["status"] == status)
        );
    }
    for sensitive in [
        endpoint.as_str(),
        "fixture-header-value",
        "fixture-env-value",
        "/bin/sh",
        root.to_str().unwrap(),
    ] {
        assert!(!body.contains(sensitive), "unexpected native data in preview");
    }

    let claude = format!("{NATIVE}&appId=claude&installationId={}", claude.0);
    let (status, body) = get(&app, &format!("{claude}&accountId=owned"), "a", basic, true).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let preview: Value = serde_json::from_str(&body).unwrap();
    assert!(preview["entries"].as_array().unwrap().iter().any(|entry| entry["id"] == "remote"
        && entry["transport"] == "sse"
        && entry["enabled"] == true));
    assert!(!body.contains(&sse_url) && !body.contains("fixture-profile-value"));
    assert_eq!(
        get(&app, &format!("{claude}&accountId=neighbor"), "a", basic, true).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(get(&app, &claude, "a", basic, true).await.0, StatusCode::BAD_REQUEST);
    for path in [&profile.0, &profile.1] {
        assert!(fs::read_to_string(path).unwrap().contains(&sse_url));
    }
    for unsupported in ["claude-desktop", "openclaw"] {
        let uri = format!("{NATIVE}&appId={unsupported}&installationId=fixture");
        let (status, body) = get(&app, &uri, "a", basic, true).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(serde_json::from_str::<Value>(&body).unwrap()["status"], "unsupported");
    }

    for endpoint in [CATALOG, PRESETS, &native] {
        assert_eq!(get(&app, endpoint, "a", None, true).await.0, StatusCode::UNAUTHORIZED);
        assert_eq!(get(&app, endpoint, "a", basic, false).await.0, StatusCode::UNAUTHORIZED);
        assert_eq!(get(&app, endpoint, "b", basic, true).await.0, StatusCode::FORBIDDEN);
        assert_eq!(get(&client, endpoint, "a", bearer, true).await.0, StatusCode::OK);
        assert_eq!(get(&client, endpoint, "a", None, true).await.0, StatusCode::UNAUTHORIZED);
        assert_eq!(get(&client, endpoint, "a", bearer, false).await.0, StatusCode::UNAUTHORIZED);
        assert_eq!(get(&client, endpoint, "b", bearer, true).await.0, StatusCode::FORBIDDEN);
    }
    for (query, status) in [
        (format!("{native}&path=/tmp/other"), StatusCode::BAD_REQUEST),
        (format!("{NATIVE}&appId=unknown&installationId=x"), StatusCode::BAD_REQUEST),
        (format!("{NATIVE}&appId=codex&installationId=foreign"), StatusCode::NOT_FOUND),
        (native.replace("targetHostId=host", "targetHostId=metadata-only"), StatusCode::FORBIDDEN),
    ] {
        assert_eq!(get(&app, &query, "a", basic, true).await.0, status);
    }
    assert_eq!(fs::read_to_string(&config).unwrap(), contents);
    assert!(!marker.exists());
    assert_eq!(listener.accept().unwrap_err().kind(), std::io::ErrorKind::WouldBlock);

    let outside = root.join("outside.toml");
    fs::rename(&config, &outside).unwrap();
    std::os::unix::fs::symlink(outside, config).unwrap();
    let (status, body) = get(&app, &native, "a", basic, true).await;
    assert_eq!(status, StatusCode::OK);
    let preview: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(preview["status"], "unreadable");
    assert!(preview["entries"].as_array().unwrap().is_empty());
    fs::write(root.join("verified"), b"").unwrap();
}
