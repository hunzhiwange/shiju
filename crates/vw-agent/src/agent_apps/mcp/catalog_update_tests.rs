use std::{net::TcpListener, sync::Barrier};

use serde_json::Value;
use vw_api_types::{
    agent_apps_auth::AuthService,
    id::{ProjectScope, TenantScope},
    mcp_management::{
        McpCatalogScope, McpConfigValue, McpConnectionDefinition, McpCredentialRef, McpFailureCode,
        McpTransport,
    },
};

use super::{
    McpCatalogStore,
    catalog_create::catalog_location,
    catalog_create_tests::{http_draft, with_database},
};
use crate::gateway::settings_store::GatewaySettingsStore;

#[test]
fn cc_mcp_t07_update_preserves_metadata_and_credential_and_rejects_stale_edit() -> anyhow::Result<()>
{
    with_database(|database| {
        let native_dir = tempfile::tempdir()?;
        let config_path = native_dir.path().join("config.json");
        let native_config = br#"{"mcpServers":{"entry":{"command":"external"}}}"#;
        std::fs::write(&config_path, native_config)?;
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        listener.set_nonblocking(true)?;
        let url = format!("http://{}/mcp", listener.local_addr()?);

        let tenant = TenantScope::new("platform", "company")?;
        let scope = McpCatalogScope::Project {
            scope: ProjectScope::new(tenant.clone(), "project".into())?,
        };
        let tenant_scope = McpCatalogScope::Tenant { scope: tenant.clone() };
        let catalog = McpCatalogStore::new(database.clone()).unwrap();
        let storage = GatewaySettingsStore::new(database)?;
        let (_, namespace) = catalog_location(&scope).unwrap();
        let (_, tenant_namespace) = catalog_location(&tenant_scope).unwrap();
        let sibling = catalog.create(&tenant_scope, http_draft("entry", &url)).unwrap();
        let sibling_payload = storage.get(&tenant, &tenant_namespace, &sibling.record.id.0)?;

        let mut draft = http_draft("entry", &url);
        draft.name = Some("Display name".into());
        draft.description = Some("Description".into());
        draft.homepage = Some("https://example.com/".into());
        draft.documentation = Some("https://example.com/docs".into());
        draft.tags = vec!["tools".into()];
        let reference = McpConfigValue::Credential {
            reference: McpCredentialRef {
                service: AuthService::ClaudeOfficial,
                account_id: "account-ref".into(),
            },
        };
        let McpConnectionDefinition::StreamableHttp { headers, .. } = &mut draft.connection else {
            unreachable!("HTTP fixture");
        };
        headers.insert("Authorization".into(), reference.clone());
        let created = catalog.create(&scope, draft).unwrap();

        let update =
            McpConnectionDefinition::Sse { url: format!("{url}/sse"), headers: Default::default() };
        let updated = catalog.update_connection(&scope, &created.record, update.clone()).unwrap();
        let mut expected = created.clone();
        expected.record.revision += 1;
        expected.connection_revision += 1;
        expected.transport = McpTransport::Sse;
        assert_eq!(updated, expected);
        let current_payload = || storage.get(&tenant, &namespace, &created.record.id.0);
        let payload = current_payload()?.unwrap();
        let stored: Value = serde_json::from_str(&payload)?;
        let mut expected_connection = serde_json::to_value(&update)?;
        expected_connection["headers"] = serde_json::json!({ "Authorization": reference });
        assert_eq!(stored["connection"], expected_connection);
        assert_eq!(stored["record"], serde_json::to_value(&expected)?);
        assert!(!serde_json::to_string(&updated)?.contains("account-ref"));

        assert_eq!(
            catalog.update_connection(
                &scope,
                &created.record,
                http_draft("entry", &url).connection
            ),
            Err(McpFailureCode::StaleRevision)
        );
        assert_eq!(
            catalog.update_connection(
                &scope,
                &updated.record,
                http_draft("entry", "file:///etc/hosts").connection
            ),
            Err(McpFailureCode::InvalidDefinition)
        );
        let other_scope =
            McpCatalogScope::Project { scope: ProjectScope::new(tenant.clone(), "other".into())? };
        assert_eq!(
            catalog.update_connection(&other_scope, &updated.record, update.clone()),
            Err(McpFailureCode::NotFound)
        );
        assert_eq!(current_payload()?.as_deref(), Some(payload.as_str()));
        assert_eq!(storage.get(&tenant, &tenant_namespace, &sibling.record.id.0)?, sibling_payload);

        let unchanged = catalog.update_connection(&scope, &updated.record, update).unwrap();
        assert_eq!(unchanged.record.revision, updated.record.revision + 1);
        assert_eq!(unchanged.connection_revision, updated.connection_revision);
        assert_eq!(std::fs::read(&config_path)?, native_config);
        assert_eq!(listener.accept().unwrap_err().kind(), std::io::ErrorKind::WouldBlock);
        Ok(())
    })
}

#[test]
fn cc_mcp_t07_concurrent_updates_from_one_revision_preserve_only_winner() -> anyhow::Result<()> {
    with_database(|database| {
        let scope = McpCatalogScope::Tenant { scope: TenantScope::new("platform", "company")? };
        let catalog = McpCatalogStore::new(database.clone()).unwrap();
        let created =
            catalog.create(&scope, http_draft("race", "http://127.0.0.1:41001/mcp")).unwrap();
        let barrier = Barrier::new(2);
        let endpoints = ["http://127.0.0.1:41002/mcp", "http://127.0.0.1:41003/mcp"];
        let results = std::thread::scope(|threads| {
            endpoints
                .map(|url| {
                    let (catalog, scope, record, barrier) =
                        (&catalog, &scope, &created.record, &barrier);
                    threads.spawn(move || {
                        barrier.wait();
                        catalog.update_connection(scope, record, http_draft("race", url).connection)
                    })
                })
                .map(|handle| handle.join().unwrap())
        });

        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        let winner = results.iter().position(Result::is_ok).unwrap();
        assert_eq!(results[1 - winner], Err(McpFailureCode::StaleRevision));
        let (tenant, namespace) = catalog_location(&scope).unwrap();
        let storage = GatewaySettingsStore::new(database)?;
        let payload = storage.get(tenant, &namespace, &created.record.id.0)?.unwrap();
        let stored: Value = serde_json::from_str(&payload)?;
        assert_eq!(stored["connection"]["url"], endpoints[winner]);
        assert_eq!(stored["record"], serde_json::to_value(results[winner].as_ref().unwrap())?);
        Ok(())
    })
}
