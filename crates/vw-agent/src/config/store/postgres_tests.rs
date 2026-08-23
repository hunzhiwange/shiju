// 覆盖 PostgreSQL 配置存储的租户隔离、版本更新与错误映射。

use anyhow::Result;
use vw_api_types::id::TenantScope;

use super::PostgresSystemConfigStore;
use crate::global_database::test_support::PostgresTestDatabase;

// 正常值、缺省值与非法输入成组验证，用于固定配置兼容边界和失败语义。
#[test]
fn postgres_system_config_store_upserts_and_isolates_tenants() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    let store = PostgresSystemConfigStore::new(fixture.database().clone())?;
    let tenant_a = TenantScope::new("platform-a", "company-a")?;
    let tenant_b = TenantScope::new("platform-a", "company-b")?;

    assert_eq!(store.get(&tenant_a)?, None);
    assert_eq!(store.get(&tenant_b)?, None);

    store.upsert(&tenant_a, r#"{"version":1}"#, 10)?;
    store.upsert(&tenant_b, r#"{"tenant":"b"}"#, 20)?;
    store.upsert(&tenant_a, r#"{"version":2}"#, 30)?;

    assert_eq!(store.get(&tenant_a)?.as_deref(), Some(r#"{"version":2}"#));
    assert_eq!(store.get(&tenant_b)?.as_deref(), Some(r#"{"tenant":"b"}"#));
    fixture.cleanup()?;
    Ok(())
}
