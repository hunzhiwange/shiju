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
const FIXTURE_ENV: &str = "SHIJU_MCP_TASK33_TEST_ROOT";

async fn post(
    app: &Router,
    company: &str,
    authorization: Option<&str>,
    member: bool,
    payload: &Value,
) -> (StatusCode, Value) {
    let mut request = Request::post("/v1/agent-apps/mcp")
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
    if matches!(status, StatusCode::CREATED | StatusCode::CONFLICT | StatusCode::BAD_REQUEST) {
        assert_eq!(response.headers().get("cache-control").unwrap(), "no-store");
    }
    let body = to_bytes(response.into_body(), 65536).await.unwrap();
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

#[tokio::test]
async fn cc_mcp_t33_create_validates_definition_and_persists_only_in_authorized_scope() {
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
                    "::cc_mcp_t33_create_validates_definition_and_persists_only_in_authorized_scope"
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
        assert!(status.success(), "MCP create router fixture failed");
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
    let payload = json!({
        "targetHostId":"host","id":"fixture","name":" Example MCP ",
        "connection":{"transport":"streamable_http","url":endpoint}
    });

    let (status, body) = post(&app, "a", Some(BASIC), true, &payload).await;
    assert_eq!(status, StatusCode::CREATED, "authorized POST must create the catalog entry");
    assert!(!body.to_string().contains(&endpoint));
    let tenant_record: McpCatalogRecord = serde_json::from_value(body).unwrap();
    assert_eq!(tenant_record.scope, tenant_scope);
    assert_eq!(tenant_record.record.id.0, "fixture");
    assert_eq!(tenant_record.name, "Example MCP");
    assert_eq!(tenant_record.record.revision, 1);

    let mut duplicate = payload.clone();
    duplicate["id"] = json!(" fixture ");
    let (status, body) = post(&app, "a", Some(BASIC), true, &duplicate).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "conflict");

    let marker = root.join("started");
    let mut project_payload = payload.clone();
    project_payload["projectId"] = json!("project");
    project_payload["targetHostId"] = json!("project-only");
    project_payload["connection"] = json!({
        "transport":"stdio","command":"/usr/bin/touch","args":[marker]
    });
    let (status, body) = post(&client, "a", Some(SKEY), true, &project_payload).await;
    assert_eq!(status, StatusCode::CREATED);
    assert!(!body.to_string().contains("/usr/bin/touch"));
    let project_record: McpCatalogRecord = serde_json::from_value(body).unwrap();
    assert_eq!(project_record.scope, project_scope);
    assert_eq!(project_record.record.id, tenant_record.record.id);

    let mut invalid = payload.clone();
    invalid["id"] = json!("invalid");
    invalid["connection"] = json!({"transport":"sse","url":"file:///private"});
    let (status, body) = post(&app, "a", Some(BASIC), true, &invalid).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "invalid_definition");

    let mut literal_secret = payload.clone();
    literal_secret["id"] = json!("literal-secret");
    literal_secret["connection"]["headers"] = json!({
        "Authorization":{"kind":"literal","value":"fixture-header-value"}
    });
    let (status, body) = post(&app, "a", Some(BASIC), true, &literal_secret).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "invalid_definition");
    assert!(!body.to_string().contains("fixture-header-value"));

    for (router, auth) in [(&app, BASIC), (&client, SKEY)] {
        assert_eq!(post(router, "a", None, true, &payload).await.0, StatusCode::UNAUTHORIZED);
        assert_eq!(
            post(router, "a", Some(auth), false, &payload).await.0,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(post(router, "b", Some(auth), true, &payload).await.0, StatusCode::FORBIDDEN);
    }
    for (host, project) in [
        ("read-only", Value::Null),
        ("unknown", Value::Null),
        ("project-only", Value::Null),
        ("host", json!("other-project")),
    ] {
        let mut denied = payload.clone();
        denied["id"] = json!("denied");
        denied["targetHostId"] = json!(host);
        denied["projectId"] = project;
        let (status, body) = post(&app, "a", Some(BASIC), true, &denied).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["code"], "permission_denied");
    }
    let mut supplied_tenant = payload;
    supplied_tenant["companyId"] = json!("b");
    let (status, body) = post(&app, "a", Some(BASIC), true, &supplied_tenant).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "invalid_definition");

    assert_eq!(store.list(&tenant_scope).unwrap(), [tenant_record]);
    assert_eq!(store.list(&project_scope).unwrap(), [project_record]);
    assert!(
        store
            .list(&McpCatalogScope::Tenant { scope: TenantScope::new("p", "b").unwrap() })
            .unwrap()
            .is_empty()
    );
    assert!(
        store
            .list(&McpCatalogScope::Project {
                scope: ProjectScope::new(tenant, "other-project".into()).unwrap()
            })
            .unwrap()
            .is_empty()
    );
    assert!(!marker.exists());
    assert_eq!(listener.accept().unwrap_err().kind(), ErrorKind::WouldBlock);
    fs::write(root.join("verified"), b"").unwrap();
}
