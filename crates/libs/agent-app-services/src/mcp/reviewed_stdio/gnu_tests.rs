use super::*;
use crate::mcp::catalog_create_tests::{http_draft, with_database};
use agent_client_protocol::McpServerStdio;
use vw_api_transport::{TenantScope, UserId};

#[test]
fn acp_platform_mcp_review_resolves_only_current_project_selection() -> anyhow::Result<()> {
    with_database(|db| {
        let store = McpCatalogStore::new(db.clone()).unwrap();
        let tenant = TenantScope::new("fixture", "mcp-review")?;
        let project = ProjectScope::new(tenant.clone(), "reviewed".into())?;
        let owner = ProjectScope::new(
            tenant.clone().with_user_id(UserId::new("session-owner")?),
            "reviewed".into(),
        )?;
        let stranger = ProjectScope::new(tenant.clone(), "other-project".into())?;
        let other_user =
            ProjectScope::new(tenant.with_user_id(UserId::new("other-user")?), "reviewed".into())?;
        let scope = McpCatalogScope::Project { scope: project.clone() };
        let executable = Path::new("/usr/bin/true").canonicalize()?;
        let argv = vec!["--fixture".to_owned()];
        let mut draft = http_draft("fixture", "unused");
        draft.connection = McpConnectionDefinition::Stdio {
            command: executable.to_str().unwrap().into(),
            args: argv.clone(),
            env: Default::default(),
            cwd: None,
        };
        let connection = draft.connection.clone();
        let record = store.create(&scope, draft).unwrap().record;
        let review = || McpLinuxStdioSelection::Select {
            sha256: "5a".repeat(32),
            allowed_tools: vec!["read_fixture".to_owned()],
        };
        let mut server = McpServerStdio::new("fixture", executable.clone()).args(argv.clone());
        server.meta = Some(serde_json::Map::from_iter([(
            "linuxStdioSelection".into(),
            serde_json::json!({"sha256":"00".repeat(32), "allowedTools":["unreviewed"]}),
        )]));
        let request = [AcpMcpServer::Stdio(server.clone())];
        assert!(store.reviewed_project_stdio(&owner, &request).is_err());
        assert!(store.configure_linux_stdio(&stranger, &record, review()).is_err());

        let legacy_selection =
            McpRuntimeSelection { project: project.clone(), record: record.clone() };
        store.select_for_runtime(&project, &scope, &legacy_selection).unwrap();
        let directory = tempfile::tempdir()?;
        let legacy = store.runtime_servers(&project, directory.path(), &[])?;
        assert_eq!(legacy.len(), 1);
        assert!(store.reviewed_project_stdio(&owner, &request).is_err());

        store.configure_linux_stdio(&project, &record, review()).unwrap();
        assert!(store.runtime_servers(&project, directory.path(), &[])?.is_empty());
        assert!(!crate::mcp::revocation::runtime_selection_is_current(
            &legacy[0],
            Some(&TenantScope::new("fixture", "mcp-review")?),
            TenantScope::new("fixture", "mcp-review")?,
            || Ok(db.clone()),
        )?);

        let proof = store.reviewed_project_stdio(&owner, &request).unwrap();
        assert_eq!(proof.executable(), executable.as_path());
        assert_eq!(proof.args(), argv.as_slice());
        assert_eq!(proof.sha256(), [0x5a; 32]);
        assert_eq!(proof.allowed_tools(), ["read_fixture"]);
        assert!(proof.is_current(&store, &owner).unwrap());
        assert!(!proof.is_current(&store, &other_user).unwrap());
        assert!(store.reviewed_project_stdio(&stranger, &request).is_err());
        server.args.push("--different".into());
        assert!(store.reviewed_project_stdio(&owner, &[AcpMcpServer::Stdio(server)]).is_err());

        store.configure_linux_stdio(&project, &record, McpLinuxStdioSelection::Disable).unwrap();
        assert!(!proof.is_current(&store, &owner).unwrap());
        assert!(store.reviewed_project_stdio(&owner, &request).is_err());
        store.configure_linux_stdio(&project, &record, review()).unwrap();
        assert!(!proof.is_current(&store, &owner).unwrap());
        let current = store.reviewed_project_stdio(&owner, &request).unwrap();
        store.update_connection(&scope, &record, connection).unwrap();
        assert!(!current.is_current(&store, &owner).unwrap());
        assert!(store.reviewed_project_stdio(&owner, &request).is_err());
        Ok(())
    })
}
