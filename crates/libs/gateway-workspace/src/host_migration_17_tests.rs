use super::authorize_workspace;
use axum::http::HeaderMap;
use vw_api_transport::{ProjectScope, TenantScope};
use vw_api_workspace_memory::WorkspaceFilename;

#[test]
fn workspace_api_denies_foreign_host_and_project_without_io() {
    use sha2::{Digest, Sha256};
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let policy_path = dir.path().join("access.json");
    let token = "a".repeat(64);
    let policy = serde_json::json!({
        "principals": [{"id":"editor", "token_sha256":hex::encode(Sha256::digest(token.as_bytes())), "grants":[{
            "platform_id":"p", "company_id":"c", "host_id":"local",
            "workspace_read":["project"], "workspace_write":["project"]
        }]}],
        "hosts":[{"id":"local", "online":true, "binding":"local", "home":dir.path(), "search_paths":[], "applications":dir.path()}]
    });
    std::fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    std::fs::set_permissions(&policy_path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let policy = Some(policy_path);
    let mut headers = HeaderMap::new();
    headers.insert("x-agent-apps-token", token.parse().unwrap());
    let mut project = "project";
    let mut target = "local";
    let tenant = TenantScope::new("p", "c").unwrap();
    let (host, _) = authorize_workspace(policy.as_deref(), &headers, &ProjectScope::new(tenant.clone(), project.into()).unwrap(), target, true).unwrap();
    assert!(authorize_workspace(policy.as_deref(), &headers, &ProjectScope::new(TenantScope::new("p", "other").unwrap(), project.into()).unwrap(), target, false).is_err());
    target = "other";
    assert!(authorize_workspace(policy.as_deref(), &headers, &ProjectScope::new(tenant.clone(), project.into()).unwrap(), target, false).is_err());
    target = "local";
    project = "other";
    assert!(authorize_workspace(policy.as_deref(), &headers, &ProjectScope::new(tenant, project.into()).unwrap(), target, true).is_err());
    assert!(!host.home.join(".openclaw/workspace").exists());
    let filename = WorkspaceFilename::Agents;
    vw_agent_apps::workspace_files::write_workspace_file("local", &host.id, &host.home, &host.home.join(".openclaw/workspace"), filename, "中文").unwrap();
    assert_eq!(vw_agent_apps::workspace_files::read_workspace_file("local", &host.id, &host.home, &host.home.join(".openclaw/workspace"), filename).unwrap().as_deref(), Some("中文"));
}
