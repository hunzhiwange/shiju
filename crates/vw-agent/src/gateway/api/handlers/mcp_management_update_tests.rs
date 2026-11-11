use std::{
    fs, io::ErrorKind, net::TcpListener, os::unix::fs::PermissionsExt, path::Path, process::Command,
};

use axum::{
    Extension, Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tower::ServiceExt;
use vw_api_types::{
    ProjectScope, TenantScope,
    mcp_management::{McpCatalogRecord, McpCatalogScope},
};

use crate::{agent_apps::mcp::McpCatalogStore, global_database::GlobalDatabase};

const TOKEN: &str = "fixture-token-for-the-mcp-catalog-writer";
const BASIC: &str = "Basic dGVzdDpzZWNyZXQ=";
const SKEY: &str = "Bearer dynamic-acceptance-skey";
const FIXTURE_ENV: &str = "SHIJU_MCP_TASK34_TEST_ROOT";

async fn put(
    app: &Router,
    company: &str,
    authorization: Option<&str>,
    member: bool,
    payload: &Value,
) -> (StatusCode, Value) {
    let mut request = Request::put("/v1/agent-apps/mcp/fixture")
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
async fn cc_mcp_t34_update_checks_revision_and_never_claims_native_sync() {
    let Some(root) = std::env::var_os(FIXTURE_ENV) else {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let path = root.path().join("access.json");
        let hosts = ["host", "read-only", "project-only"].map(|id| {
            json!({
                "id":id,"online":true,"binding":"local","home":root.path(),
                "applications":root.path().join("Applications"),"search_paths":[]
            })
        });
        let policy = json!({
            "principals":[{"id":"writer","token_sha256":hex::encode(Sha256::digest(TOKEN)),
                "grants":[{"platform_id":"p","company_id":"a","host_id":"host",
                    "mcp_catalog_write":true,"mcp_catalog_write_projects":["project"]},
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
                    "::cc_mcp_t34_update_checks_revision_and_never_claims_native_sync"
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
        assert!(status.success(), "MCP update router fixture failed");
        assert!(root.path().join("verified").is_file(), "isolated test must actually run");
        return;
    };
    let root = Path::new(&root);
    let database = GlobalDatabase::new(root.join("catalog.sqlite3")).unwrap();
    let store = McpCatalogStore::new(database.clone()).unwrap();
    let app = crate::gateway::router::build_router(Vec::new()).layer(Extension(database.clone()));
    let client = crate::gateway::dynamic_tools_acceptance_router().layer(Extension(database));
    let tenant = TenantScope::new("p", "a").unwrap();
    let tenant_scope = McpCatalogScope::Tenant { scope: tenant.clone() };
    let project_scope = McpCatalogScope::Project {
        scope: ProjectScope::new(tenant.clone(), "project".into()).unwrap(),
    };
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}/private-fixture-url", listener.local_addr().unwrap());
    let draft = || vw_agent_apps::mcp::catalog::McpCatalogDraft {
        id: "fixture".into(),
        name: Some("Display name".into()),
        description: None,
        homepage: None,
        documentation: None,
        tags: vec![],
        connection: vw_api_types::mcp_management::McpConnectionDefinition::StreamableHttp {
            url: endpoint.clone(),
            headers: Default::default(),
        },
    };
    let original = store.create(&tenant_scope, draft()).unwrap();
    let project_original = store.create(&project_scope, draft()).unwrap();
    let native = root.join("native.json");
    let native_bytes = br#"{"mcpServers":{"fixture":{"command":"external"}}}"#;
    fs::write(&native, native_bytes).unwrap();
    let marker = root.join("started");
    let payload = json!({
        "targetHostId":"host", "expectedRevision":1,
        "connection":{"transport":"stdio","command":"/usr/bin/touch","args":[marker]}
    });
    let (status, body) = put(&app, "a", Some(BASIC), true, &payload).await;
    assert_eq!(status, StatusCode::OK, "authorized PUT must update the catalog entry");
    let updated: McpCatalogRecord = serde_json::from_value(body.clone()).unwrap();
    assert_eq!(updated.record.revision, 2);
    assert_eq!(updated.connection_revision, 2);
    assert_eq!(updated.name, original.name);
    assert_eq!(updated.scope, tenant_scope);
    assert!(!body.to_string().contains("/usr/bin/touch"));
    assert!(body.get("synced").is_none());
    assert!(body.get("nativeStatus").is_none());
    let (status, body) = put(&app, "a", Some(BASIC), true, &payload).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "stale_revision");

    let mut project_payload = payload.clone();
    project_payload["projectId"] = json!("project");
    project_payload["targetHostId"] = json!("project-only");
    let (status, body) = put(&client, "a", Some(SKEY), true, &project_payload).await;
    assert_eq!(status, StatusCode::OK);
    let project_updated: McpCatalogRecord = serde_json::from_value(body).unwrap();
    assert_eq!(project_updated.record.revision, project_original.record.revision + 1);
    assert_eq!(project_updated.scope, project_scope);
    let mut current = payload.clone();
    current["expectedRevision"] = json!(2);
    for (router, auth) in [(&app, BASIC), (&client, SKEY)] {
        assert_eq!(put(router, "a", None, true, &current).await.0, StatusCode::UNAUTHORIZED);
        assert_eq!(put(router, "a", Some(auth), false, &current).await.0, StatusCode::UNAUTHORIZED);
        assert_eq!(put(router, "b", Some(auth), true, &current).await.0, StatusCode::FORBIDDEN);
    }
    for (host, project) in [
        ("read-only", Value::Null),
        ("unknown", Value::Null),
        ("project-only", Value::Null),
        ("host", json!("other-project")),
    ] {
        let mut denied = current.clone();
        denied["targetHostId"] = json!(host);
        denied["projectId"] = project;
        assert_eq!(put(&app, "a", Some(BASIC), true, &denied).await.0, StatusCode::FORBIDDEN);
    }
    let mut missing = current.clone();
    missing["projectId"] = json!("project");
    // Remove only the fixture's project entry through the store, then observe the real route.
    store.delete(&project_scope, &project_updated.record).unwrap();
    let (status, body) = put(&app, "a", Some(BASIC), true, &missing).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "not_found");
    let mut invalid = current.clone();
    invalid["connection"] = json!({"transport":"sse","url":"file:///private"});
    let (status, body) = put(&app, "a", Some(BASIC), true, &invalid).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "invalid_definition");
    let mut secret = current.clone();
    secret["connection"] = json!({"transport":"sse","url":endpoint,
        "headers":{"Authorization":{"kind":"literal","value":"fixture-secret"}}});
    let (status, body) = put(&app, "a", Some(BASIC), true, &secret).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(!body.to_string().contains("fixture-secret"));
    for (field, value) in [("companyId", json!("b")), ("expectedRevision", Value::Null)] {
        let mut invalid = current.clone();
        invalid[field] = value;
        assert_eq!(put(&app, "a", Some(BASIC), true, &invalid).await.0, StatusCode::BAD_REQUEST);
    }
    assert_eq!(store.list(&tenant_scope).unwrap(), [updated]);
    assert!(
        store
            .list(&McpCatalogScope::Tenant { scope: TenantScope::new("p", "b").unwrap() })
            .unwrap()
            .is_empty()
    );
    assert_eq!(fs::read(native).unwrap(), native_bytes);
    assert!(!marker.exists());
    assert_eq!(listener.accept().unwrap_err().kind(), ErrorKind::WouldBlock);
    fs::write(root.join("verified"), b"").unwrap();
}
