use vw_api_types::{
    agent_apps::{AppId, AppInstallationId},
    id::{ProjectScope, TenantScope},
    mcp_management::{
        McpAppTarget, McpCatalogScope, McpCliBinding, McpFailureCode, McpNativeStatus, McpRecordRef,
    },
};

use super::{
    McpCatalogStore, McpRevocation,
    catalog_create::catalog_location,
    catalog_create_tests::{http_draft, with_database},
};
use crate::gateway::settings_store::GatewaySettingsStore;

#[test]
fn cc_mcp_t08_deleted_record_is_hidden_and_retains_existing_bindings() -> anyhow::Result<()> {
    with_database(|database| {
        let tenant = TenantScope::new("platform", "company")?;
        let scope = McpCatalogScope::Project {
            scope: ProjectScope::new(tenant.clone(), "project".into())?,
        };
        let sibling = McpCatalogScope::Tenant { scope: tenant };
        let draft = || http_draft("entry", "http://127.0.0.1:41001/mcp");
        let catalog = McpCatalogStore::new(database.clone()).unwrap();
        let created = catalog.create(&scope, draft()).unwrap();
        let other = catalog.create(&sibling, draft()).unwrap();
        let bindings = [
            binding(&created.record, "first-installation", true),
            binding(&created.record, "second-installation", false),
        ];
        // Task 24 adds binding writes. Seed persisted bindings on the real backend.
        let storage = GatewaySettingsStore::new(database.clone())?;
        let (tenant, namespace) = catalog_location(&scope).unwrap();
        let mut active: serde_json::Value =
            serde_json::from_str(&storage.get(tenant, &namespace, "entry")?.unwrap())?;
        active["bindings"] = serde_json::to_value(&bindings)?;
        storage.upsert(tenant, &namespace, "entry", &serde_json::to_string(&active)?)?;

        let current =
            catalog.update_connection(&scope, &created.record, draft().connection).unwrap();
        assert_eq!(catalog.delete(&scope, &created.record), Err(McpFailureCode::StaleRevision));
        assert_eq!(catalog.list(&scope), Ok(vec![current.clone()]));
        let targets = bindings.each_ref().map(|binding| binding.target.clone());
        let mut expected = bindings.map(|binding| McpRevocation { binding, outcome: None });
        assert_eq!(
            catalog.delete(&scope, &current.record),
            Ok(expected.to_vec()),
            "deletion must preserve every previously managed binding"
        );
        let reopened = McpCatalogStore::new(database.clone()).unwrap();
        let retired: serde_json::Value =
            serde_json::from_str(&storage.get(tenant, &namespace, "entry")?.unwrap())?;
        assert!(retired.get("connection").is_none());
        assert_eq!(reopened.list(&scope), Ok(Vec::new()));
        assert_eq!(reopened.list(&sibling), Ok(vec![other.clone()]));
        assert_eq!(reopened.delete(&scope, &current.record), Ok(expected.to_vec()));
        assert_eq!(reopened.create(&scope, draft()), Err(McpFailureCode::Conflict));
        assert_eq!(
            reopened.update_connection(&scope, &current.record, draft().connection),
            Err(McpFailureCode::NotFound)
        );
        let failure = Err(McpFailureCode::NativeWriteFailed);
        assert_eq!(
            reopened.record_revocation_result(&scope, &created.record, &targets[0], failure),
            Err(McpFailureCode::StaleRevision)
        );
        assert_eq!(
            reopened.record_revocation_result(&sibling, &other.record, &targets[0], failure),
            Err(McpFailureCode::StaleRevision)
        );
        reopened.record_revocation_result(&scope, &current.record, &targets[0], failure).unwrap();
        reopened.record_revocation_result(&scope, &current.record, &targets[1], Ok(())).unwrap();
        expected[0].outcome = Some(failure);
        expected[1].outcome = Some(Ok(()));
        let reopened = McpCatalogStore::new(database).unwrap();
        assert_eq!(reopened.delete(&scope, &current.record), Ok(expected.to_vec()));
        assert_eq!(reopened.create(&scope, draft()), Err(McpFailureCode::Conflict));

        // A late failure cannot turn a completed cleanup back into a pending one.
        reopened.record_revocation_result(&scope, &current.record, &targets[1], failure).unwrap();
        assert_eq!(reopened.delete(&scope, &current.record), Ok(expected.to_vec()));
        reopened.record_revocation_result(&scope, &current.record, &targets[0], Ok(())).unwrap();
        let replacement = reopened.create(&scope, draft()).unwrap();
        assert_eq!(replacement.record.revision, current.record.revision + 1);
        assert!(replacement.connection_revision > current.connection_revision);
        assert_eq!(reopened.delete(&scope, &current.record), Err(McpFailureCode::StaleRevision));
        assert_eq!(
            reopened.record_revocation_result(&scope, &current.record, &targets[0], failure),
            Err(McpFailureCode::StaleRevision)
        );
        assert_eq!(reopened.list(&scope), Ok(vec![replacement]));
        Ok(())
    })
}

fn binding(record: &McpRecordRef, installation: &str, enabled: bool) -> McpCliBinding {
    McpCliBinding {
        record: record.clone(),
        target: McpAppTarget {
            app_id: AppId::Claude,
            installation_id: AppInstallationId(installation.into()),
            target_host_id: "host".into(),
        },
        enabled,
        native_status: McpNativeStatus::Present,
    }
}

#[test]
fn cc_mcp_t08_delete_unbound_record_allows_recreation() -> anyhow::Result<()> {
    with_database(|database| {
        let scope = McpCatalogScope::Tenant { scope: TenantScope::new("platform", "company")? };
        let draft = || http_draft("unbound", "http://127.0.0.1:41001/mcp");
        let catalog = McpCatalogStore::new(database).unwrap();
        let created = catalog.create(&scope, draft()).unwrap();

        assert_eq!(catalog.delete(&scope, &created.record), Ok(Vec::new()));
        assert_eq!(catalog.list(&scope), Ok(Vec::new()));
        assert_eq!(catalog.delete(&scope, &created.record), Ok(Vec::new()));
        let replacement = catalog.create(&scope, draft()).unwrap();
        assert_eq!(replacement.record.revision, created.record.revision + 1);
        assert_eq!(catalog.list(&scope), Ok(vec![replacement]));
        Ok(())
    })
}
