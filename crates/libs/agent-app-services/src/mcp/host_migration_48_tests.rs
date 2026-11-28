use super::{
    McpCatalogStore,
    catalog_create_tests::{http_draft, with_database},
};
use std::fs;
use vw_api_mcp_management::McpCatalogScope;
use vw_api_transport::TenantScope;

#[test]
fn project_roots_restrict_runtime_selections_to_owned_workspaces() -> anyhow::Result<()> {
    with_database(|database| {
        let root = tempfile::tempdir()?;
        let outside = tempfile::tempdir()?;
        let workspace = root.path().join("nested");
        fs::create_dir(&workspace)?;
        let tenant = TenantScope::new("platform", "company")?;
        let project = vw_api_transport::ProjectScope::new(tenant, "project".into())?;
        let scope = McpCatalogScope::Project { scope: project.clone() };
        let catalog = McpCatalogStore::new(database).unwrap();
        let saved =
            catalog.create(&scope, http_draft("selected", "http://127.0.0.1:1/mcp")).unwrap();
        catalog
            .select_for_runtime(
                &project,
                &scope,
                &vw_api_mcp_management::McpRuntimeSelection {
                    project: project.clone(),
                    record: saved.record,
                },
            )
            .unwrap();
        let root_text = root.path().to_str().unwrap();
        assert_eq!(
            catalog
                .runtime_servers_for_project_roots(&project, &workspace, [root_text], &[])?
                .len(),
            1
        );
        assert!(
            catalog
                .runtime_servers_for_project_roots(&project, outside.path(), [root_text], &[])?
                .is_empty()
        );
        assert!(
            catalog.runtime_servers_for_project_roots(&project, &workspace, ["/"], &[])?.is_empty()
        );
        assert!(
            catalog
                .runtime_servers_for_project_roots(
                    &project,
                    &workspace,
                    ["/nonexistent/shiju-project"],
                    &[]
                )?
                .is_empty()
        );
        Ok(())
    })
}
