// 覆盖 PostgreSQL 知识存储的构造、事务边界与后端装配。

// 构造器只接受 PostgreSQL 全局数据库，SQLite 后端必须在事务前失败。
use super::*;
use crate::global_database::config::DatabaseProvider;
use crate::global_database::test_support::PostgresTestDatabase;

#[test]
fn new_initializes_store_with_supplied_postgres_database() {
    let fixture = crate::global_database::test_support::postgres_test_database_unit!();
    let database = fixture.database().clone();

    let store = PostgresKnowledgeStore::new(database).expect("PostgreSQL knowledge store");

    assert_eq!(store.database.provider(), DatabaseProvider::Postgres);
    fixture.cleanup().expect("clean PostgreSQL test database");
}

#[test]
fn new_rejects_sqlite_database_without_postgres_schema_transaction() {
    let path = tempfile::tempdir().expect("temporary directory").keep().join("knowledge.sqlite");
    let database = GlobalDatabase::new(path).expect("SQLite global database");

    let error = PostgresKnowledgeStore::new(database)
        .expect_err("PostgreSQL store must reject a SQLite database");

    assert_eq!(error.to_string(), "SQLite global database has no PostgreSQL schema transaction");
}
