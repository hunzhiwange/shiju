// 验证 PostgreSQL 设置存储按租户、命名空间与键隔离增删改查。
use super::*;

use crate::global_database::test_support::PostgresTestDatabase;

#[test]
fn postgres_gateway_settings_crud_isolates_tenant_namespace_and_key() {
    let fixture = crate::global_database::test_support::postgres_test_database_unit!();
    let store =
        PostgresGatewaySettingsStore::new(fixture.database().clone()).expect("settings store");
    let tenant_a = TenantScope::new("platform-a", "company-a").expect("tenant A");
    let tenant_b = TenantScope::new("platform-b", "company-b").expect("tenant B");

    assert_eq!(store.get(&tenant_a, "preferences", "theme").expect("missing setting"), None);
    assert!(store.list(&tenant_a, "preferences").expect("empty list").is_empty());

    store
        .upsert(&tenant_a, "preferences", "theme", r#"{"theme":"dark"}"#, 10)
        .expect("insert tenant A setting");
    store
        .upsert(&tenant_b, "preferences", "theme", r#"{"theme":"light"}"#, 11)
        .expect("insert tenant B setting");
    store
        .upsert(&tenant_a, "projects", "theme", r#"{"sort":"name"}"#, 12)
        .expect("insert other namespace");
    store
        .upsert(&tenant_a, "preferences", "accent", r#"{"accent":"blue"}"#, 13)
        .expect("insert second key");
    store
        .upsert(&tenant_a, "preferences", "theme", r#"{"theme":"contrast"}"#, 14)
        .expect("update tenant A setting");

    assert_eq!(
        store.get(&tenant_a, "preferences", "theme").expect("tenant A setting").as_deref(),
        Some(r#"{"theme":"contrast"}"#)
    );
    assert_eq!(
        store.get(&tenant_b, "preferences", "theme").expect("tenant B setting").as_deref(),
        Some(r#"{"theme":"light"}"#)
    );
    assert_eq!(
        store.get(&tenant_a, "projects", "theme").expect("other namespace setting").as_deref(),
        Some(r#"{"sort":"name"}"#)
    );

    let rows = store.list(&tenant_a, "preferences").expect("list settings");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].config_key, "accent");
    assert_eq!(rows[1].config_key, "theme");

    assert!(store.delete(&tenant_a, "preferences", "theme").expect("delete setting"));
    assert!(!store.delete(&tenant_a, "preferences", "theme").expect("delete missing setting"));
    assert!(store.get(&tenant_b, "preferences", "theme").expect("tenant B survives").is_some());

    fixture.cleanup().expect("clean PostgreSQL test database");
}
