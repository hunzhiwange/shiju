//! PostgreSQL 旧入口的类型身份及无网络参数拒绝行为。

#[test]
fn memory_postgres_production_boundary() {
    use crate::memory::postgres::PostgresMemory;

    fn assert_memory<T: memory_types::memory::traits::Memory>() {}
    assert_memory::<PostgresMemory>();
    let constructor: fn(&str, &str, &str, Option<u64>, bool)
        -> anyhow::Result<memory_postgres::memory::postgres::PostgresMemory> =
        PostgresMemory::new;

    for (schema, table, expected) in [
        ("bad-schema", "memories", "storage schema"),
        ("public", "bad-table", "storage table"),
    ] {
        let error = constructor("not a postgres url", schema, table, None, false)
            .err()
            .expect("invalid identifier must be rejected before connecting");
        let message = error.to_string();
        assert!(message.contains(expected), "{message}");
        assert!(message.contains("can only contain ASCII"), "{message}");
    }
}
