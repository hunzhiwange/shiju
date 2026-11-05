use anyhow::Result;
use serde_json::json;
use vw_api_types::id::TenantScope;

use super::store::IconLibraryStore;
use crate::global_database::test_support::PostgresTestDatabase;
use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};

fn exercise(
    database: GlobalDatabase,
    reopen: impl FnOnce() -> Result<GlobalDatabase>,
) -> Result<()> {
    let tenant = TenantScope::new("platform-a", "company-a")?;
    let company = TenantScope::new("platform-a", "company-b")?;
    let platform = TenantScope::new("platform-b", "company-a")?;
    let settings = GatewaySettingsStore::new(database.clone())?;
    let store = IconLibraryStore::new(database)?;
    assert!(store.read_icons(&tenant)?.is_empty());
    let icon = json!({"id":"first","name":"图标","data":"original image","createdAt":12});
    for (scope, id) in [(&tenant, "first"), (&company, "company"), (&platform, "platform")] {
        let mut entry = icon.clone();
        entry["id"] = json!(id);
        settings.upsert(
            scope,
            "agent_apps.custom_icons",
            "default",
            &json!([entry]).to_string(),
        )?;
    }
    drop(store);
    drop(settings);
    let database = reopen()?;
    let settings = GatewaySettingsStore::new(database.clone())?;
    let store = IconLibraryStore::new(database)?;
    for (scope, id) in [(&tenant, "first"), (&company, "company"), (&platform, "platform")] {
        let icons = store.read_icons(scope)?;
        assert_eq!(icons.len(), 1);
        assert_eq!(icons[0].id, id);
        assert_eq!(icons[0].data, "original image");
    }
    for raw in ["{broken", "{}", "null"] {
        settings.upsert(&tenant, "agent_apps.custom_icons", "default", raw)?;
        assert!(store.read_icons(&tenant)?.is_empty());
    }
    let mut last = icon.clone();
    last["id"] = json!("last");
    settings.upsert(
        &tenant,
        "agent_apps.custom_icons",
        "default",
        &json!([icon, null, {"id":1}, last]).to_string(),
    )?;
    let icons = store.read_icons(&tenant)?;
    assert_eq!(icons.len(), 2);
    assert_eq!(icons[0].id, "first");
    assert_eq!(icons[1].id, "last");
    settings.delete(&tenant, "agent_apps.custom_icons", "default")?;
    settings.upsert(
        &tenant,
        "settings",
        "custom_icons",
        &json!([{"id":"legacy","name":"legacy","data":"legacy","createdAt":1}]).to_string(),
    )?;
    assert!(store.read_icons(&tenant)?.is_empty());
    Ok(())
}

#[test]
fn cc_base_icon_store_read_sqlite_reopen_isolation_and_corruption() -> Result<()> {
    let home = tempfile::tempdir()?;
    exercise(GlobalDatabase::for_home(home.path())?, || GlobalDatabase::for_home(home.path()))
}

#[test]
fn cc_base_icon_store_read_postgres_reopen_isolation_and_corruption() -> Result<()> {
    let fixture = PostgresTestDatabase::from_environment()?;
    exercise(fixture.database().clone(), || fixture.reopen())?;
    fixture.cleanup()
}
