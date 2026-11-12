//! Real scoped sessions and production HTTP handlers; only external transport is substituted.
use axum::{
    Extension, Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
    routing::get,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, os::unix::fs::PermissionsExt};
use tower::ServiceExt;
use vw_acp::server::SessionBackend;
use vw_api_types::{ProjectScope, TenantScope, mcp_management::McpAcpUsage};
const TOKEN: &str = "fixture-task50-authorized-session-reader";

#[tokio::test]
async fn cc_mcp_t50_acp_http_reads_only_explicitly_granted_scoped_sessions() {
    let Some(root) = std::env::var_os("VW_TASK50_READ_ROOT").map(std::path::PathBuf::from) else {
        let fixture = tempfile::tempdir().unwrap();
        fs::create_dir(fixture.path().join("home")).unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact","gateway::api::handlers::mcp_management_tools_tests::cc_mcp_t50_acp_http_reads_only_explicitly_granted_scoped_sessions","--nocapture"])
            .env("VW_TASK50_READ_ROOT",fixture.path()).env("VW_MODEL_RECOVERY_TEST_ROOT",fixture.path().join("home"))
            .env("VW_ACP_MOUNT_RECOVERY_DB_ROOT",fixture.path().join("home")).output().unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    };
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    let workspace = root.join("workspace");
    fs::create_dir(&workspace).unwrap();
    let tenant = TenantScope::default();
    let (project, _) = crate::project::from_directory(&tenant, &workspace).await.unwrap();
    let scope = ProjectScope::new(tenant.clone(), project.id.clone().into()).unwrap();
    let backend = crate::acp::VibeWindowBackend::new(tenant.clone());
    let session = backend.create_session(workspace).await.unwrap();
    let path = root.join("access.json");
    let policy = json!({"principals":[{"id":"reader","token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":[{
        "platform_id":tenant.platform_id(),"company_id":tenant.company_id(),"host_id":"host",
        "mcp_acp_read":[{"project_id":project.id,"session_id":session.id}]
    }]}],"hosts":[{"id":"host","online":true,"binding":"local","home":root,"search_paths":[],"applications":root.join("Applications")}]});
    fs::write(&path, policy.to_string()).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let app = Router::new()
        .route("/mounts", get(super::mcp_management_tools::read_acp))
        .layer(Extension(super::mcp_management_read::PolicyFile(Some(path.clone()))))
        .layer(Extension(tenant));
    let uri = format!(
        "/mounts?targetHostId=host&projectId={}&sessionId={}",
        scope.project_id().as_ref(),
        session.id
    );
    async fn request_read(app: &Router, uri: &str, token: Option<&str>) -> (StatusCode, Value) {
        let mut request = Request::builder().uri(uri);
        if let Some(token) = token {
            request = request.header("x-agent-apps-token", token);
        }
        let response = app.clone().oneshot(request.body(Body::empty()).unwrap()).await.unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), 65536).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }
    assert_eq!(request_read(&app, &uri, None).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(
        request_read(&app, &uri.replace(&session.id, "another-session"), Some(TOKEN)).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        request_read(
            &app,
            &uri.replace("targetHostId=host", "targetHostId=another-host"),
            Some(TOKEN)
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        request_read(
            &app,
            &uri.replace(
                &format!("projectId={}", scope.project_id().as_ref()),
                "projectId=another-project"
            ),
            Some(TOKEN)
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (status, response) = request_read(&app, &uri, Some(TOKEN)).await;
    assert_eq!(status, StatusCode::OK, "{response}");
    let response: McpAcpUsage = serde_json::from_value(response).unwrap();
    assert_eq!(response.project, scope);
    assert_eq!(response.session_id.as_ref(), session.id);
    assert_eq!(response.target_host_id, "host");
    assert!(response.mounts.is_empty());
    let wire = serde_json::to_string(&response).unwrap();
    assert!(!wire.contains("directory") && !wire.contains("mcpServers") && !wire.contains(TOKEN));
    let mut revoked = policy;
    revoked["principals"][0]["grants"][0]["mcp_acp_read"] = json!([]);
    fs::write(path, revoked.to_string()).unwrap();
    assert_eq!(request_read(&app, &uri, Some(TOKEN)).await.0, StatusCode::FORBIDDEN);
}
#[tokio::test]
async fn cc_mcp_t50_detail_routes_do_not_shadow_existing_catalog_record_ids() {
    let app = super::mcp_management_read::router().layer(Extension(TenantScope::default()));
    for id in ["tools", "acp-mounts"] {
        let request = Request::builder()
            .method("PUT")
            .uri(format!("/agent-apps/mcp/{id}"))
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::BAD_REQUEST,
            "existing record route must still reach its input validator"
        );
    }
}
