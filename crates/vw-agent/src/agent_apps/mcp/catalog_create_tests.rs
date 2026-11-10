use std::{net::TcpListener, sync::Barrier};

use serde_json::Value;
use vw_agent_apps::mcp::catalog::McpCatalogDraft;
use vw_api_types::{
    id::{ProjectScope, TenantScope},
    mcp_management::{
        McpCatalogScope, McpConnectionDefinition, McpFailureCode, McpRecordId, McpTransport,
    },
};

use super::{McpCatalogStore, catalog_create::catalog_location};
use crate::{
    gateway::settings_store::GatewaySettingsStore,
    global_database::{GlobalDatabase, test_support::PostgresTestDatabase},
};

fn with_database(test: impl FnOnce(GlobalDatabase) -> anyhow::Result<()>) -> anyhow::Result<()> {
    if std::env::var_os("VIBEWINDOW_TEST_POSTGRES_URL").is_some() {
        let fixture = PostgresTestDatabase::from_environment()?;
        println!("backend: PostgreSQL");
        test(fixture.database().clone())?;
        fixture.cleanup()
    } else {
        let home = tempfile::tempdir()?;
        println!("backend: SQLite");
        test(GlobalDatabase::for_home(home.path())?)
    }
}

fn http_draft(id: &str, url: &str) -> McpCatalogDraft {
    McpCatalogDraft {
        id: id.into(),
        name: None,
        description: None,
        homepage: None,
        documentation: None,
        tags: Vec::new(),
        connection: McpConnectionDefinition::StreamableHttp {
            url: url.into(),
            headers: Default::default(),
        },
    }
}

#[test]
fn cc_mcp_t06_create_persists_only_in_authorized_scope_without_distribution() -> anyhow::Result<()>
{
    with_database(|database| {
        let native_dir = tempfile::tempdir()?;
        let marker = native_dir.path().join("started");
        let config_path = native_dir.path().join("config.json");
        let config = br#"{"mcpServers":{"catalog-entry":{"command":"existing"}}}"#;
        std::fs::write(&config_path, config)?;
        let tenant = TenantScope::new("platform", "company")?;
        let tenant_scope = McpCatalogScope::Tenant { scope: tenant.clone() };
        let project_scope = |tenant, id: &str| -> anyhow::Result<McpCatalogScope> {
            Ok(McpCatalogScope::Project { scope: ProjectScope::new(tenant, id.into())? })
        };
        let project = project_scope(tenant.clone(), "project")?;
        let sibling = project_scope(tenant, "sibling")?;
        let other_company = project_scope(TenantScope::new("platform", "other")?, "project")?;
        let other_platform = project_scope(TenantScope::new("other", "company")?, "project")?;

        let catalog = McpCatalogStore::new(database.clone()).unwrap();
        let mut draft = http_draft(" catalog-entry ", "unused");
        draft.connection = McpConnectionDefinition::Stdio {
            command: "/bin/sh".into(),
            args: vec![
                "-c".into(),
                "touch \"$1\"".into(),
                "fixture".into(),
                marker.to_string_lossy().into_owned(),
            ],
            env: Default::default(),
            cwd: Some(native_dir.path().to_string_lossy().into_owned()),
        };
        let record = catalog.create(&project, draft).unwrap();
        assert_eq!(record.scope, project);
        assert_eq!(record.record.id, McpRecordId("catalog-entry".into()));
        assert_eq!(record.name, "catalog-entry");
        assert_eq!(record.record.revision, 1);
        assert_eq!(record.connection_revision, 1);
        assert_eq!(record.transport, McpTransport::Stdio);
        let projection = serde_json::to_value(&record)?;
        assert!(projection.get("connection").is_none());
        assert!(!projection.to_string().contains("/bin/sh"));

        let storage = GatewaySettingsStore::new(database)?;
        let rows = |scope| -> anyhow::Result<Vec<_>> {
            let (tenant, namespace) = catalog_location(scope).unwrap();
            storage.list(tenant, &namespace)
        };
        let project_rows = rows(&project)?;
        assert_eq!(project_rows.len(), 1);
        let stored: Value = serde_json::from_str(&project_rows[0].payload_json)?;
        assert_eq!(stored["record"], projection);
        assert_eq!(stored["connection"]["command"], "/bin/sh");
        for key in ["bindings", "runtimeSelection", "acpMount", "enabled"] {
            assert!(stored.get(key).is_none());
        }

        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        listener.set_nonblocking(true)?;
        let url = format!("http://{}/mcp", listener.local_addr()?);
        for scope in [&tenant_scope, &sibling, &other_company, &other_platform] {
            assert!(rows(scope)?.is_empty());
            let created = catalog.create(scope, http_draft("catalog-entry", &url)).unwrap();
            assert_eq!(&created.scope, scope);
            assert_eq!(rows(scope)?.len(), 1);
        }
        assert_eq!(rows(&project)?, project_rows);
        assert_eq!(std::fs::read(&config_path)?, config);
        assert!(!marker.exists());
        assert_eq!(listener.accept().unwrap_err().kind(), std::io::ErrorKind::WouldBlock);
        Ok(())
    })
}

#[test]
fn cc_mcp_t06_duplicate_create_preserves_winning_definition() -> anyhow::Result<()> {
    with_database(|database| {
        let scope = McpCatalogScope::Tenant { scope: TenantScope::new("platform", "company")? };
        let catalog = McpCatalogStore::new(database.clone()).unwrap();
        let barrier = Barrier::new(2);
        let endpoints = ["http://127.0.0.1:41001/mcp", "http://127.0.0.1:41002/mcp"];
        let results = std::thread::scope(|threads| {
            endpoints
                .map(|url| {
                    let (scope, catalog, barrier) = (&scope, &catalog, &barrier);
                    threads.spawn(move || {
                        barrier.wait();
                        catalog.create(scope, http_draft(" race ", url))
                    })
                })
                .map(|handle| handle.join().unwrap())
        });

        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        let winner = results.iter().position(Result::is_ok).unwrap();
        assert_eq!(results[1 - winner], Err(McpFailureCode::Conflict));
        let (tenant, namespace) = catalog_location(&scope).unwrap();
        let storage = GatewaySettingsStore::new(database)?;
        let original = storage.get(tenant, &namespace, "race")?.unwrap();
        let stored: Value = serde_json::from_str(&original)?;
        assert_eq!(stored["connection"]["url"], endpoints[winner]);
        assert_eq!(
            catalog.create(&scope, http_draft("race", "http://127.0.0.1:41003/mcp")),
            Err(McpFailureCode::Conflict)
        );
        assert_eq!(
            catalog.create(&scope, http_draft("invalid", "file:///etc/hosts")),
            Err(McpFailureCode::InvalidDefinition)
        );
        let mut invalid_metadata = http_draft("invalid-metadata", endpoints[winner]);
        invalid_metadata.homepage = Some("https://example.com/?token=private".into());
        assert_eq!(
            catalog.create(&scope, invalid_metadata),
            Err(McpFailureCode::InvalidDefinition)
        );
        let rows = storage.list(tenant, &namespace)?;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].payload_json, original);
        Ok(())
    })
}
