// 验证 PostgreSQL 知识缓存的命中、失效与租户隔离。

// 容量为零时只清理当前租户缓存，不能误删其它租户的嵌入结果。
use anyhow::Result;
use vw_api_types::TenantScope;

use super::PostgresKnowledgeStore;
use crate::global_database::test_support::PostgresTestDatabase;

#[test]
fn embedding_cache_round_trips_and_updates_existing_content() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    let store = PostgresKnowledgeStore::new(fixture.database().clone())?;
    let tenant = TenantScope::new("cache-platform", "cache-company")?;

    store.write_embedding_cache(&tenant, "hash", vec![1, 2], 2)?;
    assert_eq!(store.read_embedding_cache(&tenant, "hash")?, Some(vec![1, 2]));

    store.write_embedding_cache(&tenant, "hash", vec![3, 4], 2)?;
    assert_eq!(store.read_embedding_cache(&tenant, "hash")?, Some(vec![3, 4]));

    fixture.cleanup()
}

#[test]
fn zero_sized_embedding_cache_prunes_only_current_tenant() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    let store = PostgresKnowledgeStore::new(fixture.database().clone())?;
    let tenant = TenantScope::new("cache-platform", "cache-primary")?;
    let other = TenantScope::new("cache-platform", "cache-other")?;

    store.write_embedding_cache(&other, "shared-hash", vec![9], 1)?;
    store.write_embedding_cache(&tenant, "shared-hash", vec![1], 0)?;

    assert_eq!(store.read_embedding_cache(&tenant, "shared-hash")?, None);
    assert_eq!(store.read_embedding_cache(&other, "shared-hash")?, Some(vec![9]));

    fixture.cleanup()
}
