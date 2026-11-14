use super::*;

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
    let p = State { policy: Some(policy_path), ..Default::default() };
    let mut headers = HeaderMap::new();
    headers.insert("x-agent-apps-token", token.parse().unwrap());
    let mut scope = Scope { project_id:"project".into(), target_host_id:"local".into() };
    let tenant = TenantScope::new("p", "c").unwrap();
    let (host, _) = authorize(&p, tenant.clone(), &headers, &scope, true).unwrap();
    assert!(authorize(&p, TenantScope::new("p", "other").unwrap(), &headers, &scope, false).is_err());
    scope.target_host_id = "other".into();
    assert!(authorize(&p, tenant.clone(), &headers, &scope, false).is_err());
    scope.target_host_id = "local".into();
    scope.project_id = "other".into();
    assert!(authorize(&p, tenant, &headers, &scope, true).is_err());
    assert!(!root(&host).exists());
    let filename = WorkspaceFilename::Agents;
    vw_agent_apps::workspace_files::write_workspace_file("local", &host.id, &host.home, &root(&host), filename, "中文").unwrap();
    assert_eq!(vw_agent_apps::workspace_files::read_workspace_file("local", &host.id, &host.home, &root(&host), filename).unwrap().as_deref(), Some("中文"));
}
