// 验证 PostgreSQL 知识库模式的建表顺序、索引与幂等性。
use super::*;
use crate::global_database::test_support::PostgresTestDatabase;

#[test]
fn initialize_schema_rejects_table_missing_company_scope_without_mutation() {
    let fixture = crate::global_database::test_support::postgres_test_database_unit!();
    let database = fixture.database().clone();
    database
        .connect_postgres()
        .expect("PostgreSQL knowledge connection")
        .batch_execute(
            "CREATE TABLE knowledge_datasets (
                 platform_id TEXT NOT NULL,
                 id TEXT PRIMARY KEY NOT NULL,
                 name TEXT NOT NULL
             );
             INSERT INTO knowledge_datasets (platform_id, id, name)
             VALUES ('legacy-platform', 'legacy-id', 'preserve me');",
        )
        .expect("partially scoped knowledge table");

    let error =
        initialize_schema(&database).expect_err("table missing company scope must be rejected");

    assert_eq!(
        error.to_string(),
        "unsupported unscoped knowledge schema: table knowledge_datasets must contain platform_id and company_id"
    );
    let row = database
        .connect_postgres()
        .expect("unchanged PostgreSQL knowledge connection")
        .query_one("SELECT platform_id, name FROM knowledge_datasets WHERE id = 'legacy-id'", &[])
        .expect("legacy row remains");
    assert_eq!(row.get::<_, String>(0), "legacy-platform");
    assert_eq!(row.get::<_, String>(1), "preserve me");

    fixture.cleanup().expect("clean PostgreSQL test database");
}
