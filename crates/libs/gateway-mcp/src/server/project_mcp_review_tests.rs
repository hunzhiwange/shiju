use super::*;
use serde_json::json;
use std::{fs, os::unix::fs::PermissionsExt, path::Path as FsPath};
use vw_agent_apps::mcp::catalog::McpCatalogDraft;
use vw_api_mcp_management::McpConnectionDefinition;

#[tokio::test]
async fn acp_platform_mcp_review_requires_host_and_exact_project_grant() {
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let policy = root.path().join("access.json");
    fs::write(
        &policy,
        json!({
            "hosts":[{"id":"desktop-local","online":true,"binding":"local","home":root.path(),
                "applications":root.path(),"search_paths":[]}],
            "principals":[{"id":"reviewer",
                "token_sha256":"73be7d39a02e0001ae15d817be8da7811bc324222bb2203bd2bb8e9774404990",
                "grants":[{"platform_id":"fixture","company_id":"review","host_id":"desktop-local",
                    "provider_config_write":true,"mcp_catalog_write_projects":["project"]}]}],
        })
        .to_string(),
    )
    .unwrap();
    fs::set_permissions(&policy, fs::Permissions::from_mode(0o600)).unwrap();
    let database = GlobalDatabase::for_home(root.path()).unwrap();
    let tenant = TenantScope::new("fixture", "review").unwrap();
    let project = ProjectScope::new(tenant.clone(), "project".into()).unwrap();
    let scope = McpCatalogScope::Project { scope: project.clone() };
    let store = McpCatalogStore::new(database.clone()).unwrap();
    let executable = FsPath::new("/usr/bin/true").canonicalize().unwrap();
    let record = agent_database::run_sync_database(|| {
        Ok(store
            .create(
                &scope,
                McpCatalogDraft {
                    id: "fixture".into(),
                    name: None,
                    description: None,
                    homepage: None,
                    documentation: None,
                    tags: vec![],
                    connection: McpConnectionDefinition::Stdio {
                        command: executable.to_str().unwrap().into(),
                        args: vec![],
                        env: Default::default(),
                        cwd: None,
                    },
                },
            )
            .unwrap())
    })
    .unwrap();
    let update = |token, project_id, host, action| {
        let mut headers = HeaderMap::new();
        if let Some(token) = token {
            headers.insert("x-agent-apps-token", axum::http::HeaderValue::from_static(token));
        }
        update_catalog_record(
            Extension(PolicyFile(Some(policy.clone()))),
            Extension(tenant.clone()),
            headers,
            Path("fixture".into()),
            Some(Extension(database.clone())),
            Ok(Json(serde_json::from_value(json!({
                "targetHostId":host,"projectId":project_id,"expectedRevision":record.record.revision,
                "linuxStdioSelection":action,
            })).unwrap())),
        )
    };
    let select = || json!({"action":"select","sha256":"5a".repeat(32),"allowedTools":["read"]});
    let token = Some("review-admin-fixture");
    for (credential, target_project, target_host) in [
        (None, "project", "desktop-local"),
        (Some("unregistered"), "project", "desktop-local"),
        (token, "other", "desktop-local"),
        (token, "project", "sibling"),
    ] {
        let error = update(credential, target_project, target_host, select()).await.err().unwrap();
        assert_eq!(error.0, StatusCode::FORBIDDEN);
    }
    assert!(update(token, "project", "desktop-local", select()).await.is_ok());
    assert!(update(token, "project", "desktop-local", json!({"action":"disable"})).await.is_ok());
}
