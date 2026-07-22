use anyhow::Result;
use vw_api_types::id::TenantScope;

use super::settings_store::GatewaySettingsStore;
use crate::global_database::GlobalDatabase;
use crate::global_database::test_support::PostgresTestDatabase;

#[test]
fn gateway_settings_store_isolates_tenant_namespace_and_key() -> Result<()> {
    for_each_gateway_settings_store_provider(gateway_settings_store_isolation)?;
    Ok(())
}

fn gateway_settings_store_isolation(store: &GatewaySettingsStore) -> Result<()> {
    let tenant_a = TenantScope::new("platform-a", "company-a")?;
    let tenant_b = TenantScope::new("platform-b", "company-b")?;

    assert_eq!(store.get(&tenant_a, "preferences", "default")?, None);

    store.upsert(&tenant_a, "preferences", "default", r#"{"theme":"dark"}"#)?;
    store.upsert(&tenant_b, "preferences", "default", r#"{"theme":"light"}"#)?;
    store.upsert(&tenant_a, "projects", "default", r#"{"sort":"name"}"#)?;
    store.upsert(&tenant_a, "preferences", "secondary", r#"{"theme":"system"}"#)?;

    assert_eq!(
        store.get(&tenant_a, "preferences", "default")?.as_deref(),
        Some(r#"{"theme":"dark"}"#)
    );
    assert_eq!(
        store.get(&tenant_b, "preferences", "default")?.as_deref(),
        Some(r#"{"theme":"light"}"#)
    );
    assert_eq!(store.get(&tenant_a, "projects", "default")?.as_deref(), Some(r#"{"sort":"name"}"#));
    assert_eq!(
        store.get(&tenant_a, "preferences", "secondary")?.as_deref(),
        Some(r#"{"theme":"system"}"#)
    );

    store.upsert(&tenant_a, "preferences", "default", r#"{"theme":"contrast"}"#)?;

    assert_eq!(
        store.get(&tenant_a, "preferences", "default")?.as_deref(),
        Some(r#"{"theme":"contrast"}"#)
    );
    assert_eq!(
        store.get(&tenant_b, "preferences", "default")?.as_deref(),
        Some(r#"{"theme":"light"}"#)
    );
    Ok(())
}

#[test]
fn gateway_settings_store_lists_and_deletes_scope_rows() -> Result<()> {
    for_each_gateway_settings_store_provider(gateway_settings_store_list_and_delete)?;
    Ok(())
}

#[test]
fn gateway_settings_store_rejects_blank_values_with_specific_labels() -> Result<()> {
    let sqlite_home = tempfile::tempdir()?;
    let store = GatewaySettingsStore::new(GlobalDatabase::for_home(sqlite_home.path())?)?;
    let tenant = TenantScope::new("platform-a", "company-a")?;

    assert_eq!(
        store.get(&tenant, " ", "key").expect_err("blank namespace").to_string(),
        "gateway settings namespace must not be empty"
    );
    assert_eq!(
        store.get(&tenant, "namespace", "\t").expect_err("blank key").to_string(),
        "gateway settings config key must not be empty"
    );
    assert_eq!(
        store.upsert(&tenant, "namespace", "key", "\n").expect_err("blank payload").to_string(),
        "gateway settings payload must not be empty"
    );
    Ok(())
}

fn gateway_settings_store_list_and_delete(store: &GatewaySettingsStore) -> Result<()> {
    let tenant = TenantScope::new("platform-a", "company-a")?;
    let other_tenant = TenantScope::new("platform-b", "company-b")?;

    assert!(store.list(&tenant, "connections")?.is_empty());

    store.upsert(&tenant, "connections", "connection-b", r#"{"host":"b"}"#)?;
    store.upsert(&tenant, "connections", "connection-a", r#"{"host":"a"}"#)?;
    store.upsert(&tenant, "other", "connection-c", r#"{"host":"c"}"#)?;
    store.upsert(&other_tenant, "connections", "connection-d", r#"{"host":"d"}"#)?;

    let rows = store.list(&tenant, "connections")?;
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].config_key, "connection-a");
    assert_eq!(rows[0].payload_json, r#"{"host":"a"}"#);
    assert_eq!(rows[1].config_key, "connection-b");
    assert_eq!(rows[1].payload_json, r#"{"host":"b"}"#);

    assert!(store.delete(&tenant, "connections", "connection-a")?);
    assert!(!store.delete(&tenant, "connections", "connection-a")?);
    assert_eq!(store.get(&tenant, "connections", "connection-a")?, None);
    assert!(store.get(&tenant, "other", "connection-c")?.is_some());
    assert!(store.get(&other_tenant, "connections", "connection-d")?.is_some());

    assert!(store.get(&tenant, "", "key").is_err());
    assert!(store.list(&tenant, " ").is_err());
    assert!(store.delete(&tenant, "namespace", "").is_err());
    assert!(store.upsert(&tenant, "namespace", "key", " ").is_err());
    Ok(())
}

fn for_each_gateway_settings_store_provider(
    mut check: impl FnMut(&GatewaySettingsStore) -> Result<()>,
) -> Result<()> {
    let sqlite_home = tempfile::tempdir()?;
    let sqlite_store = GatewaySettingsStore::new(GlobalDatabase::for_home(sqlite_home.path())?)?;
    check(&sqlite_store)?;

    let postgres_fixture = crate::global_database::test_support::postgres_test_database!();
    let postgres_store = GatewaySettingsStore::new(postgres_fixture.database().clone())?;
    check(&postgres_store)?;
    postgres_fixture.cleanup()?;
    Ok(())
}
